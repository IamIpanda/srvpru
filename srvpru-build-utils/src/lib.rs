use std::{path::PathBuf, fs, io::Read, collections::HashMap};

use quote::quote;
use syn::{parse_file, Item};

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub struct Status {
    directory: Option<String>,
    file: Option<String>,
    // mod_name: Option<String>
}

impl Status {
    pub fn get_file_name(&self) -> String {
        PathBuf::from(self.file.clone().unwrap()).file_stem().unwrap().clone().to_os_string().into_string().unwrap()
    }
}

pub fn scan_directory(path: PathBuf, predicate: fn(status: &Status, item: &Item) -> ()) -> Result<()> {
    let mut status = Status {
        directory: None,
        file: None,
        // mod_name: None
    };
    _scan_directory(path, &mut status, predicate)
}

fn _scan_directory(path: PathBuf, status: &mut Status, predicate: fn(status: &Status, item: &Item) -> ()) -> Result<()> {
    println!("Scanning {:?}", path);
    status.directory = Some(path.clone().into_os_string().into_string().unwrap());
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let _type = entry.file_type()?;
        if _type.is_dir() {
            _scan_directory(entry.path(), status, predicate).ok();
        } else if _type.is_file() {
            scan_file(entry.path(), status, predicate).ok();
        }
    }
    Ok(())
}

fn scan_file(path: PathBuf, status: &mut Status, predicate: fn(status: &Status, item: &Item) -> ()) -> Result<()> {
    println!("Scanning {:?}", path);
    status.file = Some(path.clone().into_os_string().into_string().unwrap());
    let mut file = fs::File::open(path)?;  
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    scan_code(buf,  status, predicate)?;
    Ok(())
}

fn scan_code(code: String, status: &mut Status, predicate: fn(status: &Status, item: &Item) -> ()) -> Result<()> {
    let file = parse_file(&code)?;
    scan_items(file.items, status, predicate);
    Ok(())
}

fn scan_items(items: Vec<Item>, status: &mut Status, predicate: fn(status: &Status, item: &Item) -> ()) {
    for item in items {
        scan_item(item, status, predicate)
    }
}

fn scan_item(item: Item, status: &mut Status, predicate: fn(status: &Status, item: &Item) -> ()) {
    match item {
        syn::Item::Mod(item_mod) => if let Some(content) = item_mod.content { 
            scan_items(content.1, status, predicate) 
        },
        _ => predicate(&status, &item)
    }
}

pub fn is_derive(item: &Item, derive_name: &'static str) -> bool {
    let item_struct = match item {
        Item::Struct(item_struct) => item_struct,
        _ => return false
    };
    for attr in item_struct.attrs.iter() {
        if attr.path.is_ident("derive") {
            let trait_tokens = &attr.tokens;
            // TODO: parse tokens as Punctuated<TokenStream, Comma>
            if quote!(#trait_tokens).to_string().contains(derive_name) {
                return true
            }
        }
    }
    return false
}

pub fn generate_enum(_enum_name: String, contents: &HashMap<String, u32>, line: fn(option: &String, value: &u32) -> String) -> String {
    let mut str = "".to_string();
    let mut content_vec: Vec<(&String, &u32)> = contents.iter().collect();
    content_vec.sort_by(|a, b| a.1.cmp(b.1));
    for (option, value) in content_vec {
        str += "    ";
        str += &line(option, value);
        str += ",\n"
    }
    return str;
}
