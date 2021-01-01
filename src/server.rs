use crate::{Error, Result};
use config::server::{LaunchConfig, ServerConfig};
use std::path::PathBuf;

use config::ProfileSettings;
use workspace::compile;

use crate::opts::Compile;

/// Serve using an `index.html` file.
async fn serve_index(opts: ServerConfig, launch: LaunchConfig) -> Result<()> {
    // Convert to &'static reference
    let opts = server::configure(opts);
    Ok(server::launch(opts, launch).await?)
}

/// Serve either a project or a target directory.
pub async fn serve(
    target: &PathBuf,
    skip_build: bool,
    mut opts: ServerConfig,
    launch: LaunchConfig,
    args: Compile,
) -> Result<()> {
    let site_file = target.join(config::SITE_TOML);
    let index_file = target.join(config::INDEX_HTML);

    if site_file.exists() && site_file.is_file() {
        if skip_build {
            let workspace = workspace::open(&target, false, &args.member)?;
            let mut it = workspace.iter();
            if let Some(config) = it.next() {
                // Respect target build directory
                let build = config.build.as_ref().unwrap();
                let build_target = build.target.join(config::RELEASE);
                if !build_target.exists() || !build_target.is_dir() {
                    return Err(Error::NotDirectory(build_target));
                }
                opts.default_host.directory = build_target;
            }
        } else {
            let mut settings = ProfileSettings::new_release();
            settings.exec = Some(args.exec);
            settings.member = args.member;
            settings.include_drafts = Some(args.include_drafts);

            let result = compile(&target, &settings).await?;

            let mut it = result.projects.into_iter();

            // First project is the default host
            if let Some(project) = it.next() {
                let build_target = project.options.base.to_path_buf();
                opts.default_host.directory = build_target;
            }
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
