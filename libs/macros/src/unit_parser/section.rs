//! Functions for generating section parsing expressions.
use crate::unit_parser::entry::{gen_entry_default, gen_entry_ensure, gen_entry_parse};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Error, Field, Result};

/// Generate [UnitSection] implementations when using `#[derive(UnitSection)]`
pub fn gen_section_derives(input: DeriveInput) -> Result<TokenStream> {
    let mut entry_ensures = Vec::new();
    let mut entry_defaults = Vec::new();
    let mut entry_parsers = Vec::new();
    let mut entries = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        for entry in &data_struct.fields {
            entry_ensures.push(gen_entry_ensure(entry)?);
            entry_defaults.push(gen_entry_default(entry)?);
            entry_parsers.push(gen_entry_parse(entry)?);

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
            fn __parse_section(__source: &mut unit_parser::internal::SectionParser, __res: &mut Self) -> unit_parser::internal::Result<()> {
                # ( #entry_ensures )*
                loop {
                    let __pair = match __source.next() {
                        None => break,
                        Some(v) => v,
                    };

                    match __pair.0 {
                        #( #entry_parsers ),*
                        _ => {
                            log::warn!("{} is not a valid key.", __pair.0);
                        }
                    }
                }
                Ok(())
            }

            fn __load_default(__res: &mut Self) {
                #( #entry_defaults )*
            }
        }
    };

    Ok(result)
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
pub(crate) fn gen_section_parse(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let ty = &field.ty;

    let key = format!("{}", name).into_token_stream();

    let result = quote! {
        #key => {
            const _: fn() = || {
                fn assert_impl<T: Default>() {}
                assert_impl::<#ty>();
            };
            let _ = unit_parser::internal::UnitSection::__parse_section(&mut __section, &mut __res.#name);
        }
    };

    Ok(result)
}

/// Generate statements that ensure the given struct implements [UnitSection]
/// by calling a function with generic constraints.
pub(crate) fn gen_section_ensure(field: &Field) -> Result<TokenStream> {
    let ty = &field.ty;
    Ok(quote! {
        const _: fn() = || {
            fn assert_impl<T: unit_parser::internal::UnitSection>() {}
            assert_impl::<#ty>();
        };
    })
}

pub(crate) fn gen_section_default(field: &Field) -> Result<TokenStream> {
    let name = &field.ident.as_ref().unwrap();
    Ok(quote! {
        let _ = unit_parser::internal::UnitSection::__load_default(&mut __res.#name);
    })
}
