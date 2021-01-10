pub mod dependency;
pub mod features;
pub mod lock_file;
mod plugin;
pub mod plugin_spec;
pub mod registry;
pub mod version_key;

pub type ResolvedPlugins = Vec<(dependency::Dependency, plugin::Plugin)>;

pub use plugin::*;
pub use plugin_spec::PluginSpec;
pub use version_key::VersionKey;
