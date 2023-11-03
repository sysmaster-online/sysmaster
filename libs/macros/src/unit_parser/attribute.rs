//! Interfaces and functions for dealing with macro-related attributes.
use crate::unit_parser::type_transform::{is_option, is_vec};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Attribute, Error, Expr, Field, LitStr, Token, Type};

/// Attributes valid for [UnitEntry]s.
#[derive(Default)]
pub(crate) struct EntryAttributes {
    /// Whether fallback to default expression is enabled
    pub(crate) default: Option<Expr>,
    /// Whether alternative key is specified
    pub(crate) key: Option<TokenStream>,
    /// Whether append-present is specified
    pub(crate) append: bool,
    /// User's own parser
    pub(crate) parser: Option<syn::Path>,
}

impl EntryAttributes {
    /// Parses [EntryAttributes] from [syn] tokens.
    /// Pass in [syn::Type] to do type check, or pass in [None] to prevent errors from showing up multiple times
    pub(crate) fn parse_attributes(input: &Field, ty: Option<&Type>) -> syn::Result<Self> {
        let mut result = EntryAttributes::default();
        for attribute in input.attrs.iter() {
            if !attribute.path().is_ident("entry") {
                continue;
            }
            attribute.parse_nested_meta(|nested| {
                if nested.path.is_ident("default") {
                    nested.input.parse::<Token![=]>()?;
                    let value: Expr = nested.input.parse()?;
                    result.default = Some(value);
                    Ok(())
                } else if nested.path.is_ident("key") {
                    nested.input.parse::<Token![=]>()?;
                    let value: LitStr = nested.input.parse()?;
                    result.key = Some(value.into_token_stream());
                    Ok(())
                } else if nested.path.is_ident("append") {
                    result.append = true;
                    Ok(())
                } else if nested.path.is_ident("parser") {
                    nested.input.parse::<Token![=]>()?;
                    let value: syn::Path = nested.input.parse()?;
                    result.parser = Some(value);
                    Ok(())
                } else {
                    Err(Error::new_spanned(
                        attribute,
                        "Not a valid entry attribute.",
                    ))
                }
            })?;
        }
        if result.append & result.default.is_some() {
            return Err(Error::new_spanned(
                input,
                "`append` and `default` cannot co-exist.",
            ));
        }
        if let Some(ty) = ty {
            if result.default.is_none() && !result.append && !is_option(ty) {
                return Err(Error::new_spanned(
                    input,
                    "Type must be `Option` if neither default nor append is configured.",
                ));
            }
            if result.append && (!is_vec(ty)) {
                return Err(Error::new_spanned(
                    input,
                    "`append` attributed fields should be `Vec`s.",
                ));
            }
        }
        Ok(result)
    }
}

#[derive(Default)]
pub(crate) struct UnitAttributes {
    /// The suffix of a type of unit
    pub(crate) suffix: Option<LitStr>,
}

impl UnitAttributes {
    /// Parses [UnitAttributes] from [syn] tokens.
    /// Pass in [syn::Type] to do type check, or pass in [None] to prevent errors from showing up multiple times
    pub(crate) fn parse_vec(input: &[Attribute]) -> syn::Result<Self> {
        let mut result = UnitAttributes::default();
        for attribute in input.iter() {
            if attribute.path().is_ident("unit") {
                attribute.parse_nested_meta(|nested| {
                    if nested.path.is_ident("suffix") {
                        nested.input.parse::<Token![=]>()?;
                        let value: LitStr = nested.input.parse()?;
                        result.suffix = Some(value);
                        Ok(())
                    } else {
                        Err(Error::new_spanned(
                            attribute,
                            "unit, Not a valid attribute.",
                        ))
                    }
                })?;
            }
        }
        Ok(result)
    }
}
