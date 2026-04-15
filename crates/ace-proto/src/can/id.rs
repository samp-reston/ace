// region: CanId

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanId {
    Standard(StandardCanId),
    Extended(ExtendedCanId),
}

impl CanId {
    #[must_use]
    #[inline]
    pub fn is_standard(&self) -> bool {
        matches!(self, CanId::Standard(_))
    }

    #[must_use]
    #[inline]
    pub fn is_extended(&self) -> bool {
        matches!(self, CanId::Extended(_))
    }

    #[must_use]
    #[inline]
    pub fn as_raw(&self) -> u32 {
        match self {
            CanId::Standard(id) => id.value() as u32,
            CanId::Extended(id) => id.value(),
        }
    }
}

impl From<StandardCanId> for CanId {
    fn from(id: StandardCanId) -> Self {
        CanId::Standard(id)
    }
}

impl From<ExtendedCanId> for CanId {
    fn from(id: ExtendedCanId) -> Self {
        CanId::Extended(id)
    }
}

// endregion: CanId

// region: StandardCanId

/// An 11-bit standard CAN arbitration identifier.
///
/// Valid range: `0x000–0x7FF`. Construction outside this range is rejected
/// by [`StandardCanId::new`]. Use [`StandardCanId::new_unchecked`] only when
/// the value is a compile-time known constant within the valid range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardCanId(u16);

impl StandardCanId {
    const MAX: u16 = 0x7FF;

    /// Creates a `StandardCanId` from a `u16`, validating the 11-bit range.
    ///
    /// Returns `Err(DiagError::InvalidFrame)` if `val` exceeds `0x7FF`.
    pub fn new(val: u16) -> Result<Self, ace_core::DiagError> {
        if val > Self::MAX {
            Err(ace_core::DiagError::InvalidFrame(ace_core::diag_err_str(
                "id exceeds 11-bit standard CAN range",
            )))
        } else {
            Ok(Self(val))
        }
    }

    /// Creates a `StandardCanId` without validating the 11-bit range.
    ///
    /// # Safety
    ///
    /// The caller must ensure `val` is within the valid standard CAN
    /// arbitration ID range of `0x000–0x7FF`. Providing a value outside
    /// this range produces a `StandardCanId` that violates the CAN
    /// specification and will result in undefined behaviour when used
    /// with a CAN driver or hardware peripheral - including potential
    /// bus errors, frame rejection, or incorrect arbitration.
    ///
    /// Prefer [`StandardCanId::new`] unless the value is a compile-time
    /// known constant and the validation overhead is unacceptable.
    pub const unsafe fn new_unchecked(val: u16) -> Self {
        Self(val)
    }

    #[must_use]
    #[inline]
    pub fn value(&self) -> u16 {
        self.0
    }
}

// endregion: StandardCanId

// region: ExtendedCanId

/// A 29-bit extended CAN arbitration identifier.
///
/// Valid range: `0x00000000–0x1FFFFFFF`. Construction outside this range is
/// rejected by [`ExtendedCanId::new`]. Use [`ExtendedCanId::new_unchecked`]
/// only when the value is a compile-time known constant within the valid range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedCanId(u32);

impl ExtendedCanId {
    const MAX: u32 = 0x1FFF_FFFF;

    pub fn new(val: u32) -> Result<Self, ace_core::DiagError> {
        if val > Self::MAX {
            Err(ace_core::DiagError::InvalidFrame(ace_core::diag_err_str(
                "id exceeds 29-bit extended CAN range",
            )))
        } else {
            Ok(Self(val))
        }
    }

    /// Creates an `ExtendedCanId` without validating the 29-bit range.
    ///
    /// # Safety
    ///
    /// The caller must ensure `val` is within the valid extended CAN
    /// arbitration ID range of `0x00000000–0x1FFFFFFF`. The upper 3 bits
    /// of a `u32` are not part of the CAN extended ID space. Providing a
    /// value with those bits set produces an `ExtendedCanId` that violates
    /// the CAN specification and will result in undefined behaviour when
    /// used with a CAN driver or hardware peripheral - including potential
    /// bus errors, frame rejection, or incorrect arbitration.
    ///
    /// Prefer [`ExtendedCanId::new`] unless the value is a compile-time
    /// known constant and the validation overhead is unacceptable.
    pub const unsafe fn new_unchecked(val: u32) -> Self {
        Self(val)
    }

    #[must_use]
    #[inline]
    pub fn value(&self) -> u32 {
        self.0
    }
}

// endregion: ExtendedCanId
