//! Link-time plugin registry.
//!
//! Plugins register their name, their default-enabled flag, their dependencies and their
//! configuration initializer here. Nothing is loaded at runtime.

mod srvpro;
pub mod chat_command;
pub mod pass_command;
pub mod welcome;
pub mod random_match;
pub mod tip;
pub mod virtual_password;
pub mod delay_replay;
pub mod dialogues;
pub mod hide_name;
pub mod report;
pub mod deck_report;
pub mod bad_words;
pub mod cloud_replay;
pub mod recover;
pub mod retry_handle;
pub mod chat_color;
pub mod death;
pub mod miniluv;
pub mod reconnect;
pub mod side_timeout;
pub mod soumatou;
pub mod stop;
pub mod base;
pub mod haisui;

/// Plugins enabled when [`crate::message::Init`] does not name any.
#[distributed_slice]
pub static SRVPRO_DEFAULT_ENABLED_PLUGINS: [&'static str];
/// Dependencies of each plugin, enabled together with it.
#[distributed_slice]
pub static SRVPRO_PLUGIN_DEPENDENCY: [(&'static str, &'static [&'static str])];

/// Creates the default configuration value of a plugin.
pub type InitConfiguration = fn(&mut anymap3::Map<dyn anymap3::CloneAny + Send + Sync>) -> Result<(), Box<dyn std::error::Error>>;
/// Configuration initializers, keyed by plugin name.
#[distributed_slice]
pub static SRVPRO_CONFIGURATIONS: [(&'static str, InitConfiguration)];
pub use SRVPRO_CONFIGURATIONS as CONFIGURATIONS;

/// Declare the plugins this module depends on.
#[macro_export]
macro_rules! register_dependencies {
    ($($dep:path),*) => {
        #[linkme::distributed_slice($crate::plugin::SRVPRO_PLUGIN_DEPENDENCY)]
        static DEPENDENCIES: (&'static str, &'static [&'static str]) = (
            module_path!(),
            &[$($dep),*]
        );
    };
    ($($dep:literal),*) => {
        #[linkme::distributed_slice($crate::plugin::SRVPRO_PLUGIN_DEPENDENCY)]
        static DEPENDENCIES: (&'static str, &'static [&'static str]) = (
            module_path!(),
            &[$($dep),*]
        );
    };
}
pub use register_dependencies;
