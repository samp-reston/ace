use crate::{UdsError, ValidationError};
use ace_core::DiagError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum CommunicationControlRequest {
    #[frame(id_pat = "0x00..=0x03", decode_inner)]
    WithoutEnhancedAddressInformation(WithoutEnhancedAddressInformation),
    #[frame(id_pat = "0x04..=0x05", decode_inner)]
    WithEnhancedAddressInformation(WithEnhancedAddressInformation),
    #[frame(id_pat = "0x06..=0x3F")]
    IsoSaeReserved(u8),
    #[frame(id_pat = "0x40..=0x5F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x60..=0x7E")]
    SystemSupplierSpecific(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct WithoutEnhancedAddressInformation {
    pub control_type: ControlType,
    pub communication_type: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct WithEnhancedAddressInformation {
    pub control_type: ControlType,
    pub communication_type: CommunicationType,
    pub node_identification_number: NodeIdentificationNumber,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u16)]
pub enum NodeIdentificationNumber {
    #[frame(id = 0x0000)]
    IsoSaeReserved,
    #[frame(id_pat = "0x0001..=u16::MAX")]
    NodeIdentificationNumber(u16),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum ControlType {
    #[frame(id = 0x00)]
    EnableRxAndTx,
    #[frame(id = 0x01)]
    EnableRxAndDisableTx,
    #[frame(id = 0x02)]
    DisableRxAndDisableTx,
    #[frame(id = 0x03)]
    DisableRxAndTx,
    #[frame(id = 0x04)]
    EnableRxAndDisableTxWithEnhancedAddressInformation,
    #[frame(id = 0x05)]
    EnableRxAndTxWithEnhancedAddressInformation,
    #[frame(id_pat = "0x06..=0x3F")]
    IsoSaeReserved(u8),
    #[frame(id_pat = "0x40..=0x5F")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0x60..=0x7E")]
    SystemSupplierSpecific(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommunicationType {
    pub communication_type: CommunicationTypeValue,
    pub subnet: Subnet,
    pub reserved: CommunicationTypeReserved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommunicationTypeValue {
    IsoSaeReserved,
    NormalCommunicationMessages,
    NetworkManagementCommunicationMessages,
    JointMessages,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Subnet {
    DisableEnableSpecifiedCommuncationType,
    DisableEnableSubnetNumber,
    DisableEnableNetwork,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommunicationTypeReserved {
    IsoSaeReserved,
}

impl<'a> ace_core::codec::FrameRead<'a> for CommunicationType {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        let communication_type = match byte & 0x03 {
            0x00 => CommunicationTypeValue::IsoSaeReserved,
            0x01 => CommunicationTypeValue::NormalCommunicationMessages,
            0x02 => CommunicationTypeValue::NetworkManagementCommunicationMessages,
            0x03 => CommunicationTypeValue::JointMessages,
            val => {
                return Err(UdsError::Validation(
                    ValidationError::InvalidCommunicationTypeValue(val),
                ))
            }
        };

        let reserved = match (byte & 0x0C) >> 2 {
            0x00..=0x03 => CommunicationTypeReserved::IsoSaeReserved,
            val => {
                return Err(UdsError::Validation(
                    ValidationError::InvalidCommunicationReserved(val),
                ))
            }
        };

        let subnet = match (byte & 0xF0) >> 4 {
            0x00 => Subnet::DisableEnableSpecifiedCommuncationType,
            0x01..=0x0E => Subnet::DisableEnableSubnetNumber,
            0x0F => Subnet::DisableEnableNetwork,
            val => return Err(UdsError::Validation(ValidationError::InvalidSubnet(val))),
        };

        Ok(Self {
            communication_type,
            subnet,
            reserved,
        })
    }
}

impl ace_core::codec::FrameWrite for CommunicationType {
    type Error = UdsError;
    fn encode<W: ace_core::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let communication_type_bits: u8 = match self.communication_type {
            CommunicationTypeValue::IsoSaeReserved => 0x00,
            CommunicationTypeValue::NormalCommunicationMessages => 0x01,
            CommunicationTypeValue::NetworkManagementCommunicationMessages => 0x02,
            CommunicationTypeValue::JointMessages => 0x03,
        };

        let reserved_bits: u8 = match self.reserved {
            CommunicationTypeReserved::IsoSaeReserved => 0x00,
        };

        let subnet_bits: u8 = match self.subnet {
            Subnet::DisableEnableSpecifiedCommuncationType => 0x00,
            Subnet::DisableEnableSubnetNumber => 0x01,
            Subnet::DisableEnableNetwork => 0x0F,
        };

        let byte = communication_type_bits | (reserved_bits << 2) | (subnet_bits << 4);
        buf.write_bytes(&[byte]).map_err(|e| UdsError::from(e))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct CommunicationControlResponse {
    pub control_type: ControlType,
}
