use darling::FromField;
use proc_macro2::Span;
use proc_macro::TokenStream;
use quote::quote;
use syn::Path;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::spanned::Spanned;

#[derive(FromField)]
#[darling(attributes(serde), allow_unknown_fields)]
pub struct FieldOptions {
    default: DefaultDeclaration,
}
enum DefaultDeclaration {
    Trait,
    Path(Path),
}

impl darling::FromMeta for DefaultDeclaration {
    fn from_word() -> darling::Result<Self> {
        Ok(DefaultDeclaration::Trait)
    }

    fn from_string(value: &str) -> darling::Result<Self> {
        syn::parse_str(value)
            .map(DefaultDeclaration::Path)
            .map_err(|_| darling::Error::unknown_value(value))
    }
}

pub fn serde_default(input: TokenStream, error_on_missing_default: bool) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let fields = if let syn::Data::Struct(data) = input.data {
        if let syn::Fields::Named(fields) = data.fields { fields.named }
        else { return compile_error(Span::call_site(), "Can't use unnamed field").into(); }
    } 
    else { return compile_error(Span::call_site(), "serde_default can only be applied by struct").into(); };
    let mut default_definitions = Vec::new();
    for field in fields.into_iter() {
        let field_options = match FieldOptions::from_field(&field) {
            Ok(field_options) => field_options,
            Err(_) => if error_on_missing_default { return compile_error(field.span(), "This field miss a serde default config").into(); }
                            else { return quote!{ #[warn(dead_code)] const WARNING: &str = "This field miss a serde default config, no impl for Default will be generated."; }.into() }
        };
        let name = match field.clone().ident { Some(name) => name, None => return compile_error(field.span(), "Can't put default value on anonymous field").into() };
        let default_code = match field_options.default {
            DefaultDeclaration::Trait => {
                // A simple work around to transform Vec<String> to Vec::<String> by force each generic with leading colon2.
                let mut ty = field.ty;
                ty = match ty {
                    syn::Type::Group(group) => *group.elem,
                    _ => ty
                };
                if let syn::Type::Path(path) = &mut ty {
                    for segement in path.path.segments.iter_mut() {
                        if let syn::PathArguments::AngleBracketed(angel) = &mut segement.arguments {
                            angel.colon2_token = Some(syn::token::Colon2::default());
                        }
                    }
                }
                quote!(#ty::default())
            }
            DefaultDeclaration::Path(path) => { quote!(#path()) }
        };
        default_definitions.push(quote! {
            #name: #default_code,
        });
    }
    
    let default_definitions: proc_macro2::TokenStream = default_definitions.into_iter().collect();

    let stream = quote! {
        impl std::default::Default for #ident {
            fn default() -> Self {
                #ident {
                    #default_definitions
                }
            }
        }
    };
    TokenStream::from(stream)
}

fn compile_error(span: proc_macro2::Span, message: &str) -> proc_macro2::TokenStream {
    let error = syn::Error::new(span, message).to_compile_error();
    quote!(#error)
}