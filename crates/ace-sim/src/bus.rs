// region: Imports

use crate::clock::{Clock, Duration, Instant, SimClock};
use crate::fault::FaultConfig;
use crate::io::NodeAddress;
use crate::rng::{Rng, Xorshift64};
use ace_core::Vec;

// endregion: Imports

// region: Envelope

/// A message in-flight on the simulation bus.
#[derive(Debug, Clone)]
pub struct Envelope<const MAX_DATA: usize> {
    pub src: NodeAddress,
    pub dst: NodeAddress,
    pub data: Vec<u8, MAX_DATA>,
    /// Earliest time this message may be delivered.
    pub deliver_at: Instant,
}

// endregion: Envelope

// region: SimBus

/// The simulation message bus.
///
/// Connects all nodes in the simulation. Drives time forward, delivers messages, and injects
/// faults according to [`FaultConfig`].
///
/// `N` - max message payload bytes
/// `Q` - max messages in-flight simultaneously
#[derive(Debug)]
pub struct SimBus<const MAX_DATA: usize, const MAX_QUEUED: usize> {
    clock: SimClock,
    rng: Xorshift64,
    faults: FaultConfig,
    queue: Vec<Envelope<MAX_DATA>, MAX_QUEUED>,
}

impl<const MAX_DATA: usize, const MAX_QUEUED: usize> SimBus<MAX_DATA, MAX_QUEUED> {
    pub fn new(seed: u64, faults: FaultConfig) -> Self {
        Self {
            clock: SimClock::new(),
            rng: Xorshift64::new(seed),
            faults,
            queue: Vec::new(),
        }
    }

    /// Returns the current simulation time.
    pub fn now(&self) -> Instant {
        self.clock.now()
    }

    /// Advances simulation time by `duration` and returns all messages that are due for delivery
    /// at or before the new time.
    pub fn tick(&mut self, duration: Duration) -> Vec<Envelope<MAX_DATA>, MAX_QUEUED> {
        self.clock.advance(duration);
        let now = self.clock.now();

        let mut delivered = Vec::new();
        let mut remaining = Vec::new();

        for envelope in self.queue.drain(..) {
            if envelope.deliver_at <= now {
                delivered
                    .push(envelope)
                    .expect("delivery buffer is sized to the queue");
            } else {
                remaining
                    .push(envelope)
                    .expect("remaining buffer is sized to the queue");
            }
        }

        if delivered.len() > 1 {
            for i in 0..delivered.len() - 1 {
                if self
                    .rng
                    .chance(self.faults.message_reorder.0, self.faults.message_reorder.1)
                {
                    delivered.swap(i, i + 1);
                }
            }
        }

        self.queue = remaining;
        delivered
    }

    /// Enqueues a message from `src` to `dst` with fault injection applied.
    ///
    /// Returns `true` if the message was enqueued, `false` if it was dropped by fault injection or
    /// the queue is full
    pub fn send(&mut self, src: NodeAddress, dst: NodeAddress, data: &[u8]) -> bool {
        let r = self
            .rng
            .chance(self.faults.message_loss.0, self.faults.message_loss.1);

        if r {
            return false;
        }

        if self
            .rng
            .chance(self.faults.timeout.0, self.faults.timeout.1)
        {
            return false;
        }

        let mut payload = Vec::new();

        for &byte in data {
            if self
                .rng
                .chance(self.faults.corruption.0, self.faults.corruption.1)
            {
                payload
                    .push(byte ^ self.rng.next_u8())
                    .expect("payload buffer matches the source frame");
            } else {
                payload
                    .push(byte)
                    .expect("payload buffer matches the source frame");
            }
        }

        let deliver_at = if self
            .rng
            .chance(self.faults.message_delay.0, self.faults.message_delay.1)
        {
            let delay_us = self.rng.next_u64() % self.faults.max_delay.as_micros().max(1);
            self.clock.now() + Duration::from_micros(delay_us)
        } else {
            self.clock.now()
        };

        let envelope = Envelope {
            src,
            dst,
            data: payload,
            deliver_at,
        };

        if self.queue.len() >= MAX_QUEUED {
            return false;
        }

        let _ = self.queue.push(envelope);

        true
    }

    /// Returns a reference to the current fault config
    pub fn faults(&self) -> &FaultConfig {
        &self.faults
    }

    /// Returns the fault config - allows escalating fault severity during a simulation run.
    pub fn set_faults(&mut self, faults: FaultConfig) {
        self.faults = faults;
    }

    /// Returns the RNG seed-derived next value - useful for injecting spontaneous NRCs at the node
    /// level.
    pub fn next_u8(&mut self) -> u8 {
        self.rng.next_u8()
    }

    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.rng.chance(numerator, denominator)
    }
}

// endregion: SimBus
