use serde_json::Value;
use wry::{Application, Attributes, RpcRequest, RpcResponse, WindowProxy};

use json_rpc2::*;
//use log::{error, info, warn};

use crate::services::*;

/// Convert a WRY RpcRequest into a json_rpc2::Request
/// so we can process it via the service handler.
fn into_request(req: RpcRequest) -> Request {
    Request::new(req.id, req.method, req.params)
}

/// Convert a service Response into an RpcResponse for passing
/// back to the Javascript.
fn into_response(res: Response) -> RpcResponse {
    let (id, error, result): (Option<Value>, Option<RpcError>, Option<Value>) = res.into();
    if let Some(err) = error {
        RpcResponse::new_error(id, serde_json::to_value(err).ok())
    } else {
        RpcResponse::new_result(id, result)
    }
}

/// Create a native application window and display the given URL.
pub fn window(url: String) -> crate::Result<()> {

    let mut app = Application::new()?;

    let app_service: Box<dyn Service<Data = ServiceData>> =
        Box::new(AppService {});
    let console_service: Box<dyn Service<Data = ServiceData>> =
        Box::new(ConsoleService {});
    let dialog_service: Box<dyn Service<Data = ServiceData>> =
        Box::new(DialogService {});
    let project_service: Box<dyn Service<Data = ServiceData>> =
        Box::new(ProjectService {});
    let window_service: Box<dyn Service<Data = ServiceData>> =
        Box::new(WindowService {});

    let handler = Box::new(move |proxy: WindowProxy, req: RpcRequest| {
        let server = Server::new(vec![
            &app_service,
            &console_service,
            &dialog_service,
            &project_service,
            &window_service,
        ]);

        let mut req = into_request(req);

        let data = ServiceData {
            window: proxy,
        };

        if let Some(response) = server.serve(&mut req, &data) {
            Some(into_response(response))
        } else { None }
    });

    let attrs = Attributes {
        url: Some(url),
        title: "Universal Web Editor".to_string(),
        ..Default::default()
    };
    app.add_window_with_configs(
        attrs,
        Some(handler),
        None,
    )?;
    app.run();
    Ok(())
}
