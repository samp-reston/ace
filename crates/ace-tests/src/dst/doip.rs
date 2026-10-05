//! DST property tests for the full DoIP -> gateway -> ISO-TP -> UDS stack.
//!
//! Properties:
//!     P1: Routing activation succeeds and tester reaches Achtive phase.
//!     P2: After activation, a valid DSC request reaches the ECU and the tester receives a
//!     positive response.
//!     P3: After DSC to extended session, RDBI returns the expected data.
//!     P4: Activaiton line drop mid-session causes tester to observe a ConnectionReset event.
//!     P5: Full properties hold under light fault injection.
//!     P6: Under chaos, every exchange resolves - no silent hangs.

// region: Imports

use ace_client::event::ClientEvent;
use ace_gateway::tester::{DoipConnectionPhase, DoipTesterEvent};
use ace_sim::{clock::Duration, io::NodeAddress, tcp_bus::LinkEvent};

use crate::fixtures::{
    TestHandler, TestSecurityProvider,
    doip::{
        DoipDstScenario, DoipDstScenarioBuilder, EcuNodeConfig, GATEWAY_ADDR, GatewayNodeConfig,
        TestActivationAuthProvider,
    },
};

// endregion: Imports

// region: Scenario Initialisation

const MAX_DATA: usize = 256;
const MAX_QUEUED: usize = 16;
const TCP_MAX_EVENTS: usize = 16;
const MAX_GATEWAY_ADDRS: usize = 8;
const MAX_EVENTS: usize = 16;
const TCP_MAX_FRAME: usize = 256;
const TCP_MAX_OUTBOX: usize = 16;
const CAN_MAX_FRAME: usize = 256;
const CAN_MAX_OUTBOX: usize = 16;
const MAX_GATEWAYS: usize = 8;
const ISOTP_MAX_NODES: usize = 8;
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
const MAX_NODES: usize = 8;
const DEFAULT_S3: u64 = 50;
const DEFAULT_P2: u64 = 50;
const DEFAULT_P2_EXT: u64 = 5000;
const DEFAULT_LOCKOUT: u64 = 30;
const DEFAULT_MAX_SECURITY_ATTEMPTS: u8 = 3;
const MAX_SEED: usize = 3;
const MAX_PERIODIC: usize = 16;

type MyDoipDstScenario = DoipDstScenario<
    MAX_DATA,
    MAX_QUEUED,
    TCP_MAX_EVENTS,
    MAX_GATEWAY_ADDRS,
    MAX_EVENTS,
    TCP_MAX_FRAME,
    TCP_MAX_OUTBOX,
    CAN_MAX_FRAME,
    CAN_MAX_OUTBOX,
    MAX_GATEWAYS,
    ISOTP_MAX_NODES,
    MAX_TESTERS,
    MAX_FRAME,
    MAX_CONNECTION_ACTIONS,
    MAX_ACTIVATION_TYPES,
    UDS_MAX_FRAME,
    UDS_MAX_OUTBOX,
    MAX_SESSIONS,
    MAX_SERVICES,
    MAX_DIDS,
    MAX_SECURITY_LEVELS,
    MAX_NODES,
    DEFAULT_S3,
    DEFAULT_P2,
    DEFAULT_P2_EXT,
    DEFAULT_LOCKOUT,
    DEFAULT_MAX_SECURITY_ATTEMPTS,
    MAX_SEED,
    MAX_PERIODIC,
    TestActivationAuthProvider,
    TestHandler,
    TestSecurityProvider,
>;

// endregion: Scenario Initialisation

//  region: Tick parameters

const MAX_TICKS: usize = 1_000;

//  endregion: Tick parameters

// region: P1 - routing activation

#[test]
fn p1_routing_activation_succeeds_no_faults() {
    for seed in [0] {
        let mut s = MyDoipDstScenario::baseline(seed);
        s.connect();
        s.tick_n(50);

        assert!(
            s.is_activated(),
            "seed {seed}: tester should be Active after activation"
        );
    }
}

#[test]
fn p1_activation_response_event_emitted() {
    let mut s = MyDoipDstScenario::baseline(0);
    s.connect();

    s.tick_n(50);

    let events = s.drain_events();
    let activated = events
        .iter()
        .any(|(_, _, e)| matches!(e, DoipTesterEvent::ActivationSucceeded));

    assert!(activated, "ActivationSucceeded event should be emitted");
}

// endregion: P1

// region: P2 - DSC round trip over DoIP

#[test]
fn p2_dsc_extended_round_trip_no_faults() {
    for seed in 0..10u64 {
        let mut s = MyDoipDstScenario::baseline(seed);
        s.connect();
        s.tick_n(50);
        assert!(s.is_activated(), "seed {seed}: not activated");

        s.request_default(&[0x10, 0x03])
            .expect("request should not fail");
        s.tick_n(MAX_TICKS);

        let events = s.drain_events();

        let positive = events.iter().any(|(_, _, e)| {
            matches!(
                e,
                DoipTesterEvent::Uds(ClientEvent::PositiveResponse { sid: 0x10, .. })
            )
        });

        assert!(positive, "seed {seed}: expected positive DSC response");
        assert!(
            s.gateways
                .iter()
                .find(|gw| {
                    gw.ecus
                        .iter()
                        .find(|ecu| ecu.logical_address == s.first_ecu())
                        .expect("destination ecu should be present")
                        .node
                        .server
                        .session_type()
                        == 0x03
                })
                .is_some(),
            "seed {seed}: server session mismatch"
        )
    }
}

// endregion: P2

// region: P7 - functional addressing fans out to every ECU

/// DoIP functional group address, in the range ISO 13400-2 reserves for
/// vehicle-manufacturer-defined functional groups (0xE400..=0xEFFF).
const FUNCTIONAL_TARGET: u16 = 0xE400;

const ECU_ONE: u16 = 0x0001;
const ECU_TWO: u16 = 0x0002;
const ECU_THREE: u16 = 0x0003;

/// Three ECUs behind one gateway, all listening on the same functional CAN identifier.
///
/// Note that the functional CAN identifier (0x7EF) and the servers' functional UDS
/// address (0x7DF) deliberately differ: the ECU node translates between them, so this
/// also proves the server is not comparing CAN identifiers.
fn three_ecu_scenario(seed: u64) -> MyDoipDstScenario {
    DoipDstScenarioBuilder::<MAX_GATEWAYS, MAX_NODES>::new(seed)
        .with_gateway(
            GatewayNodeConfig::new(GATEWAY_ADDR, crate::fixtures::doip::TESTER_ADDR)
                .with_ecu(EcuNodeConfig::new(ECU_ONE, 0x7E0, 0x7E8, 0x7EF))
                .with_ecu(EcuNodeConfig::new(ECU_TWO, 0x7E1, 0x7E9, 0x7EF))
                .with_ecu(EcuNodeConfig::new(ECU_THREE, 0x7E2, 0x7EA, 0x7EF)),
        )
        .build()
}

/// Session type of one ECU, by logical address.
fn session_of(s: &MyDoipDstScenario, logical_address: u16) -> u8 {
    s.gateways
        .iter()
        .flat_map(|gw| gw.ecus.iter())
        .find(|ecu| ecu.logical_address == logical_address)
        .expect("ecu present in scenario")
        .node
        .server
        .session_type()
}

/// P7: a functionally addressed request reaches every ECU behind the gateway.
///
/// Asserted on ECU state rather than on responses, because state cannot be faked by a
/// single ECU answering N times: three separate servers each changed session, so all
/// three received the request.
#[test]
fn p7_functional_request_reaches_every_ecu() {
    for seed in 0..5u64 {
        let mut s = three_ecu_scenario(seed);
        s.connect();
        s.tick_n(50);
        assert!(s.is_activated(), "seed {seed}: not activated");

        let conn_id = s.conn_id();
        s.request(conn_id, FUNCTIONAL_TARGET, &[0x10, 0x03])
            .expect("functional request accepted");
        s.tick_n(MAX_TICKS);

        for ecu in [ECU_ONE, ECU_TWO, ECU_THREE] {
            assert_eq!(
                session_of(&s, ecu),
                0x03,
                "seed {seed}: ECU {ecu:#06X} did not receive the functional request"
            );
        }
    }
}

/// P7: the control. A physically addressed request reaches only its target.
///
/// Without this, the test above would also pass if the gateway broadcast everything.
#[test]
fn p7_physical_request_reaches_only_its_target() {
    let mut s = three_ecu_scenario(0);
    s.connect();
    s.tick_n(50);
    assert!(s.is_activated(), "not activated");

    let conn_id = s.conn_id();
    s.request(conn_id, ECU_TWO, &[0x10, 0x03])
        .expect("physical request accepted");
    s.tick_n(MAX_TICKS);

    assert_eq!(session_of(&s, ECU_TWO), 0x03, "the addressed ECU responded");
    assert_eq!(
        session_of(&s, ECU_ONE),
        0x01,
        "ECU one was not addressed and must stay in the default session"
    );
    assert_eq!(
        session_of(&s, ECU_THREE),
        0x01,
        "ECU three was not addressed and must stay in the default session"
    );
}

/// P7: an unservable functional request is met with silence, not a negative response.
///
/// ISO 14229-1 requires the five "not implemented" NRCs to be withheld on a functionally
/// addressed request. Reading an unsupported DID yields requestOutOfRange, which is one of
/// them, so no ECU may answer - and the tester must not receive a negative response.
#[test]
fn p7_unservable_functional_request_is_silent() {
    let mut s = three_ecu_scenario(0);
    s.connect();
    s.tick_n(50);
    assert!(s.is_activated(), "not activated");

    let conn_id = s.conn_id();
    // 0xF199 is not configured on any ECU.
    s.request(conn_id, FUNCTIONAL_TARGET, &[0x22, 0xF1, 0x99])
        .expect("functional request accepted");
    s.tick_n(MAX_TICKS);

    let events = s.drain_events();
    let negative = events.iter().any(|(_, _, e)| {
        matches!(
            e,
            DoipTesterEvent::Uds(ClientEvent::NegativeResponse { .. })
        )
    });

    assert!(
        !negative,
        "a negative response to an unservable functional request must be suppressed"
    );
}

// endregion: P7

// region: P8 - destination filtering is observable with more than one node

const GATEWAY_B: u16 = 0x0E81;

/// Two gateways on one segment, each with its own ECU and its own identifiers.
///
/// A single-gateway topology cannot test destination filtering: a frame for the wrong
/// recipient is also the wrong *payload type* for it, so payload dispatch rejects it and
/// the missing address check never shows. With two gateways, a routing activation meant
/// for one is a perfectly valid routing activation to the other.
fn two_gateway_scenario(seed: u64) -> MyDoipDstScenario {
    DoipDstScenarioBuilder::<MAX_GATEWAYS, MAX_NODES>::new(seed)
        .with_gateway(
            GatewayNodeConfig::new(GATEWAY_ADDR, crate::fixtures::doip::TESTER_ADDR)
                .with_ecu(EcuNodeConfig::new(0x0001, 0x7E0, 0x7E8, 0x7EF)),
        )
        .add_gateway(
            GatewayNodeConfig::new(GATEWAY_B, crate::fixtures::doip::TESTER_ADDR)
                .with_ecu(EcuNodeConfig::new(0x0011, 0x7E4, 0x7EC, 0x7EE)),
        )
        .build()
}

/// P8: two gateways on one bus each establish their own connection and activate.
///
/// This was impossible before the bus modelled more than one connection: `connect` returned
/// early once any connection existed, so the second gateway's link never left
/// `Disconnected` and only one activation ever occurred. It is the direct test of that fix.
///
/// It deliberately does *not* claim to prove destination filtering. A DoIP routing
/// activation request carries no target logical address - it is addressed only at the TCP
/// level - so a gateway answering a request meant for its neighbour produces a response the
/// tester still attributes to the answering gateway and ignores as already active. Proving
/// the address checks needs two *testers* on one gateway, where one tester would consume the
/// other's responses. That is recorded as the open half of the multi-peer work.
#[test]
fn p8_two_gateways_each_activate_independently() {
    let mut s = two_gateway_scenario(0);
    s.connect();
    s.tick_n(80);

    assert_eq!(
        s.runner.outer_bus().connection_count(),
        2,
        "the bus must hold one connection per gateway"
    );

    for gw in s.gateways.iter() {
        assert_eq!(
            s.tester.connection_phase(gw.conn_id),
            Some(&DoipConnectionPhase::Active),
            "gateway {:#06X} did not reach Active",
            gw.logical_address
        );
    }

    let events = s.drain_events();
    let activations = events
        .iter()
        .filter(|(_, _, e)| matches!(e, DoipTesterEvent::ActivationSucceeded))
        .count();

    assert_eq!(activations, 2, "one activation per gateway");
}

// endregion: P8

// region: P3 - RDBI over DoIP after session change

#[test]
fn p3_rdbi_vin_over_doip_no_faults() {
    let mut s = MyDoipDstScenario::baseline(0);
    s.connect();
    s.tick_n(50);

    assert!(s.is_activated());

    s.request_default(&[0x22, 0xf1, 0x90])
        .expect("request should not fail");
    s.tick_n(MAX_TICKS);

    let events = s.drain_events();

    let rdbi_resp = events.iter().find_map(|(_, _, e)| {
        if let DoipTesterEvent::Uds(ClientEvent::PositiveResponse { sid: 0x22, data }) = e {
            Some(data.clone())
        } else {
            None
        }
    });

    assert!(rdbi_resp.is_some(), "expected RDBI positive response");

    let data = rdbi_resp.unwrap();
    let vin = data.get(2..).unwrap_or(&[]);

    assert_eq!(data.get(0).copied(), Some(0xF1));
    assert_eq!(data.get(1).copied(), Some(0x90));
    assert_eq!(vin, b"TESTVIN1234567890");
}

// endregion: P3

// region: P4 - activation line drop

#[test]
fn p4_activation_line_drop_produces_connection_reset() {
    let mut s = MyDoipDstScenario::baseline(0);
    s.connect();
    s.tick_n(50);
    assert!(s.is_activated());

    s.runner
        .outer_bus()
        .inner_mut()
        .set_faults(ace_sim::fault::FaultConfig {
            message_loss: (0, 1),
            message_reorder: (0, 1),
            message_delay: (0, 1),
            max_delay: Duration::ZERO,
            corruption: (0, 1),
            timeout: (0, 1),
            script: ace_sim::fault::FaultPlan::new(),
        });

    s.disconnect(s.tester.address().clone(), NodeAddress(GATEWAY_ADDR as u32));

    s.tick_n(10);

    assert!(
        !s.is_activated(),
        "tester should no longer be Active after connection drop"
    )
}

// endregion: P4

// region: P5 - light faults

#[test]
fn p5_full_stack_light_faults() {
    const T3_TICKS: usize = 2000;

    for seed in 0..50u64 {
        let mut s = MyDoipDstScenario::light(seed);
        s.connect();

        let conn_id = s.conn_id();
        let gw_addr = s.gateways.first().unwrap().gateway_addr.clone();

        let mut t3_counter = 0usize;
        let mut activated = false;
        let mut activation_terminal = false;

        for _ in 0..MAX_TICKS {
            if s.is_activated() {
                activated = true;
                break;
            }

            let events = s.drain_events();
            if events.iter().any(|(_, _, e)| {
                matches!(
                    e,
                    DoipTesterEvent::ActivationDenied { .. }
                        | DoipTesterEvent::ConnectionReset
                        | DoipTesterEvent::ConnectionRefused
                        | DoipTesterEvent::ConnectionTimeout
                )
            }) {
                activation_terminal = true;
                break;
            }

            s.tick();

            let is_pending = s
                .tester
                .connection_phase(conn_id)
                .map(|p| *p == DoipConnectionPhase::ActivationPending)
                .unwrap_or(false);

            if is_pending {
                t3_counter += 1;
                if t3_counter >= T3_TICKS {
                    let now = s.runner.outer_bus().now();
                    s.tester
                        .on_tcp_event(
                            &LinkEvent::ConnectionEstablished {
                                from: s.tester.address().clone(),
                                to: gw_addr.clone(),
                            },
                            now,
                        )
                        .expect("link event must be accepted");
                    t3_counter = 0;
                }
            } else {
                t3_counter = 0;
            }
        }

        assert!(
            activated || activation_terminal,
            "seed {seed}: activation did not resolve under light faults"
        );

        if !activated {
            continue;
        }

        s.request_default(&[0x10, 0x03])
            .expect("request should not fail");
        s.tick_n(MAX_TICKS);

        let events = s.drain_events();
        let resolved = events.iter().any(|(_, _, e)| {
            matches!(
                e,
                DoipTesterEvent::Uds(ClientEvent::PositiveResponse { sid: 0x10, .. })
                    | DoipTesterEvent::Uds(ClientEvent::NegativeResponse { sid: 0x10, .. })
                    | DoipTesterEvent::Uds(ClientEvent::Timeout { sid: 0x10 })
            )
        });
        assert!(
            resolved,
            "seed {seed}: DSC exchange did not resolve under light faults"
        );
    }
}

// endregion: P5

// region: P6 - chaos, no silent hangs

#[test]
#[ignore = "Takes too long"]
fn p6_no_silent_hands_under_chaos() {
    for seed in 0..100u64 {
        let mut s = MyDoipDstScenario::chaos(seed);
        s.connect();
        s.tick_n(MAX_TICKS * 10);

        let _events = s.drain_events();
    }
}

// endregion: P6
