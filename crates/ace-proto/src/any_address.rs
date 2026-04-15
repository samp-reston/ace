use crate::CanAddress;
use crate::DoipAddress;

// region: AnyAddress

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyAddress {
    Can(CanAddress),
    Doip(DoipAddress),
}

impl AnyAddress {
    pub fn is_can(&self) -> bool {
        matches!(self, AnyAddress::Can(_))
    }

    pub fn is_doip(&self) -> bool {
        matches!(self, AnyAddress::Doip(_))
    }
}

impl ace_core::DiagnosticAddress for AnyAddress {
    fn address_mode(&self) -> ace_core::AddressMode {
        match self {
            AnyAddress::Doip(addr) => addr.address_mode(),
            AnyAddress::Can(addr) => addr.address_mode(),
        }
    }
}

impl From<CanAddress> for AnyAddress {
    fn from(addr: CanAddress) -> Self {
        AnyAddress::Can(addr)
    }
}

impl From<DoipAddress> for AnyAddress {
    fn from(addr: DoipAddress) -> Self {
        AnyAddress::Doip(addr)
    }
}

// endregion: AnyAddress
