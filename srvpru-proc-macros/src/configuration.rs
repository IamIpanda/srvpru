use darling::FromAttributes;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromAttributes, Debug)]
#[darling(attributes(configuration), allow_unknown_fields)]
struct ConfigurationParameters {
    filename: Option<String>
}

pub fn configuration(input: TokenStream) -> TokenStream {
   let input = parse_macro_input!(input as DeriveInput);
   let ident = input.ident.clone();
   let parameters = ConfigurationParameters::from_attributes(&input.attrs).expect("Can't get parammeters");
   let filename_tokens = match parameters.filename {
      Some(filename) => quote!(
         fn config_name(module_path: &str) -> &str {
            #filename
         }
      ),
      None => quote!()
   };
   let configuration_name = Ident::new(&input.ident.to_string().to_uppercase(), Span::call_site()) ;

   quote! {
      static #configuration_name: once_cell::sync::Lazy<arc_swap::ArcSwap<#ident>> = once_cell::sync::Lazy::new(|| {
         arc_swap::ArcSwap::new(std::sync::Arc::new(crate::srvpro::load_configuration(#ident::config_name(module_path!())).expect("Can't load configuration.")))
      });

      impl crate::srvpro::Configuration for #ident {
         type Ref = arc_swap::Guard<std::sync::Arc<Self>>;

         fn get() -> Self::Ref {
            #configuration_name.load()
         }

         fn reload() {
            let config = match crate::srvpro::load_configuration(#ident::config_name(module_path!())) {
               Ok(config) => config,
               Err(e) => {
                  warn!("Cannot load configuration: {:?}", e);
                  return
               }
            };
            #configuration_name.swap(std::sync::Arc::new(config));
         }

         #filename_tokens
      }
   }.into()
}
