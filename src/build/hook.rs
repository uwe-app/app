use std::collections::BTreeMap;
use std::process::{Command, Stdio};

use log::info;

use crate::Error;
use crate::config::HookConfig;
use super::context::Context;

fn run_hook(context: &Context, hook: &HookConfig) -> Result<(), Error> {
    let root = context.config.get_project().unwrap();
    if let Ok(root) = root.canonicalize() {
        let mut cmd = hook.path.as_ref().unwrap().clone();
        let mut args: Vec<String> = vec![];
        if let Some(arguments) = &hook.args {
            args = arguments.to_vec(); 
        }

        // Looks like a relative command, resolve to the project root
        if cmd.starts_with(".") {
            let mut buf = root.clone();
            buf.push(cmd.clone());
            cmd = buf.to_string_lossy().into_owned();
        }

        let build_target = context.options.target.to_string_lossy().into_owned();
        info!("{} {}", cmd, args.join(" "));
        Command::new(cmd)
            .env("BUILD_TARGET", build_target)
            .env("PROJECT_ROOT", root.to_string_lossy().into_owned())
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()?;

    } else {
        return Err(Error::new("Failed to get canonical path for project root".to_string()))
    }

    Ok(())
}

pub fn run(context: &Context, hooks: &BTreeMap<String, HookConfig>) -> Result<(), Error> {
    for (k, hook) in hooks {
        info!("hook {}", k);
        run_hook(context, hook)?;
    }
    //std::process::exit(1);
    Ok(())
}
