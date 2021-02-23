use std::rc::Rc;
use wry::{Application, Attributes, Callback};
use serde_json::Value;

use json_rpc2::*;
use log::{error, info, warn};

use crate::services::*;

/// Create a native application window and display the given URL.
pub fn window(url: String) -> crate::Result<()> {
    let log_info = Callback {
        name: "log_info".to_owned(),
        function: Box::new(move |_proxy, _sequence, requests| {
            let values = requests
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<String>>();
            info!("{}", values.join(" "));
            Ok(())
        }),
    };
    let log_warn = Callback {
        name: "log_warn".to_owned(),
        function: Box::new(move |_proxy, _sequence, requests| {
            let values = requests
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<String>>();
            warn!("{}", values.join(" "));
            Ok(())
        }),
    };
    let log_error = Callback {
        name: "log_error".to_owned(),
        function: Box::new(move |_proxy, _sequence, requests| {
            let values = requests
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<String>>();
            error!("{}", values.join(" "));
            Ok(())
        }),
    };

    let ipc = Callback {
        name: "external_handler".to_owned(),
        function: Box::new(move |proxy, _sequence, mut requests| {
            let window_proxy = Rc::new(proxy);

            let window_service: Box<dyn Service> = Box::new(WindowService {
                proxy: Rc::clone(&window_proxy),
            });
            let dialog_service: Box<dyn Service> = Box::new(DialogService {});
            let project_service: Box<dyn Service> = Box::new(ProjectService {});
            let services =
                vec![&window_service, &dialog_service, &project_service];

            if let Some(_) = requests.get(0) {
                let arg = requests.swap_remove(0);

                if let Value::String(msg) = arg {
                    let response = match from_str(&msg) {
                        Ok(mut req) => match handle(&services, &mut req) {
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
            }

            Ok(())
        }),
    };

    let mut app = Application::new()?;
    let attrs = Attributes {
        url: Some(url),
        title: "Universal Web Editor".to_string(),
        ..Default::default()
    };
    app.add_window(attrs, Some(vec![ipc, log_error, log_info, log_warn]), None)?;
    app.run();
    Ok(())
}
