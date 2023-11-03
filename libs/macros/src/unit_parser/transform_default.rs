//! Function for transforming string literal default values.
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Error, Expr, Lit, Type};

/// If a field is defined to be a [String] and the given default expression is a [str],
/// this functions adds `.to_string()` to the end of it.
#[allow(unused)]
pub(crate) fn transform_default(ty: &Type, default: &Expr) -> Result<TokenStream, Error> {
    // add `to_string()` suffix if ty is String
    if let Type::Path(inner) = ty {
        let path = inner
            .path
            .segments
            .last()
            .ok_or_else(|| Error::new_spanned(ty, "Invalid type."))?;
        if path.ident == "String" {
            if let Expr::Lit(expr) = default {
                if let Lit::Str(string) = &expr.lit {
                    return format!("\"{}\".to_string()", string.value())
                        .parse()
                        .map_err(|_| Error::new_spanned(default, "Invalid default expression."));
                }
            }
        }
    }
    Ok(default.into_token_stream())
}
