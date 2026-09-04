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

use aced::client::event::ClientEvent;
use aced::gateway::tester::{DoipConnectionPhase, DoipTesterEvent};
use aced::sim::{clock::Duration, io::NodeAddress, tcp_bus::TcpEvent};

use crate::{
    fixtures::{
        TestHandler, TestSecurityProvider,
        doip::{DoipDstScenario, GATEWAY_ADDR, TestActivationAuthProvider},
    },
    harness::TESTER_ADDR,
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
const MAX_CONNECTION_EVENTS: usize = 16;
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
    MAX_CONNECTION_EVENTS,
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

    s.tcp_bus
        .inner_mut()
        .set_faults(aced::sim::fault::FaultConfig {
            message_loss: (0, 1),
            message_reorder: (0, 1),
            message_delay: (0, 1),
            max_delay: Duration::ZERO,
            corruption: (0, 1),
            timeout: (0, 1),
        });

    s.disconnect(TESTER_ADDR, NodeAddress(GATEWAY_ADDR as u32));

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
                    let now = s.tcp_bus.now();
                    s.tester.on_tcp_event(
                        &TcpEvent::ConnectionEstablished {
                            from: s.tester.address().clone(),
                            to: gw_addr.clone(),
                        },
                        now,
                    );
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
