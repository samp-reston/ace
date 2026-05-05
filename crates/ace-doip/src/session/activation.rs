//! Routing Activation state machine - one instance per TCP connection.
//!
//! Regulation defines the activation sequence:
//!     1. Tester sends RoutingActivationRequest
//!     2. Gateway validates source address, activation type, auth data
//!     3. Gateway sends RoutingActivationResponse
//!     4. If successful, connection transitions to Active
//!
//! The activation line maps onto this state machine - the connection only enters Active after the
//! gateway confirms activation. The ActivationAuthProvider hook allows OEM-specific authentication
//! for CentralSecurity activation type.

// region: Imports

use ace_proto::doip::constants::{
    DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN, DOIP_ROUTING_ACTIVATION_REQ_OEM_LEN,
};

use crate::payload::{
    ActivationCode, ActivationType, RoutingActivationRequest, RoutingActivationResponse,
};

// endregion: Imports

// region: ActivationDenialReason

/// Reasons the gateway may deny a routing activation request.
///
/// Maps directly onto ActivationCode denial values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationDenialReason {
    /// Source address not registered with this gateway.
    UnknownSourceAddress,

    /// No TCP socket slots available.
    TcpSocketsFull,

    /// This TCP socket already has an active routing activation.
    AlreadyConnected,

    /// The source address is already active on another socket.
    SourceAlreadyActive,

    /// CentralSecurity activation type requires authentication data.
    MissingAuthentication,

    /// Authentication data was rejected.
    RejectedConfirmation,

    /// Activation type not supported by this gateway configuration.
    UnsupportedActivationType,

    /// Gateway requires TLS - unencrypted connection rejected.
    RequiresTls,
}
impl From<ActivationDenialReason> for ActivationCode {
    fn from(value: ActivationDenialReason) -> Self {
        match value {
            ActivationDenialReason::RejectedConfirmation => Self::DeniedRejectedConfirmation,
            ActivationDenialReason::MissingAuthentication => Self::DeniedMissingAuthentication,
            ActivationDenialReason::UnsupportedActivationType => {
                Self::DeniedUnsupportedRoutingActivationType
            }
            ActivationDenialReason::RequiresTls => Self::DeniedRequestEncryptedTlsConnection,
            ActivationDenialReason::SourceAlreadyActive => Self::DeniedSourceIsAlreadyActive,
            ActivationDenialReason::TcpSocketsFull => Self::DeniedTcpSocketsFull,
            ActivationDenialReason::AlreadyConnected => Self::DeniedTcpSocketAlreadyConnected,
            ActivationDenialReason::UnknownSourceAddress => Self::DeniedUnknownSourceAddress,
        }
    }
}

// endregion: ActivationDenialReason

// region: ActivationAuthProvider

/// Hook for OEM-specific routing activation authentication.
///
/// Called by the activation state machine when a `CentralSecurity` (0xFF) activation request is
/// received. The implementation validates the OEM-specific data in the request buffer and returns
/// either `Ok(())` to allow activation or `Err(reason)` to deny it.
///
/// For `Default` (0x00) and `WwhObd` (0x01) activation types the gateway does not call this hook -
/// activation is granted based on source address validity alone.
///
/// # Simulation
///
/// The test implementation should use the `ace-sim` seeded RNG so authentication outcomes are
/// reproducible across simulation runs.
pub trait ActivationAuthProvider {
    /// Validates OEM-specific authentication data for a CentralSecurity activation request.
    ///
    /// `source_address` - the logical address of the tester
    /// `oem_data` - the 4-byte ISO reserved / OEM buffer from the request
    fn authenticate(
        &mut self,
        source_address: u16,
        oem_data: &[u8],
    ) -> Result<(), ActivationDenialReason>;
}

// region: AlwaysAllow

/// An `ActivationAuthProvider` that always grants CentralSecurity activation. Suitable for testing
/// and development environments only.
pub struct AlwaysAllow;

impl ActivationAuthProvider for AlwaysAllow {
    fn authenticate(
        &mut self,
        _source_address: u16,
        _oem_data: &[u8],
    ) -> Result<(), ActivationDenialReason> {
        Ok(())
    }
}

// endregion: AlwaysAllow

// region: AlwaysDeny

/// An `ActivationAuthProvider` that always denies CentralSecurity activation. Useful for testing
/// denial paths.
pub struct AlwaysDeny {
    pub reason: ActivationDenialReason,
}

impl ActivationAuthProvider for AlwaysDeny {
    fn authenticate(
        &mut self,
        _source_address: u16,
        _oem_data: &[u8],
    ) -> Result<(), ActivationDenialReason> {
        Err(self.reason.clone())
    }
}

// endregion: AlwaysDeny

// endregion: ActivationAuthProvider

// region: ActivationLineState

/// The state of the logical activation line for a single TCP connection.
///
/// In vehicle terms this maps to the hardware activation line state. The line must be in `Active`
/// before `DiagnosticMessage` frames are forwarded to ECU nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationLineState {
    /// No routing activation has been attempted on this connection.
    Idle,

    /// A `RoutingActivationRequest` was received and is being processed.
    Pending {
        source_address: u16,
        activation_type: ActivationType,
    },

    /// Routing activation succeeded - diagnostic messages permitted.
    Active {
        /// Logical address of the activated tester.
        source_address: u16,

        /// Activation type that was used to activate this connection.
        activation_type: ActivationType,
    },

    /// Routing activation was denies or the line was dropped. The connection should be closed.
    Deactivated { reason: ActivationDenialReason },
}

impl ActivationLineState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn active_source_address(&self) -> Option<u16> {
        match self {
            Self::Active { source_address, .. } => Some(*source_address),
            _ => None,
        }
    }
}

// endregion: ActivationLineState

// region: ActivationStateMachine

/// Per-connection routing activation state machine.
///
/// The gateway creates one instance per TCP connection. The state machine processes
/// `RoutingActivationRequest` frames and produces `RoutingActivationResponse` frames. It enforces
/// the activation line state - only `Active` connections may carry `DiagnosticMessage` frames.
#[derive(Debug)]
pub struct ActivationStateMachine<A: ActivationAuthProvider> {
    /// Logical address of this gateway entity.
    gateway_address: u16,

    /// Set of registered tester source addresses this gateway recognises. In a real gateway this
    /// is provisioned at build time.
    registered_addresses: heapless::Vec<u16, 16>,

    /// Supported activation types for this gateway configuration.
    supported_types: heapless::Vec<ActivationType, 4>,

    /// OEM authentication provider - called for CentralSecurity activations.
    auth: A,

    /// Current activation line state for this connection.
    pub state: ActivationLineState,
}

impl<A: ActivationAuthProvider> ActivationStateMachine<A> {
    pub fn new(
        gateway_address: u16,
        registered_addresses: heapless::Vec<u16, 16>,
        supported_types: heapless::Vec<ActivationType, 4>,
        auth: A,
    ) -> Self {
        Self {
            gateway_address,
            registered_addresses,
            supported_types,
            auth,
            state: ActivationLineState::Idle,
        }
    }

    // region: Request processing

    /// Processes a `RoutingActivationRequest` and returns the response to send back to the tester.
    ///
    /// Transitions the activation line state based on the outcome. The caller is responsible for
    /// encoding the response into a DoIP frame.
    pub fn process_request(&mut self, req: &RoutingActivationRequest) -> RoutingActivationResponse {
        let source_address = u16::from_be_bytes(req.source_address);
        let activation_type = req.activation_type.clone();

        if !self.registered_addresses.contains(&source_address) {
            return self.deny(source_address, ActivationDenialReason::UnknownSourceAddress);
        }

        if !self.supported_types.contains(&activation_type) {
            return self.deny(
                source_address,
                ActivationDenialReason::UnsupportedActivationType,
            );
        }

        if self.state.is_active() {
            return self.deny(source_address, ActivationDenialReason::AlreadyConnected);
        }

        if activation_type == ActivationType::CentralSecurity {
            let mut oem_data =
                [0u8; DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN + DOIP_ROUTING_ACTIVATION_REQ_OEM_LEN];

            let (iso, oem) = oem_data.split_at_mut(DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN);
            iso.copy_from_slice(&req.reserved);
            oem.copy_from_slice(&req.reserved_for_oem);

            match self.auth.authenticate(source_address, &oem_data) {
                Ok(()) => {}
                Err(reason) => return self.deny(source_address, reason),
            }
        }

        self.state = ActivationLineState::Active {
            source_address,
            activation_type,
        };

        RoutingActivationResponse {
            logical_address: req.source_address,
            source_address: self.gateway_address.to_be_bytes(),
            activation_code: ActivationCode::SuccessfullyActivated,
            reserved: [0u8; 4],
            reserved_for_oem: None,
        }
    }

    // endregion: Request processing

    // region: Line drop

    /// Drops the activation line - models ignition off or power loss.
    ///
    /// Transitions to `Deactivated`. The gateway could close the TCP connection after calling
    /// this.
    pub fn drop_line(&mut self, reason: ActivationDenialReason) {
        self.state = ActivationLineState::Deactivated { reason };
    }

    // endregion: Line drop

    // region: Helpers

    fn deny(
        &mut self,
        source_address: u16,
        reason: ActivationDenialReason,
    ) -> RoutingActivationResponse {
        let code = ActivationCode::from(reason.clone());

        self.state = ActivationLineState::Deactivated { reason };

        RoutingActivationResponse {
            logical_address: source_address.to_be_bytes(),
            source_address: self.gateway_address.to_be_bytes(),
            activation_code: code,
            reserved: [0u8; 4],
            reserved_for_oem: None,
        }
    }

    // endregion: Helpers
}

// endregion: ActivationStateMachine
