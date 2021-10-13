use std::fs;
use std::path::Path;
use std::io::Read;

pub fn load_configuration<T: serde::de::DeserializeOwned>(name: &str) -> anyhow::Result<T> {
    let configuration_dir = "/Users/iami/Programming/mycard/srvpru/config";
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
                            file.read_to_string(&mut data).unwrap();
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
    Err(anyhow!("Cannot find configuration file for mod {}", name))
}

#[macro_export]
macro_rules! set_configuration {
    ($( $(#[$attr:meta])* $field:ident:$type:ty ),*) => {
        #[derive(serde::Deserialize, Debug)]
        pub struct Configuration {
            $($(#[$attr])* pub $field: $type,)*
        } 
        pub static CONFIGURATION: once_cell::sync::OnceCell<Configuration> = once_cell::sync::OnceCell::new();

        pub fn load_configuration() -> anyhow::Result<()> {
            let os_module_name = std::path::Path::new(file!()).file_stem().ok_or(anyhow!("Can not determain module name."))?;
            let module_name = os_module_name.to_str().ok_or(anyhow!("Can not transform module name to utf-8"))?;
            CONFIGURATION.set(crate::srvpru::plugins::load_configuration(module_name)?).map_err(|_| anyhow!("Configuration already set."))?;
            Ok(())
        }

        pub fn get_configuration() -> &'static Configuration {
            CONFIGURATION.get().expect(&format!("{} configuration not set", file!()))
        }
    };
}

#[macro_export]
macro_rules! depend_on {
    () => {
        
    };
}

expand_mod!("src/srvpru/plugins");