// ============================================================
//! * Plugin code format
// ------------------------------------------------------------
//! For plugin file, all srvpru will obey follow orders:
//! * rust config
//! * Doc
//! * `use ...`
//! * `set_configuration!`
//! * `depend_on!`
//! * `player_attach!`
//! * `room_attach!`
//! * other macro annonucement
//! * `fn init()`
//! * other format functions
//! * `fn register_handlers()`
//! * other tool functions for handler content
//! * `fn start_server()`
//! * other tool functions for server
// ============================================================

use std::fs;
use std::path::Path;
use std::io::Read;

pub fn load_configuration<T: serde::de::DeserializeOwned>(name: &str) -> anyhow::Result<T> {
    let configuration_dir = std::env::var("SRVPRU_CONFIG_PATH").unwrap_or(".".to_string());
    for entry in fs::read_dir(configuration_dir)? {
        if let Ok(file) = entry {
            let path_name = file.path();
            let path = Path::new(&path_name);
            let file_name = path.file_stem().unwrap_or_default();
            let extenion = path.extension().unwrap_or_default().to_str().unwrap_or_default();
            if file_name == name {
                if let Ok(mut file) = fs::File::open(&path_name) {
                    match extenion {
                        "toml" => {
                            let mut data = String::new();
                            file.read_to_string(&mut data)?;
                            return Ok(toml::from_str::<T>(&data)?);
                        }
                        "yaml" => return Ok(serde_yaml::from_reader::<_, T>(file)?),
                        "json" => return Ok(serde_json::from_reader::<_, T>(file)?),
                        _ => {}
                    };
                }
            }
        }
    }
    serde_json::from_str("{}").map_err(|e| anyhow!("Cannot find configuration file for mod {}"))
}

#[macro_export]
macro_rules! set_configuration {
    ($( $(#[$attr:meta])* $field:ident:$type:ty ),*) => {
        #[derive(serde::Deserialize, Debug)]
        pub struct Configuration {
            $($(#[$attr])* pub $field: $type,)*
        }

        #[doc(hidden)]
        pub static CONFIGURATION: once_cell::sync::OnceCell<Configuration> = once_cell::sync::OnceCell::new();

        #[doc(hidden)]
        pub fn load_configuration() -> anyhow::Result<()> {
            let os_module_name = std::path::Path::new(file!()).file_stem().ok_or(anyhow!("Can not determain module name."))?;
            let module_name = os_module_name.to_str().ok_or(anyhow!("Can not transform module name to utf-8"))?;
            CONFIGURATION.set(crate::srvpru::plugins::load_configuration(module_name)?).map_err(|_| anyhow!("Configuration already set."))?;
            Ok(())
        }

        /// Get configuration for current config.
        #[inline]
        pub fn get_configuration() -> &'static Configuration {
            CONFIGURATION.get().expect(&format!("{} configuration not set", file!()))
        }
    };
}

#[macro_export]
macro_rules! depend_on {
    ($($field: literal),*) => {
        #[doc(hidden)]
        fn register_dependency() -> anyhow::Result<()> {
            let os_module_name = std::path::Path::new(file!()).file_stem().ok_or(anyhow!("Can not determain module name."))?;
            let module_name = os_module_name.to_str().ok_or(anyhow!("Can not transform module name to utf-8"))?;
            crate::srvpru::Handler::register_dependencies(module_name, vec![$($field),*]);
            Ok(())
        }
    };
}

expand_mod!("src/srvpru/plugins");
