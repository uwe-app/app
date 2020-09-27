pub mod dependency;
pub mod lock_file;
mod plugin;
pub mod registry;

pub type ResolvedPlugins = Vec<(dependency::Dependency, plugin::Plugin)>;

pub use plugin::*;
