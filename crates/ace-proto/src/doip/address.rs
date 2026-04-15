// region: LogicalAddress

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalAddress(u16);

impl LogicalAddress {
    pub fn new(val: u16) -> Self {
        Self(val)
    }

    #[must_use]
    #[inline]
    pub fn value(&self) -> u16 {
        self.0
    }
}

// endregion: LogicalAddress

// region: DoipAddress

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoipAddress {
    pub logical: LogicalAddress,
    pub mode: ace_core::AddressMode,
}

impl DoipAddress {
    pub fn new(logical: LogicalAddress, mode: ace_core::AddressMode) -> Self {
        Self { logical, mode }
    }
}

impl ace_core::DiagnosticAddress for DoipAddress {
    fn address_mode(&self) -> ace_core::AddressMode {
        self.mode.clone()
    }
}
