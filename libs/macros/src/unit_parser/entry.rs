//! Functions for generating entry parsing expressions.
use crate::{
    unit_conf_parse::{get_option_inner_type, get_vec_inner_type},
    unit_parser::{attribute::EntryAttributes, transform_default::transform_default},
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
    if attribute.myparser.is_some() {
        return Ok(quote! {});
    }
    if attribute.multiple {
        ty = get_vec_inner_type(ty).unwrap();
    } else if (!attribute.must) & (attribute.default.is_none()) {
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
pub(crate) fn gen_entry_init(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let attributes = EntryAttributes::parse_attributes(field, None)?;
    Ok(match attributes.multiple {
        false => quote! {
            let mut #name = None;
        },
        true => quote! {
            let mut #name = Vec::new();
        },
    })
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

    if !attributes.multiple && attributes.subdir.is_some() {
        return Err(Error::new_spanned(
            field,
            "\'subdir\' can only be used when \'multiple\' is true.",
        ));
    }

    if attributes.multiple && attributes.must {
        return Err(Error::new_spanned(
            field,
            "\'must\' should be false when \'multiple\' is true.",
        ));
    }

    if (attributes.must || attributes.multiple) && attributes.default.is_some() {
        return Err(Error::new_spanned(
            field,
            "\'default\' can only be used when \'must\' and \'multiple\' are false.",
        ));
    }

    if attributes.multiple {
        let clear_when_empty = quote! {
            if __pair.1.as_str().is_empty() {
                #name.clear();
                continue;
            }
        };
        /* myparser */
        let parser = match attributes.myparser {
            Some(myparser) => quote! {
                match #myparser(__pair.1.as_str()) {
                    Ok(__inner) => {
                        #name.extend(__inner);
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
                            #name.push(__inner);
                        }
                        Err(_) => {
                            log::warn!("Failed to parse {} for key {}, ignoring.", __pair.0, __pair.1);
                        }
                    }
                }
            },
        };
        /* subdir */
        match attributes.subdir {
            None => {
                return Ok(quote! {
                    #key => {
                        #clear_when_empty
                        #parser
                    }
                })
            }
            Some(v) => {
                return Ok(quote! {
                    #key => {
                        #clear_when_empty
                        #parser
                        let __subdirs = __source.__parse_subdir(#v);
                        #name.extend_from_slice(&__subdirs);
                    }
                })
            }
        }
    }

    let parser = match (attributes.must, attributes.myparser) {
        (true, Some(entry_parser)) => quote! {
            let __value = #entry_parser(__pair.1.as_str()).unwrap();
            #name = Some(__value);
        },
        (true, None) => quote! {
            let __value = unit_parser::internal::UnitEntry::parse_from_str(__pair.1.as_str()).unwrap();
            #name = Some(__value);
        },
        (false, Some(entry_parser)) => quote! {
            if let Ok(__value) = #entry_parser(__pair.1.as_str()) {
                #name = Some(__value);
            }
        },
        (false, None) => quote! {
            if let Ok(__value) = unit_parser::internal::UnitEntry::parse_from_str(__pair.1.as_str()) {
                #name = Some(__value)
            }
        },
    };

    Ok(quote! {
        #key => {
            #parser
        }
    })
}

/// Generate finalization statements which are in charge of processing [Option] and [Result]s during parsing.
pub(crate) fn gen_entry_finalize(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let ty = &field.ty;
    let attributes = EntryAttributes::parse_attributes(field, None)?;
    let key = attributes
        .key
        .unwrap_or_else(|| (format!("{}", name)).into_token_stream());

    let result = match (attributes.default, attributes.multiple, attributes.must) {
        // invalid
        (Some(_), _, true) | (_, true, true) => unreachable!(),
        // apply default if empty
        (Some(default), true, false) => {
            quote! {
                if #name.is_empty() {
                    #name = #default;
                }
            }
        }
        // leave unchanged (`Vec` and `Option`)
        (None, true, false) | (None, false, false) => {
            quote! {}
        }
        // unwrap to default
        (Some(default), false, false) => {
            let default = transform_default(ty, &default)?;
            quote! {
                let #name = #name.unwrap_or(#default);
            }
        }
        // throw Error
        (None, false, true) => {
            quote! {
                let #name = #name.ok_or_else(|| {
                    unit_parser::internal::Error::EntryMissingError { key: #key.to_string()}})?;
            }
        }
    };
    Ok(result)
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

/// Generate patching statements that sets each field to a new value, if present.
/// Append the new value to the [Vec] if possible.
pub(crate) fn gen_entry_patch(field: &Field) -> Result<TokenStream> {
    let name = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Tuple structs are not supported."))?;
    let attributes = EntryAttributes::parse_attributes(field, None)?;

    let result = match (attributes.must, attributes.multiple, attributes.default) {
        // invalid
        (true, _, Some(_)) | (true, true, _) => unreachable!(),
        // append
        // TODO: or should it overwrite?
        (false, true, _) => {
            quote! {
                __from.#name.extend_from_slice(&#name);
            }
        }
        // set (as is) if not None
        (false, false, None) => {
            quote! {
                if #name.is_some() {
                    __from.#name = #name;
                }
            }
        }
        // set if not None
        (_, false, _) => {
            quote! {
                if let Some(__inner) = #name {
                    __from.#name = __inner;
                }
            }
        }
    };

    Ok(result)
}
