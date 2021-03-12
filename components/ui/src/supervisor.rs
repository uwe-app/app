use std::path::PathBuf;

use tokio::sync::oneshot::{Sender, Receiver};
use log::info;

use json_rpc2::{futures::{Service, Server}, Request, Response};
use async_trait::async_trait;
use psup_impl::{Supervisor, SupervisorBuilder, id};
use psup_json_rpc::serve;

use project::ConnectionBridge;
use crate::Result;

#[derive(Debug)]
pub struct SocketFile {
    path: PathBuf,
}

impl SocketFile {
    pub fn new() -> Result<Self> {
        let path = dirs::tmp_dir()?.join(format!("uwe-{}.sock", id()));
        Ok(Self {
            path
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug)]
pub enum ProcessMessage {
    OpenProject { path: String, reply: Sender<String> },
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
            response = Some(req.into());
        }
        Ok(response)
    }
}

pub fn supervisor(file: &SocketFile, shutdown: Receiver<()>) -> Result<Supervisor> {

    // Set up the child process supervisor
    Ok(SupervisorBuilder::new()
        .server(move |stream, _tx| {
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
