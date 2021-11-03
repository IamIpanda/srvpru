use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::Span;

use syn::parse_macro_input;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::LitStr;
use quote::quote;

include!("struct_mapper.rs");
include!("mod_expander.rs");


#[proc_macro_derive(Struct)]
pub fn ygopro_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = input.ident;
    let struct_name = struct_ident.to_string();
    let attributes = input.attrs;
    let _type = match struct_name {
        _ if struct_name.starts_with("CTOS") => "CTOS",
        _ if struct_name.starts_with("STOC") => "STOC",
        _ if struct_name.starts_with("SRVPRU") => "SRVPRU",
        _ if struct_name.starts_with("GM") => "GM",
        _ if attributes.iter().any(|attribtue| attribtue.path.is_ident("ctos")) => "CTOS",
        _ if attributes.iter().any(|attribtue| attribtue.path.is_ident("stoc")) => "STOC",
        _ if attributes.iter().any(|attribtue| attribtue.path.is_ident("srvpru")) => "SRVPRU",
        _ if attributes.iter().any(|attribtue| attribtue.path.is_ident("game_message") || attribtue.path.is_ident("gm")) => "GM",
        _ => "_"
    };
    let enum_name = if struct_name.starts_with(_type) { &struct_name[_type.len()..] } else { struct_name.as_str() };
    let type_ident = quote::format_ident!("{}", _type);
    let enum_class_ident = quote::format_ident!("MessageType");
    let enum_ident = quote::format_ident!("{}", enum_name);
    let mapped_response = if _type == "_" {
        quote!(generate_message_type(#enum_class_ident::#enum_ident))
    } 
    else {
        quote!(crate::ygopro::message::MessageType::#type_ident(#enum_class_ident::#enum_ident))
    };
        
    let expand = quote! {
        impl crate::ygopro::message::Struct for #struct_ident {}
        impl crate::ygopro::message::MappedStruct for #struct_ident {
            fn message() -> crate::ygopro::message::MessageType {
                return #mapped_response;
            }
        }
    };
    TokenStream::from(expand)
}
