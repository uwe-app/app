//! Types that encapsulate the channels for message passing.

use std::collections::HashMap;

use tokio::sync::{
    broadcast,
    mpsc,
};
use warp::ws::Message;

pub type ResponseValue = Option<Box<dyn std::error::Error + Send>>;

pub(crate) const RENDER_CHANNEL_BUFFER: usize = 128;

#[derive(Debug, Default)]
pub struct ServerChannels {
    pub(crate) render: HashMap<String, mpsc::Sender<String>>,
    pub(crate) websockets: HashMap<String, broadcast::Sender<Message>>,
}

#[derive(Debug, Default)]
pub struct WatchChannels {
    pub(crate) render: HashMap<String, mpsc::Receiver<String>>,
    pub(crate) websockets: HashMap<String, broadcast::Sender<Message>>,
}
