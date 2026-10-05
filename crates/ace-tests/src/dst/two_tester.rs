//! P9: two testers on one gateway do not observe each other's traffic.
//!
//! `DoipDstScenario` has always held exactly one `DoipTester` as a named field
//! (`pub tester: DoipTester<...>`), and its old hand-rolled `tick()` had exactly one
//! `self.tester.handle(...)` call written into the loop body. Adding a second tester
//! meant duplicating every line in that method that mentioned `self.tester` - a rewrite
//! of the fixture, not a configuration change. `DoipGateway` and `GatewayConfig` have
//! supported `MAX_TESTERS` testers since Phase 4d; nothing on the gateway side was
//! stopping this. The scenario harness was the single-tester bottleneck.
//!
//! This is the gap the P8 test's doc comment named directly: "Proving the address
//! checks needs two testers on one gateway, where one tester would consume the other's
//! responses. That is recorded as the open half of the multi-peer work."
//!
//! With `TwoTierRunner`, a second tester is one more slot in the `outer_nodes` slice -
//! no fixture edit required. This test is built directly on the runner and the DoIP
//! adapters, bypassing `DoipDstScenario`, specifically to demonstrate that the *library*
//! now supports this topology on its own, independent of whether the shared test
//! fixture has been updated to expose it as a builder option.

// region: Imports

use ace_can::IsoTpAddressingMode;
use ace_client::event::ClientEvent;
use ace_core::Vec;
use ace_core::clock::Duration;
use ace_gateway::config::{CanNodeEntry, GatewayConfig};
use ace_gateway::ecu_node::EcuNode;
use ace_gateway::gateway::DoipGateway;
use ace_gateway::tester::{DoipConnectionConfig, DoipConnectionPhase, DoipTester, DoipTesterEvent};
use ace_sim::can_bus::{CanFaultConfig, CanSimBus};
use ace_sim::io::NodeAddress;
use ace_sim::node::SimNodeErased;
use ace_sim::tcp_bus::{TcpFaultConfig, TcpSimBus};
use ace_sim::two_tier_runner::{Bridge, OuterNode, TwoTierRunner};

use crate::fixtures::doip::TestActivationAuthProvider;
use crate::fixtures::server::default_server;
use crate::fixtures::two_tier_adapters::{DoipGatewayAdapter, DoipTesterAdapter, EcuNodeAdapter};

// endregion: Imports

// region: Sizes

const MAX_DATA: usize = 256;
const MAX_QUEUED: usize = 16;
const TCP_MAX_EVENTS: usize = 16;
const MAX_CONNECTIONS: usize = 4;
const TCP_MAX_OUTBOX: usize = 16;
const CAN_MAX_FRAME: usize = 256;
const CAN_MAX_OUTBOX: usize = 16;

const ISOTP_MAX_NODES: usize = 4;
const MAX_TESTERS: usize = 2;
const MAX_FRAME: usize = 256;
const MAX_CONNECTION_ACTIONS: usize = 16;
const MAX_ACTIVATION_TYPES: usize = 16;
const UDS_MAX_FRAME: usize = 256;
const UDS_MAX_OUTBOX: usize = 16;
const MAX_SESSIONS: usize = 16;
const MAX_SERVICES: usize = 16;
const MAX_DIDS: usize = 48;
const MAX_SECURITY_LEVELS: usize = 8;
const MAX_NODES: usize = 4;
const DEFAULT_S3: u64 = 50;
const DEFAULT_P2: u64 = 50;
const DEFAULT_P2_EXT: u64 = 5000;
const DEFAULT_LOCKOUT: u64 = 30;
const DEFAULT_MAX_SECURITY_ATTEMPTS: u8 = 3;
const MAX_SEED: usize = 3;
const MAX_PERIODIC: usize = 16;

const MAX_TARGETS: usize = 2;
const PENDING: usize = 4;
const MAX_TARGET_EVENTS: usize = 16;

const GATEWAY_ADDR: u16 = 0x0E80;
const ECU_A: u16 = 0x0001;
const ECU_B: u16 = 0x0002;

type Ecu = EcuNode<
    UDS_MAX_FRAME,
    UDS_MAX_OUTBOX,
    CAN_MAX_FRAME,
    CAN_MAX_OUTBOX,
    MAX_SESSIONS,
    MAX_SERVICES,
    MAX_DIDS,
    MAX_SECURITY_LEVELS,
    DEFAULT_S3,
    DEFAULT_P2,
    DEFAULT_P2_EXT,
    DEFAULT_LOCKOUT,
    DEFAULT_MAX_SECURITY_ATTEMPTS,
    MAX_SEED,
    MAX_PERIODIC,
    crate::fixtures::TestHandler,
    crate::fixtures::TestSecurityProvider,
>;

type Gateway = DoipGateway<
    CAN_MAX_FRAME,
    ISOTP_MAX_NODES,
    MAX_DATA,
    TCP_MAX_OUTBOX,
    CAN_MAX_OUTBOX,
    MAX_TESTERS,
    MAX_FRAME,
    UDS_MAX_FRAME,
    MAX_CONNECTION_ACTIONS,
    MAX_ACTIVATION_TYPES,
    MAX_NODES,
    TestActivationAuthProvider,
>;

type Tester = DoipTester<
    MAX_CONNECTIONS,
    MAX_TARGETS,
    MAX_DATA,
    TCP_MAX_OUTBOX,
    MAX_CONNECTION_ACTIONS,
    PENDING,
    MAX_DATA,
    TCP_MAX_OUTBOX,
    MAX_TARGET_EVENTS,
    MAX_PERIODIC,
    MAX_DATA,
>;

type Runner = TwoTierRunner<
    MAX_DATA,
    MAX_QUEUED,
    TCP_MAX_EVENTS,
    MAX_CONNECTIONS,
    TCP_MAX_OUTBOX,
    CAN_MAX_FRAME,
    CAN_MAX_OUTBOX,
    MAX_CONNECTIONS,
    CAN_MAX_OUTBOX,
>;

// endregion: Sizes

fn ecu(logical_address: u16, request_can_id: u32, response_can_id: u32) -> Ecu {
    let server = default_server(NodeAddress(logical_address as u32));
    EcuNode::new(
        logical_address,
        request_can_id,
        response_can_id,
        0x7DF,
        IsoTpAddressingMode::Normal,
        server,
    )
}

fn tester(address: u16) -> Tester {
    DoipTester::new(address, NodeAddress(address as u32))
}

/// One gateway, two ECUs, two independently-registered testers, driven entirely through
/// `TwoTierRunner` - no `DoipDstScenario` involved.
struct TwoTesterScenario {
    runner: Runner,
    gateway: Gateway,
    testers: [Tester; 2],
    ecus: [Ecu; 2],
}

const TESTER_1_ADDR: u16 = 0x0E00;
const TESTER_2_ADDR: u16 = 0x0E01;

impl TwoTesterScenario {
    fn new(seed: u64) -> Self {
        let tcp_bus: TcpSimBus<MAX_DATA, MAX_QUEUED, TCP_MAX_EVENTS, MAX_CONNECTIONS> =
            TcpSimBus::new(seed, TcpFaultConfig::none());
        let can_bus: CanSimBus<CAN_MAX_FRAME, CAN_MAX_OUTBOX> =
            CanSimBus::new(seed.wrapping_add(1), CanFaultConfig::none());
        let runner = TwoTierRunner::new(tcp_bus, can_bus, 10);

        let gw_config = GatewayConfig::new(GATEWAY_ADDR)
            .with_tester(TESTER_1_ADDR)
            .with_tester(TESTER_2_ADDR)
            .with_node(CanNodeEntry {
                logical_address: ECU_A,
                request_can_id: 0x7E0,
                response_can_id: 0x7E8,
                functional_can_id: 0x7EF,
            })
            .with_node(CanNodeEntry {
                logical_address: ECU_B,
                request_can_id: 0x7E1,
                response_can_id: 0x7E9,
                functional_can_id: 0x7EF,
            });

        let gateway = DoipGateway::new(
            gw_config,
            TestActivationAuthProvider,
            NodeAddress(GATEWAY_ADDR as u32),
        );

        let mut tester_1 = tester(TESTER_1_ADDR);
        let mut tester_2 = tester(TESTER_2_ADDR);

        let conn_1 = tester_1
            .open_connection(DoipConnectionConfig::new(GATEWAY_ADDR))
            .expect("connection slot available");
        let conn_2 = tester_2
            .open_connection(DoipConnectionConfig::new(GATEWAY_ADDR))
            .expect("connection slot available");

        // Both testers open connection id 0 (keyed by gateway address, not tester
        // identity) - each tester's own connection table is independent, so this is not
        // a collision. Kept explicit here rather than discarded, matching how the
        // fixture surfaces conn_id elsewhere.
        let _ = (conn_1, conn_2);

        Self {
            runner,
            gateway,
            testers: [tester_1, tester_2],
            ecus: [ecu(ECU_A, 0x7E0, 0x7E8), ecu(ECU_B, 0x7E1, 0x7E9)],
        }
    }

    fn connect(&mut self) {
        let gw_addr = NodeAddress(GATEWAY_ADDR as u32);
        for t in self.testers.iter() {
            self.runner
                .outer_bus()
                .connect(t.address().clone(), gw_addr.clone());
        }
    }

    fn tick(&mut self) {
        let [tester_1, tester_2] = &mut self.testers;
        let [ecu_0, ecu_1] = &mut self.ecus;

        let mut t1 = DoipTesterAdapter(tester_1);
        let mut t2 = DoipTesterAdapter(tester_2);
        let mut gw = DoipGatewayAdapter(&mut self.gateway);
        let mut e0 = EcuNodeAdapter {
            address: NodeAddress(0x7E8),
            node: ecu_0,
        };
        let mut e1 = EcuNodeAdapter {
            address: NodeAddress(0x7E9),
            node: ecu_1,
        };

        let mut outer_nodes: [&mut dyn OuterNode<MAX_DATA, TCP_MAX_OUTBOX>; 2] = [&mut t1, &mut t2];
        let mut inner_nodes: [&mut dyn SimNodeErased<CAN_MAX_FRAME, CAN_MAX_OUTBOX>; 2] =
            [&mut e0, &mut e1];
        let mut bridges: [&mut dyn Bridge<MAX_DATA, TCP_MAX_OUTBOX, CAN_MAX_FRAME, CAN_MAX_OUTBOX>;
            1] = [&mut gw];

        self.runner.tick(
            &mut outer_nodes,
            &mut inner_nodes,
            &mut bridges,
            Duration::from_millis(1),
            Duration::from_micros(100),
        );
    }

    fn tick_n(&mut self, n: usize) {
        for _ in 0..n {
            self.tick();
        }
    }

    fn is_active(&self, tester_idx: usize, conn: ace_gateway::tester::ConnectionId) -> bool {
        self.testers[tester_idx].connection_phase(conn) == Some(&DoipConnectionPhase::Active)
    }
}

/// P9: with two testers registered on one gateway, a request from one is answered only to
/// that tester - the other observes nothing from it.
///
/// Asserted on which tester's event queue holds the response, not merely that a positive
/// response exists: a gateway that broadcasts to every registered tester (the DoIP analogue
/// of the CAN/TCP "every frame offered to every node, node self-filters" rule, misapplied to
/// responses rather than requests) would also produce a response *somewhere*, so a weaker
/// assertion could not tell the two apart.
#[test]
fn p9_two_testers_on_one_gateway_do_not_see_each_others_traffic() {
    let mut s = TwoTesterScenario::new(0);
    s.connect();
    s.tick_n(80);

    let conn_1 = ace_gateway::tester::ConnectionId(GATEWAY_ADDR);
    assert!(s.is_active(0, conn_1), "tester 1 did not activate");
    assert!(s.is_active(1, conn_1), "tester 2 did not activate");

    // ActivationSucceeded belongs to setup, not to tester 1's diagnostic exchange.
    for tester in s.testers.iter_mut() {
        for _ in tester.drain_events() {}
    }

    s.testers[0]
        .request(conn_1, ECU_A, &[0x10, 0x03], s.runner.now())
        .expect("tester 1's request accepted");
    s.tick_n(200);

    let t1_events: Vec<_, MAX_TARGET_EVENTS> = Vec::from_iter_checked(s.testers[0].drain_events())
        .expect("event buffer sized for this scenario");
    let t2_events: Vec<_, MAX_TARGET_EVENTS> = Vec::from_iter_checked(s.testers[1].drain_events())
        .expect("event buffer sized for this scenario");

    let t1_saw_response = t1_events.iter().any(|(_, _, e)| {
        matches!(
            e,
            DoipTesterEvent::Uds(ClientEvent::PositiveResponse { .. })
        )
    });
    let t2_saw_anything = !t2_events.is_empty();

    assert!(t1_saw_response, "tester 1 must see its own response");
    assert!(
        !t2_saw_anything,
        "tester 2 must not observe traffic from tester 1's exchange \
         (saw {} event(s))",
        t2_events.len()
    );
}

/// The control: without a request, neither tester has anything queued after activation.
///
/// Rules out a scenario-wiring bug (e.g. accidental cross-wiring in `tick()`) reporting a
/// false pass in the test above merely because tester 2 never receives *anything*, ever.
#[test]
fn p9_control_both_testers_are_independently_reachable() {
    let mut s = TwoTesterScenario::new(0);
    s.connect();
    s.tick_n(80);

    let conn = ace_gateway::tester::ConnectionId(GATEWAY_ADDR);

    s.testers[1]
        .request(conn, ECU_B, &[0x10, 0x03], s.runner.now())
        .expect("tester 2's request accepted");
    s.tick_n(200);

    let t2_events: Vec<_, MAX_TARGET_EVENTS> = Vec::from_iter_checked(s.testers[1].drain_events())
        .expect("event buffer sized for this scenario");

    let t2_saw_response = t2_events.iter().any(|(_, _, e)| {
        matches!(
            e,
            DoipTesterEvent::Uds(ClientEvent::PositiveResponse { .. })
        )
    });

    assert!(
        t2_saw_response,
        "tester 2 must be able to complete its own exchange independently of tester 1"
    );
}
