// region: Imports

use ace_can::IsoTpAddressingMode;
use ace_doip::{header::ProtocolVersion, payload::ActivationType, session::ConnectionConfig};

// endregion: Imports

// region: CanNodeEntry

/// Maps a DoIP logical address to a CAN node on the vehicle bus.
///
/// The gateway uses this table to route `DiagnosticMessage` frames from a tester to the correct
/// CAN node and back.
#[derive(Debug, Clone)]
pub struct CanNodeEntry {
    /// DoIP logical address of the ECU - used by the tester to address it.
    pub logical_address: u16,

    /// Physical CAN request ID - used for targeted (physical) addressing.
    pub request_can_id: u32,

    /// Physical CAN response ID - the ECU responds on this ID.
    pub response_can_id: u32,

    /// Functional CAN ID - used for broadcast/functional addressing.
    pub functional_can_id: u32,
}

// endregion: CanNodeEntry

// region: GatewayConfig

/// Configuration for the DoIP gateway.
///
/// Defines the gateway's own logical address, the set of ECU nodes it routes to, the registered
/// tester addresses it accepts activations from, and the activation types it supports.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// DoIP logical address of this gateway entity.
    pub logical_address: u16,

    /// DoIP Protocol Version to use for frame assembly.
    pub protocol_version: ProtocolVersion,

    /// ECU nodes reachable through this gateway.
    pub nodes: heapless::Vec<CanNodeEntry, 16>,

    /// Tester logical addresses that may activate routing on this gateway. A
    /// `RoutingActivationRequest` from an address not in this list is denied with
    /// `DeniedUnknownSourceAddress`
    pub registered_testers: heapless::Vec<u16, 16>,

    /// Activation types this gateway supports. A request for an unsupported type is denied with
    /// `DeniedUnsupportedRoutingActivationType`
    pub supported_activation_types: heapless::Vec<ActivationType, 4>,

    /// ISO-TP addressing mode for CAN communication.
    pub isotp_addressing_mode: IsoTpAddressingMode,

    /// Per-connection timing - alive check and idle timeout.
    pub connection_config: ConnectionConfig,
}

impl GatewayConfig {
    pub fn new(logical_address: u16) -> Self {
        let mut supported = heapless::Vec::new();
        let _ = supported.push(ActivationType::Default);

        Self {
            logical_address,
            protocol_version: ProtocolVersion::Iso13400_2012,
            nodes: heapless::Vec::new(),
            registered_testers: heapless::Vec::new(),
            supported_activation_types: supported,
            isotp_addressing_mode: IsoTpAddressingMode::Normal,
            connection_config: ConnectionConfig::default(),
        }
    }

    pub fn with_node(mut self, node: CanNodeEntry) -> Self {
        let _ = self.nodes.push(node);
        self
    }

    pub fn with_tester(mut self, address: u16) -> Self {
        let _ = self.registered_testers.push(address);
        self
    }

    pub fn with_activation_type(mut self, activation_type: ActivationType) -> Self {
        let _ = self.supported_activation_types.push(activation_type);
        self
    }

    pub fn with_isotp_address(mut self, mode: IsoTpAddressingMode) -> Self {
        self.isotp_addressing_mode = mode;
        self
    }

    pub fn find_node(&self, logical_address: u16) -> Option<&CanNodeEntry> {
        self.nodes
            .iter()
            .find(|n| n.logical_address == logical_address)
    }
}

// endregion: GatewayConfig
