use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{DataStruct, Expr, Fields, Ident, LitInt, LitStr, Member, Type};

pub(super) struct Generated {
    pub encode: TokenStream,
    pub decode: TokenStream,
    pub default_types: Vec<Type>,
}

pub(super) fn generate(
    data: &DataStruct,
    struct_version: u32,
    archive: &TokenStream,
    struct_name: &Ident,
) -> syn::Result<Generated> {
    let mut writes = Vec::new();
    let mut declarations = Vec::new();
    let mut arms = Vec::new();
    let mut values = Vec::new();
    let mut members = Vec::new();
    let mut default_types = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for (index, field) in data.fields.iter().enumerate() {
        let mut version = None;
        let mut id = None;
        let mut default = None;
        for attr in &field.attrs {
            if attr.path().is_ident("binary_archive") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("version") {
                        if version.is_some() {
                            return Err(meta.error("duplicate field version"));
                        }
                        version = Some(meta.value()?.parse::<LitInt>()?.base10_parse::<u32>()?);
                    } else if meta.path.is_ident("id") {
                        if id.is_some() {
                            return Err(meta.error("duplicate field id"));
                        }
                        id = Some(meta.value()?.parse::<LitStr>()?.value());
                    } else if meta.path.is_ident("default") {
                        if default.is_some() {
                            return Err(meta.error("duplicate field default"));
                        }
                        default = Some(meta.value()?.parse::<Expr>()?);
                    } else {
                        return Err(meta.error("expected field `version`, `id`, or `default`"));
                    }
                    Ok(())
                })?;
            }
        }
        let version = version.unwrap_or(struct_version);
        if version > struct_version {
            return Err(syn::Error::new_spanned(
                field,
                "field version exceeds struct version",
            ));
        }
        let id = id.unwrap_or_else(|| {
            field
                .ident
                .as_ref()
                .map(|id| id.unraw().to_string())
                .unwrap_or_else(|| index.to_string())
        });
        if id.is_empty() || !ids.insert(id.clone()) {
            return Err(syn::Error::new_spanned(
                field,
                "field ids must be nonempty and unique",
            ));
        }
        let ty = &field.ty;
        let default = match default {
            Some(value) => quote!(#value),
            None => {
                default_types.push(ty.clone());
                quote!(<#ty as ::std::default::Default>::default())
            }
        };
        let member = field
            .ident
            .clone()
            .map(Member::Named)
            .unwrap_or_else(|| Member::Unnamed(index.into()));
        let slot = format_ident!("__binary_archive_field_{index}");
        declarations
            .push(quote!(let mut #slot: ::std::option::Option<#ty> = ::std::option::Option::None;));
        writes.push(
            quote!(__binary_archive_chunk.write_versioned_field(#id, #version, &self.#member)?;),
        );
        arms.push(quote! {
            #id => {
                if __binary_archive_field_version != #version {
                    return Err(#archive::ArchiveError::InvalidData(::std::format!(
                        "field {}.{} introduction version changed: expected {}, actual {}",
                        ::std::stringify!(#struct_name), #id, #version, __binary_archive_field_version
                    )));
                }
                #slot = ::std::option::Option::Some(__binary_archive_field.read::<#ty>()?);
                __binary_archive_field.finish()?;
            }
        });
        values.push(quote! {
            match #slot {
                ::std::option::Option::Some(value) => value,
                ::std::option::Option::None => {
                    if __binary_archive_stored_version >= #version {
                        return Err(#archive::ArchiveError::InvalidData(::std::format!(
                            "missing field {}.{} in stored schema version {}",
                            ::std::stringify!(#struct_name), #id, __binary_archive_stored_version
                        )));
                    }
                    #default
                }
            }
        });
        members.push(member);
    }
    let construct = match &data.fields {
        Fields::Named(_) => quote!(Self { #(#members: #values),* }),
        Fields::Unnamed(_) => quote!(Self(#(#values),*)),
        Fields::Unit => quote!(Self),
    };
    Ok(Generated {
        default_types,
        encode: quote! {
            writer.write_chunk(#struct_version, |__binary_archive_chunk| {
                __binary_archive_chunk.write(b"BARFLD01")?;
                #(#writes)*
                Ok(())
            })?;
            Ok(())
        },
        decode: quote! {
            let mut __binary_archive_chunk = reader.read_chunk()?;
            let __binary_archive_stored_version = __binary_archive_chunk.header().version;
            #(#declarations)*
            __binary_archive_chunk.read_versioned_fields(|__binary_archive_name, __binary_archive_field_version, __binary_archive_field| {
                match __binary_archive_name {
                    #(#arms)*
                    _ => __binary_archive_field.handle_deleted_field(
                        ::std::stringify!(#struct_name), __binary_archive_name, #struct_version
                    )?,
                }
                Ok(())
            })?;
            Ok(#construct)
        },
    })
}
