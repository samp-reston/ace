use crate::common::{AsImmutableFrame, RawFrame, RawFrameMut};

// region: UdsFrame (Immutable)

/// Immutable UDS frame for zero-copy parsing.
///
/// Wraps a `&[u8]` buffer and provides structural access to the raw bytes.
/// Semantic interpretation - service identifiers, sub-functions, response
/// codes - is provided by `UdsFrameExt` in `ace-uds`.
///
/// This design allows `UdsFrame` to be used in `no_std` environments without
/// any protocol knowledge at the proto layer.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct UdsFrame<'a> {
    payload: &'a [u8],
}

impl<'a> UdsFrame<'a> {
    // region: Constructors

    /// Creates a `UdsFrame` from an immutable byte slice.
    ///
    /// Zero-copy - the slice is wrapped without any data movement.
    #[must_use]
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { payload: slice }
    }

    // endregion: Constructors

    // region: Conversions

    /// Copies this frame into the provided buffer and returns a `UdsFrameMut`.
    pub fn to_mut(&self, buf: &'a mut [u8]) -> UdsFrameMut<'a> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        UdsFrameMut::from_slice(&mut buf[..len])
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

impl RawFrame for UdsFrame<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

// endregion: Common Trait Impls

// endregion: UdsFrame (Immutable)

// region: UdsFrameMut (Mutable)

/// Mutable UDS frame for in-place construction and modification.
///
/// Wraps a `&mut [u8]` buffer. All read accessors delegate to `UdsFrame`
/// via `AsImmutableFrame`. Semantic methods are provided by `UdsFrameMut`
/// extension traits in `ace-uds`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UdsFrameMut<'a> {
    payload: &'a mut [u8],
}

impl<'a> UdsFrameMut<'a> {
    // region: Constructors

    /// Creates a `UdsFrameMut` from a mutable byte slice.
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

    /// Copies this frame into a new buffer and returns a `UdsFrameMut` over it.
    pub fn copy_to_buffer<'b>(&self, buf: &'b mut [u8]) -> UdsFrameMut<'b> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        UdsFrameMut::from_slice(&mut buf[..len])
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

impl RawFrame for UdsFrameMut<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

impl RawFrameMut for UdsFrameMut<'_> {
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

impl<'a> AsImmutableFrame<'a> for UdsFrameMut<'a> {
    type Immutable = UdsFrame<'a>;

    fn as_frame(&'a self) -> Self::Immutable {
        UdsFrame::from_slice(self.payload)
    }
}

// endregion: Common Trait Impls

// endregion: UdsFrameMut (Mutable)

// region: Conversion Traits

impl<'a> From<&'a [u8]> for UdsFrame<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl<'a> From<&'a mut [u8]> for UdsFrameMut<'a> {
    fn from(value: &'a mut [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl AsRef<[u8]> for UdsFrame<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsRef<[u8]> for UdsFrameMut<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsMut<[u8]> for UdsFrameMut<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

// endregion: Conversion Traits

// region: Display

impl core::fmt::Display for UdsFrame<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UdsFrame(len: {})", self.len())
    }
}

impl core::fmt::Display for UdsFrameMut<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "UdsFrameMut(len: {})", self.len())
    }
}

// endregion: Display

// region: Index Access

impl core::ops::Index<usize> for UdsFrame<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for UdsFrame<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::Index<usize> for UdsFrameMut<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for UdsFrameMut<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::IndexMut<usize> for UdsFrameMut<'_> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.payload[index]
    }
}

impl core::ops::IndexMut<core::ops::Range<usize>> for UdsFrameMut<'_> {
    fn index_mut(&mut self, range: core::ops::Range<usize>) -> &mut Self::Output {
        &mut self.payload[range]
    }
}

// endregion: Index Access

// region: IntoIterator

impl<'a> IntoIterator for &'a UdsFrame<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

impl<'a> IntoIterator for &'a UdsFrameMut<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

// endregion: IntoIterator
