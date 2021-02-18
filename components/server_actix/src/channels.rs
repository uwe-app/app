//! Types that encapsulate the channels for message passing.

use std::collections::HashMap;

use tokio::sync::{broadcast, mpsc, oneshot};
use warp::ws::Message;

pub type ResponseValue = Option<Box<dyn std::error::Error + Send + Sync>>;

pub(crate) const RENDER_CHANNEL_BUFFER: usize = 128;

#[derive(Debug, Clone)]
pub struct ServerChannels {
    pub(crate) render:
        HashMap<String, mpsc::Sender<(String, oneshot::Sender<ResponseValue>)>>,
    pub(crate) websockets: HashMap<String, broadcast::Sender<Message>>,
    //shutdown_tx: Option<oneshot::Sender<bool>>,
}

impl ServerChannels {
    /// Create a collection of channels where the caller
    /// owns the shutdown sender and intends to use it
    /// to shutdown the server at some point in the future.
    pub fn new() -> Self {
        Self {
            render: HashMap::new(),
            websockets: HashMap::new(),
            //shutdown_tx: None,
        }
    }

    /// Create a channels collection that owns the shutdown
    /// sender so the server is kept alive.
    ///
    /// Use this when starting a server and you do not need to
    /// call the shutdown channel.
    pub fn new_keepalive(tx: oneshot::Sender<bool>) -> Self {
        Self {
            render: HashMap::new(),
            websockets: HashMap::new(),
            //shutdown_tx: Some(tx),
        }
    }
}

#[derive(Debug, Default)]
pub struct WatchChannels {
    pub(crate) render: HashMap<
        String,
        mpsc::Receiver<(String, oneshot::Sender<ResponseValue>)>,
    >,
    pub(crate) websockets: HashMap<String, broadcast::Sender<Message>>,
}
