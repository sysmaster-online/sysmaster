//! Interfaces and functions for dealing with macro-related attributes.
use crate::unit_parser::type_transform::{is_option, is_vec};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Attribute, Error, Expr, Field, LitStr, Token, Type};

/// Attributes valid for [UnitSection]s.
#[derive(Default)]
pub(crate) struct SectionAttributes {
    /// Whether fallback to [std::default::Default] is enabled
    pub(crate) default: bool,
    /// Whether alternative key is specified
    pub(crate) key: Option<TokenStream>,
    /// Whether must-present is specified
    pub(crate) must: bool,
}

impl SectionAttributes {
    /// Parses [SectionAttributes] from [syn] tokens.
    /// Pass in [syn::Type] to do type check, or pass in [None] to prevent errors from showing up multiple times
    pub(crate) fn parse_vec(input: &Field, ty: Option<&Type>) -> syn::Result<Self> {
        let mut result = SectionAttributes::default();
        for attribute in input.attrs.iter() {
            if !attribute.path().is_ident("section") {
                continue;
            }
            attribute.parse_nested_meta(|nested| {
                if nested.path.is_ident("default") {
                    result.default = true;
                    Ok(())
                } else if nested.path.is_ident("key") {
                    nested.input.parse::<Token![=]>()?;
                    let value: LitStr = nested.input.parse()?;
                    result.key = Some(value.into_token_stream());
                    Ok(())
                } else if nested.path.is_ident("must") {
                    result.must = true;
                    Ok(())
                } else {
                    Err(Error::new_spanned(
                        attribute,
                        "section, Not a valid attribute.",
                    ))
                }
            })?;
        }
        if result.default & result.must {
            return Err(Error::new_spanned(
                input,
                "`default` and `must` cannot co-exist.",
            ));
        }
        if let Some(ty) = ty {
            if (!result.must) & (!result.default) & (!is_option(ty)) {
                return Err(Error::new_spanned(
                    input,
                    "Optional fields should be `Option`s.",
                ));
            }
        }
        Ok(result)
    }
}

/// Attributes valid for [UnitEntry]s.
#[derive(Default)]
pub(crate) struct EntryAttributes {
    /// Whether fallback to default expression is enabled
    pub(crate) default: Option<Expr>,
    /// Whether alternative key is specified
    pub(crate) key: Option<TokenStream>,
    /// Whether multiple-present is specified
    pub(crate) multiple: bool,
    /// Whether must-present is specified
    pub(crate) must: bool,
    /// Whether systemd subdir resolve is specified
    pub(crate) subdir: Option<TokenStream>,
    /// User's own parser
    pub(crate) myparser: Option<syn::Path>,
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
                } else if nested.path.is_ident("multiple") {
                    result.multiple = true;
                    Ok(())
                } else if nested.path.is_ident("must") {
                    result.must = true;
                    Ok(())
                } else if nested.path.is_ident("subdir") {
                    nested.input.parse::<Token![=]>()?;
                    let value: LitStr = nested.input.parse()?;
                    result.subdir = Some(value.into_token_stream());
                    Ok(())
                } else if nested.path.is_ident("myparser") {
                    nested.input.parse::<Token![=]>()?;
                    let value: syn::Path = nested.input.parse()?;
                    result.myparser = Some(value);
                    Ok(())
                } else {
                    Err(Error::new_spanned(
                        attribute,
                        "entry, Not a valid attribute.",
                    ))
                }
            })?;
        }
        if result.must & result.default.is_some() {
            return Err(Error::new_spanned(
                input,
                "`must` and `default` cannot co-exist.",
            ));
        }
        if result.multiple & result.must {
            return Err(Error::new_spanned(
                input,
                "`must` and `multiple` cannot co-exist.",
            ));
        }
        if (!result.multiple) & result.subdir.is_some() {
            return Err(Error::new_spanned(
                input,
                "`subdir` attributed fields must be `multiple`.",
            ));
        }
        if let Some(ty) = ty {
            if (!result.must) & (result.default.is_none()) & (!result.multiple) & (!is_option(ty)) {
                return Err(Error::new_spanned(
                    input,
                    "Optional fields should be `Option`s.",
                ));
            }
            if result.multiple & (!is_vec(ty)) {
                return Err(Error::new_spanned(
                    input,
                    "`multiple` attributed fields should be `Vec`s.",
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
