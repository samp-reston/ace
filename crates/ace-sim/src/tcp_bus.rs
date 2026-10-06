//! TCP-aware simulation bus.
//!
//! Models a connection-oriented transport between exactly two node addresses. Connection state is
//! owned by the bus - nodes do not track TCP life-cycle. The bus rejects messages between
//! disconnected nodes and can inject TCP-level faults independently of message-level faults.

// region: Imports

use crate::{
    bus::{Envelope, SimBus},
    clock::{Duration, Instant},
    fault::FaultConfig,
    io::NodeAddress,
    rng::{Rng, Xorshift64},
};
use ace_core::Vec;

// endregion: Imports

// region: TcpFaultConfig

/// TCP-level fault configuration - extends `FaultConfig` with connection-oriented faults.
///
/// Applied independently from the message-level faults in `FaultConfig`. This allows modeling a
/// reliable TCP connection with unreliable message delivery, or a flaky TCP connection that resets
/// mid-session.
#[derive(Debug, Clone)]
pub struct TcpFaultConfig {
    /// Underlying message-level fault config.
    pub message: FaultConfig,

    /// Probability that a new connection attempt is refused. Models a gateway that is at capacity
    /// or unreachable.
    pub connection_refused: (u32, u32),

    /// Probability that an established connection is reset mid-session. Models ignition off,
    /// network fault, or gateway restart.
    pub connection_reset: (u32, u32),

    /// Probability that a connection attempt times out without response.
    pub connection_timeout: (u32, u32),

    /// Nominal link bit rate, in bits per second.
    ///
    /// Determines the baseline transmission time added to every segment before any
    /// fault-injected delay. See `CanFaultConfig::bit_rate` for why a baseline of zero was a
    /// defect rather than a simplification.
    pub bit_rate: u32,
}

impl TcpFaultConfig {
    /// 100 Mbit/s - the common DoIP in-vehicle Ethernet default.
    const DEFAULT_BIT_RATE: u32 = 100_000_000;

    pub fn none() -> Self {
        Self {
            message: FaultConfig::none(),
            connection_refused: (0, 1),
            connection_reset: (0, 1),
            connection_timeout: (0, 1),
            bit_rate: Self::DEFAULT_BIT_RATE,
        }
    }

    pub fn light() -> Self {
        Self {
            message: FaultConfig::light(),
            connection_refused: (1, 200),
            connection_reset: (1, 200),
            connection_timeout: (1, 200),
            bit_rate: Self::DEFAULT_BIT_RATE,
        }
    }

    pub fn chaos() -> Self {
        Self {
            message: FaultConfig::chaos(),
            connection_refused: (1, 20),
            connection_reset: (1, 20),
            connection_timeout: (1, 20),
            bit_rate: Self::DEFAULT_BIT_RATE,
        }
    }
}

// endregion: TcpFaultConfig

// region: TcpConnectionState

/// The state of a TCP connection between two nodes on the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpConnectionState {
    /// No connection exists - messages are rejected.
    Disconnected,

    /// A connection request is in progress.
    Connecting {
        initiated_by: NodeAddress,
        at: Instant,
    },

    /// Connection is established - messages flow normally.
    Connected {
        initiator: NodeAddress,
        acceptor: NodeAddress,
    },

    /// Connection was reset by the bus fault injector. Nodes will observe this as a
    /// `LinkEvent::ConnectionReset`
    Reset,
}

impl TcpConnectionState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
}

// endregion: TcpConnectionState

// region: LinkEvent

// Connection lifecycle events are a foundation abstraction and live in
// `ace-core::io`: a real socket reports the same transitions this bus
// synthesises from its fault injector.
pub use ace_core::io::LinkEvent;

// endregion: LinkEvent

// region: TcpSimBus

/// A TCP-aware simulation bus.
///
/// Wraps `SimBus` for message delivery and adds connection state tracking and TCP-level fault
/// injection on top. The bus owns the connection state - nodes request connections and observe
/// events but do not track TCP state themselves.
///
/// `N` - max message payload bytes
/// `Q` - max messages in-flight simultaneously
/// One simulated TCP connection between a pair of nodes.
///
/// The pair is stored alongside the state so a connection can be found in *any* state.
/// Previously the peer was recoverable only once connected, which is why a connect
/// timeout reported its peer as address zero.
#[derive(Debug, Clone)]
struct TcpLink {
    initiator: NodeAddress,
    acceptor: NodeAddress,
    state: TcpConnectionState,
}

impl TcpLink {
    /// Whether this link joins the two given nodes, in either direction.
    ///
    /// Direction-insensitive because once established, either end may send.
    fn joins(&self, a: &NodeAddress, b: &NodeAddress) -> bool {
        (self.initiator == *a && self.acceptor == *b)
            || (self.initiator == *b && self.acceptor == *a)
    }
}

#[derive(Debug)]
pub struct TcpSimBus<
    const MAX_DATA: usize,
    const MAX_QUEUED: usize,
    const TCP_MAX_EVENTS: usize,
    const MAX_CONNECTIONS: usize,
> {
    /// Underlying message bus - handles delivery, delays, and message faults.
    inner: SimBus<MAX_DATA, MAX_QUEUED>,

    /// TCP fault configuration.
    tcp_faults: TcpFaultConfig,

    /// Every connection currently known to the bus, bounded by `MAX_CONNECTIONS`.
    ///
    /// This used to be a single `TcpConnectionState`, on the reasoning that multi-connection
    /// scenarios would use several bus instances. Nothing did: one bus carried every node,
    /// so a second connection was silently refused and any topology with two peers was
    /// unreachable. A tester talking to two gateways is ordinary DoIP, so the bus models it.
    links: Vec<TcpLink, MAX_CONNECTIONS>,

    /// Connect timeout duration - if a connecting state persists longer than this without the bus
    /// processing a tick, it times out.
    connect_timeout: Duration,

    /// Accumulated TCP events for nodes to drain.
    events: Vec<LinkEvent, TCP_MAX_EVENTS>,

    /// Dedicated RNG for TCP-level fault decisions, seeded independently from the message-level
    /// RNG so fault regimes can be composed freely.
    rng: Xorshift64,
}

impl<
    const MAX_DATA: usize,
    const MAX_QUEUED: usize,
    const TCP_MAX_EVENTS: usize,
    const MAX_CONNECTIONS: usize,
> TcpSimBus<MAX_DATA, MAX_QUEUED, TCP_MAX_EVENTS, MAX_CONNECTIONS>
{
    /// Creates a new `TcpSimBus`.
    ///
    /// `seed` - seeds both the message bus RNG and the TCP fault RNG. The TCP RNG uses
    /// `seed.wrapping_add(1)` so the two RNGs produce independent sequences from the same seed.
    pub fn new(seed: u64, faults: TcpFaultConfig) -> Self {
        Self {
            inner: SimBus::new(seed, faults.message.clone()),
            tcp_faults: faults,
            links: Vec::new(),
            connect_timeout: Duration::from_millis(5_000),
            events: Vec::new(),
            rng: Xorshift64::new(seed.wrapping_add(1)),
        }
    }

    // region: Connection management

    /// Requests a TCP connection from `from` to `to`.
    ///
    /// The connection may be established, refused, or time out depending on the
    /// `TcpFaultConfig`. The outcome appears as a `LinkEvent` on the next `tick`.
    ///
    /// Connecting an already-connected pair is a no-op, so repeated calls are safe.
    pub fn connect(&mut self, from: NodeAddress, to: NodeAddress) {
        if let Some(existing) = self.links.iter().position(|l| l.joins(&from, &to)) {
            if self.links[existing].state.is_connected() {
                return;
            }

            // The pair is known but not up - free its slot rather than leaking one per
            // attempt, which would exhaust the table on a reconnecting link.
            let _ = existing;
            self.links.retain(|l| !l.joins(&from, &to));
        }

        if self.links.is_full() {
            // A server out of connection resources refuses the connection. Reporting this
            // as a refusal rather than dropping it silently is what lets a consumer see
            // that it has outgrown the bus it configured.
            self.emit(LinkEvent::ConnectionRefused { from, to });
            return;
        }

        let now = self.inner.now();

        // Fault: connection refused
        if self.rng.chance(
            self.tcp_faults.connection_refused.0,
            self.tcp_faults.connection_refused.1,
        ) {
            self.emit(LinkEvent::ConnectionRefused { from, to });
            return;
        }

        // Fault: connection timeout - recorded as connecting, expired in `tick`.
        if self.rng.chance(
            self.tcp_faults.connection_timeout.0,
            self.tcp_faults.connection_timeout.1,
        ) {
            let _ = self.links.push(TcpLink {
                initiator: from.clone(),
                acceptor: to,
                state: TcpConnectionState::Connecting {
                    initiated_by: from,
                    at: now,
                },
            });
            return;
        }

        let _ = self.links.push(TcpLink {
            initiator: from.clone(),
            acceptor: to.clone(),
            state: TcpConnectionState::Connected {
                initiator: from.clone(),
                acceptor: to.clone(),
            },
        });

        let _ = self
            .events
            .push(LinkEvent::ConnectionEstablished { from, to });
    }

    /// Closes a connection cleanly.
    pub fn disconnect(&mut self, from: NodeAddress, to: NodeAddress) {
        let Some(idx) = self.links.iter().position(|l| l.joins(&from, &to)) else {
            return;
        };

        if !self.links[idx].state.is_connected() {
            return;
        }

        self.links.retain(|l| !l.joins(&from, &to));
        self.emit(LinkEvent::ConnectionClosed { from, to });
    }

    /// State of the connection joining two nodes, if the bus knows of one.
    pub fn connection_state(
        &self,
        a: &NodeAddress,
        b: &NodeAddress,
    ) -> Option<&TcpConnectionState> {
        self.links.iter().find(|l| l.joins(a, b)).map(|l| &l.state)
    }

    /// Whether the two nodes have an established connection.
    pub fn is_connected(&self, a: &NodeAddress, b: &NodeAddress) -> bool {
        self.links
            .iter()
            .any(|l| l.joins(a, b) && l.state.is_connected())
    }

    /// Whether any connection at all is established.
    pub fn any_connected(&self) -> bool {
        self.links.iter().any(|l| l.state.is_connected())
    }

    /// How many connections the bus is currently tracking.
    pub fn connection_count(&self) -> usize {
        self.links.len()
    }

    // endregion: Connection management

    // region: Message delivery

    /// Enqueues a message - rejected unless the two nodes have an established connection.
    ///
    /// Returns `true` if the message was accepted, `false` if rejected due to connection
    /// state or message-level fault injection.
    pub fn send(&mut self, src: NodeAddress, dst: NodeAddress, data: &[u8]) -> bool {
        if !self.is_connected(&src, &dst) {
            return false;
        }

        self.inner
            .send_with_min_latency(src, dst, data, self.transmission_time(data.len()))
    }

    /// Time to put a segment of `payload_len` bytes on the link at the configured bit rate.
    ///
    /// Includes a fixed 54-byte allowance for Ethernet + IP + TCP headers on top of the
    /// payload, so a single-byte DoIP message is not modelled as free.
    fn transmission_time(&self, payload_len: usize) -> Duration {
        const HEADER_OVERHEAD_BYTES: u64 = 54;

        let bits = (HEADER_OVERHEAD_BYTES + payload_len as u64) * 8;
        let bit_rate = self.tcp_faults.bit_rate.max(1) as u64;

        Duration::from_micros(bits.saturating_mul(1_000_000) / bit_rate)
    }

    // endregion: Message delivery

    // region: Tick

    /// Advances simulation time, delivers due messages, and applies connection-level fault
    /// injection to each connection independently.
    pub fn tick(&mut self, duration: Duration) -> Vec<Envelope<MAX_DATA>, MAX_QUEUED> {
        let now = self.inner.now();

        // Expire connect attempts. The peer is known now that the link stores the pair, so
        // the timeout names who it was trying to reach.
        let mut timed_out: Vec<(NodeAddress, NodeAddress), MAX_CONNECTIONS> = Vec::new();
        for link in self.links.iter() {
            if let TcpConnectionState::Connecting { at, .. } = &link.state {
                let elapsed = now.checked_duration_since(*at).unwrap_or(Duration::ZERO);

                if elapsed >= self.connect_timeout {
                    let _ = timed_out.push((link.initiator.clone(), link.acceptor.clone()));
                }
            }
        }

        for (from, to) in timed_out {
            self.links.retain(|l| !l.joins(&from, &to));
            self.emit(LinkEvent::ConnectionTimeout { from, to });
        }

        // Mid-session reset, rolled per connection so one link's fault does not decide
        // another's.
        let mut reset: Vec<(NodeAddress, NodeAddress), MAX_CONNECTIONS> = Vec::new();
        for idx in 0..self.links.len() {
            if !self.links[idx].state.is_connected() {
                continue;
            }

            if self.rng.chance(
                self.tcp_faults.connection_reset.0,
                self.tcp_faults.connection_reset.1,
            ) {
                let _ = reset.push((
                    self.links[idx].initiator.clone(),
                    self.links[idx].acceptor.clone(),
                ));
            }
        }

        for (from, to) in reset {
            self.links.retain(|l| !l.joins(&from, &to));
            self.emit(LinkEvent::ConnectionReset { from, to });
        }

        // Messages are only delivered while their own connection is up.
        //
        // A reset used to discard the bus queue wholesale, which was indistinguishable from
        // correct while one connection existed and would now drop unrelated traffic. Judging
        // each envelope against its own pair drops exactly what a reset should: anything
        // still in flight for a connection that has since gone away.
        let delivered = self.inner.tick(duration);
        let mut kept: Vec<Envelope<MAX_DATA>, MAX_QUEUED> = Vec::new();

        for envelope in delivered {
            if self.is_connected(&envelope.src, &envelope.dst) {
                let _ = kept.push(envelope);
            }
        }

        kept
    }

    // endregion: Tick

    // region: Accessors

    pub fn now(&self) -> Instant {
        self.inner.now()
    }

    /// Drains accumulated TCP events.
    /// Records a link event for the runner to drain.
    ///
    /// Panics when the queue is full. This is harness code, and the queue is sized
    /// to what the runner drains each tick, so overflow means the scenario outgrew
    /// its configuration. Dropping the event instead would let the run complete -
    /// just not the run that was asked for, with no signal that it diverged.
    fn emit(&mut self, event: LinkEvent) {
        self.events
            .push(event)
            .expect("link event queue overflowed, which would alter the run");
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = LinkEvent> + '_ {
        self.events.drain(..)
    }

    pub fn set_connect_timeout(&mut self, timeout: Duration) {
        self.connect_timeout = timeout;
    }

    pub fn set_faults(&mut self, faults: TcpFaultConfig) {
        self.inner.set_faults(faults.message.clone());
        self.tcp_faults = faults;
    }

    pub fn inner_mut(&mut self) -> &mut SimBus<MAX_DATA, MAX_QUEUED> {
        &mut self.inner
    }

    // endregion: Accessors
}

// endregion: TcpSimBus
