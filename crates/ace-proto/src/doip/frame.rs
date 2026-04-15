// region: Imports

use crate::{
    common::{AsImmutableFrame, RawFrame, RawFrameMut},
    doip::constants::DOIP_HEADER_LEN,
};

// endregion: Imports

// region: DoipFrame (Immutable)

/// Immutable DoIP frame for zero-copy parsing.
///
/// Wraps a `&[u8]` buffer and provides structural access to the raw bytes.
/// Semantic interpretation - protocol version, payload type, message parsing -
/// is provided by `DoipFrameExt` in `ace-doip`.
///
/// `payload_data()` is retained here as it is a purely structural operation
/// derived from the fixed `DOIP_HEADER_LEN` constant, carrying no protocol
/// semantic knowledge.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct DoipFrame<'a> {
    payload: &'a [u8],
}

impl<'a> DoipFrame<'a> {
    // region: Constructors

    /// Creates a `DoipFrame` from an immutable byte slice.
    ///
    /// Zero-copy - the slice is wrapped without any data movement.
    #[must_use]
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { payload: slice }
    }

    // endregion: Constructors

    // region: Structural Accessors

    /// Returns the payload data - everything after the 8-byte header.
    ///
    /// This is a purely structural operation based on the fixed `DOIP_HEADER_LEN`
    /// constant. No protocol knowledge is required to perform this slice.
    #[must_use]
    pub fn payload_data(&self) -> &[u8] {
        if self.payload.len() > DOIP_HEADER_LEN {
            &self.payload[DOIP_HEADER_LEN..]
        } else {
            &[]
        }
    }

    // endregion: Structural Accessors

    // region: Conversions

    /// Copies this frame into the provided buffer and returns a `DoipFrameMut`.
    pub fn to_mut(&self, buf: &'a mut [u8]) -> DoipFrameMut<'a> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        DoipFrameMut::from_slice(&mut buf[..len])
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

impl RawFrame for DoipFrame<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

// endregion: Common Trait Impls

// endregion: DoipFrame (Immutable)

// region: DoipFrameMut (Mutable)

/// Mutable DoIP frame for in-place construction and modification.
///
/// Wraps a `&mut [u8]` buffer. Structural mutation methods are provided here.
/// Semantic mutation - setting protocol version, payload type, updating length
/// fields - is provided by extension traits in `ace-doip`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DoipFrameMut<'a> {
    payload: &'a mut [u8],
}

impl<'a> DoipFrameMut<'a> {
    // region: Constructors

    /// Creates a `DoipFrameMut` from a mutable byte slice.
    pub fn from_slice(slice: &'a mut [u8]) -> Self {
        Self { payload: slice }
    }

    // endregion: Constructors

    // region: Structural Accessors

    /// Returns the payload data - everything after the 8-byte header.
    #[must_use]
    pub fn payload_data(&self) -> &[u8] {
        if self.payload.len() > DOIP_HEADER_LEN {
            &self.payload[DOIP_HEADER_LEN..]
        } else {
            &[]
        }
    }

    /// Returns the payload data as mutable bytes.
    pub fn payload_data_mut(&mut self) -> &mut [u8] {
        if self.payload.len() > DOIP_HEADER_LEN {
            &mut self.payload[DOIP_HEADER_LEN..]
        } else {
            &mut []
        }
    }

    // endregion: Structural Accessors

    // region: Mutation Methods

    /// Zeroes all bytes in the frame including the header.
    pub fn clear(&mut self) {
        self.payload.fill(0);
    }

    /// Zeroes only the payload data, leaving the header bytes intact.
    pub fn clear_payload(&mut self) {
        if self.payload.len() > DOIP_HEADER_LEN {
            self.payload[DOIP_HEADER_LEN..].fill(0);
        }
    }

    // endregion: Mutation Methods

    // region: Conversions

    /// Copies this frame into a new buffer and returns a `DoipFrameMut` over it.
    pub fn copy_to_buffer<'b>(&self, buf: &'b mut [u8]) -> DoipFrameMut<'b> {
        let len = self.len();
        buf[..len].copy_from_slice(self.payload);
        DoipFrameMut::from_slice(&mut buf[..len])
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

impl RawFrame for DoipFrameMut<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.payload
    }
}

impl RawFrameMut for DoipFrameMut<'_> {
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

impl<'a> AsImmutableFrame<'a> for DoipFrameMut<'a> {
    type Immutable = DoipFrame<'a>;

    fn as_frame(&'a self) -> Self::Immutable {
        DoipFrame::from_slice(self.payload)
    }
}

// endregion: Common Trait Impls

// endregion: DoipFrameMut (Mutable)

// region: Conversion Traits

impl<'a> From<&'a [u8]> for DoipFrame<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl<'a> From<&'a mut [u8]> for DoipFrameMut<'a> {
    fn from(value: &'a mut [u8]) -> Self {
        Self::from_slice(value)
    }
}

impl AsRef<[u8]> for DoipFrame<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsRef<[u8]> for DoipFrameMut<'_> {
    fn as_ref(&self) -> &[u8] {
        self.payload
    }
}

impl AsMut<[u8]> for DoipFrameMut<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.payload
    }
}

// endregion: Conversion Traits

// region: Display

impl core::fmt::Display for DoipFrame<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DoipFrame(len: {})", self.len())
    }
}

impl core::fmt::Display for DoipFrameMut<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DoipFrameMut(len: {})", self.len())
    }
}

// endregion: Display

// region: Index Access

impl core::ops::Index<usize> for DoipFrame<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for DoipFrame<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::Index<usize> for DoipFrameMut<'_> {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.payload[index]
    }
}

impl core::ops::Index<core::ops::Range<usize>> for DoipFrameMut<'_> {
    type Output = [u8];
    fn index(&self, range: core::ops::Range<usize>) -> &Self::Output {
        &self.payload[range]
    }
}

impl core::ops::IndexMut<usize> for DoipFrameMut<'_> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.payload[index]
    }
}

impl core::ops::IndexMut<core::ops::Range<usize>> for DoipFrameMut<'_> {
    fn index_mut(&mut self, range: core::ops::Range<usize>) -> &mut Self::Output {
        &mut self.payload[range]
    }
}

// endregion: Index Access

// region: IntoIterator

impl<'a> IntoIterator for &'a DoipFrame<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

impl<'a> IntoIterator for &'a DoipFrameMut<'a> {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.payload.iter()
    }
}

// endregion: IntoIterator
