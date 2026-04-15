use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::attrs::{get_repr, ContainerAttrs, FieldAttrs, VariantAttrs};
use crate::util::compile_error;

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
            "FrameWrite cannot be derived for unions",
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
                "FrameWrite requires named fields",
            )
        }
    };

    let mut stmts: Vec<TokenStream> = Vec::new();

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

        // Skipped fields contribute nothing to encode
        if attrs.skip {
            continue;
        }

        // `length` and `read_all` are decode-only hints - encode always
        // delegates to the field type's FrameWrite impl in declaration order.
        // Direct error routing: FieldError -> ContainerError via From<FieldError>.
        stmts.push(quote! {
            <#ty as ace_core::codec::FrameWrite>::encode(&self.#ident, buf)
                .map_err(|e| <#error as From<_>>::from(e))?;
        });
    }

    let (ig, tg, wc) = input.generics.split_for_impl();

    quote! {
        impl #ig ace_core::codec::FrameWrite for #name #tg #wc {
            type Error = #error;

            fn encode<__W: ace_core::codec::Writer>(&self, buf: &mut __W) -> Result<(), Self::Error> {
                #(#stmts)*
                Ok(())
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

        match &variant.fields {
            // Newtype variant - write discriminant then delegate to inner type
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                let inner_ty = &f.unnamed.first().unwrap().ty;

                if let Some(id_expr) = &attrs.id {
                    arms.push(quote! {
                        #name::#vname(inner) => {
                            <#disc_ty as ace_core::codec::FrameWrite>::encode(&(#id_expr as #disc_ty), buf)
                                .map_err(|e| <#error as From<ace_core::DiagError>>::from(e))?;
                            <#inner_ty as ace_core::codec::FrameWrite>::encode(inner, buf)
                                .map_err(|e| <#error as From<_>>::from(e))?;
                        }
                    });
                } else if attrs.decode_inner {
                    arms.push(quote! {
                        #name::#vname(inner) => {
                            <#inner_ty as ace_core::codec::FrameWrite>::encode(inner, buf)
                                .map_err(|e| <#error as From<_>>::from(e))?;
                        }
                    });
                } else {
                    // id_pat - inner value IS the raw byte, write it directly
                    arms.push(quote! {
                        #name::#vname(raw) => {
                            <#disc_ty as ace_core::codec::FrameWrite>::encode(raw, buf)
                                .map_err(|e| <#error as From<ace_core::DiagError>>::from(e))?;
                        }
                    });
                }
            }

            // Unit variant - write discriminant byte only
            Fields::Unit => {
                let id_expr = attrs.id.as_ref().unwrap();
                arms.push(quote! {
                    #name::#vname => {
                        <#disc_ty as ace_core::codec::FrameWrite>::encode(&(#id_expr as #disc_ty), buf)
                            .map_err(|e| <#error as From<ace_core::DiagError>>::from(e))?;
                    }
                });
            }

            _ => {
                return compile_error(
                    proc_macro2::Span::call_site(),
                    "FrameWrite enum variants must be newtype(T) or unit",
                )
            }
        }
    }

    let (ig, tg, wc) = input.generics.split_for_impl();

    quote! {
        impl #ig ace_core::codec::FrameWrite for #name #tg #wc {
            type Error = #error;

            fn encode<__W: ace_core::codec::Writer>(&self, buf: &mut __W) -> Result<(), Self::Error> {
                match self {
                    #(#arms)*
                }
                Ok(())
            }
        }
    }
}

// endregion: Enum
