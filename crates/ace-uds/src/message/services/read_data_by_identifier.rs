use crate::UdsError;
use ace_core::FrameIter;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReadDataByIdentifierRequest<'a> {
    pub data_identifiers: FrameIter<'a, DataIdentifier>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ReadDataByIdentifierResponse<'a> {
    pub data_identifier_responses: FrameIter<'a, DataIdentifierResponse<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DataIdentifierResponse<'a> {
    pub data_identifier: DataIdentifier,
    pub data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
#[repr(u16)]
pub enum DataIdentifier {
    #[frame(id_pat = "0x0000..=0x00FF | 0xFF02..=0xFFFF")]
    IsoSaeReserved(u16),
    #[frame(
        id_pat = "0x0100..=0xA5FF | 0xA800..=0xACFF | 0xB000..=0xB1FF | 0xC000..=0xC2FF | 0xCF00..=0xEFFF | 0xF010..=0xF0FF"
    )]
    VehicleManufacturerSpecific(u16),
    #[frame(
        id_pat = "0xA600..=0xA7FF | 0xAD00..=0xAFFF | 0xB200..=0xBFFF | 0xC300..=0xCEFF | 0xFB00..=0xFCFF"
    )]
    ReservedForLegislativeUse(u16),
    #[frame(id_pat = "0xF000..=0xF00F")]
    NetworkConfigDataForTractorTrailerApp(u16),
    #[frame(id_pat = "0xF100..=0xF17F | 0xF1A0..=0xF1EF")]
    IdOptionVehicleManufacturerSpecific(u16),
    #[frame(id = 0xF180)]
    BootSoftware,
    #[frame(id = 0xF181)]
    ApplicationSoftware,
    #[frame(id = 0xF182)]
    ApplicationData,
    #[frame(id = 0xF183)]
    BootSoftwareFingerprint,
    #[frame(id = 0xF184)]
    ApplicationSoftwareFingerprint,
    #[frame(id = 0xF185)]
    ApplicationDataFingerprint,
    #[frame(id = 0xF186)]
    ActiveDiagnosticSession,
    #[frame(id = 0xF187)]
    VehicleManufacturerSparePartNumber,
    #[frame(id = 0xF188)]
    VehicleManufacturerEcuSoftwareNumber,
    #[frame(id = 0xF189)]
    VehicleManufacturerEcuSoftwareVersionNumber,
    #[frame(id = 0xF18A)]
    SystemSupplierIdentifier,
    #[frame(id = 0xF18B)]
    EcuManufactureringDate,
    #[frame(id = 0xF18C)]
    EcuSerialNumber,
    #[frame(id = 0xF18D)]
    SupportedFunctionalUnits,
    #[frame(id = 0xF18E)]
    VehicleManufacturerKitAssemblyPartNumber,
    #[frame(id = 0xF18F)]
    RegulationXSoftwareIdentificationNumbers,
    #[frame(id = 0xF190)]
    Vin,
    #[frame(id = 0xF191)]
    VehicleManufacturerEcuHardwareNumber,
    #[frame(id = 0xF192)]
    SystemSupplierEcuHardwareNumber,
    #[frame(id = 0xF193)]
    SystemSupplierEcuHardwareVersionNumber,
    #[frame(id = 0xF194)]
    SystemSupplierEcuSoftwareNumber,
    #[frame(id = 0xF195)]
    SystemSupplierEcuSoftwareVersionNumber,
    #[frame(id = 0xF196)]
    ExhaustRegulationOrTypeApprovalNumber,
    #[frame(id = 0xF197)]
    SystemNameOrEngineType,
    #[frame(id = 0xF198)]
    RepairShopCodeOrTesterSerialNumber,
    #[frame(id = 0xF199)]
    ProgrammingDate,
    #[frame(id = 0xF19A)]
    CalibrationRepairShopCodeOrCalibrationEquipmentSerialNumber,
    #[frame(id = 0xF19B)]
    CalibrationDate,
    #[frame(id = 0xF19C)]
    CalibrationEquipmentSoftwareNumber,
    #[frame(id = 0xF19D)]
    EcuInstallationDate,
    #[frame(id = 0xF19E)]
    OdxFile,
    #[frame(id = 0xF19F)]
    Entity,
    #[frame(id_pat = "0xF1F0..=0xF1FF")]
    IdOptionSystemSupplierSpecific(u16),
    #[frame(id_pat = "0xF200..=0xF2FF")]
    Periodic(u16),
    #[frame(id_pat = "0xF300..=0xF3FF")]
    DynamicallyDefined(u16),
    #[frame(id_pat = "0xF400..=0xF5FF")]
    ObdDataIdentifier(u16),
    #[frame(id_pat = "0xF600..=0xF6FF")]
    ObdMonitor(u16),
    #[frame(id_pat = "0xF700..=0xF7FF")]
    Obd(u16),
    #[frame(id_pat = "0xF800..=0xF8FF")]
    ObdInfoType(u16),
    #[frame(id_pat = "0xF900..=0xF9FF")]
    Tachograph(u16),
    #[frame(id_pat = "0xFA00..=0xFA0F")]
    AirbagDeployment(u16),
    #[frame(id = 0xFA10)]
    NumberOfEdrDevices,
    #[frame(id = 0xFA11)]
    EdrIdentification,
    #[frame(id = 0xFA12)]
    EdrDeviceAddressInformation,
    #[frame(id_pat = "0xFA13..=0xFA18")]
    EdrEntries(u16),
    #[frame(id_pat = "0xFA19..=0xFAFF")]
    SafetySystem(u16),
    #[frame(id_pat = "0xFD00..=0xFEFF")]
    SystemSupplierSpecific(u16),
    #[frame(id = 0xFF00)]
    UdsVersion,
    #[frame(id = 0xFF01)]
    TransportLayerSegmentationSupport,
}
