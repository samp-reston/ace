use crate::DiagError;

// region: FrameRead

/// Decodes `Self` from a zero-copy buffer cursor.
///
/// The cursor `buf` is a `&mut &'a [u8]` - a mutable reference to a shared
/// slice. Each call to `decode` advances the cursor by consuming bytes from
/// the front. The lifetime `'a` ties any borrowed data in `Self` back to
/// the original buffer, ensuring zero allocation for slice fields.
///
/// # Implementing for custom types
///
/// Most types should use `#[derive(FrameRead)]` from `ace-macros`.
/// Manual implementation is required when field lengths depend on context
/// from a parent struct - use [`FrameReadWithContext`] in those cases.
///
/// # Blanket impls provided
///
/// - `u8`, `u16`, `u32` - big-endian
/// - `[u8; N]` - fixed-size arrays
/// - `&'a [u8]` - consumes all remaining bytes
/// - `Option<T: FrameRead>` - `None` if buffer empty, `Some(T::decode(...))` otherwise
/// - `FrameIter<'a, T: FrameRead>` - lazy iterator consuming all remaining bytes
pub trait FrameRead<'a>: Sized {
    type Error: core::fmt::Debug;
    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error>;
}

// endregion: FrameRead

// region: Writer

mod sealed {
    pub trait Sealed {}
}

/// Abstraction over alloc and no-alloc write targets.
///
/// Implemented for:
/// - `&mut [u8]` - no-alloc, fixed-size buffer, advances cursor on write
/// - `bytes::BytesMut` - alloc, growable buffer (feature = "alloc")
///
/// Sealed to prevent downstream implementations.
pub trait Writer: sealed::Sealed {
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), DiagError>;
}

impl sealed::Sealed for &mut [u8] {}

impl Writer for &mut [u8] {
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), DiagError> {
        if self.len() < data.len() {
            return Err(DiagError::BufferOverflow);
        }
        self[..data.len()].copy_from_slice(data);
        let tmp = core::mem::take(self);
        *self = &mut tmp[data.len()..];
        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl sealed::Sealed for bytes::BytesMut {}

#[cfg(feature = "alloc")]
impl Writer for bytes::BytesMut {
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), DiagError> {
        self.extend_from_slice(data);
        Ok(())
    }
}

// endregion: Writer

// region: FrameWrite

/// Encodes `Self` into a [`Writer`].
///
/// A single generic method covers both alloc (`BytesMut`) and no-alloc
/// (`&mut [u8]`) targets - no cfg splits required in implementations.
///
/// Most types should use `#[derive(FrameWrite)]` from `ace-macros`.
pub trait FrameWrite {
    type Error: core::fmt::Debug;
    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error>;
}

// endregion: FrameWrite

// region: FrameCodec

/// Composite trait for types that implement both [`FrameRead`] and [`FrameWrite`]
/// with the same error type.
///
/// A blanket impl covers all types satisfying both bounds - there is no need
/// to implement this trait manually.
pub trait FrameCodec<'a>: FrameRead<'a, Error = <Self as FrameWrite>::Error> + FrameWrite {}

impl<'a, T> FrameCodec<'a> for T
where
    T: FrameRead<'a> + FrameWrite,
    T: FrameRead<'a, Error = <T as FrameWrite>::Error>,
{
}

// endregion: FrameCodec

// region: Primitive FrameRead impls

impl<'a> FrameRead<'a> for u8 {
    type Error = DiagError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let val = buf.first().copied().ok_or(DiagError::LengthMismatch {
            expected: 1,
            actual: 0,
        })?;
        *buf = &buf[1..];
        Ok(val)
    }
}

impl<'a> FrameRead<'a> for u16 {
    type Error = DiagError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        if buf.len() < 2 {
            return Err(DiagError::LengthMismatch {
                expected: 2,
                actual: buf.len(),
            });
        }
        let val = u16::from_be_bytes([buf[0], buf[1]]);
        *buf = &buf[2..];
        Ok(val)
    }
}

impl<'a> FrameRead<'a> for u32 {
    type Error = DiagError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        if buf.len() < 4 {
            return Err(DiagError::LengthMismatch {
                expected: 4,
                actual: buf.len(),
            });
        }
        let val = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        *buf = &buf[4..];
        Ok(val)
    }
}

impl<'a, const N: usize> FrameRead<'a> for [u8; N] {
    type Error = DiagError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        if buf.len() < N {
            return Err(DiagError::LengthMismatch {
                expected: N,
                actual: buf.len(),
            });
        }
        let mut arr = [0u8; N];
        arr.copy_from_slice(&buf[..N]);
        *buf = &buf[N..];
        Ok(arr)
    }
}

/// Consumes all remaining bytes - only valid as a trailing field.
impl<'a> FrameRead<'a> for &'a [u8] {
    type Error = DiagError;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        let slice = *buf;
        *buf = &buf[buf.len()..];
        Ok(slice)
    }
}

/// `None` if buffer is empty, `Some(T::decode(...))` otherwise.
impl<'a, T> FrameRead<'a> for Option<T>
where
    T: FrameRead<'a>,
{
    type Error = T::Error;

    fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
        if buf.is_empty() {
            Ok(None)
        } else {
            Ok(Some(T::decode(buf)?))
        }
    }
}

// endregion: Primitive FrameRead impls

// region: Primitive FrameWrite impls

impl FrameWrite for u8 {
    type Error = DiagError;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        buf.write_bytes(&[*self])
    }
}

impl FrameWrite for u16 {
    type Error = DiagError;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        buf.write_bytes(&self.to_be_bytes())
    }
}

impl FrameWrite for u32 {
    type Error = DiagError;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        buf.write_bytes(&self.to_be_bytes())
    }
}

impl<const N: usize> FrameWrite for [u8; N] {
    type Error = DiagError;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        buf.write_bytes(self.as_ref())
    }
}

impl FrameWrite for &[u8] {
    type Error = DiagError;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        buf.write_bytes(self)
    }
}

impl<T: FrameWrite> FrameWrite for Option<T> {
    type Error = T::Error;

    fn encode<W: Writer>(&self, buf: &mut W) -> Result<(), Self::Error> {
        if let Some(inner) = self {
            inner.encode(buf)?;
        }
        Ok(())
    }
}

// endregion: Primitive FrameWrite impls

// region: Free functions

/// Consumes exactly `n` bytes from the cursor, returning a zero-copy slice.
pub fn take_n<'a>(buf: &mut &'a [u8], n: usize) -> Result<&'a [u8], DiagError> {
    if buf.len() < n {
        return Err(DiagError::LengthMismatch {
            expected: n,
            actual: buf.len(),
        });
    }
    let slice = &buf[..n];
    *buf = &buf[n..];
    Ok(slice)
}

// endregion: Free functions
