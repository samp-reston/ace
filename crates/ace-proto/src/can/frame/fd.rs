// region: CanFdFrame (Immutable)

use crate::common::{AsImmutableFrame, RawFrame, RawFrameMut};

/// Immutable CAN FD frame for zero-copy parsing.
///
/// Wraps a `&[u8]` buffer representing the raw CAN FD frame bytes as received
/// from the driver or wire. No structural assumptions are made about the
/// buffer contents - payload length enforcement, arbitration ID extraction,
/// DLC validation, and BRS/ESI flag interpretation are semantic concerns
/// provided by `CanFdFrameExt` in `ace-can`.
///
/// CAN FD payload is physically limited to 64 bytes. This constraint is
/// deliberately not enforced here - the caller working at this layer
/// accepts responsibility for the buffer they provide.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct CanFdFrame<'a> {
    payload: &'a [u8],
}

impl<'a> CanFdFrame<'a> {
    // region: Constructors

    /// Creates a `CanFdFrame` from an immutable byte slice.
    ///
    /// Zero-copy - the slice is wrapped without any data movement.
    #[must_use]
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { payload: slice }
    }

    // endregion: Constructors

    // region: Conversions

    /// Copies this frame into the provided buffer and returns a `CanFdFrameMut`.
    pub fn to_mut(&self, buf: &'a mut [u8]) -> CanFdFrameMut<'a> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        CanFdFrameMut::from_slice(&mut buf[..len])
    }

    // endregion: Conversions

    // region: Iterators

    /// Returns an iterator over all bytes in the frame.
    pub fn iter(&self) -> core::slice::Iter<'_, u8> {
        self.payload.iter()
    }

    // endregion: Iterators
}

// region: Common Trait Impls

impl RawFrame for CanFdFrame<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

// endregion: Common Trait Impls

// endregion: CanFdFrame (Immutable)

// region: CanFdFrameMut (Mutable)

/// Mutable CAN FD frame for in-place construction and modification.
///
/// Wraps a `&mut [u8]` buffer. All read accessors delegate to `CanFdFrame`
/// via `AsImmutableFrame`. Semantic methods including BRS and ESI flag
/// manipulation are provided by `CanFdFrameMutExt` in `ace-can`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanFdFrameMut<'a> {
    payload: &'a mut [u8],
}

impl<'a> CanFdFrameMut<'a> {
    // region: Constructors

    /// Creates a `CanFdFrameMut` from a mutable byte slice.
    pub fn from_slice(slice: &'a mut [u8]) -> Self {
        Self { payload: slice }
    }

    // endregion: Constructors

    // region: Mutation Methods

    /// Zeroes all bytes in the frame.
    pub fn clear(&mut self) {
        self.payload.fill(0);
    }

    // endregion: Mutation Methods

    // region: Conversions

    /// Copies this frame into a new buffer and returns a `CanFdFrameMut` over it.
    pub fn copy_to_buffer<'b>(&self, buf: &'b mut [u8]) -> CanFdFrameMut<'b> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        CanFdFrameMut::from_slice(&mut buf[..len])
    }

    // endregion: Conversions

    // region: Iterators

    /// Returns an iterator over all bytes in the frame.
    pub fn iter(&self) -> core::slice::Iter<'_, u8> {
        self.payload.iter()
    }

    // endregion: Iterators
}

// region: Common Trait Impls

impl RawFrame for CanFdFrameMut<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

impl RawFrameMut for CanFdFrameMut<'_> {
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

impl<'a> AsImmutableFrame<'a> for CanFdFrameMut<'a> {
    type Immutable = CanFdFrame<'a>;

    fn as_frame(&'a self) -> Self::Immutable {
        CanFdFrame::from_slice(self.payload)
    }
}

// endregion: Common Trait Impls

// endregion: CanFdFrameMut (Mutable)

// region: Conversion Traits

impl<'a> From<&'a [u8]> for CanFdFrame<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl<'a> From<&'a mut [u8]> for CanFdFrameMut<'a> {
    fn from(value: &'a mut [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl AsRef<[u8]> for CanFdFrame<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsRef<[u8]> for CanFdFrameMut<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsMut<[u8]> for CanFdFrameMut<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

// endregion: Conversion Traits

// region: Display

impl core::fmt::Display for CanFdFrame<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "CanFdFrame(len: {})", self.len())
    }
}

impl core::fmt::Display for CanFdFrameMut<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "CanFdFrameMut(len: {})", self.len())
    }
}

// endregion: Display

// region: Index Access

impl core::ops::Index<usize> for CanFdFrame<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for CanFdFrame<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::Index<usize> for CanFdFrameMut<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for CanFdFrameMut<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::IndexMut<usize> for CanFdFrameMut<'_> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.payload[index]
    }
}

impl core::ops::IndexMut<core::ops::Range<usize>> for CanFdFrameMut<'_> {
    fn index_mut(&mut self, range: core::ops::Range<usize>) -> &mut Self::Output {
        &mut self.payload[range]
    }
}

// endregion: Index Access

// region: IntoIterator

impl<'a> IntoIterator for &'a CanFdFrame<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

impl<'a> IntoIterator for &'a CanFdFrameMut<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

// endregion: IntoIterator
