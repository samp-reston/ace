use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{get_repr, ContainerAttrs, FieldAttrs, VariantAttrs};
use crate::util::{compile_error, has_lifetime};

// region: Entry point

pub fn derive(input: DeriveInput) -> TokenStream {
    let container = match ContainerAttrs::from_derive_input(&input) {
        Ok(c) => c,
        Err(e) => return e.write_errors(),
    };

    match &input.data {
        Data::Struct(data) => derive_struct(&input, &container, data),
        Data::Enum(data) => derive_enum(&input, &container, data),
        Data::Union(_) => compile_error(
            proc_macro2::Span::call_site(),
            "FrameRead cannot be derived for unions",
        ),
    }
}

// endregion: Entry point

// region: Struct

fn derive_struct(
    input: &DeriveInput,
    container: &ContainerAttrs,
    data: &syn::DataStruct,
) -> TokenStream {
    let name = &input.ident;
    let error = &container.error;

    let named = match &data.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return compile_error(
                proc_macro2::Span::call_site(),
                "FrameRead requires named fields",
            )
        }
    };

    let mut stmts: Vec<TokenStream> = Vec::new();
    let mut field_names: Vec<syn::Ident> = Vec::new();

    for field in named {
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        let attrs: FieldAttrs = match darling::FromField::from_field(field) {
            Ok(a) => a,
            Err(e) => return e.write_errors(),
        };

        if let Err(e) = attrs.validate(&ident.to_string()) {
            return e.write_errors();
        }

        field_names.push(ident.clone());

        let stmt = if attrs.skip {
            // Skipped - receive Default::default(), cursor not advanced
            quote! {
                let #ident: #ty = Default::default();
            }
        } else if let Some(length_expr) = &attrs.length {
            // Computed length - take_n returns DiagError directly,
            // container must implement From<DiagError>.
            quote! {
                let #ident: #ty = ace_core::codec::take_n(
                    buf,
                    (#length_expr) as usize,
                ).map_err(|e| <#error as From<ace_core::DiagError>>::from(e))?;
            }
        } else if attrs.read_all {
            // Consume all remaining bytes - no error possible
            quote! {
                let #ident: #ty = {
                    let __slice = *buf;
                    *buf = &buf[buf.len()..];
                    __slice
                };
            }
        } else {
            // Default - delegate to the field type's FrameRead impl.
            // Direct routing: FieldError -> ContainerError via From<FieldError>.
            // No intermediate DiagError - container error only needs to implement
            // From for each field error type it encounters.
            quote! {
                let #ident = <#ty as ace_core::codec::FrameRead>::decode(buf)
                    .map_err(|e| <#error as From<_>>::from(e))?;
            }
        };

        stmts.push(stmt);
    }

    let needs_lt = has_lifetime(&input.generics);
    let (ig, tg, wc) = input.generics.split_for_impl();
    let decode_lt = if needs_lt {
        quote! { 'a }
    } else {
        quote! { '_ }
    };

    quote! {
        impl #ig ace_core::codec::FrameRead<#decode_lt> for #name #tg #wc {
            type Error = #error;

            fn decode(buf: &mut &#decode_lt [u8]) -> Result<Self, Self::Error> {
                #(#stmts)*
                Ok(Self {
                    #(#field_names),*
                })
            }
        }
    }
}

// endregion: Struct

// region: Enum

fn derive_enum(
    input: &DeriveInput,
    container: &ContainerAttrs,
    data: &syn::DataEnum,
) -> TokenStream {
    let name = &input.ident;
    let error = &container.error;
    let disc_ty = get_repr(input);

    let mut arms: Vec<TokenStream> = Vec::new();

    for variant in &data.variants {
        let vname = &variant.ident;

        let attrs: VariantAttrs = match darling::FromVariant::from_variant(variant) {
            Ok(a) => a,
            Err(e) => return e.write_errors(),
        };

        if let Err(e) = attrs.validate(&vname.to_string()) {
            return e.write_errors();
        }

        let arm = match &variant.fields {
            // Newtype variant - decode inner type from remaining buffer
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                let inner_ty = &f.unnamed.first().unwrap().ty;

                if let Some(id_expr) = &attrs.id {
                    quote! {
                        __disc if __disc == (#id_expr) as #disc_ty => {
                            Ok(#name::#vname(
                                <#inner_ty as ace_core::codec::FrameRead>::decode(buf)
                                    .map_err(|e| <#error as From<_>>::from(e))?
                            ))
                        }
                    }
                } else {
                    // id_pat - raw discriminant byte passed directly into newtype(u8).
                    // Uses matches!() guard style so all arms bind __disc consistently -
                    // mixing `__disc @ pat` with `__disc if __disc == x` arms causes E0408.
                    // matches!() also correctly handles compound patterns like
                    // `0x05..=0x3F | 0x7F` that would otherwise need special parsing.
                    let pat: TokenStream = attrs
                        .id_pat
                        .as_ref()
                        .unwrap()
                        .value()
                        .parse()
                        .unwrap_or_else(|_| {
                            compile_error(proc_macro2::Span::call_site(), "invalid id_pat pattern")
                        });
                    if attrs.decode_inner {
                        quote! {
                            __disc if matches!(__disc, #pat) => {
                                Ok(#name::#vname(
                                    <#inner_ty as ace_core::codec::FrameRead>::decode(&mut { __saved }).map_err(|e| <#error as From<_>>::from(e))?
                                ))
                            }
                        }
                    } else {
                        quote! {
                                __disc if matches!(__disc, #pat) => Ok(#name::#vname(__disc)),
                        }
                    }
                }
            }

            // Unit variant - no inner data to decode
            Fields::Unit => {
                let id_expr = attrs.id.as_ref().unwrap();
                quote! {
                    __disc if __disc == (#id_expr) as #disc_ty => Ok(#name::#vname),
                }
            }

            _ => {
                return compile_error(
                    proc_macro2::Span::call_site(),
                    "FrameRead enum variants must be newtype(T) or unit",
                )
            }
        };

        arms.push(arm);
    }

    // Always emit a catch-all error arm. Even when an id_pat variant is present
    // it may not cover the full u8 range - the catch-all handles any remaining
    // values and ensures the match is exhaustive.
    // Routes through DiagError since unknown discriminant is always a transport-
    // level framing error - container must implement From<DiagError>.
    arms.push(quote! {
        _ => Err(<#error as From<ace_core::DiagError>>::from(
            ace_core::DiagError::InvalidFrame(
                heapless::String::try_from("unknown discriminant").unwrap_or_default()
            )
        )),
    });

    // Enums never borrow from the buffer directly - only use 'a if the enum
    // itself has lifetime parameters (e.g. newtype variants wrapping borrowed types).
    // Never inject 'a via with_lifetime_a - that would add a spurious lifetime
    // to types like ServiceIdentifier that have no lifetime parameters at all.
    let needs_lt = has_lifetime(&input.generics);
    let (ig, tg, wc) = input.generics.split_for_impl();
    let decode_lt = if needs_lt {
        quote! { 'a }
    } else {
        quote! { '_ }
    };

    quote! {
        impl #ig ace_core::codec::FrameRead<#decode_lt> for #name #tg #wc {
            type Error = #error;

            fn decode(buf: &mut &#decode_lt [u8]) -> Result<Self, Self::Error> {
                let __saved = *buf;
                let __disc = <#disc_ty as ace_core::codec::FrameRead>::decode(buf)
                    .map_err(|e| <#error as From<ace_core::DiagError>>::from(e))?;
                match __disc {
                    #(#arms)*
                }
            }
        }
    }
}

// endregion: Enum
