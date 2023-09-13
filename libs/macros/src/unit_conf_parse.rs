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

use quote::quote;
use syn::{spanned::Spanned, Visibility};

type StructFields = syn::punctuated::Punctuated<syn::Field, syn::Token!(,)>;

fn get_fieds_from_driver_input(st: &syn::DeriveInput) -> syn::Result<&StructFields> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = st.data
    {
        return Ok(named);
    }
    Err(syn::Error::new_spanned(
        st,
        "Must define on a Struct".to_string(),
    ))
}

pub fn do_expand(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_ident = &st.ident;
    let struct_name_literal = st.ident.to_string();
    let parser_name_literal = format!("{}Parser", struct_name_literal);
    let parser_name_ident = syn::Ident::new(&parser_name_literal, st.span());
    let serder_name = get_user_specific_serde_name(st);
    let _tmp_serder_name = serder_name.map_or_else(|| struct_name_literal, |x| x);

    let vis = &st.vis;
    let fields = get_fieds_from_driver_input(st)?;
    let setter_functions = gererate_setter_functions(fields, vis)?;

    let ret = quote! {
        #vis struct #parser_name_ident(String);

        impl #struct_ident{
            pub fn builder_parser()-> #parser_name_ident{
                #parser_name_ident(String::from(#_tmp_serder_name))
            }
            #setter_functions
        }

        impl ConfigParse for #parser_name_ident {
            type Item = #struct_ident;

            fn conf_file_parse(&self,file_content: &str) -> Result<Self::Item,IoError>{
                let value = toml_str_parse(file_content).unwrap();
                let value_tab = value.as_table().unwrap();
                let conf_value = value_tab.get(#_tmp_serder_name);
                let ret = conf_value.map_or(Err(IoError::new(
                    ErrorKind::Other,
                    format!("config file format is error,not contain [{}]",#_tmp_serder_name)
                )),|op_v| {
                    let _result = toml::from_str::<Self::Item>(op_v.to_string().as_str());
                    return _result.map_err(|e|{
                        let err =format!("parse config file error,reason is :{}", e.to_string());
                        return IoError::new(ErrorKind::Other,err)
                    });
                });
                return ret;
            }
        };
    };
    Ok(ret)
}

fn get_user_specific_serde_name(st: &syn::DeriveInput) -> Option<String> {
    let mut result = None;
    for attr in st.attrs.iter() {
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("serdeName") {
                let value = nested.value()?;
                if value.peek(syn::Ident) {
                    let token: syn::Ident = value.parse()?;
                    result = Some(token.to_string());
                } else if value.peek(syn::LitStr) {
                    let token: syn::LitStr = value.parse()?;
                    result = Some(token.value());
                }
            }
            Ok(())
        })
        .ok();
    }
    result
    // for attr in &st.attrs {
    //     if let Ok(syn::Meta::List(syn::MetaList {
    //         ref path,
    //         ref nested,
    //         ..
    //     })) = attr.parse_meta()
    //     {
    //         if let Some(p) = path.segments.first() {
    //             if p.ident == "serdeName" {
    //                 let _n = nested.first();
    //                 if let Some(syn::NestedMeta::Meta(syn::Meta::Path(p))) = _n {
    //                     if let Some(p) = p.segments.first() {
    //                         return Some(p.ident.to_string());
    //                     }
    //                 } else if let Some(syn::NestedMeta::Lit(syn::Lit::Str(lit_str))) = _n {
    //                     return Some(lit_str.value());
    //                 }
    //             }
    //         }
    //     }
    // }
    // None
}

pub(crate) fn get_option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(syn::TypePath { ref path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    ref args,
                    ..
                }) = seg.arguments
                {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn get_vec_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(syn::TypePath { ref path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if seg.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    ref args,
                    ..
                }) = seg.arguments
                {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

fn gererate_setter_functions(
    fields: &StructFields,
    vis: &Visibility,
) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let mut final_stream = proc_macro2::TokenStream::new();

    for (ident, _type) in idents.iter().zip(types.iter()) {
        let set_field_name = format!("set_{}", ident.as_ref().unwrap());
        let get_field_name = format!("get_{}", ident.as_ref().unwrap());
        let set_field_ident = syn::Ident::new(&set_field_name, ident.span());
        let get_field_ident = syn::Ident::new(&get_field_name, ident.span());
        let token_piece = if let Some(inner_ty) = get_option_inner_type(_type) {
            quote! {
                #vis fn #set_field_ident(&mut self, #ident: #inner_ty) -> &mut Self{
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
                #vis fn #get_field_ident(&self) -> #_type{
                    if self.#ident.is_none(){
                       return std::option::Option::None;
                    }else{
                        return std::option::Option::Some(self.#ident.as_ref().unwrap().clone());
                    }
                }
            }
        } else {
            quote! {
                #vis fn #set_field_ident(&mut self, #ident: #_type) -> &mut Self{
                    self.#ident = #ident;
                    self
                }
                #vis fn #get_field_ident(&self) -> #_type{
                    return self.#ident.clone();
                }
            }
        };
        final_stream.extend(token_piece);
    }
    Ok(final_stream)
}
