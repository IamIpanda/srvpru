//! Process-wide plugin configuration.
//!
//! The configuration is built from [`crate::message::Init`] at startup and replaced whenever
//! a plugin is enabled or disabled at runtime.

use std::sync::Arc;
use std::sync::LazyLock;

use arc_swap::ArcSwap;

use crate::message as srvpro;

/// Configuration key listing the plugins extra enabled at startup.
pub const ENABLE_PLUGINS_KEY: &str = "enable_plugins";
/// Configuration key listing the plugins kept disabled at startup.
pub const DISABLE_PLUGINS_KEY: &str = "disable_plugins";

/// The enabled plugins and the configuration values they own.
#[derive(Clone)]
pub struct Configuration {
    pub enable_plugins: hashbrown::HashSet<String>,
    pub configurations: anymap3::Map<dyn anymap3::CloneAny + Send + Sync>,
}

impl Configuration {
    /// An empty configuration with no plugin enabled.
    pub fn empty() -> Self {
        Self {
            enable_plugins: Default::default(),
            configurations: Default::default(),
        }
    }

    /// Build a configuration from a set of plugin names.
    pub fn resolve(enable_plugins: hashbrown::HashSet<String>) -> Self {
        let mut configuration = Self::empty();
        for plugin_name in enable_plugins {
            configuration.enable_plugin(&plugin_name);
        }
        configuration
    }

    fn init_plugin(&mut self, plugin_name: &str) {
        for (name, init_configuration) in crate::plugin::SRVPRO_CONFIGURATIONS {
            if plugin_name == *name { init_configuration(&mut self.configurations).ok(); }
        }
    }

    /// Enable a plugin and every plugin it depends on.
    pub fn enable_plugin(&mut self, plugin_name: &str) {
        if !self.enable_plugins.insert(plugin_name.to_string()) { return }
        log::debug!("Enable plugin {}", plugin_name);
        self.init_plugin(plugin_name);
        for (name, dependencies) in crate::plugin::SRVPRO_PLUGIN_DEPENDENCY {
            if plugin_name == *name {
                for dependency in *dependencies {
                    self.enable_plugin(dependency);
                }
            }
        }
    }

    /// Enable a plugin with an explicit configuration value.
    pub fn enable_plugin_with_configuration<PluginConfiguration>(&mut self, plugin_name: &str, configuration: PluginConfiguration) where PluginConfiguration: Clone + Send + Sync + 'static {
        self.enable_plugins.insert(plugin_name.to_string());
        self.configurations.insert(configuration);
        for (name, dependencies) in crate::plugin::SRVPRO_PLUGIN_DEPENDENCY {
            if plugin_name == *name {
                for dependency in *dependencies {
                    self.enable_plugin(dependency);
                }
            }
        }
    }

    /// Disable a plugin without touching the plugins that depend on it.
    pub fn disable_plugin(&mut self, plugin_name: &str) {
        self.enable_plugins.remove(plugin_name);
    }
}

/// Merge comma-separated plugin lists into [`crate::message::Init`].
pub fn apply_plugin_lists(init: &mut srvpro::Init, enable_plugins: Option<&str>, disable_plugins: Option<&str>) {
    if let Some(list) = enable_plugins {
        init.enable_plugins.extend(list.split(',').map(str::trim).filter(|plugin_name| !plugin_name.is_empty()).map(ToString::to_string));
    }
    if let Some(list) = disable_plugins {
        for plugin_name in list.split(',').map(str::trim).filter(|plugin_name| !plugin_name.is_empty()) {
            init.enable_plugins.remove(plugin_name);
        }
    }
}

static CONFIGURATION: LazyLock<ArcSwap<Configuration>> = LazyLock::new(|| ArcSwap::from_pointee(Configuration::empty()));

/// The current configuration.
pub fn get() -> Arc<Configuration> {
    CONFIGURATION.load_full()
}

/// A clone of the configuration value owned by plugin `T`.
pub fn get_configuration<T: Clone + Send + Sync + 'static>() -> Option<T> {
    CONFIGURATION.load_full().configurations.get::<T>().cloned()
}

#[handler(srvpro::Init)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as crate::GlobalHandler)]
async fn on_init(init: &srvpro::Init) {
    CONFIGURATION.store(Arc::new(Configuration::resolve(init.enable_plugins.clone())));
    for module_name in get().enable_plugins.iter() {
        crate::process(srvpro::PluginEnabled { module_name: module_name.clone() }.into_message()).await;
    }
}

/// Enable a plugin at runtime and notify the handlers.
pub async fn enable(plugin_name: &str) {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        next.enable_plugin(plugin_name);
        Arc::new(next)
    });
    crate::process(srvpro::PluginEnabled { module_name: plugin_name.to_string() }.into_message()).await;
    crate::process(srvpro::ConfigurationChanged.into_message()).await;
}

/// Enable a plugin with a configuration value at runtime and notify the handlers.
pub async fn enable_with_configuration<PluginConfiguration>(plugin_name: &str, configuration: PluginConfiguration) where PluginConfiguration: Clone + Send + Sync + 'static {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        next.enable_plugin_with_configuration(plugin_name, configuration.clone());
        Arc::new(next)
    });
    crate::process(srvpro::PluginEnabled { module_name: plugin_name.to_string() }.into_message()).await;
    crate::process(srvpro::ConfigurationChanged.into_message()).await;
}

/// Disable a plugin at runtime and notify the handlers.
pub async fn disable(plugin_name: &str) {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        next.disable_plugin(plugin_name);
        Arc::new(next)
    });
    crate::process(srvpro::PluginDisabled { module_name: plugin_name.to_string() }.into_message()).await;
    crate::process(srvpro::ConfigurationChanged.into_message()).await;
}

pub async fn update_configuration<PluginConfiguration>(configuration: PluginConfiguration) where PluginConfiguration: Clone + Send + Sync + 'static {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        next.configurations.insert(configuration.clone());
        Arc::new(next)
    });
}

pub fn rebuild<PluginName: AsRef<str>>(plugin_names: &[PluginName]) {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        for plugin_name in plugin_names {
            next.init_plugin(plugin_name.as_ref());
        }
        Arc::new(next)
    });
}

pub async fn rebuild_and_notify<PluginName: AsRef<str>>(plugin_names: &[PluginName]) {
    rebuild(plugin_names);
    crate::process(srvpro::ConfigurationChanged.into_message()).await;
}
