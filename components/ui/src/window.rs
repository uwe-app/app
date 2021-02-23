use std::rc::Rc;
use wry::{Application, Attributes, Callback};

use log::{error, info, warn};

use crate::{jsonrpc::*, services::*};

/// Create a native application window and display the given URL.
pub fn window(url: String) -> crate::Result<()> {
    /*
    let log_info = Callback {
        name: "log_info".to_owned(),
        function: Box::new(move |proxy, _sequence, requests| {
            info!("{}", requests.join(" "));
            0
        }),
    };
    let log_warn = Callback {
        name: "log_warn".to_owned(),
        function: Box::new(move |proxy, _sequence, requests| {
            warn!("{}", requests.join(" "));
            0
        }),
    };
    let log_error = Callback {
        name: "log_error".to_owned(),
        function: Box::new(move |proxy, _sequence, requests| {
            error!("{}", requests.join(" "));
            0
        }),
    };
    */

    let ipc = Callback {
        name: "external_handler".to_owned(),
        function: Box::new(move |proxy, _sequence, requests| {
            let window_proxy = Rc::new(proxy);

            let broker = Broker {};
            let window_service: Box<dyn Service> = Box::new(WindowService {
                proxy: Rc::clone(&window_proxy),
            });
            let dialog_service: Box<dyn Service> = Box::new(DialogService {});
            let project_service: Box<dyn Service> = Box::new(ProjectService {});
            let services =
                vec![&window_service, &dialog_service, &project_service];

            if let Some(arg) = requests.get(0) {
                let response = match Request::from_str(arg) {
                    Ok(mut req) => match broker.handle(&services, &mut req) {
                        Ok(result) => result,
                        Err(e) => (&mut req, e).into(),
                    },
                    Err(e) => e.into(),
                };

                let invoke = format!(
                    r#"window.ipc.responses[{}] = {}"#,
                    response.id(),
                    serde_json::to_string(&response).unwrap()
                );
                window_proxy.evaluate_script(invoke).unwrap();
            }

            0
        }),
    };

    let mut app = Application::new()?;
    let attrs = Attributes {
        url: Some(url),
        title: "Universal Web Editor".to_string(),
        ..Default::default()
    };
    app.add_window(attrs, Some(vec![ipc /*, log_info, log_warn, log_error*/]))?;
    app.run();
    Ok(())
}
