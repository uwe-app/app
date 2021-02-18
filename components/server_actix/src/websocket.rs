use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use crate::reload_server::{self, LiveReloadServer};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// do websocket handshake and start `ClientSocket` actor
pub(crate) async fn ws_index(
    r: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<LiveReloadServer>>,
) -> Result<HttpResponse, Error> {
    let socket = ClientSocket::new(srv.get_ref().clone());
    let res = ws::start(socket, &r, stream);
    //println!("{:?}", res);
    res
}

/// websocket connection is long running connection, it easier
/// to handle with an actor
pub struct ClientSocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,

    addr: Addr<LiveReloadServer>,

    id: usize,
}

/// Handle messages from server, we simply send it to peer websocket
impl Handler<reload_server::Message> for ClientSocket {
    type Result = ();
    fn handle(&mut self, msg: reload_server::Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl Actor for ClientSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.addr
            .send(reload_server::Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // Notify server
        self.addr.do_send(reload_server::Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handler for incoming `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ClientSocket {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        // process websocket messages
        //println!("WS: {:?}", msg);

        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl ClientSocket {
    fn new(addr: Addr<LiveReloadServer>) -> Self {
        Self {
            hb: Instant::now(),
            id: 0,
            addr,
        }
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                //println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}
