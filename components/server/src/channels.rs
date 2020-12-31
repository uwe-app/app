//! Types that encapsulate the channels for message passing.

use std::collections::HashMap;

use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedSender},
    oneshot,
};
use warp::ws::Message;

use config::server::ConnectionInfo;

type WebsocketSender = broadcast::Sender<Message>;
type BindSender = oneshot::Sender<ConnectionInfo>;
type RenderRequest = mpsc::UnboundedSender<String>;
pub type ResponseValue = Option<Box<dyn std::error::Error + Send>>;

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

