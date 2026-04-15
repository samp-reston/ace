mod attrs;
mod read;
mod util;
mod write;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derives `ace_uds::codec::FrameRead` for a struct or enum.
///
/// # Required container attribute
///
/// ```rust
/// #[frame(error = "UdsError")]
/// ```
///
/// The error type must implement `Into<DiagError>`.
///
/// # Struct field attributes
///
/// ## `#[frame(length = "expr")]`
///
/// Reads exactly `expr` bytes into a `&'a [u8]` field via `take_n`.
/// The expression is emitted verbatim and must evaluate to `usize`.
/// May reference any field declared before this one by name.
///
/// ```ignore
/// #[derive(FrameRead)]
/// #[frame(error = "UdsError")]
/// pub struct ReadMemoryByAddressRequest<'a> {
///     pub address_and_length_format_identifier: u8,
///     #[frame(length = "(address_and_length_format_identifier & 0x0F) as usize")]
///     pub memory_address: &'a [u8],
///     #[frame(length = "(address_and_length_format_identifier >> 4) as usize")]
///     pub memory_size: &'a [u8],
/// }
/// ```
///
/// ## `#[frame(read_all)]`
///
/// Consumes all remaining bytes into this field. Must be the last field.
/// Valid for `&'a [u8]` and `Option<&'a [u8]>`.
/// Mutually exclusive with `length`.
///
/// ```ignore
/// #[derive(FrameRead)]
/// #[frame(error = "UdsError")]
/// pub struct ControlDTCSettingRequest<'a> {
///     pub dtc_setting_type: DTCSettingType,
///     #[frame(read_all)]
///     pub dtc_setting_control_option_record: Option<&'a [u8]>,
/// }
/// ```
///
/// ## `#[frame(skip)]`
///
/// Skips this field - receives `Default::default()`, cursor not advanced.
/// Cannot be combined with `read_all` or `length`.
///
/// # Field types with no attribute needed
///
/// The following types implement `FrameRead` and are decoded automatically
/// without any `#[frame(...)]` attribute:
///
/// - `u8`, `u16`, `u32` - big-endian
/// - `[u8; N]` - fixed-size array
/// - `&'a [u8]` - consumes all remaining bytes (trailing field only)
/// - `Option<T: FrameRead>` - `None` if empty, `Some` otherwise
/// - `FrameIter<'a, T: FrameRead>` - lazy iterator over remaining bytes
/// - Any type implementing `FrameRead`
///
/// # Enum variant attributes
///
/// ## `#[frame(id = "0x01")]`
///
/// Exact discriminant byte. Reads one byte, matches this value, then
/// decodes the inner type from remaining bytes. Valid for newtype and
/// unit variants.
///
/// ## `#[frame(id_pat = "0x80..=0xFF")]`
///
/// Range or catch-all pattern. Variant must be a `u8` newtype - the raw
/// discriminant byte is passed directly as the inner value.
///
/// ```ignore
/// #[derive(FrameRead)]
/// #[frame(error = "UdsError")]
/// pub enum DTCSettingType {
///     #[frame(id = "0x01")] On,
///     #[frame(id = "0x02")] Off,
///     #[frame(id_pat = "0x03..=0xFF")] Reserved(u8),
/// }
/// ```
#[proc_macro_derive(FrameRead, attributes(frame))]
pub fn frame_read(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    read::derive(input).into()
}

/// Derives `ace_core::codec::FrameWrite` for a struct or enum.
///
/// Generates both encode paths gated by the `alloc` feature:
/// - `#[cfg(not(feature = "alloc"))]` - writes into `&mut &mut [u8]` cursor
/// - `#[cfg(feature = "alloc")]` - appends into `bytes::BytesMut`
///
/// All non-skipped fields are encoded in declaration order by delegating
/// to each field type's `FrameWrite` impl. `#[frame(length = "...")]` and
/// `#[frame(read_all)]` are decode-only hints and have no effect on encode.
///
/// Supports the same container, field, and variant attributes as `FrameRead`.
#[proc_macro_derive(FrameWrite, attributes(frame))]
pub fn frame_write(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    write::derive(input).into()
}

/// Derives both `FrameRead` and `FrameWrite`.
///
/// Convenience derive equivalent to `#[derive(FrameRead, FrameWrite)]`.
/// Use when a type participates in both decode and encode paths.
///
/// ```ignore
/// #[derive(FrameCodec)]
/// #[frame(error = "UdsError")]
/// pub struct DiagnosticSessionControlRequest {
///     pub session_type: SessionType,
/// }
/// ```
#[proc_macro_derive(FrameCodec, attributes(frame))]
pub fn frame_codec(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let read = read::derive(input.clone());
    let write = write::derive(input);
    quote::quote! { #read #write }.into()
}

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs")
}

#[test]
fn compile_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_pass/*.rs")
}
