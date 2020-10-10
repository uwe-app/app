use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::string::ToString;

use crate::Result;

static EXE: &str = "uws";
static CREATE: &str = "create";
static CLONE: &str = "clone";

/// Command bridge so other executables can run these operations 
/// without the overhead of duplicating the executable code.

/// Run a command.
fn run(cmd: &str, args: &[&str], cwd: Option<PathBuf>) -> Result<()> {
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

/// Run the create command.
pub fn create<P: AsRef<Path>>(target: P, message: &str) -> Result<()> {
    let target = target.as_ref().to_string_lossy();
    run(EXE, &[CREATE, &*target, "-m", message], None)
}

/// Run the clone command.
pub fn clone<P: AsRef<Path>>(source: impl Into<String>, target: P, pristine: Option<&str>) -> Result<()> {
    let target = target.as_ref().to_string_lossy();
    run(EXE, &[CLONE, source.into().to_string().as_str(), &*target], None)
}
