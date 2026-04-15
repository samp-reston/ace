// region: CanFrame (Immutable)

use crate::common::{AsImmutableFrame, RawFrame, RawFrameMut};

/// Immutable classic CAN frame for zero-copy parsing.
///
/// Wraps a `&[u8]` buffer representing the raw CAN frame bytes as received
/// from the driver or wire. No structural assumptions are made about the
/// buffer contents - payload length enforcement, arbitration ID extraction,
/// and DLC validation are semantic concerns provided by `CanFrameExt` in
/// `ace-can`.
///
/// Classic CAN payload is physically limited to 8 bytes. This constraint
/// is deliberately not enforced here - the caller working at this layer
/// accepts responsibility for the buffer they provide.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct CanFrame<'a> {
    payload: &'a [u8],
}

impl<'a> CanFrame<'a> {
    // region: Constructors

    /// Creates a `CanFrame` from an immutable byte slice.
    ///
    /// Zero-copy - the slice is wrapped without any data movement.
    #[must_use]
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { payload: slice }
    }

    // endregion: Constructors

    // region: Conversions

    /// Copies this frame into the provided buffer and returns a `CanFrameMut`.
    pub fn to_mut(&self, buf: &'a mut [u8]) -> CanFrameMut<'a> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        CanFrameMut::from_slice(&mut buf[..len])
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

impl RawFrame for CanFrame<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

// endregion: Common Trait Impls

// endregion: CanFrame (Immutable)

// region: CanFrameMut (Mutable)

/// Mutable classic CAN frame for in-place construction and modification.
///
/// Wraps a `&mut [u8]` buffer. All read accessors delegate to `CanFrame`
/// via `AsImmutableFrame`. Semantic methods are provided by `CanFrameMutExt`
/// in `ace-can`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanFrameMut<'a> {
    payload: &'a mut [u8],
}

impl<'a> CanFrameMut<'a> {
    // region: Constructors

    /// Creates a `CanFrameMut` from a mutable byte slice.
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

    /// Copies this frame into a new buffer and returns a `CanFrameMut` over it.
    pub fn copy_to_buffer<'b>(&self, buf: &'b mut [u8]) -> CanFrameMut<'b> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        CanFrameMut::from_slice(&mut buf[..len])
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

impl RawFrame for CanFrameMut<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

impl RawFrameMut for CanFrameMut<'_> {
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

impl<'a> AsImmutableFrame<'a> for CanFrameMut<'a> {
    type Immutable = CanFrame<'a>;

    fn as_frame(&'a self) -> Self::Immutable {
        CanFrame::from_slice(self.payload)
    }
}

// endregion: Common Trait Impls

// endregion: CanFrameMut (Mutable)

// region: Conversion Traits

impl<'a> From<&'a [u8]> for CanFrame<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl<'a> From<&'a mut [u8]> for CanFrameMut<'a> {
    fn from(value: &'a mut [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl AsRef<[u8]> for CanFrame<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsRef<[u8]> for CanFrameMut<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsMut<[u8]> for CanFrameMut<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

// endregion: Conversion Traits

// region: Display

impl core::fmt::Display for CanFrame<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "CanFrame(len: {})", self.len())
    }
}

impl core::fmt::Display for CanFrameMut<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "CanFrameMut(len: {})", self.len())
    }
}

// endregion: Display

// region: Index Access

impl core::ops::Index<usize> for CanFrame<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for CanFrame<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::Index<usize> for CanFrameMut<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for CanFrameMut<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::IndexMut<usize> for CanFrameMut<'_> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.payload[index]
    }
}

impl core::ops::IndexMut<core::ops::Range<usize>> for CanFrameMut<'_> {
    fn index_mut(&mut self, range: core::ops::Range<usize>) -> &mut Self::Output {
        &mut self.payload[range]
    }
}

// endregion: Index Access

// region: IntoIterator

impl<'a> IntoIterator for &'a CanFrame<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

impl<'a> IntoIterator for &'a CanFrameMut<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

// endregion: IntoIterator
