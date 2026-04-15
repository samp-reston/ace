/// Default `DoIP` Port for UDP and TCP connections.
pub const DOIP_PORT: usize = 13400;
/// Default `DoIP` Port for TLS.
pub const DOIP_TLS_PORT: usize = 3496;

/// Payload Type: Generic Negative Acknowledge
pub const DOIP_GENERIC_NACK: u16 = 0x0000;
/// Payload Type: Vehicle Identification Request
pub const DOIP_VEHICLE_IDENTIFICATION_REQ: u16 = 0x0001;
/// Payload Type: Vehicle Identification Request with EID
pub const DOIP_VEHICLE_IDENTIFICATION_REQ_EID: u16 = 0x0002;
/// Payload Type: Vehicle Identification Request with VIN
pub const DOIP_VEHICLE_IDENTIFICATION_REQ_VIN: u16 = 0x0003;
/// Payload Type: Vehicle Announcement Message
pub const DOIP_VEHICLE_ANNOUNCEMENT_MESSAGE: u16 = 0x0004;
/// Payload Type: Routing Activation Request
pub const DOIP_ROUTING_ACTIVATION_REQUEST: u16 = 0x0005;
/// Payload Type: Routing Activation Response
pub const DOIP_ROUTING_ACTIVATION_RESPONSE: u16 = 0x0006;
/// Payload Type: Alive Check Request
pub const DOIP_ALIVE_CHECK_REQUEST: u16 = 0x0007;
/// Payload Type: Alive Check Response
pub const DOIP_ALIVE_CHECK_RESPONSE: u16 = 0x0008;
/// Payload Type: Entity Status Request
pub const DOIP_ENTITY_STATUS_REQUEST: u16 = 0x4001;
/// Payload Type: Entity Status Response
pub const DOIP_ENTITY_STATUS_RESPONSE: u16 = 0x4002;
/// Payload Type: Power Information Request
pub const DOIP_POWER_INFORMATION_REQUEST: u16 = 0x4003;
/// Payload Type: Power Information Response
pub const DOIP_POWER_INFORMATION_RESPONSE: u16 = 0x4004;
/// Payload Type: Diagnostic Message
pub const DOIP_DIAGNOSTIC_MESSAGE: u16 = 0x8001;
/// Payload Type: Diagnostic Message Acknowledge
pub const DOIP_DIAGNOSTIC_MESSAGE_ACK: u16 = 0x8002;
/// Payload Type: Diagnostic Message Negative Acknowledge
pub const DOIP_DIAGNOSTIC_MESSAGE_NACK: u16 = 0x8003;
/// Payload Type: Periodic Diagnostic Message
pub const DOIP_PERIODIC_DIAGNOSTIC_MESSAGE: u16 = 0x8004;

// DoIP Header //
/// `DoIP` Header: `DoIP` Version Offset
pub const DOIP_VERSION_OFFSET: usize = 0;
/// `DoIP` Header: `DoIP` Version Length
pub const DOIP_VERSION_LEN: usize = 1;
/// `DoIP` Header: `DoIP` Inverse Version Offset
pub const DOIP_INV_VERSION_OFFSET: usize = DOIP_VERSION_OFFSET + DOIP_VERSION_LEN;
/// `DoIP` Header: `DoIP` Inverse Version Length
pub const DOIP_INV_VERSION_LEN: usize = 1;
/// `DoIP` Header: Payload Type Offset
pub const DOIP_TYPE_OFFSET: usize = DOIP_INV_VERSION_OFFSET + DOIP_INV_VERSION_LEN;
/// `DoIP` Header: Payload Type Length
pub const DOIP_TYPE_LEN: usize = 2;
/// `DoIP` Header: Payload Length Offset
pub const DOIP_LENGTH_OFFSET: usize = DOIP_TYPE_OFFSET + DOIP_TYPE_LEN;
/// `DoIP` Header: Payload Length Length
pub const DOIP_LENGTH_LEN: usize = 4;
/// `DoIP` Header: Header Length
pub const DOIP_HEADER_LEN: usize = DOIP_LENGTH_OFFSET + DOIP_LENGTH_LEN;

// DoIP Payload Version //
/// `DoIP` Payload Version: Reserved Version
pub const RESERVED_VER: u8 = 0x00;
/// `DoIP` Payload Version: ISO-13400 2010 Version
pub const ISO13400_2010: u8 = 0x01;
/// `DoIP` Payload Version: ISO-13400 2012 Version
pub const ISO13400_2012: u8 = 0x02;
/// `DoIP` Payload Version: ISO-13400 2019 Version
pub const ISO13400_2019: u8 = 0x03;
/// `DoIP` Payload Version: ISO-13400 `2019_AMD1` Version
pub const ISO13400_2019_AMD1: u8 = 0x04;
/// `DoIP` Payload Version: Default Version
pub const DEFAULT_VALUE: u8 = 0xFF;

// Generic NACK //
/// Generic Negative Acknowledge: Payload Offset
pub const DOIP_GENERIC_NACK_OFFSET: usize = DOIP_HEADER_LEN;
/// Generic Negative Acknowledge: Payload Length
pub const DOIP_GENERIC_NACK_LEN: usize = 1;

// Common //
/// `DoIP` Generic: Vehicle Identification Number (VIN) Length
pub const DOIP_COMMON_VIN_LEN: usize = 17;
/// `DoIP` Generic: Entity Identification (EID) Length
pub const DOIP_COMMON_EID_LEN: usize = 6;

// Vehicle identification request //
/// Vehicle Identification Request: Entity Identification (EID) Offset
pub const DOIP_VEHICLE_IDENTIFICATION_EID_OFFSET: usize = DOIP_HEADER_LEN;
/// Vehicle Identification Request: Vehicle Identification Number (VIN) Offset
pub const DOIP_VEHICLE_IDENTIFICATION_VIN_OFFSET: usize = DOIP_HEADER_LEN;

// Routing activation request //
/// Routing Activation Request: Source Offset
pub const DOIP_ROUTING_ACTIVATION_REQ_SRC_OFFSET: usize = DOIP_HEADER_LEN;
/// Routing Activation Request: Source Length
pub const DOIP_ROUTING_ACTIVATION_REQ_SRC_LEN: usize = 2;
/// Routing Activation Request: Type Offset
pub const DOIP_ROUTING_ACTIVATION_REQ_TYPE_OFFSET: usize =
    DOIP_ROUTING_ACTIVATION_REQ_SRC_OFFSET + DOIP_ROUTING_ACTIVATION_REQ_SRC_LEN;
/// Routing Activation Request: Type Length (v1)
pub const DOIP_ROUTING_ACTIVATION_REQ_TYPE_LEN_V1: usize = 2;
/// Routing Activation Request: Type Length (v2)
pub const DOIP_ROUTING_ACTIVATION_REQ_TYPE_LEN_V2: usize = 1;
/// Routing Activation Request: ISO (v1) Offset
pub const DOIP_ROUTING_ACTIVATION_REQ_ISO_OFFSET_V1: usize =
    DOIP_ROUTING_ACTIVATION_REQ_TYPE_OFFSET + DOIP_ROUTING_ACTIVATION_REQ_TYPE_LEN_V1;
/// Routing Activation Request: ISO (v2) Offset
pub const DOIP_ROUTING_ACTIVATION_REQ_ISO_OFFSET_V2: usize =
    DOIP_ROUTING_ACTIVATION_REQ_TYPE_OFFSET + DOIP_ROUTING_ACTIVATION_REQ_TYPE_LEN_V2;
/// Routing Activation Request: ISO Length
pub const DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN: usize = 4;
/// Routing Activation Request: OEM Offset (v1)
pub const DOIP_ROUTING_ACTIVATION_REQ_OEM_OFFSET_V1: usize =
    DOIP_ROUTING_ACTIVATION_REQ_ISO_OFFSET_V1 + DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN;
/// Routing Activation Request: OEM Offset (v2)
pub const DOIP_ROUTING_ACTIVATION_REQ_OEM_OFFSET_V2: usize =
    DOIP_ROUTING_ACTIVATION_REQ_ISO_OFFSET_V2 + DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN;
/// Routing Activation Request: OEM Length
pub const DOIP_ROUTING_ACTIVATION_REQ_OEM_LEN: usize = 4;
/// Routing Activation Request: Length
pub const DOIP_ROUTING_ACTIVATION_REQ_LEN: usize = DOIP_ROUTING_ACTIVATION_REQ_SRC_LEN
    + DOIP_ROUTING_ACTIVATION_REQ_TYPE_LEN_V2
    + DOIP_ROUTING_ACTIVATION_REQ_ISO_LEN;

// Routing activation response //
/// Routing Activation Response: Tester Offset
pub const DOIP_ROUTING_ACTIVATION_RES_TESTER_OFFSET: usize = DOIP_HEADER_LEN;
/// Routing Activation Response: Tester Length
pub const DOIP_ROUTING_ACTIVATION_RES_TESTER_LEN: usize = 2;
/// Routing Activation Response: Entity Offset
pub const DOIP_ROUTING_ACTIVATION_RES_ENTITY_OFFSET: usize =
    DOIP_ROUTING_ACTIVATION_RES_TESTER_OFFSET + DOIP_ROUTING_ACTIVATION_RES_TESTER_LEN;
/// Routing Activation Response: Entity Length
pub const DOIP_ROUTING_ACTIVATION_RES_ENTITY_LEN: usize = 2;
/// Routing Activation Response: Code Offset
pub const DOIP_ROUTING_ACTIVATION_RES_CODE_OFFSET: usize =
    DOIP_ROUTING_ACTIVATION_RES_ENTITY_OFFSET + DOIP_ROUTING_ACTIVATION_RES_ENTITY_LEN;
/// Routing Activation Response: Code Length
pub const DOIP_ROUTING_ACTIVATION_RES_CODE_LEN: usize = 1;
/// Routing Activation Response: ISO Offset
pub const DOIP_ROUTING_ACTIVATION_RES_ISO_OFFSET: usize =
    DOIP_ROUTING_ACTIVATION_RES_CODE_OFFSET + DOIP_ROUTING_ACTIVATION_RES_CODE_LEN;
/// Routing Activation Response: ISO Length
pub const DOIP_ROUTING_ACTIVATION_RES_ISO_LEN: usize = 4;
/// Routing Activation Response: OEM Offset
pub const DOIP_ROUTING_ACTIVATION_RES_OEM_OFFSET: usize =
    DOIP_ROUTING_ACTIVATION_RES_ISO_OFFSET + DOIP_ROUTING_ACTIVATION_RES_ISO_LEN;
/// Routing Activation Response: OEM Length
pub const DOIP_ROUTING_ACTIVATION_RES_OEM_LEN: usize = 4;
/// Routing Activation Response: Length
pub const DOIP_ROUTING_ACTIVATION_RES_LEN: usize = DOIP_ROUTING_ACTIVATION_RES_TESTER_LEN
    + DOIP_ROUTING_ACTIVATION_RES_ENTITY_LEN
    + DOIP_ROUTING_ACTIVATION_RES_CODE_LEN
    + DOIP_ROUTING_ACTIVATION_RES_ISO_LEN;

// Vehicle announcement message //
/// Vehicle Announcement Message: VIN Offset
pub const DOIP_VEHICLE_ANNOUNCEMENT_VIN_OFFSET: usize = DOIP_HEADER_LEN;
/// Vehicle Announcement Message: Address Offset
pub const DOIP_VEHICLE_ANNOUNCEMENT_ADDRESS_OFFSET: usize =
    DOIP_VEHICLE_ANNOUNCEMENT_VIN_OFFSET + DOIP_COMMON_VIN_LEN;
/// Vehicle Announcement Message: Address Length
pub const DOIP_VEHICLE_ANNOUNCEMENT_ADDRESS_LEN: usize = 2;
/// Vehicle Announcement Message: EID Offset
pub const DOIP_VEHICLE_ANNOUNCEMENT_EID_OFFSET: usize =
    DOIP_VEHICLE_ANNOUNCEMENT_ADDRESS_OFFSET + DOIP_VEHICLE_ANNOUNCEMENT_ADDRESS_LEN;
/// Vehicle Announcement Message: GID Offset
pub const DOIP_VEHICLE_ANNOUNCEMENT_GID_OFFSET: usize =
    DOIP_VEHICLE_ANNOUNCEMENT_EID_OFFSET + DOIP_COMMON_EID_LEN;
/// Vehicle Announcement Message: GID Length
pub const DOIP_VEHICLE_ANNOUNCEMENT_GID_LEN: usize = 6;
/// Vehicle Announcement Message: Action Offset
pub const DOIP_VEHICLE_ANNOUNCEMENT_ACTION_OFFSET: usize =
    DOIP_VEHICLE_ANNOUNCEMENT_GID_OFFSET + DOIP_VEHICLE_ANNOUNCEMENT_GID_LEN;
/// Vehicle Announcement Message: Action Length
pub const DOIP_VEHICLE_ANNOUNCEMENT_ACTION_LEN: usize = 1;
/// Vehicle Announcement Message: Sync Offset
pub const DOIP_VEHICLE_ANNOUNCEMENT_SYNC_OFFSET: usize =
    DOIP_VEHICLE_ANNOUNCEMENT_ACTION_OFFSET + DOIP_VEHICLE_ANNOUNCEMENT_ACTION_LEN;
/// Vehicle Announcement Message: Sync Length
pub const DOIP_VEHICLE_ANNOUNCEMENT_SYNC_LEN: usize = 1;
/// Vehicle Announcement Message: Length - Short
pub const DOIP_VEHICLE_ANNOUNCEMENT_LEN_SHORT: usize =
    DOIP_VEHICLE_ANNOUNCEMENT_SYNC_OFFSET - DOIP_HEADER_LEN;
/// Vehicle Announcement Message: Length - Long
pub const DOIP_VEHICLE_ANNOUNCEMENT_LEN_LONG: usize =
    DOIP_VEHICLE_ANNOUNCEMENT_SYNC_OFFSET + DOIP_VEHICLE_ANNOUNCEMENT_SYNC_LEN - DOIP_HEADER_LEN;

// Alive check response //
/// Alive Check Response: Source Offset
pub const DOIP_ALIVE_CHECK_RESPONSE_SOURCE_OFFSET: usize = DOIP_HEADER_LEN;
/// Alive Check Response: Source Length
pub const DOIP_ALIVE_CHECK_RESPONSE_SOURCE_LEN: usize = 2;

// Entity status response //
/// Entity Status Response: Node Offset
pub const DOIP_ENTITY_STATUS_RESPONSE_NODE_OFFSET: usize = DOIP_HEADER_LEN;
/// Entity Status Response: Node Length
pub const DOIP_ENTITY_STATUS_RESPONSE_NODE_LEN: usize = 1;
/// Entity Status Response: Max Concurrent TCP Sockets Offset
pub const DOIP_ENTITY_STATUS_RESPONSE_MCTS_OFFSET: usize =
    DOIP_ENTITY_STATUS_RESPONSE_NODE_OFFSET + DOIP_ENTITY_STATUS_RESPONSE_NODE_LEN;
/// Entity Status Response: Max Concurrent TCP Sockets Length
pub const DOIP_ENTITY_STATUS_RESPONSE_MCTS_LEN: usize = 1;
/// Entity Status Response: Number of Currently established TCP Sockets Offset
pub const DOIP_ENTITY_STATUS_RESPONSE_NCTS_OFFSET: usize =
    DOIP_ENTITY_STATUS_RESPONSE_MCTS_OFFSET + DOIP_ENTITY_STATUS_RESPONSE_MCTS_LEN;
/// Entity Status Response: Number of Currently established TCP Sockets Length
pub const DOIP_ENTITY_STATUS_RESPONSE_NCTS_LEN: usize = 1;
/// Entity Status Response: Max Data Size Offset
pub const DOIP_ENTITY_STATUS_RESPONSE_MDS_OFFSET: usize =
    DOIP_ENTITY_STATUS_RESPONSE_NCTS_OFFSET + DOIP_ENTITY_STATUS_RESPONSE_NCTS_LEN;
/// Entity Status Response: Max Data Size Length
pub const DOIP_ENTITY_STATUS_RESPONSE_MDS_LEN: usize = 4;
/// Entity Status Response: Length
pub const DOIP_ENTITY_STATUS_RESPONSE_LEN: usize = DOIP_ENTITY_STATUS_RESPONSE_NODE_LEN
    + DOIP_ENTITY_STATUS_RESPONSE_MCTS_LEN
    + DOIP_ENTITY_STATUS_RESPONSE_NCTS_LEN
    + DOIP_ENTITY_STATUS_RESPONSE_MDS_LEN;

// Diagnostic power mode information response //
/// Power Mode Response: Power Mode Offset
pub const DOIP_POWER_MODE_OFFSET: usize = DOIP_HEADER_LEN;
/// Power Mode Response: Power Mode Length
pub const DOIP_POWER_MODE_LEN: usize = 1;

// Common //
/// `DoIP` Common: Source Offset
pub const DOIP_DIAG_COMMON_SOURCE_OFFSET: usize = DOIP_HEADER_LEN;
/// `DoIP` Common: Source Length
pub const DOIP_DIAG_COMMON_SOURCE_LEN: usize = 2;
/// `DoIP` Common: Target Offset
pub const DOIP_DIAG_COMMON_TARGET_OFFSET: usize =
    DOIP_DIAG_COMMON_SOURCE_OFFSET + DOIP_DIAG_COMMON_SOURCE_LEN;
/// `DoIP` Common: Target Length
pub const DOIP_DIAG_COMMON_TARGET_LEN: usize = 2;

// Diagnostic message //
/// Diagnostic Message: Data Offset
pub const DOIP_DIAG_MESSAGE_DATA_OFFSET: usize =
    DOIP_DIAG_COMMON_TARGET_OFFSET + DOIP_DIAG_COMMON_TARGET_LEN;

// Diagnostic message ACK //
/// Diagnostic Message Acknowledge: Code Offset
pub const DOIP_DIAG_MESSAGE_ACK_CODE_OFFSET: usize =
    DOIP_DIAG_COMMON_TARGET_OFFSET + DOIP_DIAG_COMMON_TARGET_LEN;
/// Diagnostic Message Acknowledge: Code Length
pub const DOIP_DIAG_MESSAGE_ACK_CODE_LEN: usize = 1;
/// Diagnostic Message Acknowledge: Previous Offset
pub const DOIP_DIAG_MESSAGE_ACK_PREVIOUS_OFFSET: usize =
    DOIP_DIAG_MESSAGE_ACK_CODE_OFFSET + DOIP_DIAG_MESSAGE_ACK_CODE_LEN;

// Diagnostic message NACK //
/// Diagnostic Message Negative Acknowledge: Code Offset
pub const DOIP_DIAG_MESSAGE_NACK_CODE_OFFSET: usize =
    DOIP_DIAG_COMMON_TARGET_OFFSET + DOIP_DIAG_COMMON_TARGET_LEN;
/// Diagnostic Message Negative Acknowledge: Code Length
pub const DOIP_DIAG_MESSAGE_NACK_CODE_LEN: usize = 1;
/// Diagnostic Message Negative Acknowledge: Previous Offset
pub const DOIP_DIAG_MESSAGE_NACK_PREVIOUS_OFFSET: usize =
    DOIP_DIAG_MESSAGE_NACK_CODE_OFFSET + DOIP_DIAG_MESSAGE_NACK_CODE_LEN;
