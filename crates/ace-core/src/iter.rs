use core::marker::PhantomData;

use crate::{
    codec::{FrameRead, FrameWrite, Writer},
    DiagError,
};

// region: FrameIter

/// A lazy iterator that decodes elements of type `T` from a byte slice.
///
/// Provides a zero-copy, no_std safe way to iterate over a repeated sequence
/// of `T` values encoded contiguously in a buffer. Each call to `next()`
/// advances the internal cursor by however many bytes `T::decode` consumes.
///
/// The caller is responsible for slicing the correct byte range before
/// constructing `FrameIter` - it does not advance a parent cursor itself.
/// This allows callers to control exactly which bytes are in scope, including
/// cases where element stride must be computed from prior fields.
///
/// # Example - macro generated field
///
/// ```ignore
/// # use ace_core::FrameIter;
///
/// #[derive(FrameRead)]
/// #[frame(error = "UdsError")]
/// pub struct DefineByIdentifierRequest<'a> {
///     pub dynamically_defined_data_identifier: u16,
///     pub source_data: FrameIter<'a, SourceData>,
/// }
///
/// #[derive(FrameRead)]
/// #[frame(error = "UdsError")]
/// pub struct SourceData {
///     pub field_1: u16,
///     pub field_2: [u8; 2],
/// }
/// ```
///
/// # Example - manual impl with computed stride
///
/// ```ignore
/// # use ace_core::FrameIter;
/// impl<'a> FrameRead<'a> for DefineByMemoryAddressRequest<'a> {
///     type Error = UdsError;
///
///     fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
///         let address_and_length_format_identifier = u8::decode(buf)?;
///         let memory_address_length = (address_and_length_format_identifier & 0x0F) as usize;
///         let memory_size_length    = (address_and_length_format_identifier >> 4)   as usize;
///         let stride = memory_address_length + memory_size_length;
///         let data_len = (buf.len() / stride) * stride;
///         let memory_data = FrameIter::new(&buf[..data_len]);
///         *buf = &buf[data_len..];
///         Ok(Self { address_and_length_format_identifier, memory_data })
///     }
/// }
/// ```
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct FrameIter<'a, T> {
    buf: &'a [u8],
    _marker: PhantomData<T>,
}

impl<'a, T> FrameIter<'a, T> {
    /// Constructs a `FrameIter` over the given byte slice.
    ///
    /// The caller controls which bytes are passed in - this does not
    /// advance any parent cursor.
    #[must_use]
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            _marker: PhantomData,
        }
    }

    /// Returns the number of undecoded bytes remaining.
    #[inline]
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.buf.len()
    }

    /// Returns `true` if no bytes remain to be decoded.
    #[inline]
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns the raw undecoded bytes.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &'a [u8] {
        self.buf
    }
}

// endregion: FrameIter

// region: Iterator impl

impl<'a, T> Iterator for FrameIter<'a, T>
where
    T: FrameRead<'a>,
{
    type Item = Result<T, T::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.is_empty() {
            return None;
        }
        let mut tmp = self.buf;
        match T::decode(&mut tmp) {
            Ok(value) => {
                self.buf = tmp;
                Some(Ok(value))
            }
            Err(e) => {
                // Poison the buffer on error - further iteration is undefined
                self.buf = &[];
                Some(Err(e))
            }
        }
    }
}

// endregion: Iterator impl

// region: FrameRead impl

/// `FrameIter` implements `FrameRead` for use as a trailing field in
/// derived structs. Consumes all remaining bytes from the cursor.
impl<'a, T> FrameRead<'a> for FrameIter<'a, T>
where
    T: FrameRead<'a>,
{
    type Error = T::Error;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let slice = *buf;
        *buf = &buf[buf.len()..];
        Ok(Self::new(slice))
    }
}

// endregion: FrameRead impl

// region: FrameWrite impl

/// Encodes by writing the raw underlying bytes directly.
///
/// `FrameIter` stores the original encoded bytes verbatim - encoding simply
/// copies them out. No re-decode/re-encode cycle is needed or performed.
/// No bounds on `T` required since the bytes are written as-is.
impl<'a, T> FrameWrite for FrameIter<'a, T> {
    type Error = DiagError;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        buf.write_bytes(self.as_slice())
    }
}

// endregion: FrameWrite impl
