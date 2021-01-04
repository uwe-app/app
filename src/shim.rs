//! Shim for detecting a version from `.uwe-version` and 
//! running a matching executable for the version.
use thiserror::Error;

use std::{
    env,
    process::Command,
    collections::BTreeMap,
    ffi::{OsStr, OsString},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not execute process: {0} {1}")]
    Exec(String, String),

    #[error("Could not set Ctrl-C handler")]
    WindowsCtrlC,

    /// Error for when a pinned compiler is not available in the releases manifest.
    #[error("Version {0}@{1} is not available; try to install it with `uvm install {1}`")]
    VersionNotAvailable(String, String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    Release(#[from] release::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn fork(app_name: &str) -> Result<()> {
    let (mut local_version, version_file) = release::find_local_version(env::current_dir()?)?;

    let pin_version = if let Some(version) = local_version.take() {
        version 
    } else { release::default_version()? };

    let releases = release::mount()?;
    if !releases.contains(&pin_version) {
        return Err(Error::VersionNotAvailable(
            app_name.to_string(),
            pin_version.to_string()))  
    }

    let binary_dir = dirs::releases_dir()?.join(pin_version.to_string());
    if !binary_dir.exists() || !binary_dir.is_dir() {
        return Err(Error::VersionNotAvailable(
            app_name.to_string(),
            pin_version.to_string()))  
    }

    let binary_file = binary_dir.join(app_name);
    if !binary_file.exists() || !binary_file.is_file() {
        return Err(Error::VersionNotAvailable(
            app_name.to_string(),
            pin_version.to_string()))  
    }

    let args: Vec<String> = env::args().skip(1).collect();
    let _ = process(binary_file)
        .args(args.as_slice())
        .exec_replace()?;

    Ok(())
}

// Modified from the process builder in cargo so that replacing the current 
// process should work ok on windows with regards to Ctrl+C handling.
//
// Currently, not tested on windows.

// SEE: https://github.com/rust-lang/cargo/blob/1f6c6bd5e7bbdf596f7e88e6db347af5268ab113/src/cargo/util/process_builder.rs

/// A helper function to create a `ProcessBuilder`.
pub fn process<T: AsRef<OsStr>>(cmd: T) -> ProcessBuilder {
    ProcessBuilder {
        program: cmd.as_ref().to_os_string(),
        args: Vec::new(),
        env: BTreeMap::new(),
    }
}

/// A builder object for an external process, similar to `std::process::Command`.
#[derive(Clone, Debug)]
pub struct ProcessBuilder {
    /// The program to execute.
    program: OsString,
    /// A list of arguments to pass to the program.
    args: Vec<OsString>,
    /// Any environment variables that should be set for the program.
    env: BTreeMap<String, Option<OsString>>,
}

impl ProcessBuilder {
    /// (chainable) Adds multiple `args` to the args list.
    pub fn args<T: AsRef<OsStr>>(&mut self, args: &[T]) -> &mut ProcessBuilder {
        self.args
            .extend(args.iter().map(|t| t.as_ref().to_os_string()));
        self
    }

    /// (chainable) Sets an environment variable for the process.
    pub fn env<T: AsRef<OsStr>>(&mut self, key: &str, val: T) -> &mut ProcessBuilder {
        self.env
            .insert(key.to_string(), Some(val.as_ref().to_os_string()));
        self
    }

    /// Runs the process, waiting for completion, and mapping non-success exit codes to an error.
    #[cfg(windows)]
    pub fn exec(&self) -> Result<()> {
        let mut command = self.build_command();

        let name = self.program.to_string_lossy().to_string();
        let args = self.args.iter().map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<String>>().join(" ");

        let exit = command.status().map_err(|_| {
            Error::Exec(name.clone(), args.clone())
        })?;

        if exit.success() {
            Ok(())
        } else {
            Err(Error::Exec(name, args))
        }
    }

    /// Replaces the current process with the target process.
    ///
    /// On Unix, this executes the process using the Unix syscall `execvp`, which will block
    /// this process, and will only return if there is an error.
    ///
    /// On Windows this isn't technically possible. Instead we emulate it to the best of our
    /// ability. One aspect we fix here is that we specify a handler for the Ctrl-C handler.
    /// In doing so (and by effectively ignoring it) we should emulate proxying Ctrl-C
    /// handling to the application at hand, which will either terminate or handle it itself.
    /// According to Microsoft's documentation at
    /// <https://docs.microsoft.com/en-us/windows/console/ctrl-c-and-ctrl-break-signals>.
    /// the Ctrl-C signal is sent to all processes attached to a terminal, which should
    /// include our child process. If the child terminates then we'll reap them in Cargo
    /// pretty quickly, and if the child handles the signal then we won't terminate
    /// (and we shouldn't!) until the process itself later exits.
    pub fn exec_replace(&self) -> Result<()> {
        imp::exec_replace(self)
    }

    /// Converts `ProcessBuilder` into a `std::process::Command`, and handles the jobserver, if
    /// present.
    pub fn build_command(&self) -> Command {
        let mut command = Command::new(&self.program);
        for arg in &self.args {
            command.arg(arg);
        }
        for (k, v) in &self.env {
            match *v {
                Some(ref v) => {
                    command.env(k, v);
                }
                None => {
                    command.env_remove(k);
                }
            }
        }
        command
    }
}

#[cfg(unix)]
mod imp {
    use super::{Error, Result, ProcessBuilder};
    use std::os::unix::process::CommandExt;

    pub fn exec_replace(builder: &ProcessBuilder) -> Result<()> {
        let mut command = builder.build_command();
        let _error = command.exec();
        let name = builder.program.to_string_lossy().to_string();
        let args = builder.args.iter().map(|a| a.to_string_lossy().to_string())
            .collect::<Vec<String>>().join(" ");
        Err(Error::Exec(name, args))
    }
}

#[cfg(windows)]
mod imp {
    use super::{Error, Result, ProcessBuilder};
    use winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
    use winapi::um::consoleapi::SetConsoleCtrlHandler;

    unsafe extern "system" fn ctrlc_handler(_: DWORD) -> BOOL {
        // Do nothing; let the child process handle it.
        TRUE
    }

    pub fn exec_replace(builder: &ProcessBuilder) -> Result<()> {
        unsafe {
            if SetConsoleCtrlHandler(Some(ctrlc_handler), TRUE) == FALSE {
                return Err(Error::WindowsCtrlC);
            }
        }

        // Just execute the process as normal.
        builder.exec()
    }
}
