use darling::FromDeriveInput;
// storage.rs
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields};

use crate::{
    attrs::{ContainerAttrs, FieldAttrs},
    util::compile_error,
};

/// Generates the Owned struct, its FrameRead/FrameWrite impls, and ToOwned
/// for the borrowed struct. Called from read::derive_struct and
/// write::derive_struct when any field has `bytes` or `iter`.
pub fn derive(
    input: &DeriveInput,
    container: &ContainerAttrs,
    data: &syn::DataStruct,
) -> TokenStream {
    let name = &input.ident;
    let error = &container.error;
    let owned_name = quote::format_ident!("{}Owned", name);

    let named = match &data.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return compile_error(
                proc_macro2::Span::call_site(),
                "FrameCodec requires named fields",
            );
        }
    };

    // Collect derives from the original struct to forward to Owned
    let derives: Vec<_> = input
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .collect();

    let mut field_names: Vec<syn::Ident> = Vec::new();
    let mut owned_field_tys: Vec<TokenStream> = Vec::new();
    let mut owned_decode_stmts: Vec<TokenStream> = Vec::new();
    let mut owned_encode_stmts: Vec<TokenStream> = Vec::new();
    let mut to_owned_stmts: Vec<TokenStream> = Vec::new();

    for field in named {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        let attrs: FieldAttrs = match darling::FromField::from_field(field) {
            Ok(a) => a,
            Err(e) => return e.write_errors(),
        };

        field_names.push(ident.clone());

        if attrs.skip {
            owned_field_tys.push(quote! { pub #ident: #ty });
            owned_decode_stmts.push(quote! { let #ident: #ty = Default::default(); });
            owned_encode_stmts.push(quote! {});
            to_owned_stmts.push(quote! { #ident: self.#ident.clone() });
            continue;
        }

        if attrs.bytes {
            if let Some(length_expr) = &attrs.length {
                owned_decode_stmts.push(quote! {
                    let #ident: ::alloc::vec::Vec<u8> = ace_core::codec::take_n(
                        buf,
                        (#length_expr) as usize,
                    ).map_err(|e| <#error as From<ace_core::CodecError>>::from(e))?.to_vec();
                });
            } else if attrs.read_all {
                owned_decode_stmts.push(quote! {
                    let #ident: ::alloc::vec::Vec<u8> = {
                        let v = buf.to_vec();
                        *buf = &buf[buf.len()..];
                        v
                    };
                });
            } else {
                return compile_error(
                    proc_macro2::Span::call_site(),
                    "bytes fields must have either `length` or `read_all`",
                );
            }
            owned_field_tys.push(quote! { pub #ident: ::alloc::vec::Vec<u8> });
            owned_encode_stmts.push(quote! {
                ace_core::codec::FrameWrite::encode(&self.#ident, buf)
                    .map_err(|e| <#error as From<_>>::from(e))?;
            });
            to_owned_stmts.push(quote! { #ident: self.#ident.to_vec() });
        } else if attrs.iter {
            let inner_ty = extract_iter_inner(ty);
            owned_decode_stmts.push(quote! {
                let #ident: ::alloc::vec::Vec<#inner_ty> = {
                    let __slice = *buf;
                    *buf = &buf[buf.len()..];
                    ace_core::codec::FrameIter::<#inner_ty>::new(__slice)
                        .collect::<Result<::alloc::vec::Vec<_>, _>>()
                        .map_err(|e| <#error as From<_>>::from(e))?
                };
            });
            owned_field_tys.push(quote! { pub #ident: ::alloc::vec::Vec<#inner_ty> });
            owned_encode_stmts.push(quote! {
                for __item in &self.#ident {
                    ace_core::codec::FrameWrite::encode(__item, buf)
                        .map_err(|e| <#error as From<_>>::from(e))?;
                }
            });
            to_owned_stmts.push(quote! {
                #ident: self.#ident.clone()
                    .collect::<Result<::alloc::vec::Vec<_>, _>>()
                    .map_err(|e| <#error as From<_>>::from(e))?
            });
        } else {
            // Non-storage field - same in both, just clone for to_owned
            owned_field_tys.push(quote! { pub #ident: #ty });
            owned_decode_stmts.push(quote! {
                let #ident = <#ty as ace_core::codec::FrameRead>::decode(buf)
                    .map_err(|e| <#error as From<_>>::from(e))?;
            });
            owned_encode_stmts.push(quote! {
                <#ty as ace_core::codec::FrameWrite>::encode(&self.#ident, buf)
                    .map_err(|e| <#error as From<_>>::from(e))?;
            });
            to_owned_stmts.push(quote! { #ident: self.#ident.clone() });
        }
    }

    let (ig, tg, wc) = input.generics.split_for_impl();

    quote! {
        // Owned struct - alloc only
        #[cfg(feature = "alloc")]
        #(#derives)*
        pub struct #owned_name {
            #(#owned_field_tys),*
        }

        // FrameRead for Owned
        #[cfg(feature = "alloc")]
        impl<'a> ace_core::codec::FrameRead<'a> for #owned_name {
            type Error = #error;
            fn decode(buf: &mut &'a [u8]) -> Result<Self, Self::Error> {
                #(#owned_decode_stmts)*
                Ok(Self { #(#field_names),* })
            }
        }

        // FrameWrite for Owned
        #[cfg(feature = "alloc")]
        impl ace_core::codec::FrameWrite for #owned_name {
            type Error = #error;
            fn encode<__W: ace_core::codec::Writer>(
                &self,
                buf: &mut __W,
            ) -> Result<(), Self::Error> {
                #(#owned_encode_stmts)*
                Ok(())
            }
        }

        // ToOwned for the borrowed struct
        #[cfg(feature = "alloc")]
        impl #ig ace_core::codec::IntoOwned for #name #tg #wc {
            type Owned = #owned_name;

            fn into_owned(self) -> Self::Owned {
                #owned_name {
                    #(#to_owned_stmts),*
                }
            }
        }
    }
}

pub fn derive_if_needed(input: DeriveInput) -> TokenStream {
    let container = match ContainerAttrs::from_derive_input(&input) {
        Ok(c) => c,
        Err(e) => return e.write_errors(),
    };

    let data = match &input.data {
        syn::Data::Struct(d) => d,
        _ => return quote! {},
    };

    let named = match &data.fields {
        Fields::Named(f) => &f.named,
        _ => return quote! {},
    };

    let has_storage_fields = named.iter().any(|f| {
        darling::FromField::from_field(f)
            .map(|a: FieldAttrs| a.bytes || a.iter)
            .unwrap_or(false)
    });

    if !has_storage_fields {
        return quote! {};
    }

    derive(&input, &container, data)
}

fn extract_iter_inner(ty: &syn::Type) -> TokenStream {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                let inner = args
                    .args
                    .iter()
                    .filter_map(|a| {
                        if let syn::GenericArgument::Type(t) = a {
                            Some(quote! { #t })
                        } else {
                            None
                        }
                    })
                    .last();
                if let Some(t) = inner {
                    return t;
                }
            }
        }
    }
    compile_error(
        proc_macro2::Span::call_site(),
        "iter field must be FrameIter<'a, T>",
    )
}
