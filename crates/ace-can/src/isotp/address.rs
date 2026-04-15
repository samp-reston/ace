#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsoTpAddressingMode {
    /// Full 8 bytes available for PCI + payload.
    /// CAN ID alone identifies source and target.
    Normal,
    /// First data byte is N_TA (target address).
    /// 7 bytes available for PCI + payload.
    Extended,
    /// First data byte is N_AE (address extension).
    /// 7 bytes available for PCI + payload.
    /// Extended 29-bit CAN IDs only.
    Mixed,
}

impl IsoTpAddressingMode {
    /// Byte offset within the data field where the PCI byte lives.
    pub fn pci_offset(&self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Extended | Self::Mixed => 1,
        }
    }

    /// Maximum payload bytes in a classic CAN single frame.
    pub fn max_sf_payload_classic(&self) -> usize {
        7 // 8 bytes - 1 PCI byte, no address byte in PCI budget
    }

    /// Maximum payload bytes in a CAN FD single frame.
    pub fn max_sf_payload_fd(&self) -> usize {
        62 // 64 bytes - 2 PCI bytes (FD escape), no address byte in PCI budget
    }
}
