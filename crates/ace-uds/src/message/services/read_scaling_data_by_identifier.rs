use crate::{message::DataIdentifier, UdsError};
use ace_core::{DiagError, FrameIter};
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReadScalingDataByIdentifierRequest<'a> {
    pub data_identifiers: FrameIter<'a, DataIdentifier>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReadScalingDataByIdentifierResponse<'a> {
    pub data_identifier: DataIdentifier,
    pub scaling_bytes: FrameIter<'a, ScalingByte<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScalingByte<'a> {
    pub high_nibble: ScalingByteHighNibble,
    pub low_nibble: ScalingByteLowNibble,
    pub extension: ScalingByteExtension<'a>,
}

impl<'a> ace_core::codec::FrameRead<'a> for ScalingByte<'a> {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        let high_nibble = ScalingByteHighNibble::decode(&mut &[byte >> 4][..])?;
        let low_nibble = ScalingByteLowNibble::decode(&mut &[byte & 0x0F][..])?;
        let length = (byte & 0x0F) as usize;

        let extension = match &high_nibble {
            ScalingByteHighNibble::BitMappedReportedWithoutMask => {
                let mut ext_buf =
                    ace_core::codec::take_n(buf, length).map_err(|e| UdsError::from(e))?;
                ScalingByteExtension::BitMappedReportedWithoutMask(
                    BitMappedReportedWithoutMask::decode(&mut ext_buf)?,
                )
            }
            ScalingByteHighNibble::Formula => {
                let mut ext_buf =
                    ace_core::codec::take_n(buf, length).map_err(|e| UdsError::from(e))?;
                ScalingByteExtension::Formula(Formula::decode(&mut ext_buf)?)
            }
            ScalingByteHighNibble::UnitFormat => {
                let mut ext_buf =
                    ace_core::codec::take_n(buf, length).map_err(|e| UdsError::from(e))?;
                ScalingByteExtension::UnitFormat(UnitFormat::decode(&mut ext_buf)?)
            }
            ScalingByteHighNibble::StateAndConnectionType => {
                let mut ext_buf =
                    ace_core::codec::take_n(buf, length).map_err(|e| UdsError::from(e))?;
                ScalingByteExtension::StateAndConnectionType(StateAndConnectionType::decode(
                    &mut ext_buf,
                )?)
            }
            _ => ScalingByteExtension::None,
        };

        Ok(Self {
            high_nibble,
            low_nibble,
            extension,
        })
    }
}

impl ace_core::codec::FrameWrite for ScalingByte<'_> {
    type Error = UdsError;
    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let high: u8 = match &self.high_nibble {
            ScalingByteHighNibble::UnsignedNumeric => 0x0,
            ScalingByteHighNibble::SignedNumberic => 0x1,
            ScalingByteHighNibble::BitMappedReportedWithoutMask => 0x2,
            ScalingByteHighNibble::BitMappedReportedWithMask => 0x3,
            ScalingByteHighNibble::BinaryCodedDecimal => 0x4,
            ScalingByteHighNibble::StateEncodedVariable => 0x5,
            ScalingByteHighNibble::ASCII => 0x6,
            ScalingByteHighNibble::SignedFloatingPoint => 0x7,
            ScalingByteHighNibble::Packet => 0x8,
            ScalingByteHighNibble::Formula => 0x9,
            ScalingByteHighNibble::UnitFormat => 0xA,
            ScalingByteHighNibble::StateAndConnectionType => 0xB,
            ScalingByteHighNibble::IsoSaeReserved(v) => *v,
        };

        let low: u8 = match &self.low_nibble {
            ScalingByteLowNibble::NumberOfBytes(n) => *n,
        };

        buf.write_bytes(&[(high << 4) | low])
            .map_err(|e| UdsError::from(e))?;

        match &self.extension {
            ScalingByteExtension::BitMappedReportedWithoutMask(inner) => inner.encode(buf)?,
            ScalingByteExtension::Formula(inner) => inner.encode(buf)?,
            ScalingByteExtension::UnitFormat(inner) => inner.encode(buf)?,
            ScalingByteExtension::StateAndConnectionType(inner) => inner.encode(buf)?,
            ScalingByteExtension::None => {}
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u8)]
pub enum ScalingByteHighNibble {
    #[frame(id = 0x0)]
    UnsignedNumeric,
    #[frame(id = 0x1)]
    SignedNumberic,
    #[frame(id = 0x2)]
    BitMappedReportedWithoutMask,
    #[frame(id = 0x3)]
    BitMappedReportedWithMask,
    #[frame(id = 0x4)]
    BinaryCodedDecimal,
    #[frame(id = 0x5)]
    StateEncodedVariable,
    #[frame(id = 0x6)]
    ASCII,
    #[frame(id = 0x7)]
    SignedFloatingPoint,
    #[frame(id = 0x8)]
    Packet,
    #[frame(id = 0x9)]
    Formula,
    #[frame(id = 0xA)]
    UnitFormat,
    #[frame(id = 0xB)]
    StateAndConnectionType,
    #[frame(id_pat = "0xC..=0xF")]
    IsoSaeReserved(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u8)]
pub enum ScalingByteLowNibble {
    #[frame(id_pat = "0x0..=0xF")]
    NumberOfBytes(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingByteExtension<'a> {
    BitMappedReportedWithoutMask(BitMappedReportedWithoutMask<'a>),
    Formula(Formula<'a>),
    UnitFormat(UnitFormat),
    StateAndConnectionType(StateAndConnectionType),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BitMappedReportedWithoutMask<'a> {
    pub mask_byte: u8,
    pub remaining: &'a [u8],
}

impl<'a> ace_core::codec::FrameRead<'a> for BitMappedReportedWithoutMask<'a> {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let mask_byte = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        let remaining = *buf;
        *buf = &buf[buf.len()..];

        Ok(Self {
            mask_byte,
            remaining,
        })
    }
}

impl ace_core::codec::FrameWrite for BitMappedReportedWithoutMask<'_> {
    type Error = UdsError;
    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        buf.write_bytes(&[self.mask_byte])
            .map_err(|e| UdsError::from(e))?;
        buf.write_bytes(self.remaining)
            .map_err(|e| UdsError::from(e))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct Formula<'a> {
    pub formula_identifier: FormulaIdentifier,
    pub c0_high_byte: u8,
    pub c0_low_byte: u8,
    pub remaining: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum FormulaIdentifier {
    #[frame(id = 0x00)]
    LinearFormula,
    #[frame(id = 0x01)]
    LinearScaleWithOffset,
    #[frame(id = 0x02)]
    ReciprocalWithScaleAndOffset,
    #[frame(id = 0x03)]
    ScaledReciprocal,
    #[frame(id = 0x04)]
    OffsetThenScale,
    #[frame(id = 0x05)]
    RationalFormula,
    #[frame(id = 0x06)]
    LinearScale,
    #[frame(id = 0x07)]
    Reciprocal,
    #[frame(id = 0x08)]
    Offset,
    #[frame(id = 0x09)]
    RatioScale,
    #[frame(id_pat = "0x0A..=0x7F")]
    IsoSaeReserved(u8),
    #[frame(id_pat = "0x80..=0xFF")]
    VehicleManufacturerSpecific(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u8)]
pub enum UnitFormat {
    #[frame(id = 0x00)]
    NoUnit,
    #[frame(id = 0x01)]
    Metre,
    #[frame(id = 0x02)]
    Foot,
    #[frame(id = 0x03)]
    Inch,
    #[frame(id = 0x04)]
    Yard,
    #[frame(id = 0x05)]
    MileEnglish,
    #[frame(id = 0x06)]
    Gram,
    #[frame(id = 0x07)]
    TonMetric,
    #[frame(id = 0x08)]
    Second,
    #[frame(id = 0x09)]
    Minute,
    #[frame(id = 0x0A)]
    Hour,
    #[frame(id = 0x0B)]
    Day,
    #[frame(id = 0x0C)]
    Year,
    #[frame(id = 0x0D)]
    Ampere,
    #[frame(id = 0x0E)]
    Volt,
    #[frame(id = 0x0F)]
    Coulomb,
    #[frame(id = 0x10)]
    Ohm,
    #[frame(id = 0x11)]
    Farad,
    #[frame(id = 0x12)]
    Henry,
    #[frame(id = 0x13)]
    Siemens,
    #[frame(id = 0x14)]
    Weber,
    #[frame(id = 0x15)]
    Tesla,
    #[frame(id = 0x16)]
    Kelvin,
    #[frame(id = 0x17)]
    Celcius,
    #[frame(id = 0x18)]
    Fahrenheit,
    #[frame(id = 0x19)]
    Candela,
    #[frame(id = 0x1A)]
    Radian,
    #[frame(id = 0x1B)]
    Degree,
    #[frame(id = 0x1C)]
    Hertz,
    #[frame(id = 0x1D)]
    Joule,
    #[frame(id = 0x1E)]
    Newton,
    #[frame(id = 0x1F)]
    Kilopond,
    #[frame(id = 0x20)]
    PoundForce,
    #[frame(id = 0x21)]
    Watt,
    #[frame(id = 0x22)]
    HorsePowerMetric,
    #[frame(id = 0x23)]
    HorsePowerUkUs,
    #[frame(id = 0x24)]
    Pascal,
    #[frame(id = 0x25)]
    Bar,
    #[frame(id = 0x26)]
    Atmosphere,
    #[frame(id = 0x27)]
    PoundForcePerSquareInch,
    #[frame(id = 0x28)]
    Becquerel,
    #[frame(id = 0x29)]
    Lumen,
    #[frame(id = 0x2A)]
    Lux,
    #[frame(id = 0x2B)]
    Litre,
    #[frame(id = 0x2C)]
    GallonBritish,
    #[frame(id = 0x2D)]
    GallonUsLiq,
    #[frame(id = 0x2E)]
    CubicInch,
    #[frame(id = 0x2F)]
    MeterPerSecond,
    #[frame(id = 0x30)]
    KilometerPerHour,
    #[frame(id = 0x31)]
    MilePerHour,
    #[frame(id = 0x32)]
    RevolutionsPerSecond,
    #[frame(id = 0x33)]
    RevolutionsPerMinute,
    #[frame(id = 0x34)]
    Counts,
    #[frame(id = 0x35)]
    Percent,
    #[frame(id = 0x36)]
    MilligramPerStroke,
    #[frame(id = 0x37)]
    MeterPerSquareSecond,
    #[frame(id = 0x38)]
    NewtonMeter,
    #[frame(id = 0x39)]
    LitrePerMinute,
    #[frame(id = 0x3A)]
    WattPerSquareMeter,
    #[frame(id = 0x3B)]
    BarPerSecond,
    #[frame(id = 0x3C)]
    RadiansPerSecond,
    #[frame(id = 0x3D)]
    RadiansPerSquareSecond,
    #[frame(id = 0x3E)]
    KilogramPerSquareMeter,
    #[frame(id = 0x3F)]
    Reserved,
    #[frame(id = 0x40)]
    Exa,
    #[frame(id = 0x41)]
    Peta,
    #[frame(id = 0x42)]
    Tera,
    #[frame(id = 0x43)]
    Giga,
    #[frame(id = 0x44)]
    Mega,
    #[frame(id = 0x45)]
    Kilo,
    #[frame(id = 0x46)]
    Hecto,
    #[frame(id = 0x47)]
    Deca,
    #[frame(id = 0x48)]
    Deci,
    #[frame(id = 0x49)]
    Centi,
    #[frame(id = 0x4A)]
    Milli,
    #[frame(id = 0x4B)]
    Micro,
    #[frame(id = 0x4C)]
    Nano,
    #[frame(id = 0x4D)]
    Pico,
    #[frame(id = 0x4E)]
    Femto,
    #[frame(id = 0x4F)]
    Atto,
    #[frame(id = 0x50)]
    YearMonthDay,
    #[frame(id = 0x51)]
    DayMonthYear,
    #[frame(id = 0x52)]
    MonthDayYear,
    #[frame(id = 0x53)]
    Week,
    #[frame(id = 0x54)]
    UtcHourMinuteSecond,
    #[frame(id = 0x55)]
    HourMinuteSecond,
    #[frame(id = 0x56)]
    SecondMinuteHourDayMonthYear,
    #[frame(id = 0x57)]
    SecondMinuteHourDayMonthYearLocalMinuteOffsetLocalHourOffset,
    #[frame(id = 0x58)]
    SecondMinuteHourMonthDayYear,
    #[frame(id = 0x59)]
    SecondMinuteHourMonthDayYearLocalMinuteOffsetLocalHourOffset,
    #[frame(id_pat = "0x5A..=0xFF")]
    IsoSaeReserved(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateAndConnectionType {
    pub activity: StateAndConnectionTypeActivity,
    pub signal: StateAndConnectionTypeSignal,
    pub input_signal: StateAndConnectionTypeInputSignal,
    pub resistor: StateAndConnectionTypeResistor,
}

impl<'a> ace_core::codec::FrameRead<'a> for StateAndConnectionType {
    type Error = UdsError;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let byte = *buf
            .first()
            .ok_or(UdsError::from(DiagError::LengthMismatch {
                expected: 1,
                actual: 0,
            }))?;
        *buf = &buf[1..];

        let activity = match byte & 0x07 {
            0x00 => StateAndConnectionTypeActivity::NotActive,
            0x01 => StateAndConnectionTypeActivity::ActiveFunction1,
            0x02 => StateAndConnectionTypeActivity::ErrorDetected,
            0x03 => StateAndConnectionTypeActivity::NotAvailable,
            0x04 => StateAndConnectionTypeActivity::ActiveFunction2,
            v => StateAndConnectionTypeActivity::Reserved(v),
        };

        let signal = match (byte >> 3) & 0x03 {
            0x00 => StateAndConnectionTypeSignal::SignalAtLowLevel,
            0x01 => StateAndConnectionTypeSignal::SignalAtMiddleLevel,
            0x02 => StateAndConnectionTypeSignal::SignalAtHighLevel,
            _ => StateAndConnectionTypeSignal::Reserved,
        };

        let input_signal = match (byte >> 5) & 0x01 {
            0x00 => StateAndConnectionTypeInputSignal::InputSignal,
            _ => StateAndConnectionTypeInputSignal::NotDefined,
        };

        let resistor = match (byte >> 6) & 0x03 {
            0x00 => StateAndConnectionTypeResistor::NotAvailableInEcuConnector,
            0x01 => StateAndConnectionTypeResistor::PullDownResistor,
            0x02 => StateAndConnectionTypeResistor::PullUpResistor,
            _ => StateAndConnectionTypeResistor::PullUpAndPullDown,
        };

        Ok(Self {
            activity,
            signal,
            input_signal,
            resistor,
        })
    }
}

impl ace_core::codec::FrameWrite for StateAndConnectionType {
    type Error = UdsError;
    fn encode<W: ace_core::codec::Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        let activity_bits: u8 = match self.activity {
            StateAndConnectionTypeActivity::NotActive => 0x00,
            StateAndConnectionTypeActivity::ActiveFunction1 => 0x01,
            StateAndConnectionTypeActivity::ErrorDetected => 0x02,
            StateAndConnectionTypeActivity::NotAvailable => 0x03,
            StateAndConnectionTypeActivity::ActiveFunction2 => 0x04,
            StateAndConnectionTypeActivity::Reserved(v) => v,
        };

        let signal_bits: u8 = match self.signal {
            StateAndConnectionTypeSignal::SignalAtLowLevel => 0x00,
            StateAndConnectionTypeSignal::SignalAtMiddleLevel => 0x01,
            StateAndConnectionTypeSignal::SignalAtHighLevel => 0x02,
            StateAndConnectionTypeSignal::Reserved => 0x03,
        };

        let input_signal_bit: u8 = match self.input_signal {
            StateAndConnectionTypeInputSignal::InputSignal => 0x00,
            StateAndConnectionTypeInputSignal::NotDefined => 0x01,
        };

        let resistor_bits: u8 = match self.resistor {
            StateAndConnectionTypeResistor::NotAvailableInEcuConnector => 0x00,
            StateAndConnectionTypeResistor::PullDownResistor => 0x01,
            StateAndConnectionTypeResistor::PullUpResistor => 0x02,
            StateAndConnectionTypeResistor::PullUpAndPullDown => 0x03,
        };

        let byte =
            activity_bits | (signal_bits << 3) | (input_signal_bit << 5) | (resistor_bits << 6);

        buf.write_bytes(&[byte]).map_err(|e| UdsError::from(e))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u8)]
pub enum StateAndConnectionTypeActivity {
    #[frame(id = 0x00)]
    NotActive,
    #[frame(id = 0x01)]
    ActiveFunction1,
    #[frame(id = 0x02)]
    ErrorDetected,
    #[frame(id = 0x03)]
    NotAvailable,
    #[frame(id = 0x04)]
    ActiveFunction2,
    #[frame(id_pat = "0x05..=0x07")]
    Reserved(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u8)]
pub enum StateAndConnectionTypeSignal {
    #[frame(id = 0x00)]
    SignalAtLowLevel,
    #[frame(id = 0x01)]
    SignalAtMiddleLevel,
    #[frame(id = 0x02)]
    SignalAtHighLevel,
    #[frame(id = 0x03)]
    Reserved,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u8)]
pub enum StateAndConnectionTypeInputSignal {
    #[frame(id = 0x00)]
    InputSignal,
    #[frame(id = 0x01)]
    NotDefined,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u8)]
pub enum StateAndConnectionTypeResistor {
    #[frame(id = 0x00)]
    NotAvailableInEcuConnector,
    #[frame(id = 0x01)]
    PullDownResistor,
    #[frame(id = 0x02)]
    PullUpResistor,
    #[frame(id = 0x03)]
    PullUpAndPullDown,
}
