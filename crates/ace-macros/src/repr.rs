use darling::FromDeriveInput;
// repr.rs
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::{
    attrs::{ContainerAttrs, VariantAttrs, get_repr},
    util::{compile_error, has_lifetime},
};

pub fn derive(input: DeriveInput) -> TokenStream {
    if has_lifetime(&input.generics) {
        return quote! {};
    }

    let has_repr = input.attrs.iter().any(|a| a.path().is_ident("repr"));
    if !has_repr {
        return quote! {};
    }

    let data = match &input.data {
        Data::Enum(d) => d,
        _ => unreachable!("guarded in frame_codec"),
    };

    let name = &input.ident;
    let disc_ty = get_repr(&input);
    let container = match ContainerAttrs::from_derive_input(&input) {
        Ok(c) => c,
        Err(e) => return e.write_errors(),
    };

    let error = &container.error;

    let mut into_arms: Vec<TokenStream> = Vec::new();

    for variant in &data.variants {
        let vname = &variant.ident;

        let attrs: VariantAttrs = match darling::FromVariant::from_variant(variant) {
            Ok(a) => a,
            Err(e) => return e.write_errors(),
        };

        let arm = match &variant.fields {
            Fields::Unit => {
                let id_expr = attrs.id.as_ref().unwrap();
                quote! { #name::#vname => (#id_expr) as #disc_ty, }
            }
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                let inner_ty = &f.unnamed.first().unwrap().ty;

                if let Some(id_expr) = &attrs.id {
                    // id newtype - inner is decoded payload, discriminant is the literal
                    quote! { #name::#vname(_) => (#id_expr) as #disc_ty, }
                } else if attrs.decode_inner {
                    quote! { #name::#vname(inner) => <#disc_ty as From<#inner_ty>>::from(inner), }
                } else {
                    // id_pat - inner IS the raw discriminant value
                    quote! { #name::#vname(inner) => inner as #disc_ty, }
                }
            }
            _ => {
                return compile_error(
                    proc_macro2::Span::call_site(),
                    "FrameCodec repr conversions: variants must be unit or newtype(T)",
                );
            }
        };

        into_arms.push(arm);
    }

    quote! {
        impl From<#name> for #disc_ty {
            fn from(value: #name) -> Self {
                match value {
                    #(#into_arms)*
                }
            }
        }

        impl TryFrom<#disc_ty> for #name {
            type Error = #error;

            fn try_from(value: #disc_ty) -> Result<Self, Self::Error> {
                use ace_core::codec::FrameRead;
                let bytes = value.to_be_bytes();
                let mut buf = &bytes[..];
                Self::decode(&mut buf)
            }
        }
    }
}
