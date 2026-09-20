pub mod config_manager {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::LazyLock;

    use arc_swap::ArcSwap;
    use hashbrown::HashMap;

    const CONFIG_FILE_EXTENSIONS: [&str; 6] = ["json", "yml", "yaml", "toml", "conf", "ini"];

    pub struct ConfigManager {
        entries: HashMap<String, String>,
    }

    impl ConfigManager {
        fn empty() -> Self {
            ConfigManager { entries: HashMap::new() }
        }

        fn new() -> Self {
            let mut config_manager = ConfigManager { entries: HashMap::new() };
            let config_path = std::env::var("SRVPRO_CONFIG_PATH").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/config").to_string());
            if let Ok(entries) = read_config_directory(Path::new(&config_path)) {
                config_manager.entries.extend(entries);
            }
            for (key, value) in std::env::vars() {
                if let Some(name) = key.strip_prefix("SRVPRO_") {
                    config_manager.entries.insert(environment_key(name), value);
                }
            }
            config_manager
        }

        pub fn get(&self, key: &str) -> Option<&str> {
            self.entries.get(key).map(|s| s.as_str())
        }

        pub fn prefixed(&self, prefix: &str) -> BTreeMap<String, String> {
            self.entries.iter().filter(|(key, _)| key.starts_with(prefix)).map(|(key, value)| (key.clone(), value.clone())).collect()
        }
    }

    fn read_config_directory(config_path: &Path) -> std::io::Result<HashMap<String, String>> {
        let mut entries = HashMap::new();
        for entry in fs::read_dir(config_path)? {
            let Ok(entry) = entry else { continue };
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }
            let Some(extension) = file_path.extension().and_then(|extension| extension.to_str()) else { continue };
            let extension = extension.to_ascii_lowercase();
            if !CONFIG_FILE_EXTENSIONS.contains(&extension.as_str()) {
                continue;
            }
            let Some(file_name) = file_path.file_stem().and_then(|file_name| file_name.to_str()) else { continue };
            let Some(content) = fs::read_to_string(&file_path).ok() else { continue };
            let Some(entries_from_file) = parse_config_file(&content, &extension, file_name) else { continue };
            entries.extend(entries_from_file);
        }
        Ok(entries)
    }

    fn parse_config_file(content: &str, extension: &str, file_name: &str) -> Option<HashMap<String, String>> {
        let value: serde_json::Value = match extension {
            "json" => serde_json::from_str(content).ok()?,
            "yml" | "yaml" => serde_yaml::from_str(content).ok()?,
            "toml" => toml::from_str(content).ok()?,
            "conf" | "ini" => from_ini(content)?,
            _ => return None,
        };
        let object = value.as_object()?;
        let mut entries = HashMap::with_capacity(object.len());
        for (key, value) in object {
            entries.insert(format!("{}_{}", file_name.to_ascii_lowercase(), key.to_lowercase()), value_to_entry_string(value));
        }
        Some(entries)
    }

    fn from_ini(content: &str) -> Option<serde_json::Value> {
        let mut object = serde_json::Map::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(eq) = line.find('=') else { continue };
            let key = line[..eq].trim();
            let value = line[eq + 1..].trim();
            object.insert(key.to_ascii_lowercase(), serde_json::Value::String(value.to_string()));
        }
        Some(serde_json::Value::Object(object))
    }

    fn environment_key(name: &str) -> String {
        name.to_ascii_lowercase()
    }

    fn value_to_entry_string(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(string) => string.clone(),
            _ => value.to_string(),
        }
    }

    static CONFIG_MANAGER: LazyLock<ArcSwap<ConfigManager>> = LazyLock::new(|| ArcSwap::from_pointee(ConfigManager::empty()));

    #[handler(crate::message::Init, priority = 10)]
    #[register_to(crate::SRVPRO_GLOBAL_HANDLERS as crate::GlobalHandler)]
    fn on_init() {
        CONFIG_MANAGER.store(Arc::new(ConfigManager::new()));
    }

    pub fn load() -> arc_swap::Guard<Arc<ConfigManager>> {
        CONFIG_MANAGER.load()
    }

    pub fn update(entries: Vec<(String, String)>) {
        CONFIG_MANAGER.rcu(|current| {
            let mut next = ConfigManager { entries: current.entries.clone() };
            next.entries.extend(entries.iter().cloned());
            Arc::new(next)
        });
    }

    pub fn remove(keys: &[String]) {
        CONFIG_MANAGER.rcu(|current| {
            let mut next = ConfigManager { entries: current.entries.clone() };
            for key in keys {
                next.entries.remove(key);
            }
            Arc::new(next)
        });
    }
}
