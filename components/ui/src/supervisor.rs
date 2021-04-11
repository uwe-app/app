use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use log::{info, warn};
use tokio::sync::{mpsc, oneshot, Mutex};

use async_trait::async_trait;
use json_rpc2::{
    futures::{Server, Service},
    Request, Response,
};
use once_cell::sync::OnceCell;
use psup_impl::{id, Message, Supervisor, SupervisorBuilder};
use psup_json_rpc::serve;

use crate::Result;
use config::server::ConnectionInfo;
use project::ConnectionBridge;

/// Store the web server connection info for each supervised process.
pub(crate) fn project_servers(
) -> &'static RwLock<HashMap<String, ConnectionInfo>> {
    static INSTANCE: OnceCell<RwLock<HashMap<String, ConnectionInfo>>> =
        OnceCell::new();
    INSTANCE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[derive(Debug)]
pub struct SocketFile {
    path: PathBuf,
}

impl SocketFile {
    pub fn new() -> Result<Self> {
        let path = dirs::tmp_dir()?.join(format!("uwe-{}.sock", id()));
        Ok(Self { path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug)]
pub enum ProcessMessage {
    OpenProject {
        path: String,
        reply: oneshot::Sender<String>,
    },
    CloseProject {
        worker_id: String,
    },
}

struct SupervisorService;

#[async_trait]
impl Service for SupervisorService {
    type Data = ();
    async fn handle(
        &self,
        req: &mut Request,
        _ctx: &Self::Data,
    ) -> json_rpc2::Result<Option<Response>> {
        let mut response = None;
        if req.matches("connected") {
            let info: ConnectionBridge = req.deserialize()?;
            println!("Got connected message in supervisor {:?}", info);
            let mut servers = project_servers().write().unwrap();
            servers.insert(info.id, info.connection);
            response = Some(req.into());
        }
        Ok(response)
    }
}

pub fn supervisor(
    file: &SocketFile,
    shutdown: oneshot::Receiver<()>,
    proxy: mpsc::Receiver<ProcessMessage>,
) -> Result<Supervisor> {
    let proxy = Arc::new(Mutex::new(proxy));

    // Set up the child process supervisor
    Ok(SupervisorBuilder::new()
        .server(move |stream, tx| {

            // Listen for messages on the proxy channel which are brokered
            // by the editor but originate from the UI RPC services
            let rx = Arc::clone(&proxy);
            tokio::task::spawn(async move {
                let mut rx = rx.lock().await;
                while let Some(msg) = rx.recv().await {
                    match msg {
                        ProcessMessage::CloseProject { worker_id } => {
                            let control_msg = Message::Shutdown {id: worker_id};
                            if let Err(e) = tx.send(control_msg).await {
                                warn!("Failed to send to supervisor control channel: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
            });

            tokio::task::spawn(async move {
                let (reader, writer) = tokio::io::split(stream);
                tokio::task::spawn(async move {
                    let service: Box<dyn Service<Data = ()>> =
                        Box::new(SupervisorService {});
                    let server = Server::new(vec![&service]);
                    serve::<(), _, _, _, _, _>(
                        server,
                        &(),
                        reader,
                        writer,
                        |req| info!("{:?}", req),
                        |res| info!("{:?}", res),
                        |reply| {
                            info!("{:?}", reply);
                            Ok(None)
                        },
                    )
                    .await?;
                    Ok::<(), psup_impl::Error>(())
                });
            });
        })
        .path(file.path())
        .shutdown(shutdown)
        .build())
}
