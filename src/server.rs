use std::convert::TryInto;
use std::path::PathBuf;

use crate::{Error, Result};
use config::server::{LaunchConfig, ServerConfig};

use config::{server::HostConfig, ProfileSettings, ProfileName};
use workspace::{compile, HostInfo, HostResult};

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

                // TODO: try to load redirects from `redirects.json`
            }
        } else {
            let mut settings = ProfileSettings::from(&ProfileName::Release);
            settings.exec = Some(args.exec);
            settings.member = args.member;
            settings.include_drafts = Some(args.include_drafts);

            let result = compile(&target, &settings).await?;

            let host_result: HostResult = result.into();
            let mut host_configs: Vec<(HostInfo, HostConfig)> =
                host_result.try_into()?;

            for (info, host) in host_configs.iter_mut() {
                host.directory =
                    info.project.options.build_target().to_path_buf();
            }

            let mut it = host_configs.into_iter();
            let (_, default_host) = it.next().unwrap();
            opts.default_host = default_host;

            let hosts: Vec<HostConfig> = it.map(|(_, host)| host).collect();

            opts.hosts = hosts;
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
