use proc_macro2::{Span, TokenStream};
use syn::Generics;

// region: Lifetime helpers

pub fn has_lifetime(generics: &Generics) -> bool {
    generics.lifetimes().next().is_some()
}

// endregion: Lifetime helpers

// region: Error helpers

pub fn compile_error(span: Span, msg: &str) -> TokenStream {
    let msg = msg.to_string();
    quote::quote_spanned! { span => compile_error!(#msg); }
}

// endregion: Error helpers
