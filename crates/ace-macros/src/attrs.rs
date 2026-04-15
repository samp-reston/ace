use darling::{FromDeriveInput, FromField, FromVariant};
use syn::{DeriveInput, Expr};

// region: Container

/// Top-level `#[frame(...)]` attributes on a struct or enum.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(frame))]
pub struct ContainerAttrs {
    /// Error type for the generated impl. Must implement `Into<DiagError>`.
    ///
    /// ```ignore
    /// #[frame(error = "UdsError")]
    /// ```
    pub error: syn::Path,
}

pub fn get_repr(input: &DeriveInput) -> syn::Path {
    input
        .attrs
        .iter()
        .find(|a| a.path().is_ident("repr"))
        .and_then(|a| a.parse_args::<syn::Path>().ok())
        .unwrap_or_else(|| syn::parse_quote!(u8))
}

// endregion: Container

// region: Field

/// Per-field `#[frame(...)]` attributes on struct fields.
#[derive(Debug, Default, FromField)]
#[darling(attributes(frame), default)]
pub struct FieldAttrs {
    /// Consume all remaining bytes into this field.
    ///
    /// Valid for `&'a [u8]` and `Option<&'a [u8]>` trailing fields.
    /// Must be the last field in the struct.
    /// Mutually exclusive with `length`.
    ///
    /// ```ignore
    /// #[frame(read_all)]
    /// pub dtc_setting_control_option_record: Option<&'a [u8]>,
    /// ```
    pub read_all: bool,

    /// Derive the byte count for this field from an expression.
    ///
    /// The expression is emitted verbatim and must evaluate to `usize`.
    /// May only reference fields declared before this one.
    /// Mutually exclusive with `read_all`.
    ///
    /// ```ignore
    /// #[frame(length = "(address_and_length_format_identifier & 0x0F) as usize")]
    /// pub memory_address: &'a [u8],
    /// ```
    pub length: Option<Expr>,

    /// Skip this field in decode and encode.
    ///
    /// Field receives `Default::default()` on decode.
    /// Contributes nothing on encode.
    /// Field type must implement `Default`.
    /// Cannot be combined with `read_all` or `length`.
    pub skip: bool,
}

impl FieldAttrs {
    pub fn validate(&self, field_name: &str) -> Result<(), darling::Error> {
        if self.read_all && self.length.is_some() {
            return Err(darling::Error::custom(format!(
                "field `{field_name}`: `read_all` and `length` are mutually exclusive"
            )));
        }
        if self.skip && (self.read_all || self.length.is_some()) {
            return Err(darling::Error::custom(format!(
                "field `{field_name}`: `skip` cannot be combined with `read_all` or `length`"
            )));
        }
        Ok(())
    }
}

// endregion: Field

// region: Variant

/// Per-variant `#[frame(...)]` attributes on enum variants.
#[derive(Debug, Default, FromVariant)]
#[darling(attributes(frame), default)]
pub struct VariantAttrs {
    /// Exact discriminant byte for this variant.
    ///
    /// Used in decode as a match guard and in encode to write the
    /// discriminant byte before the inner type.
    ///
    /// ```ignore
    /// #[frame(id = "0x01")]
    /// On,
    /// ```
    pub id: Option<Expr>,

    /// Range or catch-all pattern for this variant.
    ///
    /// The variant must be a `u8` newtype - the raw discriminant byte
    /// is passed directly as the inner value on decode, and written
    /// directly on encode.
    ///
    /// ```ignore
    /// #[frame(id_pat = "0x80..=0xFF")]
    /// Reserved(u8),
    /// ```
    pub id_pat: Option<syn::LitStr>,

    /// Re-decode the inner type from the full buffer including the discriminant byte. Only
    /// valid for `id_pat` variants where the innertype implements `FrameRead`.
    pub decode_inner: bool,
}

impl VariantAttrs {
    pub fn validate(&self, variant_name: &str) -> Result<(), darling::Error> {
        match (&self.id, &self.id_pat) {
            (None, None) => Err(darling::Error::custom(format!(
                "variant `{variant_name}`: must have `#[frame(id = \"...\")]` or `#[frame(id_pat = \"...\")]`"
            ))),
            (Some(_), Some(_)) => Err(darling::Error::custom(format!(
                "variant `{variant_name}`: `id` and `id_pat` are mutually exclusive"
            ))),
            _ => Ok(()),
        }
    }
}

// endregion: Variant
