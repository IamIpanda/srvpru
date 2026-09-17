use std::sync::Arc;
use std::sync::LazyLock;

use arc_swap::ArcSwap;

use crate::message as srvpro;

#[derive(Clone)]
pub struct Configuration {
    pub enable_plugins: hashbrown::HashSet<String>,
    pub configurations: anymap3::Map<dyn anymap3::CloneAny + Send + Sync>,
}

impl Default for Configuration {
    fn default() -> Self {
        let mut configuration = Self {
            enable_plugins: hashbrown::HashSet::new(),
            configurations: anymap3::Map::new(),
        };
        for plugin_name in crate::plugin::SRVPRO_DEFAULT_ENABLED_PLUGINS {
            configuration.enable_plugin(plugin_name);
        }
        configuration
    }
}

impl Configuration {
    pub fn empty() -> Self {
        Self {
            enable_plugins: Default::default(),
            configurations: Default::default(),
        }
    }

    pub fn enable_plugin(&mut self, plugin_name: &str) {
        if !self.enable_plugins.insert(plugin_name.to_string()) { return }
        log::info!("Enable plugin {}", plugin_name);
        for (name, init_configuration) in crate::plugin::SRVPRO_CONFIGURATIONS {
            if plugin_name == *name { init_configuration(&mut self.configurations).ok(); }
        }
        for (name, dependencies) in crate::plugin::SRVPRO_PLUGIN_DEPENDENCY {
            if plugin_name == *name {
                for dependency in *dependencies {
                    self.enable_plugin(dependency);
                }
            }
        }
    }

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

    pub fn disable_plugin(&mut self, plugin_name: &str) {
        self.enable_plugins.remove(plugin_name);
    }
}

static CONFIGURATION: LazyLock<ArcSwap<Configuration>> = LazyLock::new(|| ArcSwap::from_pointee(Configuration::default()));

pub fn get() -> Arc<Configuration> {
    CONFIGURATION.load_full()
}

#[handler(srvpro::Init)]
#[register_to(crate::SRVPRO_GLOBAL_HANDLERS as crate::GlobalHandler)]
async fn on_init() {
    for module_name in get().enable_plugins.iter() {
        crate::process(srvpro::PluginEnabled { module_name: module_name.clone() }.into_message()).await;
    }
}

pub async fn enable(plugin_name: &str) {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        next.enable_plugin(plugin_name);
        Arc::new(next)
    });
    crate::process(srvpro::PluginEnabled { module_name: plugin_name.to_string() }.into_message()).await;
    crate::process(srvpro::ConfigurationChanged.into_message()).await;
}

pub async fn enable_with_configuration<PluginConfiguration>(plugin_name: &str, configuration: PluginConfiguration) where PluginConfiguration: Clone + Send + Sync + 'static {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        next.enable_plugin_with_configuration(plugin_name, configuration.clone());
        Arc::new(next)
    });
    crate::process(srvpro::PluginEnabled { module_name: plugin_name.to_string() }.into_message()).await;
    crate::process(srvpro::ConfigurationChanged.into_message()).await;
}

pub async fn disable(plugin_name: &str) {
    CONFIGURATION.rcu(|current| {
        let mut next = (**current).clone();
        next.disable_plugin(plugin_name);
        Arc::new(next)
    });
    crate::process(srvpro::PluginDisabled { module_name: plugin_name.to_string() }.into_message()).await;
    crate::process(srvpro::ConfigurationChanged.into_message()).await;
}
