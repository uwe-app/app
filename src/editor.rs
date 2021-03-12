use std::path::PathBuf;
use log::{info, warn};

use crate::{opts::Editor, Error, Result};
use config::server::{ConnectionInfo, HostConfig, ServerConfig};

use ui::{ProcessMessage, SocketFile};

use psup_impl::Task;

// NOTE: Must **not** execute on the tokio runtime as the event loop
// NOTE: used for webview rendering must execute on the main thread (macOS)
pub fn run(args: &Editor) -> Result<()> {

    let socket = SocketFile::new()?;
    let socket_path = socket.path().to_path_buf();
    let ctrlc_path = socket.path().to_path_buf();

    ctrlc::set_handler(move || {
        // Clean up the socket file
        let _ = std::fs::remove_file(&ctrlc_path);
        std::process::exit(0);
    })
    .expect("Could not set Ctrl-C handler");

    // Load user projects list
    project::load()?;

    // NOTE: this channel must be `std::sync::mpsc` as the window
    // NOTE: must run on the main thread (MacOS)
    let (tx, rx) = std::sync::mpsc::channel::<ConnectionInfo>();

    // Set up a channel for services to spawn child processes
    let (ps_tx, mut ps_rx) = tokio::sync::mpsc::channel::<ProcessMessage>(64);

    // Channel used to shutdown all child worker processes
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let is_project_editor = args.project.is_some();

    // WARN: We cannot launch the window directly from the server
    // WARN: callback otherwise it's event loop and the tokio runtime
    // WARN: event loop collide and the window will not respond.
    //
    // WARN: To prevent this issue **both** the server and the window
    // WARN: must be spawned in separate threads.
    std::thread::spawn(move || {
        let mut server: ServerConfig = Default::default();
        server.set_allow_ssl_from_env(false);
        server.set_listen(config::LOOPBACK_IP.to_string());
        server.set_port(0);
        server.set_disable_signals(true);
        server.set_workers(2);

        // Run the editor UI on localhost
        let mut editor_host: HostConfig = Default::default();
        editor_host.set_name(config::LOOPBACK_IP.to_string());

        #[cfg(debug_assertions)]
        editor_host.set_directory(PathBuf::from("editor/build/debug"));

        #[cfg(not(debug_assertions))]
        editor_host.set_directory(std::env::current_dir()?);

        #[cfg(not(debug_assertions))]
        editor_host.set_embedded(Some(ui::editor()));

        editor_host.set_require_index(false);
        editor_host.set_disable_cache(true);
        editor_host.set_log(false);

        server.add_host(editor_host);

        //println!("Server {:#?}", &server);

        /*
        editor_host.set_webdav(Some(WebDavConfig::new(
            "/webdav".to_string(),
            info.source.to_path_buf(),
            false,
        )));
        */

        // Get the child process supervisor
        let mut supervisor = ui::supervisor(&socket, shutdown_rx)?;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {

            // Start the process supervisor
            supervisor.run().await?;

            tokio::task::spawn(async move {
                while let Some(msg) = ps_rx.recv().await {
                    println!("Got incoming message {:?}", msg);

                    match msg {
                        ProcessMessage::OpenProject {path, reply} => {
                            #[cfg(not(debug_assertions))]
                            let cmd = "uwe";
                            #[cfg(not(debug_assertions))]
                            let args = &[
                                "dev",
                                "--headless",
                                "--port",
                                "0",
                                "--addr",
                                config::LOOPBACK_IP,
                                &path];

                            #[cfg(debug_assertions)]
                            let cmd = "cargo";
                            #[cfg(debug_assertions)]
                            let args = &[
                                "run",
                                "--",
                                "dev",
                                "--headless",
                                "--port",
                                "0",
                                "--addr",
                                config::LOOPBACK_IP,
                                &path];

                            let task = Task::new(cmd).args(args).daemon(true);
                            let worker_id = supervisor.spawn(task);
                            let _ = reply.send(worker_id);
                        }
                    }
                }
            });

            // Start the editor web server
            server::open(server, move |info| {
                // Notify that the web server is ready
                // so the UI window is displayed
                let _ = tx.send(info);
            })
            .await?;
            Ok::<(), Error>(())
        })?;

        Ok::<(), Error>(())
    });

    // Wait for the web server to start before opening the UI window
    match rx.recv() {
        Ok(info) => {
            let url = if is_project_editor {
                // Must be canonical so the id matches
                let project = args.project.as_ref().unwrap().canonicalize()?;
                let project_id = project::checksum(&project)?;
                format!("{}/?project={}", info.to_url(), project_id)
            } else {
                info.to_url()
            };
            info!("Editor {:#?}", url);
            ui::window(url, ps_tx)?;

            // Clean up the socket file
            let _ = std::fs::remove_file(&socket_path);

            // NOTE: When the window is closed the thread resumes and
            // NOTE: this code executes, we need to ensure that spawned
            // NOTE: worker processes are closed.
            //
            // NOTE: We don't need to do this when SIGINT is received via Ctrl+c
            // NOTE: as that will terminate the child processes.
            let _ = shutdown_tx.send(());
        }
        Err(_e) => {
            warn!("Failed to receive connection info from the web server");
        }
    }

    Ok(())
}
