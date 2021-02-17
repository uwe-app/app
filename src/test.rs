use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tokio::process::Command;

use once_cell::sync::OnceCell;

use log::{debug, error, info};

use structopt::StructOpt;
use tokio::sync::oneshot;

use config::{
    server::ConnectionInfo,
    test::{IntegrationTestConfig, BASE_URL},
    ProfileSettings,
};
use server::ServerChannels;
use workspace::{build, default_compiler, BuildResult, ProjectBuilder};

use crate::{
    opts::{self, uwe::Uwe, Test},
    Error, Result,
};

#[derive(Debug)]
struct TestState {
    project: PathBuf,
    opts: Test,
}

fn get_state(state: Option<RwLock<TestState>>) -> &'static RwLock<TestState> {
    static INSTANCE: OnceCell<RwLock<TestState>> = OnceCell::new();
    INSTANCE.get_or_init(|| state.unwrap())
}

pub async fn run(opts: Test) -> Result<()> {
    let project = opts::project_path(&opts.project)?;
    let profile = ProfileSettings::from(&opts.profile);
    let state = TestState {
        opts,
        project: project.to_path_buf(),
    };

    get_state(Some(RwLock::new(state)));

    build(&project, &profile, test_compiler, Default::default()).await?;
    Ok(())
}

fn parse_rest_opts() -> Option<Vec<String>> {
    let app = Uwe::clap();
    let env_args = std::env::args().collect::<Vec<_>>();
    let matcher = app.get_matches_from(env_args);
    // NOTE: these must match the command name and option name!!!
    if let Some(ref subcommand) = matcher.subcommand_matches("test") {
        let rest = subcommand.values_of("project").unwrap().collect::<Vec<_>>();
        if !rest.is_empty() {
            let pos = rest.iter().position(|&arg| arg == "--");
            if let Some(mut pos) = pos {
                // Skip the -- part
                if pos < rest.len() - 1 {
                    pos += 1;
                }
                let remainder = &rest[pos..];
                let list =
                    remainder.iter().map(|s| s.to_string()).collect::<Vec<_>>();

                return Some(list);
            }
        }
    }
    None
}

fn get_runner_opts<P: AsRef<Path>>(
    build_dir: P,
    settings: &IntegrationTestConfig,
) -> Result<Vec<String>> {
    let opts = if let Some(opts) = parse_rest_opts() {
        opts
    } else {
        let mut opts = Vec::new();
        let opts_file = build_dir.as_ref().join(settings.opts());
        if opts_file.exists() && opts_file.is_file() {
            let file = File::open(&opts_file)?;
            for res in io::BufReader::new(file).lines() {
                let line = res?;
                opts.push(line);
            }
        }
        opts
    };
    Ok(opts)
}

async fn spawn_test_runner<P: AsRef<Path>>(
    url: &str,
    build_dir: P,
    settings: &IntegrationTestConfig,
) -> Result<()> {
    let command = settings.command();
    let mut args = settings.args().clone();
    let mut env = settings.env().clone();
    env.insert(BASE_URL.to_string(), url.to_string());

    let runner_opts = get_runner_opts(build_dir.as_ref(), settings)?;
    for arg in runner_opts.into_iter() {
        args.push(arg);
    }

    info!("Test {} ({})", url, build_dir.as_ref().display());
    info!("{} {}", command, args.join(" "));
    for (k, v) in env.iter() {
        debug!("{} {}", k, v);
    }

    let mut child = Command::new(command)
        .current_dir(build_dir)
        .envs(env)
        .args(&args)
        .spawn()
        .map_err(|_| Error::CommandSpawn(command.to_string()))?;

    let status = child
        .wait()
        .await
        .map_err(|_| Error::CommandExec(command.to_string()))?;

    if status.success() {
        info!("Tests passed ✓");
    } else {
        return Err(Error::IntegrationTestFail(
            command.to_string(),
            args.join(" "),
        ));
    }

    Ok(())
}

async fn test_compiler(builder: ProjectBuilder) -> BuildResult {
    let project = default_compiler(builder).await?;

    let state = get_state(None);
    let writer = state.write().unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let mut server_opts = opts::server_config(
        &writer.opts.project,
        &writer.opts.server,
        config::PORT,
        config::PORT_SSL,
    );

    let build_dir = project.options.build_target();

    if writer.opts.server.port.is_none() {
        server_opts.port = 0;
    }

    if writer.opts.server.ssl_port.is_none() {
        if let Some(ref mut tls) = server_opts.tls {
            tls.port = 0;
        }
    }

    server_opts.redirect_insecure = false;
    server_opts.default_host.directory = build_dir.to_path_buf();

    let spawn_dir = project.config.project().to_path_buf();

    //println!("{:#?}", server_opts);

    let channels = ServerChannels::new_shutdown(shutdown_rx);
    let (bind_tx, bind_rx) = oneshot::channel::<ConnectionInfo>();

    let runner_settings = project.config.test().integration().clone();

    let _ = tokio::task::spawn(async move {
        let info = bind_rx.await?;
        let url = info.to_url();
        info!("Serve {}", &url);

        let test_result =
            spawn_test_runner(&url, &spawn_dir, &runner_settings).await;

        info!("Shutdown {}", &url);
        let _ = shutdown_tx.send(());

        if let Err(e) = test_result {
            error!("{}", e);
            std::process::exit(1);
        }

        Ok::<(), Error>(())
    });

    // Convert to &'static reference
    let server_opts = server::configure(server_opts);
    // Launch the test server
    server::start(server_opts, bind_tx, channels).await?;

    Ok(project)
}
