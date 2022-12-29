extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn do_expand(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let data = &ast.data;
    let data = match data {
        syn::Data::Enum(data) => data,
        _ => panic!(),
    };

    let mut match_token = quote! {};
    for variant in &data.variants {
        let cur = &variant.ident;
        match_token = quote! {
            #match_token
            #name::#cur => write!(f, "{}", stringify!(#cur).to_lowercase()),
        };
    }

    let gen = quote! {
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    #match_token
                }
            }
        }
    };

    gen
}
