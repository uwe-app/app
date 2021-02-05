use std::path::PathBuf;
use std::sync::RwLock;
use once_cell::sync::OnceCell;

use log::info;

use tokio::sync::oneshot;

use config::{
    ProfileSettings,
    server::ConnectionInfo,
};
use server::ServerChannels;
use workspace::{default_compiler, build, ProjectBuilder, BuildResult};

use crate::{
    opts::{self, Test},
    Result,
};

#[derive(Debug)]
struct TestState {
    project: PathBuf,
    opts: Test,
}

fn get_state(state: Option<RwLock<TestState>>) -> &'static RwLock<TestState> {
    static INSTANCE: OnceCell<RwLock<TestState>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        state.unwrap()
    })
}

pub async fn run(opts: Test) -> Result<()> {
    let project = opts::project_path(&opts.project)?;
    let profile = ProfileSettings::from(&opts.profile);
    let state = TestState { opts, project: project.to_path_buf() };

    get_state(Some(RwLock::new(state)));

    build(&project, &profile, test_compiler).await?;
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

    server_opts.port = 0;
    if let Some(ref mut tls) = server_opts.tls {
        tls.port = 0; 
    }

    server_opts.redirect_insecure = false;
    server_opts.default_host.directory = build_dir.to_path_buf();

    println!("{:#?}", server_opts);

    let channels = ServerChannels::new_shutdown(shutdown_rx);

    let (bind_tx, bind_rx) = oneshot::channel::<ConnectionInfo>();

    let _ = tokio::task::spawn(async move {
        let info = bind_rx.await.unwrap();
        let url = info.to_url();
        info!("Serve {}", &url);

        // TODO: run cypress
        
        info!("Shutdown {}", &url);
        shutdown_tx.send(()).unwrap();
    });

    // Convert to &'static reference
    let server_opts = server::configure(server_opts);
    // Launch the test server
    server::start(server_opts, bind_tx, channels).await?;

    Ok(project)
}
