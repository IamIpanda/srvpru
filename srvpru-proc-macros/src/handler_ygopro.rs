use syn::{parse_macro_input, Meta, NestedMeta, FnArg};
use quote::quote;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use darling::{FromMeta, ToTokens};

use crate::utils::clone_token_stream;

pub fn before(attr: TokenStream, input: TokenStream) -> TokenStream {
    let (mut input, input2) = clone_token_stream(input);
    input.extend(generate_ctor(attr.into(), input2, quote!(crate::srvpro::HandlerOccasion::Before)).into_iter());
    input
}

pub fn after(attr: TokenStream, input: TokenStream) -> TokenStream {
    let (mut input, input2) = clone_token_stream(input);
    input.extend(generate_ctor(attr.into(), input2, quote!(crate::srvpro::HandlerOccasion::After)).into_iter());
    input
}

#[derive(FromMeta)]
#[darling(allow_unknown_fields)]
struct HandlerParameter {
    priority: Option<u8>
}

pub fn generate_ctor(attr: TokenStream, input: TokenStream, occasion: proc_macro2::TokenStream) -> TokenStream {
    let mut priority = 100u8;
    let mut ty: Option<proc_macro2::TokenStream> = None;
    let input = parse_macro_input!(input as syn::ItemFn);
    let mut attributes = parse_macro_input!(attr as syn::AttributeArgs);
 
    if let Some(NestedMeta::Meta(Meta::Path(path))) = attributes.get(0) {
        ty = Some(path.to_token_stream());
        attributes.remove(0usize);
    }
    
    if let Ok(param) = HandlerParameter::from_list(&attributes) {
        if let Some(_priority) = param.priority { priority = _priority }
    }

    if matches!(ty, None) {
        for argument in input.sig.inputs.iter() {
            match argument {
                FnArg::Receiver(_) => (),
                FnArg::Typed(pat_type) => {
                    if pat_type.pat.to_token_stream().to_string() == "message" {
                        if let syn::Type::Reference(r) = *(pat_type.ty.clone()) {
                            ty = Some(r.elem.to_token_stream());
                        }
                        else {
                            ty = Some(pat_type.ty.to_token_stream());
                        }
                    }
                }
            }
        }
    }

    if matches!(ty, None) {
        let error = syn::Error::new(Span::call_site(), "Cannot determine message type").to_compile_error();
        return quote!(#error).into()
    }

    let name = input.sig.ident;
    let name_string = name.to_string();
    let ctor_function_name = Ident::new(&format!("{}_ctor", name.to_string()), Span::call_site());
    quote! {
        #[ctor::ctor]
        fn #ctor_function_name() {
            crate::srvpro::HandlerGroup::<#ty>::get_precursor(#occasion).register_handler(#priority, #name_string, #occasion, #name);
        }
    }.into()
}
