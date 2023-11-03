//! Functions for generating unit parsing expressions.
use crate::unit_parser::{
    attribute::UnitAttributes,
    section::{gen_section_default, gen_section_ensure, gen_section_parse},
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Error};

/// Generate [UnitConfig] implementations when using `#[derive(UnitConfig)]`
pub fn gen_unit_derives(input: DeriveInput) -> syn::Result<TokenStream> {
    let attributes = UnitAttributes::parse_vec(&input.attrs)?;
    let mut sections = Vec::new();
    let mut section_ensures = Vec::new();
    let mut section_parsers = Vec::new();
    let mut section_defaults = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        for entry in &data_struct.fields {
            section_ensures.push(gen_section_ensure(entry)?);
            section_parsers.push(gen_section_parse(entry)?);
            section_defaults.push(gen_section_default(entry)?);
            let ident = entry.ident.as_ref().ok_or_else(|| {
                Error::new_spanned(&entry, "An entry must have an explicit name.")
            })?;
            sections.push(ident);
        }
    } else {
        return Err(Error::new_spanned(
            &input,
            "A UnitConfig cannot be an enum or an union.",
        ));
    }

    let parse_parsers = section_parsers.iter();

    let ident = &input.ident;

    let suffix = attributes
        .suffix
        .map_or(quote! {""}, |x| x.to_token_stream());

    let result = quote! {
         impl unit_parser::internal::UnitConfig for #ident {
            const SUFFIX: &'static str = #suffix;
            fn __parse_unit(__source: unit_parser::internal::UnitParser, __res: &mut Self) -> unit_parser::internal::Result<()> {
                let mut __source = __source;
                #( #section_ensures )*
                loop {
                    let mut __section = match __source.next() {
                        None => break,
                        Some(v) => v,
                    };
                    match __section.name {
                        #( #parse_parsers ),*
                        _ => {
                            log::debug!("{} is not a valid section.", __section.name);
                        }
                    }
                    __source.progress(__section.finish());
                }
                Ok(())
            }

            fn __load_default(__res: &mut Self) {
                #( #section_defaults )*
            }
        }
    };

    Ok(result)
}
