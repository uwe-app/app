use std::sync::{Arc};
use psup_impl::{Supervisor, SupervisorBuilder};

use tokio::sync::{Mutex, mpsc::Receiver};

use crate::Result;

#[derive(Debug)]
pub enum ProcessMessage {

}

pub fn supervisor(ps_rx: Receiver<ProcessMessage>) -> Result<Supervisor> {
    let arx = Arc::new(Mutex::new(ps_rx));

    // Set up the child process supervisor
    Ok(SupervisorBuilder::new()
        .server(move |_stream, _tx| {
            let rx = Arc::clone(&arx);
            //let (reader, mut writer) = stream.into_split();
            tokio::task::spawn(async move {
                let mut rx = rx.lock().await;

                tokio::select!(
                    msg = rx.recv() => {
                        if let Some(msg) = msg {
                            println!("Got a message from the service broker {:?}", msg);
                        }
                    }
                );

                // Handle worker connection here
                // Use the `tx` supervisor control channel
                // to spawn and shutdown workers
                Ok::<(), psup_impl::Error>(())
            });
        })
        .path(dirs::socket_file()?)
        .build())
}
