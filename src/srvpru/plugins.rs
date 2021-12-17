// ====================================================================================================
//  Plugin
// ----------------------------------------------------------------------------------------------------
//! Plugin owns all plugins, and offer general functions which plugins use.
//! 
//! For plugin file, all srvpru should obey follow orders:
//! * rust config
//! * rust docs
//! * `use ...`
//! * [`set_configuration!`]
//! * [`depend_on!`]
//! * [`player_attach!`]
//! * [`room_attach!`]
//! * other macro annonucement
//! * `fn init()`
//! * other format functions
//! * `fn register_handlers()`
//! * other tool functions for handler content
//! * `fn start_server()`
//! * other tool functions for server
// ====================================================================================================

use std::fs;
use std::path::Path;
use std::io::Read;

// ----------------------------------------------------------------------------------------------------
//  load_configuration
// ----------------------------------------------------------------------------------------------------
/// Try to load configuration file from path. \
/// Will try file `.json`, `.yaml`, and `.toml`. \
/// If no file is found, will try to deserialize it from `"{}"`.
/// 
/// Set environment variable `SRVPRU_CONFIG_PATH` to point out where configs put.
/// 
/// #### Arguments
/// * `T`: target configuration struct. for most of cases, use `Configuration`.
/// * `name`: the file name without extansion.
// ----------------------------------------------------------------------------------------------------
pub fn load_configuration<T: serde::de::DeserializeOwned>(name: &str) -> anyhow::Result<T> {
    let configuration_dir = crate::srvpru::configuration_path(); 
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
    
    serde_json::from_str("{}").map_err(|e| anyhow!(format!("Cannot find configuration file for mod {}: {}", name, e)))
}

#[doc(hidden)]
pub fn process_plugin_result(name: &str, result: anyhow::Result<()>) {
    match result {
        Ok(_) => info!("Loaded plugin {}", name),
        Err(err) => {
            if plugin_enabled(name) {
                error!("Load plugin {} failed: {}", name, err)
            }
            else {
                info!("Plugin {} load failed, but ignored because it not enabled: {}", name, err)
            }
        }
    };
}


// ----------------------------------------------------------------------------------------------------
//  set_configuration!
// ----------------------------------------------------------------------------------------------------
/// Set a configuration struct named `Configuration` for current mod. \
/// Need to call `load_configuration()?` in `init()`.
/// 
/// If all fields are marked with `#[serde(default)]`, the `Configuration` will implement `Default`.
/// 
/// #### Example
/// ```
/// set_configuration! {
///     some_field: String,
///     my_configuration_field: u64  
/// }
/// 
/// pub fn init() -> anyhow::Result {
///     load_configuration()?;
///     println!("{}", get_configuration().some_field);
///     Ok(())
/// }
/// ```
// ----------------------------------------------------------------------------------------------------
#[macro_export]
macro_rules! set_configuration {
    ($( $(#[$attr:meta])* $field:ident:$type:ty ),*) => {
        #[derive(serde::Serialize, serde::Deserialize, try_serde_default, Debug)]
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
        /// Will **PANIC** if load failed.
        #[inline]
        pub fn get_configuration() -> &'static Configuration {
            CONFIGURATION.get().expect(&format!("{} configuration not set", file!()))
        }
    };
}

#[macro_export]
macro_rules! set_reloadable_configuration {
    ($( $(#[$attr:meta])* $field:ident:$type:ty ),*) => {
        #[derive(serde::Serialize, serde::Deserialize, try_serde_default, Debug)]
        pub struct Configuration {
            $($(#[$attr])* pub $field: $type,)*
        }

        #[doc(hidden)]
        pub static CONFIGURATION: once_cell::sync::OnceCell<parking_lot::RwLock<Configuration>> = once_cell::sync::OnceCell::new();

        pub fn init_configuration(plugin_name: &'static str) -> anyhow::Result<()> {
            load_configuration()?;
            register_reload_handlers(plugin_name)?;
            Ok(())
        }

        #[doc(hidden)]
        fn load_configuration() -> anyhow::Result<()> {
            let os_module_name = std::path::Path::new(file!()).file_stem().ok_or(anyhow!("Can not determain module name."))?;
            let module_name = os_module_name.to_str().ok_or(anyhow!("Can not transform module name to utf-8"))?;
            let configuration = crate::srvpru::plugins::load_configuration(module_name)?;
            if let Some(configuration_wrapper) = CONFIGURATION.get() {
                let mut guard = configuration_wrapper.write();
                *guard = configuration;
            }
            else {
                CONFIGURATION.set(parking_lot::RwLock::new(configuration)).map_err(|_| anyhow!("Failed to set configuration."))?;
            }
            Ok(())
        }

        fn register_reload_handlers(plugin_name: &'static str) -> anyhow::Result<()> {
            let os_module_name = std::path::Path::new(file!()).file_stem().ok_or(anyhow!("Can not determain module name."))?;
            let module_name = os_module_name.to_str().ok_or(anyhow!("Can not transform module name to utf-8"))?;
            crate::srvpru::Handler::before_message::<crate::ygopro::message::srvpru::Reload, _>(100, &format!("{}_configuration_reloader", module_name), |_, _| Box::pin(async move {
                load_configuration()?;
                Ok(false)
            })).register_for_plugin(plugin_name);
            Ok(())
        }

        /// Get configuration for current config.
        /// Will **PANIC** if load failed.
        #[inline]
        pub fn get_configuration<'a>() -> parking_lot::RwLockReadGuard<'a, Configuration> {
            let configuration = CONFIGURATION.get().expect(&format!("{} configuration not set", file!()));
            configuration.read()
        }
    };
}

// ----------------------------------------------------------------------------------------------------
//  depend_on!
// ----------------------------------------------------------------------------------------------------
/// Register dependency for current plugin mod. \
/// Need to call `register_dependencies()?` in `init()`.
/// 
/// #### Example
/// ```
/// depend_on! [
///     "stage_recorder",
///     "position_recorder"
/// ]
/// 
/// fn init() -> anyhow::Result() {
///     register_dependecies()?;
///     Ok(())
/// }
/// ```
// ----------------------------------------------------------------------------------------------------
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

// ----------------------------------------------------------------------------------------------------
//  expand_plugins_undeer_dir!
// ----------------------------------------------------------------------------------------------------
/// Generate following two codes for each file under given directory:
/// ```
/// mod [handler_name];
/// ```
/// and
/// ```
/// pub fn init() -> anyhow::Result {
///     [handler_name]::init();
/// }
/// ```
// ----------------------------------------------------------------------------------------------------
#[macro_export]
macro_rules! expand_plugins_under_dir {
    ($directory: literal) => {  
        execute_for_each_under_dir!($directory, pub mod #name);
        /// Init all plugins under this mod.
        pub fn init() -> anyhow::Result<()> {
            execute_for_each_under_dir!($directory, crate::srvpru::plugins::process_plugin_result("#name", #name::init()));
            Ok(())
        }
    };
}

// ----------------------------------------------------------------------------------------------------
//  plugin_enabled
// ----------------------------------------------------------------------------------------------------
/// Check if a plugin is listed in [srvpru configuration](crate::srvpru::Configuration#plguins).
// ----------------------------------------------------------------------------------------------------
pub fn plugin_enabled(plugin_name: &str) -> bool {
    crate::srvpru::get_configuration().plugins.contains(&plugin_name.to_string())
}

// ----------------------------------------------------------------------------------------------------
//  use_http_client
// ----------------------------------------------------------------------------------------------------
/// Create a simple [client](reqwest::Client) for http call. \
/// Call `get_http_client()`.
// ----------------------------------------------------------------------------------------------------
#[macro_export]
macro_rules! use_http_client {
    () => {
        static REQWEST_CLIENT: once_cell::sync::OnceCell<reqwest::Client> = once_cell::sync::OnceCell::new();

        /// get a http client reference for http call.
        #[doc(hidden)]
        fn get_http_client<'a>() -> &'a reqwest::Client {
            REQWEST_CLIENT.get_or_init(|| reqwest::Client::new())
        }
    }
}

expand_plugins_under_dir!("src/srvpru/plugins");