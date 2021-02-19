extern crate log;
extern crate pretty_env_logger;

use std::time::SystemTime;

use log::info;
use semver::Version;
use structopt::StructOpt;

use config::{server::HostConfig, ProfileSettings};

use publisher::PublishProvider;

use uwe::{
    self, fatal,
    opts::{
        self,
        uwe::{Command, Uwe},
    },
    Result,
};

async fn run(cmd: Command) -> Result<()> {
    match cmd {
        Command::New { args } => {
            let opts = uwe::new::ProjectOptions {
                source: args.plugin,
                git: args.git,
                prefix: args.prefix,
                path: args.path,
                message: args.message,
                target: args.target,
                language: args.language,
                host: args.host,
                locales: args.locales,
                bare: args.bare,
                remote_name: args.remote_name,
                remote_url: args.remote_url,
            };
            uwe::new::project(opts).await?;
        }

        Command::Lang { cmd } => {
            uwe::lang::run(cmd).await?;
        }

        Command::Test { args } => {
            uwe::test::run(args).await?;
        }

        Command::Sync { args } => {
            uwe::sync::run(args).await?;
        }

        Command::Clean { args } => {
            let project = opts::project_path(&args.project)?;
            uwe::clean::clean(project).await?;
        }

        Command::Docs { args } => {
            let version = config::generator::semver();
            let plugin = plugin::install_docs(Some(version)).await?;

            let target = plugin.base().join(config::PUBLIC_HTML);
            let mut opts = uwe::opts::server_config(
                //&target,
                &args.server,
                config::PORT_DOCS,
                config::PORT_DOCS_SSL,
            );

            let host = HostConfig::new_directory(target);
            opts.add_host(host);

            uwe::docs::open(opts).await?;
        }

        Command::Server { args } => {
            uwe::server::serve(
                (args.project, args.directory, args.config),
                args.server,
                args.open,
                args.build_opts,
            )
            .await?;
        }

        Command::Task { cmd } => {
            uwe::task::run(cmd).await?;
        }

        Command::Publish { args } => {
            let project = opts::project_path(&args.project)?;
            let opts = uwe::publish::PublishOptions {
                provider: PublishProvider::Aws,
                env: args.env,
                project,
                exec: args.exec,
                sync_redirects: args.sync_redirects,
            };
            uwe::publish::publish(opts).await?;
        }

        Command::Build { args } => {
            let project = opts::project_path(&args.project)?;

            let paths = if args.paths.len() > 0 {
                Some(args.paths)
            } else {
                None
            };

            let build_args = ProfileSettings {
                paths,
                release: Some(true),
                name: args.profile,
                exec: Some(args.compile.exec),
                member: args.compile.member,
                include_drafts: Some(args.compile.include_drafts),
                ..Default::default()
            };

            let now = SystemTime::now();
            match uwe::build::compile(&project, build_args).await {
                Ok(_) => {
                    if let Ok(t) = now.elapsed() {
                        info!("{:?}", t);
                    }
                }
                Err(e) => uwe::print_error(e),
            }
        }

        Command::Dev { args } => {
            let project = opts::project_path(&args.project)?;

            let paths = if args.paths.len() > 0 {
                Some(args.paths)
            } else {
                None
            };

            let tls =
                uwe::opts::tls_config(None, &args.server, config::PORT_SSL);

            let build_args = ProfileSettings {
                paths,
                name: args.profile,
                launch: args.launch,
                host: Some(args.server.addr),
                port: args.server.port,
                exec: Some(args.compile.exec),
                member: args.compile.member,
                include_drafts: Some(args.compile.include_drafts),
                tls,
                ..Default::default()
            };

            if let Err(e) =
                uwe::dev::run(&project, build_args, args.server.authority).await
            {
                uwe::print_error(e);
            }
        }

        Command::Editor { args } => {
            let project = opts::project_path(&args.project)?;
            let tls =
                uwe::opts::tls_config(None, &args.server, config::PORT_SSL);

            let build_args = ProfileSettings {
                paths: None,
                name: args.profile,
                launch: None,
                host: Some(args.server.addr),
                port: args.server.port,
                exec: Some(args.compile.exec),
                member: args.compile.member,
                include_drafts: Some(args.compile.include_drafts),
                tls,
                ..Default::default()
            };

            if let Err(e) =
                uwe::editor::run(&project, build_args, args.server.authority)
                    .await
            {
                uwe::print_error(e);
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Uwe::from_args();

    uwe::panic_hook();
    uwe::log_level(&*args.log_level).or_else(fatal)?;

    // Configure the generator meta data ahead of time

    // Must configure the version here otherwise option_env!() will
    // use the version from the workspace package which we don't really
    // care about, the top-level version is the one that interests us.
    let name = env!("CARGO_PKG_NAME").to_string();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let bin_name = env!("CARGO_BIN_NAME").to_string();
    let user_agent = format!("{}/{}", &name, &version);
    let semver: Version = version.parse().unwrap();

    info!("{}", &version);

    let app_data = config::generator::AppData {
        name,
        bin_name,
        version,
        user_agent,
        semver,
    };
    config::generator::get(Some(app_data));

    Ok(run(args.cmd).await.or_else(fatal)?)
}
