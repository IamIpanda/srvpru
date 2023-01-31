use std::{path::PathBuf, fs::File, io::Write, collections::HashMap};

use darling::FromAttributes;
use srvpru_build_utils::scan_directory;
use srvpru_build_utils::is_derive;
use srvpru_build_utils::generate_enum;

use syn::Item;

#[derive(FromAttributes, Debug)]
#[darling(attributes(message), allow_unknown_fields)]
struct MessageParameters {
    flag: u8
}

static mut MESSAGE_FLAGS: Option<HashMap<String, HashMap<String, u32>>> = None;
fn scan_structs() {
    unsafe { MESSAGE_FLAGS = Some(HashMap::new()) }
    let path = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    scan_directory(PathBuf::from(path).join("src"), |status, item| {
        if ! is_derive(item, "Message") { return; }
        let item_struct = match item {
            Item::Struct(item) => item,
            _ => return
        };
        println!("See a Message struct {:?}.", item_struct.ident.to_string());
        let parameter = match MessageParameters::from_attributes(&item_struct.attrs) {
            Ok(parameter) => parameter,
            Err(err) => { println!("{:?}", err); return; }
        };

        unsafe {
            let message_flags = MESSAGE_FLAGS.as_mut().unwrap();
            let sub_flags = message_flags.entry(status.get_file_name()).or_insert(HashMap::new());
            sub_flags.insert(item_struct.ident.to_string(), parameter.flag as u32);
        }
    }).ok();
    let message_flags = unsafe { &MESSAGE_FLAGS.take().unwrap() };
    for (file_name, sub_flags) in message_flags {
        let write_path = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join(format!("{}.rs", file_name));
        let mut file = File::create(write_path).unwrap();
        let enum_string = generate_enum("MessageType".to_string(), sub_flags, |option, value| format!("{} = {}", option, value));
        write!(file, "build_it![\n{}];\n", enum_string).ok();
    }
}   

fn main() {
    scan_structs();    
}
