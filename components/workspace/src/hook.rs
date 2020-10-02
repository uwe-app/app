use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::Arc;

use log::{debug, info};

use collator::Collate;
use compiler::BuildContext;
use config::{HookConfig, ProfileName};

use crate::{Error, Result};

pub enum Phase {
    Before,
    After,
}

pub fn exec(ctx: &Arc<BuildContext>, hook: &HookConfig) -> Result<()> {
    let collation = ctx.collation.read().unwrap();

    let project_root =
        ctx.config.project().canonicalize().map_err(|_| {
            Error::CanonicalProjectRoot(ctx.config.project().to_path_buf())
        })?;

    let hook_root = hook
        .base()
        .canonicalize()
        .map_err(|_| Error::CanonicalProjectRoot(hook.base().to_path_buf()))?;

    debug!("Hook root {}", hook_root.display());

    let mut cmd = hook.path.clone();
    let mut args: Vec<String> = vec![];
    if let Some(arguments) = &hook.args {
        args = arguments.to_vec();
    }

    //let cmd_path = PathBuf::from(cmd);

    // Looks like a relative command, resolve to the project root
    if cmd.starts_with(".") {
        let mut buf = hook_root.clone();
        buf.push(cmd.clone());
        cmd = buf.to_string_lossy().into_owned();
    }

    let build_source = ctx.options.source.canonicalize()?;
    let build_target = collation.get_path().canonicalize()?;

    let node = ctx.config.node.as_ref().unwrap();
    let node_env = ctx
        .options
        .settings
        .name
        .get_node_env(node.debug.clone(), node.release.clone());

    info!("{} {}", cmd, args.join(" "));
    debug!("BUILD_PROJECT {}", project_root.display());
    debug!("BUILD_SOURCE {}", build_source.display());
    debug!("BUILD_TARGET {}", build_target.display());
    debug!("BUILD_HOOK {}", hook_root.display());
    debug!("NODE_ENV {}", &node_env);

    let mut command = Command::new(cmd);

    command
        .current_dir(&hook_root)
        .env("NODE_ENV", node_env)
        .env("BUILD_PROJECT", project_root.to_string_lossy().into_owned())
        .env("BUILD_SOURCE", build_source.to_string_lossy().into_owned())
        .env("BUILD_TARGET", build_target.to_string_lossy().into_owned())
        .env("BUILD_HOOK", hook_root.to_string_lossy().into_owned())
        .args(args);

    if hook.stdout.is_some() && hook.stdout.unwrap() {
        command.stdout(Stdio::inherit());
    }

    if hook.stderr.is_some() && hook.stderr.unwrap() {
        command.stderr(Stdio::inherit());
    }

    command.output()?;

    Ok(())
}

pub fn collect<'a>(
    hooks: &'a HashSet<HookConfig>,
    phase: Phase,
    name: &ProfileName,
) -> Vec<&'a HookConfig> {
    hooks
        .into_iter()
        .filter(|v| {
            let result = match phase {
                Phase::Before => v.after.is_none(),
                Phase::After => v.after.is_some() && v.after.unwrap(),
            };
            result
        })
        .filter(|v| {
            if let Some(ref profiles) = v.profiles {
                profiles.contains(name)
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
}

pub fn run(ctx: &Arc<BuildContext>, hooks: Vec<&HookConfig>) -> Result<()> {
    for hook in hooks {
        exec(ctx, hook)?;
    }
    Ok(())
}
