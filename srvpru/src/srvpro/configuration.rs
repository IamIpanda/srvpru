use std::fs::File;
use std::fs::read_dir;
use std::path::Path;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::de::DeserializeOwned;

pub trait Configuration {
    type Ref;

    fn get() -> Self::Ref;
    fn reload();
    fn config_name(module_path: &str) -> &str {
        module_path.split("::").last().unwrap_or(module_path)
    }
}

impl Configuration for serde_json::Value {
    type Ref = arc_swap::ArcSwap<Self>;
    fn get() -> Self::Ref { panic!("Not implemented"); }
    fn reload() { panic!("Not implemented"); }
}

static ALL_IN_ONE_CONFIGURATION: Lazy<RwLock<serde_json::Value>> = Lazy::new(|| 
    RwLock::new(load_configuration::<serde_json::Value>("").unwrap_or(serde_json::json!({})))
);

pub fn load_configuration<T: Configuration + DeserializeOwned>(module_name: &str) -> anyhow::Result<T> {
    let mut module_name = T::config_name(module_name);
    if module_name == "" {
        module_name = "srvpru";
    }
    else {
        let all_in_one = ALL_IN_ONE_CONFIGURATION.read();
        let value = &all_in_one[&module_name];
        if !matches!(value, serde_json::Value::Null) {
            return Ok(serde_json::from_value(value.clone())?);
        }
    }
    let configuration_directory = "/Users/iami/Programming/mycard/srvpru2/srvpru/config".to_string();
    match read_dir(configuration_directory)?.into_iter().find_map(move |entry| {
        let file = entry.ok()?;
        let path_name = file.path();
        let path = Path::new(&path_name);
        let (file_name, extension) = get_stem_and_extension(path)?;
        if file_name != module_name {
            trace!("File name don't match: {}, {}", path_name.to_str().unwrap_or("NO_PATH"), module_name);
            return None
        }
        match extension {
            "json" => Some(serde_json::from_reader(File::open(path).ok()?).ok()?),
            _ => None
        }
    }) {
        Some(conf) => return Ok(conf),
        None => {
            warn!("Can't find proper configuration file for {}. Try to get a default value.", module_name);
            Ok(serde_json::from_str("{}")?)
        }
    }
}

fn get_stem_and_extension(path: &Path) -> Option<(&str, &str)> {
    let file_name = path.file_stem()?.to_str()?;
    let file_extension = path.extension()?.to_str()?;
    Some((file_name, file_extension))
}

#[test]
fn get_ygopro_config() {
    let config = load_configuration::<super::YgoproConfiguration>("").unwrap();
    println!("{:?}", config);
}
