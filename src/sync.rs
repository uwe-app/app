use log::info;

use crate::{
    opts::{self, Sync},
    Error, Result,
};

pub async fn run(opts: Sync) -> Result<()> {
    let project = opts::project_path(&opts.project)?;
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    let remote_opt = opts.remote;
    let branch_opt = opts.branch;

    let (config, _) = workspace::settings(&project, true, &vec![])?;

    let remote = if let Some(ref remote) = remote_opt {
        remote
    } else {
        config.sync().remote.as_ref().unwrap()
    };

    let branch = if let Some(ref branch) = branch_opt {
        branch
    } else {
        config.sync().branch.as_ref().unwrap()
    };

    info!(
        "Sync {} (remote: {}, branch: {})",
        config.project().display(),
        remote,
        branch
    );

    scm::sync(
        &project,
        remote.to_string(),
        branch.to_string(),
        opts.add,
        opts.message,
    )
    .map_err(Error::from)
}
