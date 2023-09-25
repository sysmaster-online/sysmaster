//! Functions for generating unit parsing expressions.
use crate::unit_parser::{
    attribute::UnitAttributes,
    section::{
        gen_section_ensure, gen_section_finalize, gen_section_init, gen_section_parse,
        gen_section_patches,
    },
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Error};

/// Generate [UnitConfig] implementations when using `#[derive(UnitConfig)]`
pub fn gen_unit_derives(input: DeriveInput) -> syn::Result<TokenStream> {
    let attributes = UnitAttributes::parse_vec(&input.attrs)?;
    let mut sections = Vec::new();
    let mut section_ensures = Vec::new();
    let mut section_inits = Vec::new();
    let mut section_parsers = Vec::new();
    let mut section_finalizes = Vec::new();
    let mut section_patches = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        for entry in &data_struct.fields {
            section_ensures.push(gen_section_ensure(entry)?);
            section_inits.push(gen_section_init(entry)?);
            section_parsers.push(gen_section_parse(entry)?);
            section_finalizes.push(gen_section_finalize(entry)?);
            section_patches.push(gen_section_patches(entry)?);
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

    let parse_parsers = section_parsers.iter().map(|x| &x.0);
    let patch_parsers = section_parsers.iter().map(|x| &x.1);

    let ident = &input.ident;

    let suffix = attributes
        .suffix
        .map_or(quote! {""}, |x| x.to_token_stream());

    let result = quote! {
         impl unit_parser::internal::UnitConfig for #ident {
            const SUFFIX: &'static str = #suffix;
            fn __parse_unit(__source: unit_parser::internal::UnitParser) -> unit_parser::internal::Result<Self> {
                let mut __source = __source;
                #( #section_ensures )*
                #( #section_inits )*
                loop {
                    if let Some(mut __section) = __source.next() {
                        match __section.name {
                            #( #parse_parsers ),*
                            _ => {
                                log::warn!("{} is not a valid section.", __section.name);
                            }
                        }
                        __source.progress(__section.finish());
                    } else {
                        break;
                    }
                }
                #( #section_finalizes )*
                Ok(Self {
                    #( #sections ),*
                })
            }

            fn __patch_unit(__source: unit_parser::internal::UnitParser, __from: &mut Self) -> unit_parser::internal::Result<()> {
                let mut __source = __source;
                #( #section_inits )*
                loop {
                    if let Some(mut __section) = __source.next() {
                        match __section.name {
                            #( #patch_parsers ),*
                            _ => {
                                log::warn!("{} is not a valid section.", __section.name);
                            }
                        }
                        __source.progress(__section.finish());
                    } else {
                        break;
                    }
                }
                #( #section_patches )*
                Ok(())
             }
        }
    };

    Ok(result)
}
