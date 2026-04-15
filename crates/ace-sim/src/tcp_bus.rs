//! TCP-aware simulation bus.
//!
//! Models a connection-oriented transport between exactly two node addresses. Connection state is
//! owned by the bus - nodes do not track TCP life-cycle. The bus rejects messages between
//! disconnected nodes and can inject TCP-level faults independently of message-level faults.

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
}

impl TcpFaultConfig {
    pub fn none() -> Self {
        Self {
            message: FaultConfig::none(),
            connection_refused: (0, 1),
            connection_reset: (0, 1),
            connection_timeout: (0, 1),
        }
    }

    pub fn light() -> Self {
        Self {
            message: FaultConfig::light(),
            connection_refused: (1, 200),
            connection_reset: (1, 200),
            connection_timeout: (1, 200),
        }
    }

    pub fn chaos() -> Self {
        Self {
            message: FaultConfig::chaos(),
            connection_refused: (1, 20),
            connection_reset: (1, 20),
            connection_timeout: (1, 20),
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
    /// `TcpEvent::ConnectionReset`
    Reset,
}

impl TcpConnectionState {
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
}

// endregion: TcpConnectionState

// region: TcpEvent

/// Events the `TcpSimBus` delivers to nodes alongside messages.
///
/// Nodes train these via `TcpSimBus::drain_events()` after each tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpEvent {
    /// A connection request from `from` to `to` was accepted.
    ConnectionEstablished { from: NodeAddress, to: NodeAddress },

    /// A connection request was refused by the bus fault injector.
    ConnectionRefused { from: NodeAddress, to: NodeAddress },

    /// A connection request timed out
    ConnectionTimeout { from: NodeAddress, to: NodeAddress },

    /// An established connection was reset by the bus fault injector. Both nodes should threat
    /// this as a TCP RST
    ConnectionReset { from: NodeAddress, to: NodeAddress },

    /// The connection was closed cleanly by one of the nodes.
    ConnectionClosed { from: NodeAddress, to: NodeAddress },
}

// endregion: TcpEvent

// region: TcpSimBus

/// A TCP-aware simulation bus.
///
/// Wraps `SimBus` for message delivery and adds connection state tracking and TCP-level fault
/// injection on top. The bus owns the connection state - nodes request connections and observe
/// events but do not track TCP state themselves.
///
/// `N` - max message payload bytes
/// `Q` - max messages in-flight simultaneously
pub struct TcpSimBus<const N: usize, const Q: usize> {
    /// Underlying message bus - handles delivery, delays, and message faults.
    inner: SimBus<N, Q>,

    /// TCP fault configuration.
    tcp_faults: TcpFaultConfig,

    /// Current connection state between node pairs. For simplicity each bus instance models one
    /// logical TCP connection between a tester and a gateway. Multi-connection scenarios use
    /// multiple `TcpSimBus` instances.
    connection: TcpConnectionState,

    /// Connect timeout duration - if a connecting state persists longer than this without the bus
    /// processing a tick, it times out.
    connect_timeout: Duration,

    /// Accumulated TCP events for nodes to drain.
    events: Vec<TcpEvent, 16>,

    /// Dedicated RNG for TCP-level fault decisions, seeded independently from the message-level
    /// RNG so fault regimes can be composed freely.
    rng: Xorshift64,
}

impl<const N: usize, const Q: usize> TcpSimBus<N, Q> {
    /// Creates a new `TcpSimBus`.
    ///
    /// `seed` - seeds both the message bus RNG and the TCP fault RNG. The TCP RNG uses
    /// `seed.wrapping_add(1)` so the two RNGs produce independent sequences from the same seed.
    pub fn new(seed: u64, faults: TcpFaultConfig) -> Self {
        Self {
            inner: SimBus::new(seed, faults.message.clone()),
            tcp_faults: faults,
            connection: TcpConnectionState::Disconnected,
            connect_timeout: Duration::from_millis(5_000),
            events: Vec::new(),
            rng: Xorshift64::new(seed.wrapping_add(1)),
        }
    }

    // region: Connection management

    /// Requests a TCP connection from `from` to `to`.
    ///
    /// The connection may be established, refused, or timeout depending on the `TcpFaultConfig`.
    /// The outcome appears as a `TcpEvent` on the next `tick`.
    pub fn connect(&mut self, from: NodeAddress, to: NodeAddress) {
        if self.connection.is_connected() {
            return;
        }

        let now = self.inner.now();

        // Fault: connection refused
        if self.rng.chance(
            self.tcp_faults.connection_refused.0,
            self.tcp_faults.connection_refused.1,
        ) {
            let _ = self.events.push(TcpEvent::ConnectionRefused {
                from: from.clone(),
                to: to.clone(),
            });
            return;
        }

        // Fault: connection timeout - record connecting state, timeout is check in tick()
        if self.rng.chance(
            self.tcp_faults.connection_timeout.0,
            self.tcp_faults.connection_timeout.1,
        ) {
            self.connection = TcpConnectionState::Connecting {
                initiated_by: from,
                at: now,
            };
            return;
        }

        // Success
        self.connection = TcpConnectionState::Connected {
            initiator: from.clone(),
            acceptor: to.clone(),
        };

        let _ = self
            .events
            .push(TcpEvent::ConnectionEstablished { from, to });
    }

    /// Closes the connection cleanly from the given node.
    pub fn disconnect(&mut self, from: NodeAddress, to: NodeAddress) {
        if self.connection.is_connected() {
            self.connection = TcpConnectionState::Disconnected;
            let _ = self.events.push(TcpEvent::ConnectionClosed { from, to });
        }
    }

    pub fn connection_state(&self) -> &TcpConnectionState {
        &self.connection
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_connected()
    }

    // endregion: Connection management

    // region: Message delivery

    /// Enqueues a message - rejected if the connection is not established.
    ///
    /// Returns `true` if the message was accepted, `false` if rejected due to connection state or
    /// message-level fault injection.
    pub fn send(&mut self, src: NodeAddress, dst: NodeAddress, data: &[u8]) -> bool {
        if !self.connection.is_connected() {
            return false;
        }
        self.inner.send(src, dst, data)
    }

    // endregion: Message delivery

    // region: Tick

    /// Advances simulation time, delivers due messages, and checks connection-level fault
    /// injection.
    pub fn tick(&mut self, duration: Duration) -> Vec<Envelope<N>, Q> {
        let now = self.inner.now();

        // Check connecting timeout
        if let TcpConnectionState::Connecting { initiated_by, at } = &self.connection.clone() {
            let elapsed = now.checked_duration_since(*at).unwrap_or(Duration::ZERO);

            if elapsed >= self.connect_timeout {
                let from = initiated_by.clone();

                self.connection = TcpConnectionState::Disconnected;

                let _ = self.events.push(TcpEvent::ConnectionTimeout {
                    from: from.clone(),
                    to: NodeAddress(0),
                });
            }
        }

        // Check mid-session connection reset fault
        if self.connection.is_connected() {
            if self.rng.chance(
                self.tcp_faults.connection_reset.0,
                self.tcp_faults.connection_reset.1,
            ) {
                if let TcpConnectionState::Connected {
                    initiator,
                    acceptor,
                } = self.connection.clone()
                {
                    self.connection = TcpConnectionState::Reset;

                    let _ = self.events.push(TcpEvent::ConnectionReset {
                        from: initiator,
                        to: acceptor,
                    });
                }
            }
        }

        // If connection was just reset, clear the queue and return nothing
        if matches!(self.connection, TcpConnectionState::Reset) {
            self.connection = TcpConnectionState::Disconnected;

            return Vec::new();
        }

        self.inner.tick(duration)
    }

    // endregion: Tick

    // region: Accessors

    pub fn now(&self) -> Instant {
        self.inner.now()
    }

    /// Drains accumulated TCP events.
    pub fn drain_events(&mut self) -> impl Iterator<Item = TcpEvent> + '_ {
        self.events.drain(..)
    }

    pub fn set_connect_timeout(&mut self, timeout: Duration) {
        self.connect_timeout = timeout;
    }

    pub fn set_faults(&mut self, faults: TcpFaultConfig) {
        self.inner.set_faults(faults.message.clone());
        self.tcp_faults = faults;
    }

    pub fn inner_mut(&mut self) -> &mut SimBus<N, Q> {
        &mut self.inner
    }

    // endregion: Accessors
}

// endregion: TcpSimBus
