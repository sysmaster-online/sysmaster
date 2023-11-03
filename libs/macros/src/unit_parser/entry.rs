//! Functions for generating entry parsing expressions.
use crate::{
    unit_conf_parse::{get_option_inner_type, get_vec_inner_type},
    unit_parser::attribute::EntryAttributes,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Error, Field, Result};

/// Generate statements that ensure the given struct implements [UnitEntry]
/// by calling a function with generic constraints.
pub(crate) fn gen_entry_ensure(field: &Field) -> Result<TokenStream> {
    let mut ty = &field.ty;
    let attribute = EntryAttributes::parse_attributes(field, None)?;
    /* No need to check when user has defined his own parser */
    if attribute.parser.is_some() {
        return Ok(quote! {});
    }
    if attribute.append {
        ty = get_vec_inner_type(ty).unwrap();
    } else if attribute.default.is_none() {
        ty = get_option_inner_type(ty).unwrap();
    }
    Ok(quote! {
        const _: fn() = || {
            fn assert_impl<T: unit_parser::internal::UnitEntry>() {}
            assert_impl::<#ty>();
        };
    })
}

/// Generate variable initialization statements.
/// example:
/// ```
/// let mut Field1 = None;
/// ```
pub(crate) fn gen_entry_default(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let _ty = &field.ty;
    let attributes = EntryAttributes::parse_attributes(field, None)?;
    if attributes.append {
        return Ok(quote! {
            __res.#name = Vec::new();
        });
    }
    match attributes.default {
        None => Ok(quote! {
            __res.#name = None;
        }),
        Some(v) => Ok(quote! {
            __res.#name = #v;
        }),
    }
}

/// Generate entry parsing statements, in the form of an arm in a `match` statement.
/// example:
/// ```
/// match __entry.0 {
///     "Field1" => {
///         if let Ok(__value) = unit_parser::UnitEntry::parse_from_str(__entry.1) {
///             if __value.is_some() {
///                 Field1 = __value;
///             }
///         }
///     }
/// }
/// ```
pub(crate) fn gen_entry_parse(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let ty = &field.ty;
    let attributes = EntryAttributes::parse_attributes(field, Some(ty))?;
    let key = attributes
        .key
        .unwrap_or_else(|| (format!("{}", name)).into_token_stream());

    if attributes.append {
        let clear_when_empty = quote! {
            if __pair.1.as_str().is_empty() {
                __res.#name.clear();
                continue;
            }
        };
        /* parser */
        let parser = match attributes.parser {
            Some(myparser) => quote! {
                match #myparser(__pair.1.as_str()) {
                    Ok(__inner) => {
                        __res.#name.extend(__inner);
                    }
                    Err(_) => {
                        log::warn!("Failed to parse {} for key {}, ignoring.", __pair.0, __pair.1);
                    }
                }
            },
            None => quote! {
                for __part in __pair.1.split_ascii_whitespace(){
                    match unit_parser::internal::UnitEntry::parse_from_str(__part){
                        Ok(__inner) => {
                            __res.#name.push(__inner);
                        }
                        Err(_) => {
                            log::warn!("Failed to parse {} for key {}, ignoring.", __pair.0, __pair.1);
                        }
                    }
                }
            },
        };
        return Ok(quote! {
            #key => {
                #clear_when_empty
                #parser
            }
        });
    }
    let apply_value = match attributes.default {
        None => quote! {
            __res.#name = Some(__value);
        },
        Some(_) => quote! {
            __res.#name = __value;
        },
    };
    let parser = match attributes.parser {
        Some(entry_parser) => quote! {
            let __value = #entry_parser(__pair.1.as_str()).unwrap();
            #apply_value;
        },
        None => quote! {
            let __value = unit_parser::internal::UnitEntry::parse_from_str(__pair.1.as_str()).unwrap();
            #apply_value;
        },
    };

    Ok(quote! {
        #key => {
            #parser
        }
    })
}

/// Generate implementation statements for custom enums.
/// Uses each variant's name as their value.
pub(crate) fn gen_entry_derives(input: DeriveInput) -> Result<TokenStream> {
    if let Data::Enum(inner) = input.data {
        let ident = &input.ident;
        let mut match_arms = Vec::new();

        for variant in inner.variants.iter() {
            let name = &variant.ident;
            let value = format!("{}", name);
            // TODO: support for alt-key
            let result = quote! {
                #value => Ok(Self::#name)
            };
            match_arms.push(result);
        }

        Ok(quote! {
            impl unit_parser::internal::UnitEntry for #ident {
                type Error = ();
                fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
                    match input.as_ref() {
                        #( #match_arms ,)*
                        _ => Err(()),
                    }
                }
            }
        })
    } else {
        Err(Error::new_spanned(
            input,
            "UnitEntry can only be derived on enum definitions.",
        ))
    }
}
