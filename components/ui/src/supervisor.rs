use psup_impl::{Supervisor, SupervisorBuilder};

use tokio::sync::oneshot::{Sender, Receiver};

use crate::Result;

#[derive(Debug)]
pub enum ProcessMessage {
    OpenProject { path: String, reply: Sender<()> },
}

pub fn supervisor(shutdown: Receiver<()>) -> Result<Supervisor> {
    // Set up the child process supervisor
    Ok(SupervisorBuilder::new()
        .server(move |_stream, _tx| {
            tokio::task::spawn(async move {
                // Handle worker connection here
                // Use the `tx` supervisor control channel
                // to spawn and shutdown workers
                Ok::<(), psup_impl::Error>(())
            });
        })
        .path(dirs::socket_file()?)
        .shutdown(shutdown)
        .build())
}
