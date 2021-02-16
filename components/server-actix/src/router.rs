use std::convert::Infallible;
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use serde_json::json;

use futures::future;
use futures_util::sink::SinkExt;
use futures_util::StreamExt;

use tokio::sync::{broadcast, mpsc, oneshot, RwLock};

use serde::Serialize;

use bracket::Registry;
use log::{error, info, trace};

use crate::{
    channels::{ResponseValue, ServerChannels},
    drop_privileges::*,
    Error,
};

use config::server::{ConnectionInfo, HostConfig, PortType, ServerConfig};

pub fn parser() -> &'static Registry<'static> {
    static INSTANCE: OnceCell<Registry> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut registry = Registry::new();
        let _ = registry.insert("error", include_str!("error.html"));
        registry
    })
}

pub async fn serve(
    opts: &'static ServerConfig,
    bind: oneshot::Sender<ConnectionInfo>,
    mut channels: ServerChannels,
) -> crate::Result<()> {
    let addr = opts.get_sock_addr(PortType::Infer)?;
    let default_host: &'static HostConfig = &opts.default_host;
    let should_watch = default_host.watch;

    let mut configs = vec![default_host];
    for host in opts.hosts.iter() {
        configs.push(host);
    }

    println!("Configure actix web server...");

    Ok(())
}
