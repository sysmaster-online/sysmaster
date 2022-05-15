mod unit_conf_parse;

use proc_macro::TokenStream;

#[proc_macro_derive(ConfigParseM, attributes(serdeName))]
pub fn derive_configparse(input: TokenStream) -> TokenStream {
    let st = syn::parse_macro_input!(input as syn::DeriveInput);
    match unit_conf_parse::do_expand(&st){
        Ok(data) => data.into(),
        Err(_) => todo!(),
    }
}






#[cfg(test)]
mod tests {
    #[test]
    fn proc_it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}