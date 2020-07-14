use std::collections::HashMap;
use std::process::{Command, Stdio};

use log::{debug, info};

use config::{HookConfig, ProfileName};

use super::context::Context;
use crate::Error;

pub enum Phase {
    Before,
    After,
}

pub fn exec(hook: &HookConfig) -> Result<(), Error> {

    let runtime = runtime::runtime().read().unwrap();

    let project_root = runtime.config.get_project();
    debug!("hook root {}", project_root.display());
    if let Ok(root) = project_root.canonicalize() {
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

        let mut build_target = runtime.options.target.clone().canonicalize()?;
        build_target = build_target.strip_prefix(&root)?.to_path_buf();

        let node = runtime.config.node.as_ref().unwrap();
        let node_env = runtime.options.settings.name
            .get_node_env(node.debug.clone(), node.release.clone());

        info!("{} {}", cmd, args.join(" "));
        debug!("PROJECT_ROOT {}", root.display());
        debug!("BUILD_TARGET {}", build_target.display());
        debug!("NODE_ENV {}", &node_env);

        let mut command = Command::new(cmd);

        command
            .current_dir(&root)
            .env("NODE_ENV", node_env)
            .env("BUILD_TARGET", build_target.to_string_lossy().into_owned())
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
        return Err(Error::CanonicalProjectRoot(project_root));
    }

    Ok(())
}

pub fn collect(hooks: HashMap<String, HookConfig>, phase: Phase, name: &ProfileName) -> Vec<(String, HookConfig)> {
    hooks
        .into_iter()
        .filter(|(_, v)| {
            let result = match phase {
                Phase::Before => v.after.is_none(),
                Phase::After => v.after.is_some() && v.after.unwrap(),
            };
            result
        })
        .filter(|(_, v)| {
            if let Some(ref profiles) = v.profiles {
                profiles.contains(name) 
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
}

pub fn run(hooks: Vec<(String, HookConfig)>) -> Result<(), Error> {
    for (k, hook) in hooks {
        info!("hook {}", k);
        exec(&hook)?;
    }
    Ok(())
}
