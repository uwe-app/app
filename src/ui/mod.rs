use serde::{Deserialize, Serialize};
use wry::{Application, Attributes, Callback, WindowProxy};

#[derive(Debug, Deserialize)]
pub enum IpcRequestEvent {
    #[serde(rename = "enter-fullscreen")]
    EnterFullScreen,
    #[serde(rename = "exit-fullscreen")]
    ExitFullScreen,
    #[serde(rename = "open-folder")]
    OpenFolder,
}

#[derive(Debug, Serialize)]
pub enum IpcResponseEvent {
    #[serde(rename = "error")]
    Error,

    #[serde(rename = "ok")]
    Ok,

    #[serde(rename = "open-folder")]
    OpenFolder { path: String },
    #[serde(rename = "dialog-cancel")]
    DialogCancel,
}

#[derive(Debug, Deserialize)]
pub struct IpcRequest {
    #[serde(skip)]
    pub id: i32,
    pub event: IpcRequestEvent,
}

#[derive(Debug, Serialize)]
pub struct IpcResponse {
    pub id: i32,
    pub event: IpcResponseEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl IpcResponse {
    fn ok(id: i32) -> Self {
        Self {
            id,
            event: IpcResponseEvent::Ok,
            ..Default::default()
        }
    }

    fn into_error(id: i32, e: impl std::error::Error) -> Self {
        Self {
            id,
            event: IpcResponseEvent::Error,
            message: Some(e.to_string()),
        }
    }
}

impl Default for IpcResponse {
    fn default() -> Self {
        Self {
            event: IpcResponseEvent::Error,
            id: 0,
            message: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error(transparent)]
    Wry(#[from] wry::Error),
}

type Result<T> = std::result::Result<T, IpcError>;

pub struct IpcRouter;

impl IpcRouter {
    pub fn handle(
        proxy: &WindowProxy,
        id: i32,
        req: IpcRequest,
    ) -> Result<IpcResponse> {
        let mut response = IpcResponse::ok(id);

        match req.event {
            IpcRequestEvent::OpenFolder => {
                let folder = tinyfiledialogs::select_folder_dialog(
                    "Choose a project",
                    "",
                );
                if let Some(ref path) = folder {
                    response = IpcResponse {
                        id,
                        event: IpcResponseEvent::OpenFolder {
                            path: path.to_string(),
                        },
                        ..Default::default()
                    };
                } else {
                    response = IpcResponse {
                        id,
                        event: IpcResponseEvent::DialogCancel,
                        ..Default::default()
                    };
                }
            }
            IpcRequestEvent::EnterFullScreen => {
                proxy.set_fullscreen(true)?;
            }
            IpcRequestEvent::ExitFullScreen => {
                proxy.set_fullscreen(false)?;
            }
        }
        Ok(response)
    }
}

pub(crate) fn window(url: String) -> wry::Result<()> {
    let callback = Callback {
        name: "onIpcRequest".to_owned(),
        function: Box::new(move |proxy, sequence, requests| {
            let mut response: IpcResponse = Default::default();

            if let Some(arg) = requests.get(0) {
                let request = serde_json::from_str::<IpcRequest>(arg);
                match request {
                    Ok(mut req) => {
                        req.id = sequence;
                        //println!("Got request {:#?}", req);
                        match IpcRouter::handle(&proxy, sequence, req) {
                            Ok(res) => response = res,
                            Err(e) => {
                                response = IpcResponse::into_error(sequence, e)
                            }
                        }
                    }
                    Err(e) => {
                        response = IpcResponse::into_error(sequence, e);
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
