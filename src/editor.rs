use log::{info, warn};
use std::path::PathBuf;

use crate::{opts::Editor, Error, Result};
use config::server::{ConnectionInfo, HostConfig, ServerConfig};

// NOTE: Must **not** execute on the tokio runtime as the event loop
// NOTE: used for webview rendering must execute on the main thread (macOS)
pub fn run(args: &Editor) -> Result<()> {

    // Load user projects list
    project::load()?;

    let (tx, rx) = std::sync::mpsc::channel::<ConnectionInfo>();

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

        //println!("3) Spawn servers for each active project");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            server::open(server, move |info| {
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
            ui::window(url)?;
        }
        Err(_e) => {
            warn!("Failed to receive connection info from the web server");
        }
    }

    Ok(())
}
