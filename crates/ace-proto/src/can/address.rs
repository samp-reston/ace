use crate::CanId;

// region: CanAddress

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanAddress {
    pub id: CanId,
    pub mode: ace_core::AddressMode,
}

impl CanAddress {
    pub fn new(id: CanId, mode: ace_core::AddressMode) -> Self {
        Self { id, mode }
    }
}

impl ace_core::DiagnosticAddress for CanAddress {
    fn address_mode(&self) -> ace_core::AddressMode {
        self.mode.clone()
    }
}

// endregion: CanAddress
