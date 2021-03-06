use std::path::Path;
use std::fs;
use proc_macro::TokenStream;
use proc_macro2::Span;

use syn::Token;
use syn::parse_macro_input;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::LitStr;
use quote::quote;

struct InitPluginsInput {
    path: LitStr,
    _comma: Token![,],
    execution: proc_macro2::TokenStream,
}

impl Parse for InitPluginsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(InitPluginsInput { 
            path: input.parse()?, 
            _comma: input.parse()?,
            execution: input.parse()? 
        })
    }
} 

pub fn execute_for_each_under_dir(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as InitPluginsInput);
    let path = input.path.value();
    let execution = input.execution.to_string();
    let logical_path = "crate::".to_string() + &path.replace("src/", "").replace("/", "::") + "::";
    let dir = match std::env::var_os("CARGO_MANIFEST_DIR") {
        Some(manifest_dir) => std::path::PathBuf::from(manifest_dir).join(path),
        None => std::path::PathBuf::from(path),
    };
    println!("Searching directory {:?}...", &dir);
    println!("Generate logical path is {:}", &logical_path);

    let expanded = match scan_directory(dir) {
        Ok(names) => names.into_iter().map(|name| {
            let fullname = logical_path.clone() + &name;
            let actual_execution = execution.replace("# name", &name)
                                                   .replace("#name", &name)
                                                   .replace("# fullname", &fullname)
                                                   .replace("#fullname", &fullname);
            let actual_execution_stream: TokenStream = actual_execution.parse().unwrap();
            let actual_execution_stream: proc_macro2::TokenStream = actual_execution_stream.into();
            quote!( #actual_execution_stream; )
        }).collect(),
        Err(err) => syn::Error::new(Span::call_site(), err).to_compile_error(),
    };
    TokenStream::from(expanded)
}

fn scan_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() { continue; }
        let file_name = entry.file_name();
        if file_name == "mod.rs" || file_name == "lib.rs" || file_name == "main.rs" { continue; }

        let path = Path::new(&file_name);
        if path.extension() == Some(std::ffi::OsStr::new("rs")) {
            if let Some(module_name) = path.file_stem() {
                if let Ok(module_name) = module_name.to_os_string().into_string() {
                    names.push(module_name);
                }
            }
        }
    }
    names.sort();
    Ok(names)
}
