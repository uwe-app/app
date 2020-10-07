use std::path::PathBuf;
use std::collections::HashMap;

use dirs::home;

use crate::Result;

static BASH: &str = "bash";
static ZSH: &str = "zsh";

#[cfg(unix)]
pub(crate) fn get_source_env() -> String {
    format!("source $HOME/.uwe/env\n")
}

#[cfg(windows)]
pub(crate) fn get_source_env() -> String {
    todo!("Handle source env file for windows");
}

#[cfg(unix)]
pub fn get_env_content(bin_dir: &PathBuf) -> String {
    format!("export PATH=\"{}:$PATH\"\n", bin_dir.display())
}

#[cfg(windows)]
pub fn get_env_content(bin_dir: &PathBuf) -> String {
    todo!("Handle env content for windows");
}

// Write out the env file
pub(crate) fn write(bin_dir: &PathBuf) -> Result<()> {
    let content = get_env_content(bin_dir);
    let env = cache::get_env_file()?;
    utils::fs::write_string(env, content)?;
    Ok(())
}

/// Attempt to update shell profiles to include the source for ~/.uwe/env.
pub(crate) fn update_shell_profile() -> Result<(bool, bool, String, PathBuf)> {
    let mut files: HashMap<String, Vec<String>> = HashMap::new();
    files.insert(
        BASH.to_string(),
        vec![".profile".to_string(), ".bashrc".to_string()],
    );
    files.insert(
        ZSH.to_string(),
        vec![".profile".to_string(), ".zshrc".to_string()],
    );

    let mut shell_ok = false;
    let mut shell_write = false;
    let mut shell_name = String::from("");
    let mut shell_file = PathBuf::from("");

    if let Some(home_dir) = home::home_dir() {
        let source_path = get_source_env();
        if let Ok(shell) = std::env::var("SHELL") {
            let shell_path = PathBuf::from(shell);
            if let Some(name) = shell_path.file_name() {
                let name = name.to_string_lossy().into_owned();
                shell_name = name.to_string();

                if let Some(entries) = files.get(&name) {
                    for f in entries {
                        let mut file = home_dir.clone();
                        file.push(f);
                        if file.exists() {
                            let mut contents = utils::fs::read_string(&file)?;
                            if !contents.contains(&source_path) {
                                contents.push_str(&source_path);
                                utils::fs::write_string(&file, contents)?;
                                shell_write = true;
                            }
                            shell_ok = true;
                            shell_file = file;
                        }
                    }
                }
            }
        }
    }

    // TODO: handle shells with no profile files yet!

    Ok((shell_ok, shell_write, shell_name, shell_file))
}
