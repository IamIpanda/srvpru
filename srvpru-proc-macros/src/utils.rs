use proc_macro::TokenStream;
use quote::quote;
use quote::ToTokens;

pub fn compile_error(span: proc_macro2::Span, message: &str) -> proc_macro2::TokenStream {
    let error = syn::Error::new(span, message).to_compile_error();
    quote!(#error)
}

pub fn clone_token_stream(source: TokenStream) -> (TokenStream, TokenStream) {
    let source: proc_macro2::TokenStream = source.into();
    (source.clone().into(), source.into())
}

#[allow(dead_code)]
pub fn to_string(expr: &impl ToTokens) -> String {
    quote!(#expr).to_string()
}
