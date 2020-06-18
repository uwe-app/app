use std::collections::HashMap;
use std::process::{Command, Stdio};

use log::{info, debug};

use crate::Error;
use crate::config::HookConfig;
use super::context::Context;

pub enum Phase {
    Before,
    After,
}

pub fn exec(context: &Context, hook: &HookConfig) -> Result<(), Error> {
    let root = context.config.get_project();
    debug!("hook root {}", root.display());
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
        let mut command = Command::new(cmd);

        let node_env = context.options.tag.get_node_env(
            context.config.node.as_ref().unwrap().debug.clone(),
            context.config.node.as_ref().unwrap().release.clone());

        command
            .env("NODE_ENV", node_env)
            .env("BUILD_TARGET", build_target)
            .env("PROJECT_ROOT", root.to_string_lossy().into_owned())
            .args(args);

        if hook.stdout.is_some() && hook.stdout.unwrap() {
            command.stdout(Stdio::inherit());
        }

        if hook.stderr.is_some() && hook.stderr.unwrap() {
            command.stderr(Stdio::inherit());
        }

        command.output()?;

    } else {
        return Err(
            Error::new(
                format!("Failed to get canonical path for project root '{}'", root.display())))
    }

    Ok(())
}

pub fn collect(hooks: HashMap<String, HookConfig>, phase: Phase) -> Vec<(String, HookConfig)> {
    hooks
        .into_iter()
        .filter(|(_, v)| {
            let result = match phase {
                Phase::Before => {
                    v.after.is_none()
                },
                Phase::After => {
                    v.after.is_some() && v.after.unwrap()
                }
            };
            result
        })
        .collect::<Vec<_>>()
}

pub fn run(context: &Context, hooks: Vec<(String, HookConfig)>) -> Result<(), Error> {
    for (k, hook) in hooks {
        info!("hook {}", k);
        exec(context, &hook)?;
    }
    Ok(())
}
