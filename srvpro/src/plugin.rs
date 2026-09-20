mod srvpro;
mod chat_command;
pub mod welcome;
mod random_match;
mod tip;
mod virtual_password;
mod delay_replay;
pub mod dialogues;
mod hide_name;
mod report;
mod deck_report;
pub mod bad_words;
mod retry_handle;
mod chat_color;
mod death;
mod reconnect;
mod side_timeout;
mod soumatou;
pub mod stop;
pub mod base;

use linkme::distributed_slice;

#[distributed_slice]
pub static SRVPRO_DEFAULT_ENABLED_PLUGINS: [&'static str];
#[distributed_slice]
pub static SRVPRO_PLUGIN_DEPENDENCY: [(&'static str, &'static [&'static str])];

pub type InitConfiguration = fn(&mut anymap3::Map<dyn anymap3::CloneAny + Send + Sync>) -> Result<(), Box<dyn std::error::Error>>;
#[distributed_slice]
pub static SRVPRO_CONFIGURATIONS: [(&'static str, InitConfiguration)];
pub use SRVPRO_CONFIGURATIONS as CONFIGURATIONS;

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
