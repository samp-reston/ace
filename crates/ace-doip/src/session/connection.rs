//! Per-TCP-connection state machine
//!
//! Each TCP connection to the gateway has its own `ConnectionState` which owns the activation line
//! state machine and tracks alive check timing. The gateway creates one `ConnectionState` per
//! accepted TCP connection and drops it when the connection closes.

// region: Imports

use crate::error::DoipError;
use crate::{
    header::PayloadType,
    payload::{DiagnosticNackCode, RoutingActivationRequest, RoutingActivationResponse},
    session::{ActivationAuthProvider, ActivationDenialReason, ActivationStateMachine},
};
use ace_core::FrameRead;
use ace_core::clock::{Duration, Instant};

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

// region: ConnectionAction

/// Events produced by the connection state machine.
///
/// The gateway collects these after each `handle` or `tick` call and acts on them - sending
/// frames, routing messages, closing sockets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionAction<const UDS_MAX_FRAME: usize> {
    /// Send this `RoutingActivationResponse` frame back to the tester.
    SendActivationResponse(RoutingActivationResponse),

    /// Forward these raw UDS bytes to the ECU at the given target address.
    ForwardToEcu {
        source_address: u16,
        target_address: u16,
        uds_data: heapless::Vec<u8, UDS_MAX_FRAME>,
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

// endregion: ConnectionAction

// region: ConnectionState

/// State machine for a single TCP connection to the gateway.
///
/// Owns the `ActivationStateMachine` for this connection and tracks idle/alive-check timing. The
/// gateway drives this via `handle_frame` and `tick`.
#[derive(Debug)]
pub struct ConnectionState<
    // region: ConnectionState Constants
    const UDS_MAX_FRAME: usize,
    const CONNECTION_MAX_ACTIONS: usize,
    // endregion: ConnectionState Constants

    // region: ActivationState Constants
    const MAX_TESTERS: usize,
    const MAX_ACTIVATION_TYPES: usize,
    // endregion: ActivationState Constants
    A: ActivationAuthProvider,
> {
    config: ConnectionConfig,
    phase: ConnectionPhase,
    activation: ActivationStateMachine<MAX_TESTERS, MAX_ACTIVATION_TYPES, A>,
    last_rx: Instant,
    actions: heapless::Vec<ConnectionAction<UDS_MAX_FRAME>, CONNECTION_MAX_ACTIONS>,
}

impl<
    const UDS_MAX_FRAME: usize,
    const CONNECTION_MAX_ACTIONS: usize,
    const MAX_TESTERS: usize,
    const MAX_ACTIVATION_TYPES: usize,
    A: ActivationAuthProvider,
> ConnectionState<UDS_MAX_FRAME, CONNECTION_MAX_ACTIONS, MAX_TESTERS, MAX_ACTIVATION_TYPES, A>
{
    pub fn new(
        config: ConnectionConfig,
        activation: ActivationStateMachine<MAX_TESTERS, MAX_ACTIVATION_TYPES, A>,
        now: Instant,
    ) -> Self {
        Self {
            config,
            phase: ConnectionPhase::Connected,
            activation,
            last_rx: now,
            actions: heapless::Vec::new(),
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
    pub fn handle(
        &mut self,
        payload_type: &PayloadType,
        payload_data: &[u8],
        now: Instant,
    ) -> Result<(), DoipError> {
        self.last_rx = now;

        match payload_type {
            PayloadType::RoutingActivationRequest => self.on_routing_activation(payload_data),
            PayloadType::DiagnosticMessage => self.on_diagnostic_message(payload_data),
            PayloadType::AliveCheckRequest => self.on_alive_check_request(),
            PayloadType::AliveCheckResponse => {
                self.on_alive_check_response();
                Ok(())
            }

            // Other payload types - not handled at connection level
            _ => Ok(()),
        }
    }

    /// Queues an action for the caller to perform.
    ///
    /// Fails rather than dropping. Every variant of [`ConnectionAction`] is something
    /// the caller *must* do for the connection to stay in step - close it, ack it,
    /// nack it, forward it - so a silently discarded action desynchronises the caller
    /// from this state machine with no signal.
    fn emit(&mut self, action: ConnectionAction<UDS_MAX_FRAME>) -> Result<(), DoipError> {
        self.actions
            .push(action)
            .map_err(|_| DoipError::ActionQueueFull)
    }

    /// Advances connection timers - idle timeout and alive check timeout.
    pub fn tick(&mut self, now: Instant) -> Result<(), DoipError> {
        match &self.phase {
            ConnectionPhase::Active => {
                let idle = now
                    .checked_duration_since(self.last_rx)
                    .unwrap_or(Duration::ZERO);

                if idle > self.config.idle_timeout {
                    self.phase = ConnectionPhase::AliveCheckPending { sent_at: now };
                    self.emit(ConnectionAction::SendAliveCheckRequest)?;
                }
            }
            ConnectionPhase::AliveCheckPending { sent_at } => {
                let elapsed = now
                    .checked_duration_since(*sent_at)
                    .unwrap_or(Duration::ZERO);

                if elapsed > self.config.alive_check_timeout {
                    self.phase = ConnectionPhase::Closing;
                    self.emit(ConnectionAction::Close)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Drops the activation line - models ignition off or power loss mid-session.
    pub fn drop_activation_line(
        &mut self,
        reason: ActivationDenialReason,
    ) -> Result<(), DoipError> {
        self.activation.drop_line(reason);
        self.phase = ConnectionPhase::Closing;
        self.emit(ConnectionAction::Close)
    }

    /// Drains accumulated actions for the caller to perform.
    pub fn drain_actions(&mut self) -> impl Iterator<Item = ConnectionAction<UDS_MAX_FRAME>> + '_ {
        self.actions.drain(..)
    }

    // endregion: Public surface

    // region: Frame handlers

    fn on_routing_activation(&mut self, payload_data: &[u8]) -> Result<(), DoipError> {
        let mut cursor = payload_data;
        let req = match RoutingActivationRequest::decode(&mut cursor) {
            Ok(r) => r,
            Err(_) => {
                // Malformed activation request - close connection
                self.phase = ConnectionPhase::Closing;
                return self.emit(ConnectionAction::Close);
            }
        };

        let resp = self.activation.process_request(&req);

        self.emit(ConnectionAction::SendActivationResponse(resp))?;

        if self.activation.state.is_active() {
            self.phase = ConnectionPhase::Active;
            Ok(())
        } else {
            self.phase = ConnectionPhase::Closing;
            self.emit(ConnectionAction::Close)
        }
    }

    fn on_diagnostic_message(&mut self, payload_data: &[u8]) -> Result<(), DoipError> {
        let source = u16::from_be_bytes([
            payload_data.first().copied().unwrap_or(0),
            payload_data.get(1).copied().unwrap_or(0),
        ]);
        let target = u16::from_be_bytes([
            payload_data.get(2).copied().unwrap_or(0),
            payload_data.get(3).copied().unwrap_or(0),
        ]);

        if !self.is_active() {
            return self.emit(ConnectionAction::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::InvalidSourceAddress,
            });
        }

        if self.activation.state.active_source_address() != Some(source) {
            return self.emit(ConnectionAction::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::InvalidSourceAddress,
            });
        }

        let uds_bytes = payload_data.get(4..).unwrap_or(&[]);

        if uds_bytes.is_empty() {
            return self.emit(ConnectionAction::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::TransportProtocolError,
            });
        }

        let mut uds_data: heapless::Vec<u8, UDS_MAX_FRAME> = heapless::Vec::new();

        if uds_data.extend_from_slice(uds_bytes).is_err() {
            return self.emit(ConnectionAction::SendDiagnosticNack {
                source_address: source,
                target_address: target,
                nack_code: DiagnosticNackCode::OutOfMemory,
            });
        }

        // Deliberately does NOT acknowledge here.
        //
        // A DoIP acknowledgement states the message was accepted for processing, which
        // is only known once whoever handles `ForwardToEcu` has established it can route
        // the request. Emitting the ack first meant a tester was told its request had
        // been accepted and could then be refused for the same request - and on a
        // functionally addressed request it claimed every ECU had received something
        // most of them had not. The ack is now the caller's answer to the forward.
        self.emit(ConnectionAction::ForwardToEcu {
            source_address: source,
            target_address: target,
            uds_data,
        })
    }

    fn on_alive_check_request(&mut self) -> Result<(), DoipError> {
        self.emit(ConnectionAction::SendAliveCheckResponse)
    }

    fn on_alive_check_response(&mut self) {
        if matches!(self.phase, ConnectionPhase::AliveCheckPending { .. }) {
            self.phase = ConnectionPhase::Active;
        }
    }

    // endregion: Frame handlers
}

// endregion: ConnectionState
