use std::collections::HashMap;

use tokio::sync::broadcast;
use tokio::sync::oneshot;
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

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Warp(#[from] warp::Error),

    #[error(transparent)]
    TrySend(#[from] tokio::sync::mpsc::error::TrySendError<ConnectionInfo>),
}

type WebsocketSender = broadcast::Sender<Message>;
type BindSender = oneshot::Sender<ConnectionInfo>;
type Result<T> = std::result::Result<T, Error>;

mod drop_privileges;
mod launch;
pub mod redirect;
mod router;

pub use launch::*;

/// Encapsulates the communication channels for a virtual host.
#[derive(Debug)]
pub struct HostChannel {
    /// The channel used to send reload messages to connected websockets.
    pub reload: Option<WebsocketSender>,
}

/// Maps the virtual host communication channels by host name.
#[derive(Debug, Default)]
pub struct Channels {
    pub bind: Option<BindSender>,
    pub hosts: HashMap<String, HostChannel>,
}

impl Channels {
    pub fn new(bind: BindSender) -> Self {
        Self {
            bind: Some(bind),
            hosts: HashMap::new(),
        }
    }

    pub fn get_host_reload(&self, name: &str) -> WebsocketSender {
        if let Some(channel) = self.hosts.get(name) {
            if let Some(ref reload) = channel.reload {
                return reload.clone();
            }
        }

        let (ws_tx, _) = broadcast::channel::<Message>(10);
        ws_tx
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
