//! Types that encapsulate the channels for message passing.

use std::collections::HashMap;

use tokio::sync::{broadcast, mpsc, oneshot};
use warp::ws::Message;

pub type ResponseValue = Option<Box<dyn std::error::Error + Send + Sync>>;

pub(crate) const RENDER_CHANNEL_BUFFER: usize = 128;

#[derive(Debug)]
pub struct ServerChannels {
    pub(crate) render: HashMap<String, mpsc::Sender<String>>,
    pub(crate) websockets: HashMap<String, broadcast::Sender<Message>>,
    pub(crate) render_responses: HashMap<String, mpsc::Receiver<ResponseValue>>,
    pub(crate) shutdown: oneshot::Receiver<()>,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
}

impl ServerChannels {
    pub fn new(tx: oneshot::Sender::<()>, rx: oneshot::Receiver::<()>) -> Self {
        Self {
            render: HashMap::new(),
            websockets: HashMap::new(),
            render_responses: HashMap::new(),
            shutdown_tx: Some(tx),
            shutdown: rx,
        }
    }

    pub fn new_shutdown(rx: oneshot::Receiver::<()>) -> Self {
        Self {
            render: HashMap::new(),
            websockets: HashMap::new(),
            render_responses: HashMap::new(),
            shutdown_tx: None,
            shutdown: rx,
        }
    }
}

#[derive(Debug, Default)]
pub struct WatchChannels {
    pub(crate) render: HashMap<String, mpsc::Receiver<String>>,
    pub(crate) websockets: HashMap<String, broadcast::Sender<Message>>,
    pub(crate) render_responses: HashMap<String, mpsc::Sender<ResponseValue>>,
}
