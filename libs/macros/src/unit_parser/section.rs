//! Functions for generating section parsing expressions.
use crate::{
    unit_conf_parse::get_option_inner_type,
    unit_parser::{
        attribute::SectionAttributes,
        entry::{
            gen_entry_ensure, gen_entry_finalize, gen_entry_init, gen_entry_parse, gen_entry_patch,
        },
    },
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Error, Field, Result};

/// Generate [UnitSection] implementations when using `#[derive(UnitSection)]`
pub fn gen_section_derives(input: DeriveInput) -> Result<TokenStream> {
    let mut entry_ensures = Vec::new();
    let mut entry_inits = Vec::new();
    let mut entry_parsers = Vec::new();
    let mut entry_finalizes = Vec::new();
    let mut entries = Vec::new();
    let mut entry_patches = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        for entry in &data_struct.fields {
            entry_ensures.push(gen_entry_ensure(entry)?);
            entry_inits.push(gen_entry_init(entry)?);
            entry_parsers.push(gen_entry_parse(entry)?);
            entry_finalizes.push(gen_entry_finalize(entry)?);
            entry_patches.push(gen_entry_patch(entry)?);

            let ident = entry.ident.as_ref().ok_or_else(|| {
                Error::new_spanned(&entry, "An entry must have an explicit name.")
            })?;
            entries.push(ident);
        }
    } else {
        return Err(Error::new_spanned(
            input,
            "A UnitSection cannot be an enum or an union.",
        ));
    }

    let ident = &input.ident;

    let result = quote! {
        impl unit_parser::internal::UnitSection for #ident {
            fn __parse_section(__source: &mut unit_parser::internal::SectionParser) -> unit_parser::internal::Result<Option<Self>> {
                # ( #entry_ensures )*
                # ( #entry_inits )*
                loop {
                    if let Some(__pair) = __source.next() {
                        match __pair.0 {
                            #( #entry_parsers ),*
                            _ => {
                                log::warn!("{} is not a valid key.", __pair.0);
                            }
                        }
                    } else {
                        break;
                    }
                }
                #( #entry_finalizes )*
                Ok(Some(Self {
                    #( #entries ),*
                }))
            }

            fn __patch_section(__source: &mut unit_parser::internal::SectionParser, __from: &mut Self) -> unit_parser::internal::Result<()> {
                # ( #entry_ensures )*
                # ( #entry_inits )*
                loop {
                    if let Some(__pair) = __source.next() {
                        match __pair.0 {
                            #( #entry_parsers ),*
                            _ => {
                                log::warn!("{} is not a valid key.", __pair.0);
                            }
                        }
                    } else {
                        break;
                    }
                }
                #( #entry_patches )*
                Ok(())
            }
        }
    };

    Ok(result)
}

/// Generate variable initialization statements.
/// example:
/// ```
/// let mut Section1 = None;
/// ```
pub(crate) fn gen_section_init(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    Ok(quote! {
        let mut #name = None;
    })
}

/// Generate section parsing statements, in the form of an arm in a `match` statement.
/// example:
/// ```
/// match section.name {
///     "Section1" => {
///         if let Ok(__value) = unit_parser::UnitSection::__parse_section(__section) {
///             if __value.is_some() {
///                 Section1 = __value;
///             }
///         }
///     }
/// }
/// ```
pub(crate) fn gen_section_parse(field: &Field) -> Result<(TokenStream, TokenStream)> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let ty = &field.ty;
    let attributes = SectionAttributes::parse_vec(field, Some(ty))?;
    let key = attributes
        .key
        .unwrap_or_else(|| (format!("{}", name)).into_token_stream());

    let result = match (attributes.default, attributes.must) {
        // invalid
        (true, true) => unreachable!(),
        // convert Error to Option
        (true, false) => (
            // ensure the struct implements `Default` by calling an function with generic constraints
            quote! {
                #key => {
                    const _: fn() = || {
                        fn assert_impl<T: Default>() {}
                        assert_impl::<#ty>();
                    };
                    if let Ok(__value) = unit_parser::internal::UnitSection::__parse_section(&mut __section) {
                        if __value.is_some() {
                            #name = __value;
                        }
                    } else {
                        log::warn!("Failed to parse section {}.", #key);
                    }
                }
            },
            quote! {
                #key => {
                    let __section_partial = &mut __from.#name;
                    if let Err(_)= unit_parser::internal::UnitSection::__patch_section(&mut __section, __section_partial) {
                        log::warn!("Failed to parse section {}, skipping.", #key)
                    }
                }
            },
        ),
        // convert Error to Option
        (false, false) => (
            quote! {
                #key => {
                    if let Ok(__value) = unit_parser::internal::UnitSection::__parse_section(&mut __section) {
                        #name = __value;
                    } else {
                        log::warn!("Failed to parse section {}, skipping.", #key);
                    }
                }
            },
            quote! {
                #key => {
                    if let Some(__section_partial) = &mut __from.#name {
                        if let Err(_) = unit_parser::internal::UnitSection::__patch_section(&mut __section, __section_partial) {
                            log::warn!("Failed to patch section {}, skipping.", #key);
                        }
                    } else {
                        if let Ok(__inner) = unit_parser::internal::UnitSection::__parse_section(&mut __section) {
                            __from.#name = __inner;
                        }
                    }
               }
            },
        ),
        // throw Error
        (false, true) => (
            quote! {
                #key => {
                    let __value = unit_parser::internal::UnitSection::__parse_section(&mut __section)?
                        .ok_or_else(||unit_parser::internal::Error::SectionParsingError{ key: #key.to_string() })?;
                    #name = Some(__value);
                }
            },
            quote! {
                #key => {
                    let __section_partial = &mut __from.#name;
                    unit_parser::internal::UnitSection::__patch_section(&mut __section, __section_partial)
                        .map_err(|_| unit_parser::internal::Error::SectionParsingError{ key: #key.to_string() })?;                }
            },
        ),
    };

    Ok(result)
}

/// Generate statements that ensure the given struct implements [UnitSection]
/// by calling a function with generic constraints.
pub(crate) fn gen_section_ensure(field: &Field) -> Result<TokenStream> {
    let mut ty = &field.ty;
    let attribute = SectionAttributes::parse_vec(field, None)?;
    if (!attribute.must) & (!attribute.default) {
        ty = get_option_inner_type(ty).unwrap();
    }
    Ok(quote! {
        const _: fn() = || {
            fn assert_impl<T: UnitSection>() {}
            assert_impl::<#ty>();
        };
    })
}

/// Generate finalization statements which are in charge of processing [Option] and [Result]s during parsing.
pub(crate) fn gen_section_finalize(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let ty = &field.ty;
    let attributes = SectionAttributes::parse_vec(field, None)?;
    let key = attributes
        .key
        .unwrap_or_else(|| (format!("{}", name)).into_token_stream());

    let result = match (attributes.default, attributes.must) {
        (true, true) => unreachable!(),
        // fallback to default
        (true, false) => {
            quote! {
                let #name: #ty = #name.unwrap_or(Default::default());
            }
        }
        // throw Error
        (false, true) => {
            quote! {
                let #name = #name.ok_or_else(||unit_parser::internal::Error::SectionMissingError { key: #key.to_string()})?;
            }
        }
        // leave unchanged
        (false, false) => {
            quote! {}
        }
    };

    Ok(result)
}

/// Generate patching statements that sets each field to a new value, if present.
pub(crate) fn gen_section_patches(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let attributes = SectionAttributes::parse_vec(field, None)?;

    let result = match (attributes.must, attributes.default) {
        // invalid
        (true, true) => unreachable!(),
        // unwrap inner if new value is Some
        (true, false) | (false, true) => {
            quote! {
                 if let Some(__inner) = #name {
                     __from.#name = __inner;
                 }
            }
        }
        // set to new value if new value is Some
        (false, false) => {
            quote! {
                if #name.is_some() {
                    __from.#name = #name;
                }
            }
        }
    };

    Ok(result)
}
