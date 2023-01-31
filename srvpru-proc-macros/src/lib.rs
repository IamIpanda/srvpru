mod mod_expander;
mod handler_ygopro;
mod handler_http;
mod attachment;
mod configuration;
mod struct_mapper;
mod serde;
mod utils;

use proc_macro::TokenStream;

// ----------------------------------------------------------------------------------------------------
/// ### execute_for_each_under_dir
// ----------------------------------------------------------------------------------------------------
/// generate code for each file under target directory.    
/// - use `#name` for file name.   
/// - use `#fullname` for generated class name.
/// 
/// A changed version for [automod](https://crates.io/crates/automod).
// ----------------------------------------------------------------------------------------------------
#[proc_macro]
pub fn execute_for_each_under_dir(input: TokenStream) -> TokenStream {
    mod_expander::execute_for_each_under_dir(input)
}

// ----------------------------------------------------------------------------------------------------
/// ### ygopro_struct
// ----------------------------------------------------------------------------------------------------
/// Derive macro generating an impl of the trait `Struct` and `MappedStruct`.
// --------------------------------------D-------------------------------------------------------------
#[proc_macro_derive(Struct)]
pub fn ygopro_struct(input: TokenStream) -> TokenStream {
    struct_mapper::ygopro_struct(input)
}

#[proc_macro_derive(Message, attributes(message))]
pub fn ygopro_message(input: TokenStream) -> TokenStream {
    struct_mapper::ygopro_message(input)
}

#[proc_macro_derive(PlayerAttachment)]
pub fn player_attachment(input: TokenStream) -> TokenStream {
    attachment::player_attachment(input) 
}

#[proc_macro_derive(RoomAttachment)]
pub fn room_attachment(input: TokenStream) -> TokenStream {
    attachment::room_attachment(input)
}

#[proc_macro_derive(Serialize)]
pub fn fake_serialize(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = input.ident;
    quote::quote! {        
        impl serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                warn!("Trying to serialize a internal object.");
                serializer.serialize_unit()
            }
        }
    }.into()
}

#[proc_macro_derive(Deserialize)]
pub fn fake_deserialize(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = input.ident;
    quote::quote!(
        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
                panic!("Try to deserialize a srvpru message");
            }
        }
    ).into()
}

#[proc_macro_derive(Configuration, attributes(configuration))]
pub fn configuration(input: TokenStream) -> TokenStream {
    crate::configuration::configuration(input)
}

#[proc_macro_attribute]
pub fn before(args: TokenStream, input: TokenStream) -> TokenStream {
    handler_ygopro::before(args, input)
}

#[proc_macro_attribute]
pub fn after(args: TokenStream, input: TokenStream) -> TokenStream {
    handler_ygopro::after(args, input)
}

#[proc_macro_attribute]
pub fn api(args: TokenStream, input: TokenStream) -> TokenStream {
    handler_http::api(args, input)
}

// ----------------------------------------------------------------------------------------------------
/// ### serde_default
// ----------------------------------------------------------------------------------------------------
/// Derive macro generating an impl [`Default`](std::default::Default) from serde config.
/// 
/// Reference from a [issue](https://github.com/dtolnay/request-for-implementation/issues/4) 
///            and a [uncommited crate](https://github.com/TedDriggs/serde_default)
// ----------------------------------------------------------------------------------------------------
#[proc_macro_derive(serde_default)]
pub fn serde_default(input: TokenStream) -> TokenStream {
    serde::serde_default(input, true)
}

// ----------------------------------------------------------------------------------------------------
/// ### try_serde_default
// ----------------------------------------------------------------------------------------------------
/// Try to implement [`Default`](std::default::Default) from serde config.
/// If any field miss a `default` config, nothing will happen.
// ----------------------------------------------------------------------------------------------------
#[proc_macro_derive(try_serde_default)]
pub fn try_serde_default(input: TokenStream) -> TokenStream {
    serde::serde_default(input, false)
}
