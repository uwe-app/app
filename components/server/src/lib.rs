use std::collections::HashMap;

use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedSender},
    oneshot,
};
use warp::ws::Message;

use config::server::{ConnectionInfo, ServerConfig};
use once_cell::sync::OnceCell;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to get user name when dropping privileges (getlogin)")]
    DropPrivilegeGetLogin,

    #[error("Failed to get user info when dropping privileges (getpwnam)")]
    DropPrivilegeGetInfo,

    #[error("Failed to drop privileges")]
    DropPrivilegeFail,

    #[error("Failed to set group when dropping privileges (setgid)")]
    DropPrivilegeGroup,

    #[error("Failed to set user when dropping privileges (setuid)")]
    DropPrivilegeUser,

    #[error("Failed to get user id when dropping privileges (getuid)")]
    DropPrivilegeGetUserId,

    #[error("No virtual hosts for live reload")]
    NoLiveHosts,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Warp(#[from] warp::Error),

    #[error(transparent)]
    Notify(#[from] notify::Error),

    #[error(transparent)]
    TrySend(#[from] tokio::sync::mpsc::error::TrySendError<ConnectionInfo>),

    #[error(transparent)]
    SendError(#[from] tokio::sync::mpsc::error::SendError<String>),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Workspace(#[from] workspace::Error),
}

pub type ErrorCallback = fn(Error);

type WebsocketSender = broadcast::Sender<Message>;
type BindSender = oneshot::Sender<ConnectionInfo>;
type RenderRequest = mpsc::UnboundedSender<String>;
type ResponseValue = Option<Box<dyn std::error::Error + Send>>;
type Result<T> = std::result::Result<T, Error>;

mod drop_privileges;
mod launch;
pub mod redirect;
mod router;
mod watch;

pub use launch::*;
pub use watch::watch;

/// Encapsulates the communication channels for a virtual host.
#[derive(Debug)]
pub struct HostChannel {
    /// The channel used to send reload messages to connected websockets.
    pub(crate) reload: WebsocketSender,

    /// The channel used to request a render for a page URL.
    pub(crate) render_request: RenderRequest,
    // The channel used to send a render response back to the web server.
    //pub(crate) render_response: (UnboundedSender<ResponseValue>, UnboundedReceiver<ResponseValue>),
}

impl HostChannel {
    pub fn new(reload: WebsocketSender, render_request: RenderRequest) -> Self {
        Self {
            reload,
            render_request,
            //render_response: mpsc::unbounded_channel::<ResponseValue>(),
        }
    }

    /*
    pub fn get_render_response_tx(&self) -> &UnboundedSender<ResponseValue> {
        &self.render_response.0
    }

    pub fn get_render_response_rx(&mut self) -> &mut UnboundedReceiver<ResponseValue> {
        &mut self.render_response.1
    }
    */
}

/// Maps the virtual host communication channels by host name.
#[derive(Debug, Default)]
pub struct Channels {
    pub bind: Option<BindSender>,
    pub hosts: HashMap<String, HostChannel>,
    pub render_responses: HashMap<String, UnboundedSender<ResponseValue>>,
}

impl Channels {
    pub fn new(bind: BindSender) -> Self {
        Self {
            bind: Some(bind),
            hosts: HashMap::new(),
            render_responses: HashMap::new(),
        }
    }

    pub fn get_host_reload(&self, name: &str) -> WebsocketSender {
        if let Some(channel) = self.hosts.get(name) {
            return channel.reload.clone();
        }

        let (ws_tx, _) = broadcast::channel::<Message>(10);
        ws_tx
    }

    pub fn get_host_render_request(&self, name: &str) -> RenderRequest {
        if let Some(channel) = self.hosts.get(name) {
            return channel.render_request.clone();
        }

        let (render_tx, _) = mpsc::unbounded_channel::<String>();
        render_tx
    }
}

/// When the web server routes are configured various strings need
/// to have the `static` lifetime. This function converts a server
/// configuration to a `&'static` reference so strings in the server
/// configuration can be used when constructing the filters.
pub fn configure(config: ServerConfig) -> &'static ServerConfig {
    static INSTANCE: OnceCell<ServerConfig> = OnceCell::new();
    INSTANCE.get_or_init(|| config)
}
