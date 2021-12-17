mod mod_expander;
mod struct_mapper;
mod serde;

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

#[proc_macro]
pub fn srvpru_handler(input: TokenStream) -> TokenStream {
    struct_mapper::srvpru_handler(input)
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