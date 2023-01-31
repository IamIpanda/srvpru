use darling::{FromAttributes, FromMeta};
use darling::util::Override;
use proc_macro2::{Span, Ident};
use syn::DeriveInput;
use proc_macro::TokenStream;

use syn::parse_macro_input;
use quote::quote;

use crate::utils::compile_error;

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

#[derive(FromAttributes, Debug)]
#[darling(attributes(message), allow_unknown_fields)]
struct MessageParameters {
    ctos: Option<Override<()>>,
    stoc: Option<Override<()>>,
    gm: Option<Override<()>>,
    srvpru: Option<Override<()>>,
    flag: Option<u8>,
    no_length: Option<Override<()>>,
    mod_name: Option<String>
}


pub fn ygopro_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = input.ident;
    let attributes = input.attrs;
    let message_parameter = match MessageParameters::from_attributes(&attributes) {
        Ok(param) => param,
        Err(err) => return compile_error(Span::call_site(), &format!("Cannot parse message paramter:\n{:?}", err)).into()
    };
    let direction = if message_parameter.ctos.is_some()  { "CTOS" }
                     else if message_parameter.stoc.is_some()  { "STOC" }
                     else if message_parameter.gm.is_some()    {  "GM"  }
                     else if message_parameter.srvpru.is_some(){ "Other"}
                     else { return compile_error(Span::call_site(), "Don't specify a direction.").into(); };
    let ident = Ident::from_string(direction).unwrap();
    let lower_ident = Ident::from_string(&direction.clone().to_lowercase()).unwrap();
    let mod_name = match message_parameter.mod_name {
        Some(name) => {
            let ident = Ident::from_string(&name).unwrap();
            quote!(#ident)
        }
        None => quote!(crate)
    };
    let length_description = match message_parameter.no_length {
        Some(_) => quote!(),
        None => quote!{ impl #mod_name::serde::LengthDescribed for #struct_ident {} }
    };

    if direction == "Other" {
        if let Some(flag) = message_parameter.flag {
            quote!{
                impl #mod_name::message::PureMessage for #struct_ident {}
                impl #mod_name::message::Message for #struct_ident {
                    fn message_type() -> #mod_name::message::MessageType {
                        #mod_name::message::MessageType::#ident("srvpru", #flag)
                    }
                }
                #length_description
            }
        }
        else {
            compile_error(Span::call_site(), "Don't offer a flag")
        }
    } else {
        quote!{
            impl #mod_name::message::PureMessage for #struct_ident {}
            impl #mod_name::message::Message for #struct_ident {
                fn message_type() -> #mod_name::message::MessageType {
                    #mod_name::message::MessageType::#ident(#mod_name::message::#lower_ident::MessageType::#struct_ident)
                }
            }
            #length_description
        }
    }.into()
}
