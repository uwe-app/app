use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Run a command.
pub fn run(cmd: &str, args: &[&str], cwd: Option<PathBuf>) -> std::io::Result<()> {
    let cwd = if let Some(cwd) = cwd {
        cwd.to_path_buf()
    } else {
        std::env::current_dir()?
    };

    let mut command = Command::new(cmd);
    command.current_dir(cwd).args(args);
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    command.output()?;
    Ok(())
}
