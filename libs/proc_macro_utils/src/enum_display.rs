// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

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
