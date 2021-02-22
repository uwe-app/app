use serde_json::Value;
use std::rc::Rc;
use wry::{Application, Attributes, Callback, WindowProxy};

use crate::jsonrpc::*;

pub struct ProjectService;

impl Service for ProjectService {
    fn handle(&self, req: &mut JsonRpcRequest) -> Result<Option<JsonRpcResponse>> {
        let mut response = None;
        if req.matches("project.open") {
            println!("Got project open!");
            response = Some(JsonRpcResponse::reply(req));
        }
        Ok(response)
    }
}

pub struct DialogService;

impl Service for DialogService {
    fn handle(&self, req: &mut JsonRpcRequest) -> Result<Option<JsonRpcResponse>> {
        let mut response = None;
        if req.matches("folder.open") {
            let title: String = req.into_params()?;
            let folder =
                tinyfiledialogs::select_folder_dialog(&title, "");
            if let Some(ref path) = folder {
                response = Some(JsonRpcResponse::response(
                    req,
                    Some(Value::String(path.to_string())),
                ));
            } else {
                response = Some(JsonRpcResponse::reply(req));
            }
        }
        Ok(response)
    }
}

pub struct WindowService {
    proxy: Rc<WindowProxy>,
}

impl Service for WindowService {
    fn handle(&self, req: &mut JsonRpcRequest) -> Result<Option<JsonRpcResponse>> {
        let mut response = None;
        if req.matches("window.set_fullscreen") {
            let flag: bool = req.into_params()?;
            self.proxy.set_fullscreen(flag).map_err(box_error)?;
            response = Some(JsonRpcResponse::reply(req));
        }
        Ok(response)
    }
}

/// Create a native application window and display the given URL.
pub fn window(url: String) -> crate::Result<()> {
    let callback = Callback {
        name: "onIpcRequest".to_owned(),
        function: Box::new(move |proxy, sequence, requests| {
            let mut response: JsonRpcResponse = Default::default();

            let window_proxy = Rc::new(proxy);

            let broker = Broker {};
            let window_service: Box<dyn Service> = Box::new(WindowService {
                proxy: Rc::clone(&window_proxy),
            });
            let dialog_service: Box<dyn Service> = Box::new(DialogService {});
            let project_service: Box<dyn Service> = Box::new(ProjectService {});
            let services = vec![&window_service, &dialog_service, &project_service];

            if let Some(arg) = requests.get(0) {
                match JsonRpcRequest::from_str(arg) {
                    Ok(mut req) => match broker.handle(&services, &mut req) {
                        Ok(result) => {
                            response = result;
                        }
                        Err(e) => {
                            response = (&mut req, e).into();
                        }
                    },
                    Err(e) => {
                        response = JsonRpcResponse::error(
                            e.to_string(),
                            sequence as isize,
                            Value::Null,
                        )
                    }
                }
            }

            let invoke = format!(
                "onIpcMessage({})",
                serde_json::to_string(&response).unwrap()
            );
            window_proxy.evaluate_script(invoke).unwrap();

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
