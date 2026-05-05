use crate::{UdsError, ValidationError};
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ClearDiagnosticInformationRequest {
    pub group_of_dtc: DtcGroup,
    pub memory_selection: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ClearDiagnosticInformationResponse {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DtcGroup {
    Reserved([u8; 3]),
    VehicleManufacturerSpecific([u8; 3]),
    FunctionalGroup(FunctionalGroup),
    All,
}

impl<'a> ace_core::codec::FrameRead<'a> for DtcGroup {
    type Error = UdsError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let bytes = ace_core::codec::take_n(buf, 3).map_err(|e| UdsError::from(e))?;
        let value = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);

        let group = match value {
            0x000000..=0x0000FF => DtcGroup::Reserved([bytes[0], bytes[1], bytes[2]]),
            0x000100..=0xFFFEFF => {
                DtcGroup::VehicleManufacturerSpecific([bytes[0], bytes[1], bytes[2]])
            }
            0xFFFF00..=0xFFFFFE => {
                let functional_group = FunctionalGroup::decode(&mut &[bytes[2]][..])?;
                DtcGroup::FunctionalGroup(functional_group)
            }
            0xFFFFFF => DtcGroup::All,
            val => return Err(UdsError::Validation(ValidationError::InvalidDtcGroup(val))),
        };
        Ok(group)
    }
}

impl ace_core::codec::FrameWrite for DtcGroup {
    type Error = UdsError;

    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let bytes = match self {
            DtcGroup::Reserved(b) => *b,
            DtcGroup::VehicleManufacturerSpecific(b) => *b,
            DtcGroup::FunctionalGroup(fg) => {
                let mut tmp = [0u8; 1];
                fg.encode(&mut tmp.as_mut_slice())?;
                [0x00, 0xFF, tmp[0]]
            }
            DtcGroup::All => [0xFF, 0xFF, 0xFF],
        };
        buf.write_bytes(&bytes).map_err(|e| UdsError::from(e))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum FunctionalGroup {
    #[frame(id_pat = "0x00..=0x32 | 0x34..=0xCF | 0xE0..=0xFD | 0xFF")]
    IsoSaeReserved(u8),
    #[frame(id = 0x33)]
    EmissionsSystem,
    #[frame(id = 0xD0)]
    SafetySystem,
    #[frame(id_pat = "0xD1..=0xDF")]
    LegislativeSystem(u8),
    #[frame(id = 0xFE)]
    VOBDSystem,
}
