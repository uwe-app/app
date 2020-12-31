//! Types that encapsulate the channels for message passing.

use std::collections::HashMap;

use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedSender, UnboundedReceiver},
    oneshot,
};
use warp::ws::Message;

use config::server::ConnectionInfo;

type WebsocketSender = broadcast::Sender<Message>;
type BindSender = oneshot::Sender<ConnectionInfo>;
type RenderRequest = mpsc::UnboundedSender<String>;
pub type ResponseValue = Option<Box<dyn std::error::Error + Send>>;

/// Maps the virtual host communication channels by host name.
#[derive(Debug)]
pub struct Channels {
    pub bind: Option<BindSender>,

    pub render: HashMap<String, (UnboundedSender<String>, UnboundedReceiver<String>)>,
    pub websockets: HashMap<String, (broadcast::Sender<Message>, broadcast::Receiver<Message>)>,
}

impl Channels {
    pub fn new(bind: BindSender) -> Self {
        Self {
            bind: Some(bind),
            websockets: HashMap::new(),
            render: HashMap::new(),
        }
    }

    pub fn get_host_reload(&self, name: &str) -> WebsocketSender {
        if let Some((ws_tx, _)) = self.websockets.get(name) {
            return ws_tx.clone();
        }

        let (ws_tx, _) = broadcast::channel::<Message>(128);
        ws_tx
    }

    pub fn get_host_render_request(&self, name: &str) -> RenderRequest {
        if let Some((render_tx, _)) = self.render.get(name) {
            return render_tx.clone();
        }
        let (render_tx, _) = mpsc::unbounded_channel::<String>();
        render_tx
    }
}
