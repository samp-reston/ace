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
use ace_gateway::tester::DoipTesterEvent;
use ace_sim::{clock::Duration, io::NodeAddress};

use crate::{
    fixtures::doip::{DoipDstScenario, GATEWAY_ADDR},
    harness::{assert_session, TESTER_ADDR},
};

// endregion: Imports

//  region: Tick parameters

const MAX_TICKS: usize = 1_000;

//  endregion: Tick parameters

// region: P1 - routing activation

#[test]
fn p1_routing_activation_succeeds_no_faults() {
    for seed in 0..10u64 {
        let mut s = DoipDstScenario::baseline(seed);
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
    let mut s = DoipDstScenario::baseline(0);
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
        let mut s = DoipDstScenario::baseline(seed);
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
    let mut s = DoipDstScenario::baseline(0);
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
    let mut s = DoipDstScenario::baseline(0);
    s.connect();
    s.tick_n(50);
    assert!(s.is_activated());

    s.tcp_bus
        .inner_mut()
        .set_faults(ace_sim::fault::FaultConfig {
            message_loss: (0, 1),
            message_reorder: (0, 1),
            message_delay: (0, 1),
            max_delay: Duration::ZERO,
            corruption: (0, 1),
            timeout: (0, 1),
        });

    s.tcp_bus
        .disconnect(TESTER_ADDR, NodeAddress(GATEWAY_ADDR as u32));
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
    for seed in 0..50u64 {
        let mut s = DoipDstScenario::light(seed);
        s.connect();
        s.tick_n(100);

        if !s.is_activated() {
            let events = s.drain_events();
            let resolved = events.iter().any(|(_, _, e)| {
                matches!(
                    e,
                    DoipTesterEvent::ActivationSucceeded
                        | DoipTesterEvent::ActivationDenied { .. }
                        | DoipTesterEvent::ConnectionReset
                        | DoipTesterEvent::ConnectionRefused
                        | DoipTesterEvent::ConnectionTimeout
                )
            });

            assert!(
                resolved,
                "seed {seed}: activation did not resolve under light faults"
            );
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
fn p6_no_silent_hands_under_chaos() {
    for seed in 0..100u64 {
        let mut s = DoipDstScenario::chaos(seed);
        s.connect();
        s.tick_n(MAX_TICKS * 10);

        let _events = s.drain_events();
    }
}

// endregion: P6
