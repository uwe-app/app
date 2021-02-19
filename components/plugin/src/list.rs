use log::{info, warn};

use config::{
    lock_file::LockFile, plugin::dependency::DependencyTarget, Config,
};

use crate::{
    dependencies::{self, DependencyTree, PluginDependencyState},
    Result,
};

const TREE_BAR: &str = "│";
const TREE_BRANCH: &str = "├──";
const TREE_CORNER: &str = "└──";

/// List the plugin dependencies for a project.
pub async fn list_dependencies(config: &Config) -> Result<()> {
    info!("{} ({})", config.host(), config.project().display());
    if let Some(ref dependencies) = config.dependencies() {
        let path = LockFile::get_lock_file(config.project());
        let lock = LockFile::load(&path)?;

        let tree = dependencies::resolve(config.project(), dependencies, &lock)
            .await?;
        print_dependencies(&tree, 0)?;
    } else {
        info!("No plugin dependencies defined");
    }
    Ok(())
}

fn format_item(name: &str, state: &PluginDependencyState) -> String {
    let info: Option<String> =
        if let Some(ref target) = state.dependency().target {
            match target {
                DependencyTarget::File { ref path } => {
                    Some(format!("{}", path.display()))
                }
                DependencyTarget::Archive { ref archive } => {
                    Some(format!("{}", archive.display()))
                }
                DependencyTarget::Repo {
                    ref git,
                    ref prefix,
                } => {
                    if let Some(ref prefix) = prefix {
                        Some(format!("{} ({})", git, prefix))
                    } else {
                        Some(format!("{}", git))
                    }
                }
                DependencyTarget::Local { .. } => Some(String::from("scoped")),
            }
        } else {
            None
        };

    let item = if let Some(ref version) = state.target_version() {
        format!("{}@{}", name, version)
    } else {
        format!("{}", name)
    };

    if let Some(info) = info {
        format!("{} ({})", item, info)
    } else {
        item
    }
}

fn print_dependencies(tree: &DependencyTree, depth: usize) -> Result<()> {
    let size = tree.len();
    let indent = "    ".repeat(depth);
    for (index, (name, state)) in tree.iter().enumerate() {
        let last = index == size - 1;
        let mark = if last { TREE_CORNER } else { TREE_BRANCH };
        let initial = if depth == 0 {
            indent.clone()
        } else {
            format!("{}{}", TREE_BAR, &indent[1..])
        };
        if state.not_found() && !state.is_local_scope() {
            warn!("{}{} {}", initial, mark, format_item(name, state));
        } else {
            info!("{}{} {}", initial, mark, format_item(name, state));
        }

        if !state.transitive().is_empty() {
            print_dependencies(state.transitive(), depth + 1)?;
        }
    }
    Ok(())
}
