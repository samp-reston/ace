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

    /// Whether this address is a functional group address.
    ///
    /// ISO 13400-2 reserves 0xE000..=0xE3FF for WWH-OBD functional groups and
    /// 0xE400..=0xEFFF for vehicle-manufacturer-defined ones. Everything else
    /// identifies a single node.
    #[must_use]
    #[inline]
    pub const fn is_functional(&self) -> bool {
        matches!(self.0, 0xE000..=0xEFFF)
    }

    /// Derives the addressing mode from the address itself.
    ///
    /// The mode is a property of the address, not an independent fact a caller
    /// supplies - so it is computed here rather than asserted at each call site,
    /// where two callers could disagree about the same address.
    #[must_use]
    #[inline]
    pub fn address_mode(&self) -> ace_core::AddressMode {
        if self.is_functional() {
            ace_core::AddressMode::Functional
        } else {
            ace_core::AddressMode::Physical
        }
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

    /// Builds an address with the mode implied by the address value.
    ///
    /// Prefer this to [`Self::new`] when the mode is not already known from context:
    /// it cannot produce an address whose mode contradicts its value.
    #[must_use]
    pub fn classify(address: u16) -> Self {
        let logical = LogicalAddress::new(address);
        let mode = logical.address_mode();
        Self { logical, mode }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_core::AddressMode;

    #[test]
    fn functional_group_ranges_are_recognised() {
        // ISO 13400-2 boundaries, checked either side.
        assert!(!LogicalAddress::new(0xDFFF).is_functional());
        assert!(
            LogicalAddress::new(0xE000).is_functional(),
            "WWH-OBD lower bound"
        );
        assert!(LogicalAddress::new(0xE3FF).is_functional());
        assert!(
            LogicalAddress::new(0xE400).is_functional(),
            "manufacturer lower bound"
        );
        assert!(
            LogicalAddress::new(0xEFFF).is_functional(),
            "manufacturer upper bound"
        );
        assert!(!LogicalAddress::new(0xF000).is_functional());
    }

    #[test]
    fn classify_derives_the_mode_from_the_value() {
        assert_eq!(DoipAddress::classify(0x1726).mode, AddressMode::Physical);
        assert_eq!(DoipAddress::classify(0xE400).mode, AddressMode::Functional);
    }
}

impl ace_core::DiagnosticAddress for DoipAddress {
    fn address_mode(&self) -> ace_core::AddressMode {
        self.mode.clone()
    }
}
