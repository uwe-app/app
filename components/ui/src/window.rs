use serde_json::Value;
use std::rc::Rc;
use wry::{Application, Attributes, Callback, WindowProxy};

use crate::jsonrpc::*;

pub struct ProjectService;

impl Service for ProjectService {
    fn handle(&self, req: &JsonRpcRequest) -> Result<Option<JsonRpcResponse>> {
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
    fn handle(&self, req: &JsonRpcRequest) -> Result<Option<JsonRpcResponse>> {
        let mut response = None;
        if req.matches("folder.open") {
            // TODO: parse out the title!
            let folder =
                tinyfiledialogs::select_folder_dialog("Choose a project", "");
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
    fn handle(&self, req: &JsonRpcRequest) -> Result<Option<JsonRpcResponse>> {
        let mut response = None;
        if req.matches("window.enter_fullscreen") {
            self.proxy.set_fullscreen(true).map_err(box_error)?;
            response = Some(JsonRpcResponse::reply(req));
        } else if req.matches("window.exit_fullscreen") {
            self.proxy.set_fullscreen(false).map_err(box_error);
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
                let request = serde_json::from_str::<JsonRpcRequest>(arg);
                match request {
                    Ok(req) => match broker.handle(&services, &req) {
                        Ok(result) => {
                            response = result;
                        }
                        Err(e) => {
                            response = JsonRpcResponse::error(
                                e.to_string(),
                                sequence as usize,
                                Value::Null,
                            )
                        }
                    },
                    Err(e) => {
                        response = JsonRpcResponse::error(
                            e.to_string(),
                            sequence as usize,
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
