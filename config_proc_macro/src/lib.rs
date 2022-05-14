use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

type StructFields = syn::punctuated::Punctuated<syn::Field, syn::Token!(,)>;
#[proc_macro_derive(ConfigParseM, attributes(serdeName))]
pub fn derive(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as syn::DeriveInput);
    match do_expand(&st){
        Ok(data) => data.into(),
        Err(_) => todo!(),
    }
}


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

fn do_expand(st: &syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_ident = &st.ident;
    let struct_name_literal = st.ident.to_string();
    let parser_name_literal = format!("{}Parser",struct_name_literal);
    let parser_name_ident = syn::Ident::new(&parser_name_literal,st.span());
    let serder_name = get_user_specific_serde_name(&st);
    let _tmp_serder_name = serder_name.map_or_else(||struct_name_literal,|x|x);

    let fields = get_fieds_from_driver_input(st)?;
    let setter_functions = gererate_setter_functions(&fields)?;
    let vis = &st.vis;
    let ret = quote! {
        #vis struct #parser_name_ident(String);
        
        impl #struct_ident{
            pub fn builder_paser()-> #parser_name_ident{
                #parser_name_ident(String::from(#_tmp_serder_name))
            }
            #setter_functions
        }

        impl ConfigParse for #parser_name_ident {
            type Item = #struct_ident;
            fn toml_file_parse(&self, _: &str) -> Result<Confs, std::io::Error> 
            {
                todo!();
            }

            fn conf_file_parser(&self,file_content: &str) -> Result<Option<Self::Item>,IoError>{
                let value = toml_str_parse(file_content).unwrap();
                let value_tab = value.as_table().unwrap();
                let service_value = value_tab.get(#_tmp_serder_name);
                let ret: Self::Item = toml::from_str(service_value.unwrap().to_string().as_str()).unwrap();
                Ok(Some(ret))
            }
        };
    };
    return Ok(ret);
}

fn get_user_specific_serde_name(st: &syn::DeriveInput)-> Option<String>{
    
    for attr in &st.attrs{
        if let Ok(syn::Meta::List(syn::MetaList{
            ref path,
            ref nested,
            ..
        })) = attr.parse_meta(){
           
            if let Some(p) = path.segments.first() {
                if p.ident == "serdeName" {
                    let _n = nested.first();
                    if let Some(syn::NestedMeta::Meta(syn::Meta::Path(p))) = _n{
                        if let Some(p) = p.segments.first() {
                            return Some(std::string::String::from(p.ident.to_string()));
                        }
                    }else if let Some(syn::NestedMeta::Lit(syn::Lit::Str(lit_str)))  = _n{
                        return Some(lit_str.value().to_string());
                    }
                }
            }
        }
    }
    None
}


fn get_option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(syn::TypePath {ref path, .. }) = ty{
        if let Some(seg) = path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    ref args,
                    ..
                }) = seg.arguments{
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

fn gererate_setter_functions(fields: &StructFields) -> syn::Result<proc_macro2::TokenStream> {
    let idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let types: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let mut final_stream = proc_macro2::TokenStream::new();
   
    for (ident, _type) in idents.iter().zip(types.iter()) {
        let token_piece;
        let set_field_name = format!("set_{}",ident.as_ref().unwrap().to_string());
        let get_field_name = format!("get_{}",ident.as_ref().unwrap().to_string());
        let set_field_ident = syn::Ident::new(&set_field_name, ident.span());
        let get_field_ident = syn::Ident::new(&get_field_name, ident.span());
        if let Some(inner_ty) = get_option_inner_type(_type) {
            token_piece = quote!{
                fn #set_field_ident(&mut self, #ident: #inner_ty) -> &mut Self{
                    self.#ident = std::option::Option::Some(#ident);
                    self
                }
                fn #get_field_ident(&self) -> #inner_ty{
                    return self.#ident.as_ref().unwrap().clone() ;
                }
            };
        }else{
            token_piece = quote! {
                fn #set_field_ident(&mut self, #ident: #_type) -> &mut Self{
                    self.#ident = #ident;
                    self
                }
                fn #get_field_ident(&self) -> #_type{
                    return self.#ident.clone();
                }
            };
        }
        final_stream.extend(token_piece);
    }
    Ok(final_stream)
}




#[cfg(test)]
mod tests {
    #[test]
    fn proc_it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}