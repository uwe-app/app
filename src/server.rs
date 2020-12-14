use crate::{Error, Result};
use config::server::{LaunchConfig, ServerConfig};
use std::path::PathBuf;

use scopeguard::defer;

use config::{lock_file::LockFile, ProfileSettings};
use workspace::{compile, lock};

/// Serve using an `index.html` file.
async fn serve_index(opts: ServerConfig, launch: LaunchConfig) -> Result<()> {
    // Convert to &'static reference
    let opts = server::configure(opts);
    let mut channels = Default::default();
    Ok(server::launch(opts, launch, &mut channels).await?)
}

/// Serve either a project or a target directory.
pub async fn serve(
    target: &PathBuf,
    skip_build: bool,
    mut opts: ServerConfig,
    launch: LaunchConfig,
) -> Result<()> {
    let site_file = target.join(config::SITE_TOML);
    let index_file = target.join(config::INDEX_HTML);

    if site_file.exists() && site_file.is_file() {
        if skip_build {
            let mut workspace = workspace::open(&target, false)?;
            let mut it = workspace.iter();
            if let Some(entry) = it.next() {
                // Respect target build directory
                let build = entry.config.build.as_ref().unwrap();
                let build_target = build.target.join(config::RELEASE);
                if !build_target.exists() || !build_target.is_dir() {
                    return Err(Error::NotDirectory(build_target));
                }
                opts.default_host.directory = build_target;
            }
        } else {
            let lock_path = LockFile::get_lock_file(&target);
            let lock_file = lock::acquire(&lock_path)?;
            defer! { let _ = lock::release(lock_file); }

            let args = ProfileSettings::new_release();
            let result = compile(&target, &args).await?;

            let mut it = result.projects.into_iter();

            // First project is the default host
            if let Some(project) = it.next() {
                let build_target = project.options.base.to_path_buf();
                opts.default_host.directory = build_target;
            }

            /*
            // FIXME: support multiple projects (workspaces)
            // TODO: add other host configs
            for project in it {
                //println!("Project options {:?}", project.options);
            }
            */
        }

        Ok(serve_index(opts, launch).await?)
    } else if index_file.exists() && index_file.is_file() {
        Ok(serve_index(opts, launch).await?)
    } else {
        Err(Error::NoServerFile(
            config::SITE_TOML.to_string(),
            config::INDEX_HTML.to_string(),
            target.to_path_buf(),
        ))
    }
}
