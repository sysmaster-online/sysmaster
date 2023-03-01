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

//! This crate provides Deserializ's function for sub unit conf.
//! ```edition 2018
//! # use macros::ConfigParseM;
//! # use std::io::{Error as IoError, ErrorKind};
//! # use utils::config_parser::{toml_str_parse, ConfigParse};
//! # use serde::{Deserialize, Serialize};
//! # [derive(Serialize, Deserialize, ConfigParseM)]
//! # [serdeName("Service")]
//! # struct ServiceConf {
//!    #[serde(alias = "Type", default = "ServiceType::default")]
//!    #service_type: ServiceType,
//! #
//! # fn main() {}
//! ```

mod enum_display;
mod unit_conf_parse;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// proc-macro: ConfigParseM
#[proc_macro_derive(ConfigParseM, attributes(serdeName))]
pub fn derive_configparse(input: TokenStream) -> TokenStream {
    let st = parse_macro_input!(input as syn::DeriveInput);
    match unit_conf_parse::do_expand(&st) {
        Ok(data) => data.into(),
        Err(_) => todo!(),
    }
}

/// proc-macro EnumDisplay, this implements Display for enum
#[proc_macro_derive(EnumDisplay)]
pub fn enum_display_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input);
    proc_macro::TokenStream::from(enum_display::do_expand(&ast))
}

#[cfg(test)]
mod tests {
    #[test]
    fn proc_it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
