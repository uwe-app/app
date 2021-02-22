use wry::{Application, Attributes, Callback, WindowProxy};

use crate::{webview_ipc as ipc, Result};

pub struct Router;

impl Router {
    pub fn handle(
        proxy: &WindowProxy,
        id: i32,
        req: ipc::Request,
    ) -> Result<ipc::Response> {
        let mut response = ipc::Response::ok(id);

        match req.event {
            ipc::RequestEvent::OpenFolder => {
                let folder = tinyfiledialogs::select_folder_dialog(
                    "Choose a project",
                    "",
                );
                if let Some(ref path) = folder {
                    response = ipc::Response {
                        id,
                        event: ipc::ResponseEvent::OpenFolder {
                            path: path.to_string(),
                        },
                        ..Default::default()
                    };
                } else {
                    response = ipc::Response {
                        id,
                        event: ipc::ResponseEvent::DialogCancel,
                        ..Default::default()
                    };
                }
            }
            ipc::RequestEvent::EnterFullScreen => {
                proxy.set_fullscreen(true)?;
            }
            ipc::RequestEvent::ExitFullScreen => {
                proxy.set_fullscreen(false)?;
            }
        }
        Ok(response)
    }
}

/// Create a native application window and display the given URL.
pub fn window(url: String) -> Result<()> {
    let callback = Callback {
        name: "onIpcRequest".to_owned(),
        function: Box::new(move |proxy, sequence, requests| {
            let mut response: ipc::Response = Default::default();

            if let Some(arg) = requests.get(0) {
                let request = serde_json::from_str::<ipc::Request>(arg);
                match request {
                    Ok(mut req) => {
                        req.id = sequence;
                        //println!("Got request {:#?}", req);
                        match Router::handle(&proxy, sequence, req) {
                            Ok(res) => response = res,
                            Err(e) => {
                                response = ipc::Response::into_error(sequence, e)
                            }
                        }
                    }
                    Err(e) => {
                        response = ipc::Response::into_error(sequence, e);
                    }
                }
            }

            let invoke = format!(
                "onIpcMessage({})",
                serde_json::to_string(&response).unwrap()
            );
            proxy.evaluate_script(invoke).unwrap();
            0
        }),
    };

    let mut app = Application::new()?;
    let attrs = Attributes {
        url: Some(url),
        title: "Universal Web Editor".to_string(),
        ..Default::default()
    };
    app.add_window(attrs, Some(vec![callback]))?;
    app.run();
    Ok(())
}
