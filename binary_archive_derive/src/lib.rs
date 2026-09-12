//! Derive implementation for `binary_archive`. Enable its `derive` feature.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, GenericParam, LitInt, parse_macro_input, parse_quote};

mod versioned;

/// Generates `BinaryEncode` and `BinaryDecode` for a struct.
#[proc_macro_derive(BinaryArchive, attributes(binary_archive))]
pub fn derive_binary_archive(input: TokenStream) -> TokenStream {
    expand(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let mut version = None;
    let mut versioned = false;
    for attr in &input.attrs {
        if attr.path().is_ident("binary_archive") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("versioned") {
                    if versioned {
                        return Err(meta.error("duplicate `versioned` setting"));
                    }
                    versioned = true;
                    return Ok(());
                }
                if !meta.path.is_ident("version") {
                    return Err(meta.error("expected `version = <u32 literal>` or `versioned`"));
                }
                if version.is_some() {
                    return Err(meta.error("duplicate archive version"));
                }
                version = Some(meta.value()?.parse::<LitInt>()?.base10_parse::<u32>()?);
                Ok(())
            })?;
        }
    }
    let version = version.unwrap_or(1);
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "BinaryArchive supports structs only",
        ));
    };
    // Field annotations opt into the new format; the explicit struct setting
    // is useful for version 1, where all fields inherit the struct version.
    versioned |= data.fields.iter().any(|field| {
        field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("binary_archive"))
    });
    let archive = match crate_name("binary_archive")
        .map_err(|err| syn::Error::new(Span::call_site(), err))?
    {
        FoundCrate::Itself => quote!(::binary_archive),
        FoundCrate::Name(name) => {
            let name = format_ident!("{}", name);
            quote!(::#name)
        }
    };
    let name = &input.ident;
    let members: Vec<_> = data
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            field
                .ident
                .clone()
                .map(syn::Member::Named)
                .unwrap_or_else(|| syn::Member::Unnamed(index.into()))
        })
        .collect();
    let field_types: Vec<_> = data.fields.iter().map(|field| &field.ty).collect();
    let values = field_types
        .iter()
        .map(|ty| quote!(__binary_archive_chunk.read::<#ty>()?));
    let construct = match &data.fields {
        Fields::Named(_) => quote!(Self { #(#members: #values),* }),
        Fields::Unnamed(_) => quote!(Self(#(#values),*)),
        Fields::Unit => quote!(Self),
    };

    let mut encode_generics = input.generics.clone();
    let mut decode_generics = input.generics.clone();
    for param in encode_generics.type_params_mut() {
        param.bounds.push(parse_quote!(#archive::BinaryEncode));
    }
    for param in decode_generics.type_params_mut() {
        param.bounds.push(parse_quote!(#archive::BinaryDecode));
    }
    let versioned_code = if versioned {
        let generated = versioned::generate(data, version, &archive, name)?;
        for ty in &generated.default_types {
            decode_generics
                .make_where_clause()
                .predicates
                .push(parse_quote!(#ty: ::std::default::Default));
        }
        Some(generated)
    } else {
        None
    };
    let (encode_body, decode_body) = if let Some(generated) = versioned_code {
        (generated.encode, generated.decode)
    } else {
        (
            quote! {
                writer.write_chunk(#version, |__binary_archive_chunk| {
                    #(__binary_archive_chunk.write(&self.#members)?;)*
                    Ok(())
                })?;
                Ok(())
            },
            quote! {
                let mut __binary_archive_chunk = reader.read_chunk()?;
                if __binary_archive_chunk.header().version != #version {
                    return Err(#archive::ArchiveError::InvalidData(::std::format!(
                        "unsupported {} version: expected {}, actual {}",
                        ::std::stringify!(#name), #version, __binary_archive_chunk.header().version
                    )));
                }
                let value = #construct;
                __binary_archive_chunk.finish()?;
                Ok(value)
            },
        )
    };
    let (encode_impl, _, encode_where) = encode_generics.split_for_impl();
    let (decode_impl, _, decode_where) = decode_generics.split_for_impl();
    let (_, type_generics, _) = input.generics.split_for_impl();
    // Method type parameters must not collide with user type/const parameters.
    let mut io_name = "__BinaryArchiveIo".to_owned();
    while input.generics.params.iter().any(|param| match param {
        GenericParam::Type(param) => param.ident == io_name,
        GenericParam::Const(param) => param.ident == io_name,
        GenericParam::Lifetime(_) => false,
    }) {
        io_name.push('_');
    }
    let io = format_ident!("{}", io_name);
    Ok(quote! {
        impl #encode_impl #archive::BinaryEncode for #name #type_generics #encode_where {
            fn encode<#io: ::std::io::Write + ::std::io::Seek>(
                &self, writer: &mut #archive::ArchiveWriter<#io>
            ) -> #archive::ArchiveResult<()> {
                #encode_body
            }
        }
        impl #decode_impl #archive::BinaryDecode for #name #type_generics #decode_where {
            fn decode<#io: ::std::io::Read + ::std::io::Seek>(
                reader: &mut #archive::ArchiveReader<#io>
            ) -> #archive::ArchiveResult<Self> {
                #decode_body
            }
        }
    })
}
