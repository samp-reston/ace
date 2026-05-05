//! Per-TCP-connection state machine
//!
//! Each TCP connection to the gateway has its own `ConnectionState` which owns the activation line
//! state machine and tracks alive check timing. The gateway creates one `ConnectionState` per
//! accepted TCP connection and drops it when the connection closes.

// region: Imports

use crate::{
    header::PayloadType,
    payload::{DiagnosticNackCode, RoutingActivationRequest, RoutingActivationResponse},
    session::{ActivationAuthProvider, ActivationDenialReason, ActivationStateMachine},
};
use ace_core::FrameRead;
use ace_sim::clock::{Duration, Instant};

// endregion: Imports

// region: ConnectionConfig

/// Per-connection timing configuration.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// How long to wait for an alive check response before considering the connection dead.
    pub alive_check_timeout: Duration,

    /// How long an active connection may be idle before the gateway sends an alive check request.
    pub idle_timeout: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            alive_check_timeout: Duration::from_millis(500),
            idle_timeout: Duration::from_millis(5_000),
        }
    }
}

// endregion: ConnectionConfig

// region: ConnectionPhase

/// The life-cycle phase of a TCP connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionPhase {
    /// TCP connected, waiting for RoutingActivationRequest.
    Connected,

    /// Routing activation complete - diagnostic messages permitted.
    Active,

    /// Gateway sent an AliveCheckRequest, waiting for response.
    AliveCheckPending { sent_at: Instant },

    /// Connection is being torn down.
    Closing,
}

// endregion: ConnectionPhase

// region: ConnectionEvent

/// Events produced by the connection state machine.
///
/// The gateway collects these after each `handle` or `tick` call and acts on them - sending
/// frames, routing messages, closing sockets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionEvent<const BUF: usize = 4096> {
    /// Send this `RoutingActivationResponse` frame back to the tester.
    SendActivationResponse(RoutingActivationResponse),

    /// Forward these raw UDS bytes to the ECU at the given target address.
    ForwardToEcu {
        source_address: u16,
        target_address: u16,
        uds_data: heapless::Vec<u8, BUF>,
    },

    /// Send a `DiagnosticMessageAck` back to the tester.
    SendDiagnosticAck {
        source_address: u16,
        target_address: u16,
    },

    /// Send a `DiagnosticMessageNack` back to the tester.
    SendDiagnosticNack {
        source_address: u16,
        target_address: u16,
        nack_code: DiagnosticNackCode,
    },

    /// Send an `AliveCheckRequest` to the tester.
    SendAliveCheckRequest,

    /// Send an `AliveCheckResponse` back to the tester.
    SendAliveCheckResponse,

    /// Close this TCP connection - activation denied or alive check failed.
    Close,
}

// endregion: ConnectionEvent

// region: ConnectionState

/// State machine for a single TCP connection to the gateway.
///
/// Owns the `ActivationStateMachine` for this connection and tracks idle/alive-check timing. The
/// gateway drives this via `handle_frame` and `tick`.
#[derive(Debug)]
pub struct ConnectionState<A: ActivationAuthProvider, const BUF: usize = 4096> {
    config: ConnectionConfig,
    phase: ConnectionPhase,
    activation: ActivationStateMachine<A>,
    last_rx: Instant,
    events: heapless::Vec<ConnectionEvent<BUF>, 8>,
}

impl<A: ActivationAuthProvider, const BUF: usize> ConnectionState<A, BUF> {
    pub fn new(
        config: ConnectionConfig,
        activation: ActivationStateMachine<A>,
        now: Instant,
    ) -> Self {
        Self {
            config,
            phase: ConnectionPhase::Connected,
            activation,
            last_rx: now,
            events: heapless::Vec::new(),
        }
    }

    // region: Public surface

    pub fn is_active(&self) -> bool {
        self.phase == ConnectionPhase::Active
    }

    pub fn active_source_address(&self) -> Option<u16> {
        self.activation.state.active_source_address()
    }

    /// Processes a validated DoIP payload - raw bytes after the 8-byte header.
    ///
    /// The caller has already validated the DoIP header and decoded the payload type. This method
    /// receives the payload bytes and the already-decoded payload type for dispatch.
    pub fn handle(&mut self, payload_type: &PayloadType, payload_data: &[u8], now: Instant) {
        self.last_rx = now;

        match payload_type {
            PayloadType::RoutingActivationRequest => self.on_routing_activation(payload_data),
            PayloadType::DiagnosticMessage => self.on_diagnostic_message(payload_data),
            PayloadType::AliveCheckRequest => self.on_alive_check_request(),
            PayloadType::AliveCheckResponse => self.on_alive_check_response(),

            // Other payload types - not handled at connection level
            _ => {}
        }
    }

    /// Advances connection timers - idle timeout and alive check timeout.
    pub fn tick(&mut self, now: Instant) {
        match &self.phase {
            ConnectionPhase::Active => {
                let idle = now
                    .checked_duration_since(self.last_rx)
                    .unwrap_or(Duration::ZERO);

                if idle > self.config.idle_timeout {
                    self.phase = ConnectionPhase::AliveCheckPending { sent_at: now };
                    let _ = self.events.push(ConnectionEvent::SendAliveCheckRequest);
                }
            }
            ConnectionPhase::AliveCheckPending { sent_at } => {
                let elapsed = now
                    .checked_duration_since(*sent_at)
                    .unwrap_or(Duration::ZERO);

                if elapsed > self.config.alive_check_timeout {
                    self.phase = ConnectionPhase::Closing;
                    let _ = self.events.push(ConnectionEvent::Close);
                }
            }
            _ => {}
        }
    }

    /// Drops the activation line - models ignition off or power loss mid-session.
    pub fn drop_activation_line(&mut self, reason: ActivationDenialReason) {
        self.activation.drop_line(reason);
        self.phase = ConnectionPhase::Closing;
        let _ = self.events.push(ConnectionEvent::Close);
    }

    /// Drains accumulated connection events.
    pub fn drain_events(&mut self) -> impl Iterator<Item = ConnectionEvent<BUF>> + '_ {
        self.events.drain(..)
    }

    // endregion: Public surface

    // region: Frame handlers

    fn on_routing_activation(&mut self, payload_data: &[u8]) {
        let mut cursor = payload_data;
        let req = match RoutingActivationRequest::decode(&mut cursor) {
            Ok(r) => r,
            Err(_) => {
                // Malformed activation request - close connection
                self.phase = ConnectionPhase::Closing;
                let _ = self.events.push(ConnectionEvent::Close);
                return;
            }
        };

        let resp = self.activation.process_request(&req);

        let _ = self
            .events
            .push(ConnectionEvent::SendActivationResponse(resp));

        if self.activation.state.is_active() {
            self.phase = ConnectionPhase::Active;
        } else {
            self.phase = ConnectionPhase::Closing;
            let _ = self.events.push(ConnectionEvent::Close);
        }
    }

    fn on_diagnostic_message(&mut self, payload_data: &[u8]) {
        let source = u16::from_be_bytes([
            payload_data.get(0).copied().unwrap_or(0),
            payload_data.get(1).copied().unwrap_or(0),
        ]);
        let target = u16::from_be_bytes([
            payload_data.get(2).copied().unwrap_or(0),
            payload_data.get(3).copied().unwrap_or(0),
        ]);

        if !self.is_active() {
            let _ = self.events.push(ConnectionEvent::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::InvalidSourceAddress,
            });
            return;
        }

        if self.activation.state.active_source_address() != Some(source) {
            let _ = self.events.push(ConnectionEvent::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::InvalidSourceAddress,
            });
            return;
        }

        let uds_bytes = payload_data.get(4..).unwrap_or(&[]);

        if uds_bytes.is_empty() {
            let _ = self.events.push(ConnectionEvent::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::TransportProtocolError,
            });
            return;
        }

        let mut uds_data: heapless::Vec<u8, BUF> = heapless::Vec::new();

        if uds_data.extend_from_slice(uds_bytes).is_err() {
            let _ = self.events.push(ConnectionEvent::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::OutOfMemory,
            });
            return;
        }

        let _ = self.events.push(ConnectionEvent::SendDiagnosticAck {
            source_address: source,
            target_address: target,
        });

        let _ = self.events.push(ConnectionEvent::ForwardToEcu {
            source_address: source,
            target_address: target,
            uds_data,
        });
    }

    fn on_alive_check_request(&mut self) {
        let _ = self.events.push(ConnectionEvent::SendAliveCheckResponse);
    }

    fn on_alive_check_response(&mut self) {
        if matches!(self.phase, ConnectionPhase::AliveCheckPending { .. }) {
            self.phase = ConnectionPhase::Active;
        }
    }

    // endregion: Frame handlers
}

// endregion: ConnectionState
