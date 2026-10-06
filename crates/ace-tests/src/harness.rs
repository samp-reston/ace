// region: Imports

use ace_client::client::UdsClient;
use ace_client::event::ClientEvent;
use ace_core::Vec;
use ace_server::server::UdsServer;
use ace_sim::bus::SimBus;
use ace_sim::clock::Duration;
use ace_sim::fault::FaultConfig;
use ace_sim::node::SimNodeErased;
use ace_sim::{io::NodeAddress, node::SimRunner};

use crate::fixtures::{TestHandler, TestSecurityProvider};

// endregion: Imports

// region: Addresses

/// Default tester (client) node address.
pub const TESTER_ADDR: NodeAddress = NodeAddress(0x0FF1);

/// Default ECU (server) node address.
pub const ECU_ADDR: NodeAddress = NodeAddress(0x0001);

// endregion: Addresses

// region: DstScenario

/// A wired simulation scenario with one client and one server.
///
/// Constructed with a seed and fault config. The seed makes the entire run reproducible - the same
/// seed and fault config always produce the same sequence of events.
pub struct DstScenario {
    pub runner: SimRunner<SIM_MAX_FRAME, SIM_MAX_OUTBOX>,
    pub server: MyUdsServer,
    pub client: DstScenarioUdsClient,
}

pub type MyUdsServer = UdsServer<
    UDS_MAX_FRAME,
    UDS_MAX_OUTBOX,
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
    TestHandler,
    TestSecurityProvider,
>;

const PENDING: usize = 8;
const SIM_MAX_FRAME: usize = 4096;
const SIM_MAX_OUTBOX: usize = 8;
const MAX_TARGET_EVENTS: usize = 4;
const PERIODIC_DIDS: usize = 2;
const MAX_DATA: usize = 256;
const UDS_MAX_FRAME: usize = 4096;
const UDS_MAX_OUTBOX: usize = 8;
const MAX_SESSIONS: usize = 8;
const MAX_SERVICES: usize = 16;
const MAX_DIDS: usize = 128;
const MAX_SECURITY_LEVELS: usize = 8;
const DEFAULT_S3: u64 = 5_000;
const DEFAULT_P2: u64 = 50;
const DEFAULT_P2_EXT: u64 = 4_000;
const DEFAULT_LOCKOUT: u64 = 10_000;
const DEFAULT_MAX_SECURITY_ATTEMPTS: u8 = 8;
const MAX_SEED: usize = 3;
const MAX_PERIODIC: usize = 16;

pub type DstScenarioUdsClient =
    UdsClient<PENDING, SIM_MAX_FRAME, SIM_MAX_OUTBOX, MAX_TARGET_EVENTS, PERIODIC_DIDS, MAX_DATA>;

impl DstScenario {
    /// Creates a new scenario with the given seed and fault config.
    pub fn new(seed: u64, faults: FaultConfig) -> Self {
        let bus = SimBus::new(seed, faults);
        let runner = SimRunner::new(bus);

        let server = crate::fixtures::server::default_server(ECU_ADDR.clone());
        let client = crate::fixtures::client::default_client(TESTER_ADDR.clone(), ECU_ADDR.clone());

        Self {
            runner,
            server,
            client,
        }
    }

    /// Advances the simulation by one tick of `duration`.
    pub fn tick(&mut self, duration: Duration) {
        self.runner.tick(
            &mut [
                &mut self.server as &mut dyn SimNodeErased<SIM_MAX_FRAME, SIM_MAX_OUTBOX>,
                &mut self.client as &mut dyn SimNodeErased<SIM_MAX_FRAME, SIM_MAX_OUTBOX>,
            ],
            duration,
        );
    }
    /// Ticks `n` times with the given duration.
    pub fn tick_n(&mut self, n: usize, duration: Duration) {
        for _ in 0..n {
            self.tick(duration);
        }
    }

    /// Ticks until the client has no pending requests or `max_ticks` is reached. Returns the
    /// number of ticks taken.
    pub fn tick_until_quiet(&mut self, tick_duration: Duration, max_ticks: usize) -> usize {
        for i in 0..max_ticks {
            if self.client.pending_count() == 0 {
                return i;
            }

            self.tick(tick_duration);
        }

        max_ticks
    }
}

// endregion: DstScenario

// region: Assertion Helpers

/// Drains client events and returns the first `PositiveResponse` for `sid`. Panics if no matching
/// positive response is found.
pub fn expect_positive(client: &mut DstScenarioUdsClient, sid: u8) -> Vec<u8, MAX_DATA> {
    client
        .drain_events()
        .find_map(|e| match e {
            ClientEvent::PositiveResponse { sid: s, data } if s == sid => Some(data),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected positive response for SID 0x{:02X}", sid))
}

/// Drains client events and returns the NRC byte of the first `NegativeResponse` for `sid`. Panics
/// if none is found.
pub fn expect_nrc(client: &mut DstScenarioUdsClient, sid: u8) -> u8 {
    client
        .drain_events()
        .find_map(|e| match e {
            ClientEvent::NegativeResponse { sid: s, nrc } if s == sid => Some(nrc),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected negative response for SID 0x{:02X}", sid))
}

/// Drains client events and asserts a `Timeout` event exists for `sid`. Panics if none found.
pub fn expect_timeout(client: &mut DstScenarioUdsClient, sid: u8) {
    let found = client
        .drain_events()
        .any(|e| matches!(e, ClientEvent::Timeout { sid: s } if s == sid));
    if !found {
        panic!("expected timeout for SID 0x{:02X}", sid);
    }
}

/// Drains client events and returns data from the first `PeriodicData` event for `did`. Panics if
/// none found.
pub fn expect_periodic(client: &mut DstScenarioUdsClient, did: u8) -> Vec<u8, MAX_DATA> {
    client
        .drain_events()
        .find_map(|e| match e {
            ClientEvent::PeriodicData { did: d, data } if d == did => Some(data),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected periodic data for DID 0x{:02X}", did))
}

/// Asserts the server is in the expected session type.
pub fn assert_session(server: &MyUdsServer, expected: u8) {
    assert_eq!(
        server.session_type(),
        expected,
        "expected session 0x{:02X}, got 0x{:02X}",
        expected,
        server.session_type()
    )
}

/// Asserts the server has the expected security level unlocked.
pub fn assert_security(server: &MyUdsServer, expected: u8) {
    assert_eq!(
        server.security_level(),
        expected,
        "expected security level 0x{:02X}, got 0x{:02X}",
        expected,
        server.security_level()
    )
}

// endregion: Assertion Helpers

// region: Seed Ranges
//
// Standard seed ranges used across all DST tests. Run the same property under multiple seeds to
// explore the state space.

/// Seeds for baseline (no fault) runs.
pub const SEEDS_BASELINE: core::ops::Range<u64> = 0..10;

/// Seeds for light fault runs.
pub const SEEDS_LIGHT: core::ops::Range<u64> = 0..50;

/// Seeds for chaos runs.
pub const SEEDS_CHAOS: core::ops::Range<u64> = 0..100;

// endregion: Seed Ranges
