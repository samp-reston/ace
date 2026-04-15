//! CAN-aware simulation bus
//!
//! Models a CAN bus between an ISO-TP node and one or more ECU nodes. CAN is connection-less -no
//! session state, no handshake. The bus carries raw CAN frames with CAN-specific fault injection
//! layered on top of the message-level faults from `FaultConfig`.
//!
//! CAN-specific faults:
//!     - Bit error: a transmitted frame is corrupted at the bit level
//!     - Arbitration loss: a frame is silently dropped as if lost to arbitration (another node won
//!     the bus)
//!     - Bus-off: the bus enters an error state and stops delivering frames until reset.

// region: Imports

use heapless::Vec;

use crate::{
    bus::{Envelope, SimBus},
    clock::{Duration, Instant},
    fault::FaultConfig,
    io::NodeAddress,
    rng::{Rng, Xorshift64},
};

// endregion: Imports

// region: CanFaultConfig

// CAN-level fault configuration.
#[derive(Debug, Clone)]
pub struct CanFaultConfig {
    /// Underlying message-level fault config.
    pub message: FaultConfig,

    /// Probability a frame is dropped due to arbitration loss
    pub arbitration_loss: (u32, u32),

    /// Probability a frame triggers a bit error (corruption + retry drop).
    pub bit_error: (u32, u32),

    /// Probability the bus enters bus-off state on any given tick. When bus-off, all frames are
    /// dropped until `reset_bus_off()`.
    pub bus_off: (u32, u32),
}

impl CanFaultConfig {
    pub fn none() -> Self {
        Self {
            message: FaultConfig::none(),
            arbitration_loss: (0, 1),
            bit_error: (0, 1),
            bus_off: (0, 1),
        }
    }

    pub fn light() -> Self {
        Self {
            message: FaultConfig::light(),
            arbitration_loss: (1, 200),
            bit_error: (1, 200),
            bus_off: (1, 200),
        }
    }

    pub fn chaos() -> Self {
        Self {
            message: FaultConfig::chaos(),
            arbitration_loss: (1, 10),
            bit_error: (1, 10),
            bus_off: (1, 10),
        }
    }
}

// endregion: CanFaultConfig

// region: CanBusState

// The operational state of the simulated CAN bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanBusState {
    /// Bus is operational - frames are delivered normally.
    Active,

    /// Bus is in error-passive state - frames may still be delivered but error counts are
    /// elevated.
    ErrorPassive,

    /// Bus-off - no frames are delivered until the bus is reset.
    BusOff { since: Instant },
}

impl CanBusState {
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Active | Self::ErrorPassive)
    }
}

// endregion: CanBusState

// region: CanEvent

/// Events the `CanSimBus` delivers to nodes alongside messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanEvent {
    /// Bus transitioned to bus-off state.
    BusOff,

    /// Bus recovered from bus-off state.
    BusRecovered,

    /// A bit error was detected on a frame from `src`.
    BitError { src: NodeAddress },
}

// endregion: CanEvent

// region: CanSimBus

/// A CAN-aware simulation bus.
///
/// Wraps `SimBus` for message delivery and adds CAN bus state and CAN-specific fault injection.
/// Connection-less - any node may send to any other node as long as the bus is operational.
///
/// `N` - max frame payload bytes (8 for classic CAN, 64 for CAN FD)
/// `Q` - max frames in-flight simultaneously
pub struct CanSimBus<const N: usize, const Q: usize> {
    /// Underlying message bus.
    inner: SimBus<N, Q>,

    /// CAN fault configuration.
    can_faults: CanFaultConfig,

    /// Current bus state.
    bus_state: CanBusState,

    /// Accumulated CAN events for nodes to drain.
    events: Vec<CanEvent, 16>,

    /// Dedicated RNG for CAN-level fault decisions.
    rng: Xorshift64,
}

impl<const N: usize, const Q: usize> CanSimBus<N, Q> {
    /// Creates a new `CanSimBus`.
    ///
    /// `seed` - seeds both the message bus RNG and the CAN fault RNG. The CAN RNG uses
    /// `seed.wrapping_add(2)` for independence.
    pub fn new(seed: u64, faults: CanFaultConfig) -> Self {
        Self {
            inner: SimBus::new(seed, faults.message.clone()),
            can_faults: faults,
            bus_state: CanBusState::Active,
            events: heapless::Vec::new(),
            rng: Xorshift64::new(seed.wrapping_add(2)),
        }
    }

    // region: Message Delivery

    /// Enqueues a CAN frame - rejected if the bus is in bus-off state or CAN-level fault injection
    /// drops it.
    ///
    /// Returns `true` if the frame was accepted.
    pub fn send(&mut self, src: NodeAddress, dst: NodeAddress, data: &[u8]) -> bool {
        if !self.bus_state.is_operational() {
            return false;
        }

        if self.rng.chance(
            self.can_faults.arbitration_loss.0,
            self.can_faults.arbitration_loss.1,
        ) {
            return false;
        }

        if self
            .rng
            .chance(self.can_faults.bit_error.0, self.can_faults.bit_error.1)
        {
            let _ = self.events.push(CanEvent::BitError { src: src.clone() });
            return false;
        }

        self.inner.send(src, dst, data)
    }

    // endregion: Message Delivery

    // region: Bus State Management

    /// Manually triggers bus-off - useful for DST fault injection.
    pub fn trigger_bus_off(&mut self) {
        let now = self.inner.now();

        self.bus_state = CanBusState::BusOff { since: now };

        let _ = self.events.push(CanEvent::BusOff);
    }

    /// Resets the bus from bus-off state back to Active.
    pub fn reset_bus_off(&mut self) {
        if matches!(self.bus_state, CanBusState::BusOff { .. }) {
            self.bus_state = CanBusState::Active;
            let _ = self.events.push(CanEvent::BusRecovered);
        }
    }

    pub fn bus_state(&self) -> &CanBusState {
        &self.bus_state
    }

    // endregion: Bus State Management

    // region: Tick

    /// Advances simulation time, delivers due frames, and checks bus-off fault injection.
    pub fn tick(&mut self, duration: Duration) -> heapless::Vec<Envelope<N>, Q> {
        if self.bus_state.is_operational() {
            if self
                .rng
                .chance(self.can_faults.bus_off.0, self.can_faults.bus_off.1)
            {
                let now = self.inner.now();

                self.bus_state = CanBusState::BusOff { since: now };

                let _ = self.events.push(CanEvent::BusOff);
            }
        }

        if !self.bus_state.is_operational() {
            let _ = self.inner.tick(duration);
            return heapless::Vec::new();
        }

        self.inner.tick(duration)
    }

    // endregion: Tick

    // region: Accessors

    pub fn now(&self) -> Instant {
        self.inner.now()
    }

    /// Drains accumulated CAN events.
    pub fn drain_events(&mut self) -> impl Iterator<Item = CanEvent> + '_ {
        self.events.drain(..)
    }

    pub fn set_faults(&mut self, faults: CanFaultConfig) {
        self.inner.set_faults(faults.message.clone());
        self.can_faults = faults;
    }

    pub fn inner_mut(&mut self) -> &mut SimBus<N, Q> {
        &mut self.inner
    }

    // endregion: Accessors
}

// endregion: CanSimBus
