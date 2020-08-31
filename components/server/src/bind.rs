use tokio::sync::oneshot;

use log::{error, info};

use crate::Error;
use config::server::{ServerConfig, LaunchConfig, ConnectionInfo};
use super::{router, BindSender, Channels};

pub async fn bind(
    options: &'static ServerConfig,
    launch: LaunchConfig,
    bind: Option<BindSender>,
    channels: &Channels,
) -> Result<(), Error> {

    // Create a channel to receive the bind address.
    let (ctx, crx) = oneshot::channel::<ConnectionInfo>();

    let _ = tokio::task::spawn(async move {
        let info = crx.await.unwrap();

        let url = info.to_url();
        info!("Serve {}", &url);

        // Most of the time we want to open a browser unless explictly
        // disabled however in the case of the live reload logic it 
        // takes control of opening the browser so that:
        //
        // 1) Don't start to compile until we have bound to a port.
        // 2) Don't open a browser window unless the build succeeds.
        // 
        if launch.open {
            // It is ok if this errors we just don't open a browser window
            open::that(&url).map(|_| ()).unwrap_or(());
        }

        if let Some(bind) = bind {
            if let Err(_) = bind.send(info) {
                error!("Failed to notify of server bind event");
                std::process::exit(1);
            }
        }

    });

    router::serve(options, ctx, channels).await?;

    Ok(())
}
