//! Interfaces and functions for dealing with macro-related attributes.
use crate::unit_parser::type_transform::{is_option, is_vec};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Attribute, Error, Expr, Field, LitStr, Token, Type};

/// Attributes valid for [UnitSection]s.
pub(crate) struct SectionAttributes {
    pub(crate) default: bool,
    pub(crate) key: Option<TokenStream>,
    pub(crate) must: bool,
}

impl Default for SectionAttributes {
    fn default() -> Self {
        Self {
            default: false,
            key: None,
            must: false,
        }
    }
}

impl SectionAttributes {
    /// Parses [SectionAttributes] from [syn] tokens.
    /// Pass in [syn::Type] to do type check, or pass in [None] to prevent errors from showing up multiple times
    pub(crate) fn parse_vec(input: &Field, ty: Option<&Type>) -> syn::Result<Self> {
        let mut result = SectionAttributes::default();
        for attribute in input.attrs.iter() {
            if attribute.path().is_ident("section") {
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
                        Err(Error::new_spanned(attribute, "Not a valid attribute."))
                    }
                })?;
            }
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
pub(crate) struct EntryAttributes {
    pub(crate) default: Option<Expr>,
    pub(crate) key: Option<TokenStream>,
    pub(crate) multiple: bool,
    pub(crate) must: bool,
    pub(crate) subdir: Option<TokenStream>,
}

impl Default for EntryAttributes {
    fn default() -> Self {
        Self {
            default: None,
            key: None,
            multiple: false,
            must: false,
            subdir: None,
        }
    }
}

impl EntryAttributes {
    /// Parses [EntryAttributes] from [syn] tokens.
    /// Pass in [syn::Type] to do type check, or pass in [None] to prevent errors from showing up multiple times
    pub(crate) fn parse_vec(input: &Field, ty: Option<&Type>) -> syn::Result<Self> {
        let mut result = EntryAttributes::default();
        for attribute in input.attrs.iter() {
            if attribute.path().is_ident("entry") {
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
                    } else {
                        Err(Error::new_spanned(attribute, "Not a valid attribute."))
                    }
                })?;
            }
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
            if (!result.must) & (!result.default.is_some()) & (!result.multiple) & (!is_option(ty))
            {
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

pub(crate) struct UnitAttributes {
    pub(crate) suffix: Option<LitStr>,
}

impl Default for UnitAttributes {
    fn default() -> Self {
        Self { suffix: None }
    }
}

impl UnitAttributes {
    /// Parses [UnitAttributes] from [syn] tokens.
    /// Pass in [syn::Type] to do type check, or pass in [None] to prevent errors from showing up multiple times
    pub(crate) fn parse_vec(input: &Vec<Attribute>) -> syn::Result<Self> {
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
                        Err(Error::new_spanned(attribute, "Not a valid attribute."))
                    }
                })?;
            }
        }
        Ok(result)
    }
}
