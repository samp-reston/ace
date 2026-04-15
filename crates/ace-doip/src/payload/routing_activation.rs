use crate::error::DoipError;
use ace_macros::FrameCodec;
use ace_proto::doip::constants::{
    DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN, DOIP_ROUTING_ACTIVATION_REQ_OEM_LEN,
    DOIP_ROUTING_ACTIVATION_REQ_SRC_LEN, DOIP_ROUTING_ACTIVATION_RES_ENTITY_LEN,
    DOIP_ROUTING_ACTIVATION_RES_ISO_LEN, DOIP_ROUTING_ACTIVATION_RES_TESTER_LEN,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct RoutingActivationRequest {
    pub source_address: [u8; DOIP_ROUTING_ACTIVATION_REQ_SRC_LEN],
    pub activation_type: ActivationType,
    pub reserved: [u8; DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN],
    pub reserved_for_oem: [u8; DOIP_ROUTING_ACTIVATION_REQ_OEM_LEN],
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum ActivationType {
    #[frame(id = 0x00)]
    Default,
    #[frame(id = 0x01)]
    WwhObd,
    #[frame(id_pat = "0x02..=0xDF")]
    Reserved(u8),
    #[frame(id = 0xE0)]
    CentralSecurity,
    #[frame(id_pat = "0xE1..=0xFF")]
    ReservedForOem(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct RoutingActivationResponse {
    pub logical_address: [u8; DOIP_ROUTING_ACTIVATION_RES_TESTER_LEN],
    pub source_address: [u8; DOIP_ROUTING_ACTIVATION_RES_ENTITY_LEN],
    pub activation_code: ActivationCode,
    pub reserved: [u8; DOIP_ROUTING_ACTIVATION_RES_ISO_LEN],
    pub reserved_for_oem: Option<[u8; DOIP_ROUTING_ACTIVATION_REQ_OEM_LEN]>,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum ActivationCode {
    #[frame(id = 0x00)]
    DeniedUnknownSourceAddress,
    #[frame(id = 0x01)]
    DeniedTcpSocketsFull,
    #[frame(id = 0x02)]
    DeniedTcpSocketAlreadyConnected,
    #[frame(id = 0x03)]
    DeniedSourceIsAlreadyActive,
    #[frame(id = 0x04)]
    DeniedMissingAuthentication,
    #[frame(id = 0x05)]
    DeniedRejectedConfirmation,
    #[frame(id = 0x06)]
    DeniedUnsupportedRoutingActivationType,
    #[frame(id = 0x07)]
    DeniedRequestEncryptedTlsConnection,
    #[frame(id = 0x08)]
    DeniedVehicleInCriticalState,
    #[frame(id_pat = "0x09..=0x0F | 0x12..=0xDF | 0xFF")]
    Reserved(u8),
    #[frame(id = 0x10)]
    SuccessfullyActivated,
    #[frame(id = 0x11)]
    ActivatedConfirmationRequired,
    #[frame(id_pat = "0xE0..=0xFE")]
    ReservedForOem(u8),
}

impl From<ActivationCode> for u8 {
    fn from(value: ActivationCode) -> Self {
        match value {
            ActivationCode::DeniedUnknownSourceAddress => 0x00,
            ActivationCode::DeniedTcpSocketsFull => 0x01,
            ActivationCode::DeniedTcpSocketAlreadyConnected => 0x02,
            ActivationCode::DeniedSourceIsAlreadyActive => 0x03,
            ActivationCode::DeniedMissingAuthentication => 0x04,
            ActivationCode::DeniedRejectedConfirmation => 0x05,
            ActivationCode::DeniedUnsupportedRoutingActivationType => 0x06,
            ActivationCode::DeniedRequestEncryptedTlsConnection => 0x07,
            ActivationCode::DeniedVehicleInCriticalState => 0x08,
            ActivationCode::Reserved(b) => b,
            ActivationCode::SuccessfullyActivated => 0x10,
            ActivationCode::ActivatedConfirmationRequired => 0x11,
            ActivationCode::ReservedForOem(b) => b,
        }
    }
}
