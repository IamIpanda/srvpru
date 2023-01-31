use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::Ident;
use syn::parse_macro_input;

use crate::utils::clone_token_stream;

#[derive(FromMeta)]
#[darling(allow_unknown_fields)]
struct HandlerParameter {
    method: Option<String>,
    path: Option<String>
}

pub fn api(attr: TokenStream, input: TokenStream) -> TokenStream {
    let (input, mut input2) = clone_token_stream(input);
    let input = parse_macro_input!(input as syn::ItemFn);
    let attributes = parse_macro_input!(attr as syn::AttributeArgs);
    let name = input.sig.ident;
    let mut method = "GET".to_string();
    let mut path = "/".to_string() + &name.to_string();

    if let Ok(param) = HandlerParameter::from_list(&attributes) {
        if let Some(_method) = param.method { method = _method.clone() }
        if let Some(_path) = param.path { path = _path.clone() }
    }

    method = method.to_uppercase();
    path = format!("\"{}\"", path);
    let ctor_function_name = Ident::new(&format!("{}_ctor", name.to_string()), Span::call_site());

    let extender: TokenStream = quote! {
        #[ctor::ctor]
        fn #ctor_function_name() {
            unsafe { crate::srvpro::api::ROUTER }.route(#path, routing::on::<H, T, B>(axum::routing::#method, #name));
        }
    }.into();

    input2.extend(extender.into_iter());
    input2
}