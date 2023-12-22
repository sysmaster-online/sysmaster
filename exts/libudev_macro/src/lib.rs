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

//! This crate provides a procedual macro RefUnref to derive ref and unref methods
//! for a Rc wrapped raw pointer.

use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(RefUnref)]
/// Gerenrate ref and unref methods.
///
/// The input parameter is a raw pointer.
/// It will be recovered into Rust Rc and do Rc clone or drop.
pub fn derive_ref_unref(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    /* Generate an AST */
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = &input.ident;

    let fn_ref = syn::Ident::new(&format!("{}_ref_impl", quote!(#name)), input.span());
    let fn_unref = syn::Ident::new(&format!("{}_unref_impl", quote!(#name)), input.span());

    let expanded = quote! {
        #[no_mangle]
        /// Recover Rc from the raw pointer and do Rc clone.
        ///
        /// Return the raw pointer of the cloned Rc.
        pub extern "C" fn #fn_ref(#name: *mut #name) -> *mut #name {
            if #name.is_null() {
                return std::ptr::null_mut();
            }

            let recover = unsafe { Rc::from_raw(#name) };
            let ret = recover.clone();
            let _ = Rc::into_raw(recover);
            Rc::into_raw(ret) as *mut #name
        }

        #[no_mangle]
        /// Recover Rc from the raw pointer and do Rc drop.
        ///
        /// Return a null pointer.
        pub extern "C" fn #fn_unref(#name: *mut #name) -> *mut #name {
            if #name.is_null() {
                return std::ptr::null_mut();
            }

            let _ = unsafe { Rc::from_raw(#name) };
            std::ptr::null_mut()
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_attribute]
/// append _impl to the original function name
pub fn append_impl(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut func = parse_macro_input!(item as syn::ItemFn);

    let name = &func.sig.ident;
    let new_name = format!("{}_impl", name);

    func.sig.ident = syn::Ident::new(&new_name, name.span());

    let output = quote! {
        #func
    };

    output.into()
}
