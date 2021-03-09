use serde_json::Value;
use wry::{Application, Attributes, RpcRequest, RpcResponse, WindowProxy};

use json_rpc2::{futures::{Service, Server}, Request, Response, RpcError};
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

    let rt = tokio::runtime::Runtime::new().unwrap();

    let handler = Box::new(move |proxy: WindowProxy, req: RpcRequest| {
        let mut req = into_request(req);

        // Synchronous handling for window operations because
        // we cannot send `&WindowProxy` between threads which is
        // required for use with futures.
        if req.method().starts_with("window") {
            let window_service: Box<dyn json_rpc2::Service<Data = WindowProxy>> =
                Box::new(WindowService {});
            let server = json_rpc2::Server::new(vec![&window_service]);
            if let Some(response) = server.serve(&mut req, &proxy) {
                Some(into_response(response))
            } else { None }
        } else {

            let server = Server::new(vec![
                &app_service,
                &console_service,
                &dialog_service,
                &project_service,
            ]);

            rt.block_on(async move {
                let data = ServiceData {};
                if let Some(response) = server.serve(&mut req, &data).await {
                    if let Some(id) = req.id_mut().take() {
                        let script = if let Some(err) = response.error() {
                            let err = serde_json::to_value(err).unwrap();
                            RpcResponse::into_error_script(id, err).unwrap()
                        } else if let Some(res) = response.result() {
                            let res = serde_json::to_value(res).unwrap();
                            RpcResponse::into_result_script(id, res).unwrap()
                        } else {
                            RpcResponse::into_result_script(id, Value::Null).unwrap()
                        };
                        let _ = proxy.evaluate_script(&script);
                    }
                }
            });

            None
        }

        /*
        if let Some(response) = server.serve(&mut req, &data) {
            Some(into_response(response))
        } else { None }
        */
        //None
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
