use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use log::{debug, info};

use compiler::BuildContext;
use config::{HookConfig, ProfileName};

use crate::{Error, Result};

pub enum Phase {
    Before,
    After,
}

pub fn exec(
    ctx: &Arc<BuildContext>,
    hook: &HookConfig,
    changed: Option<&PathBuf>,
) -> Result<()> {
    let collation = ctx.collation.read().unwrap();

    let project_root = ctx.config.project().canonicalize().map_err(|_| {
        Error::CanonicalProjectRoot(ctx.config.project().to_path_buf())
    })?;

    let mut cmd = hook.path.clone();
    let mut args: Vec<String> = vec![];
    if let Some(arguments) = &hook.args {
        args = arguments.to_vec();
    }

    // Looks like a relative command, resolve to the project root
    if cmd.starts_with(".") {
        cmd = project_root.join(&cmd).to_string_lossy().into_owned();
    }

    let build_source = ctx.options.source.canonicalize()?;
    let build_target = collation.get_path().canonicalize()?;

    let node_env = ctx.options.settings.get_node_env(ctx.config.node());

    info!("{} {}", cmd, args.join(" "));
    debug!("BUILD_PROJECT {}", project_root.display());
    debug!("BUILD_SOURCE {}", build_source.display());
    debug!("BUILD_TARGET {}", build_target.display());
    debug!("NODE_ENV {}", &node_env);

    let mut command = Command::new(cmd);

    command
        .current_dir(&project_root)
        .env("NODE_ENV", node_env)
        .env("BUILD_PROJECT", project_root.to_string_lossy().into_owned())
        .env("BUILD_SOURCE", build_source.to_string_lossy().into_owned())
        .env("BUILD_TARGET", build_target.to_string_lossy().into_owned())
        .args(args);

    if let Some(file) = changed {
        command.env("BUILD_FILE", file.to_string_lossy().into_owned());
    }

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
            let after = v.after.is_some() && v.after.unwrap();
            let result = match phase {
                Phase::Before => !after,
                Phase::After => after,
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

pub fn run(
    ctx: &Arc<BuildContext>,
    hooks: Vec<&HookConfig>,
    changed: Option<&PathBuf>,
) -> Result<()> {
    for hook in hooks {
        exec(ctx, hook, changed)?;
    }
    Ok(())
}
