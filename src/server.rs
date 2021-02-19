use std::convert::TryInto;
use std::path::PathBuf;

use config::{
    server::{HostConfig, LaunchConfig, ServerConfig},
    ProfileName, ProfileSettings,
};
use workspace::{compile, HostInfo, HostResult};

use crate::{
    opts::{project_path, server_config, web_server::WebServerOpts, Compile},
    Error, Result,
};

async fn serve_project(
    project: PathBuf,
    mut opts: ServerConfig,
    launch: LaunchConfig,
    args: Compile,
    skip_build: bool,
) -> Result<()> {
    if skip_build {
        let workspace = workspace::open(&project, false, &args.member)?;
        let mut it = workspace.iter();
        if let Some(config) = it.next() {
            // Respect target build directory
            let build = config.build.as_ref().unwrap();
            let build_target = project.join(build.target.join(config::RELEASE));
            if !build_target.exists() || !build_target.is_dir() {
                return Err(Error::NotDirectory(build_target));
            }

            let mut host: HostConfig = Default::default();
            host.directory = build_target;
            opts.add_host(host);
        }
    } else {
        let mut settings = ProfileSettings::from(&ProfileName::Release);
        settings.exec = Some(args.exec);
        settings.member = args.member;
        settings.include_drafts = Some(args.include_drafts);

        let result = compile(&project, &settings, Default::default()).await?;

        let host_result: HostResult = result.into();
        let mut host_configs: Vec<(HostInfo, HostConfig)> =
            host_result.try_into()?;

        for (info, host) in host_configs.iter_mut() {
            host.directory = info.project.options.build_target().to_path_buf();
        }

        opts.hosts = host_configs.into_iter().map(|(_, host)| host).collect();
    }

    Ok(server::launch(opts, launch).await?)
}

/// Serve either a project, directory or load from a config.
pub async fn serve(
    targets: (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>),
    skip_build: bool,
    server: WebServerOpts,
    open: bool,
    args: Compile,
) -> Result<()> {
    let mut given = Vec::new();
    if targets.0.is_some() {
        given.push(true)
    }
    if targets.1.is_some() {
        given.push(true)
    }
    if targets.2.is_some() {
        given.push(true)
    }
    if given.len() > 1 {
        return Err(Error::TooManyServerTargets);
    }

    let launch = LaunchConfig { open };

    // Handle project
    if let Some(project) = targets.0 {
        // Must call project_path() so we respect `.uwe-version` !!!
        let project = project_path(&project)?;
        let opts = server_config(&server, config::PORT, config::PORT_SSL);

        serve_project(project, opts, launch, args, skip_build).await?;
    // Handle directory
    } else if let Some(directory) = targets.1 {
        let mut opts = server_config(&server, config::PORT, config::PORT_SSL);

        let host = HostConfig::new_directory(directory);
        opts.add_host(host);

        server::launch(opts, launch).await?;
    // Handle configuration file
    } else if let Some(config) = targets.2 {
        if !config.exists() || !config.is_file() {
            return Err(Error::NotFile(config));
        }

        let opts = ServerConfig::load(config)?;

        println!("Config opts {:#?}", opts);

        server::launch(opts, launch).await?;
    } else {
        return Err(Error::NoServerTargets);
    }

    Ok(())
}
