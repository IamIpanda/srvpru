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
