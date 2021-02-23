use serde_json::Value;
use std::rc::Rc;
use wry::WindowProxy;

use json_rpc2::*;

pub struct ProjectService;

impl Service for ProjectService {
    fn handle(&self, req: &mut Request) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("project.open") {
            //println!("Got project open!");
            response = Some(req.into());
        }
        Ok(response)
    }
}

pub struct DialogService;

impl Service for DialogService {
    fn handle(&self, req: &mut Request) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("folder.open") {
            let title: String = req.into_params()?;
            let folder = tinyfiledialogs::select_folder_dialog(&title, "");
            response = if let Some(ref path) = folder {
                Some((req, Value::String(path.to_string())).into())
            } else {
                Some(req.into())
            };
        }
        Ok(response)
    }
}

pub struct WindowService {
    pub(crate) proxy: Rc<WindowProxy>,
}

impl Service for WindowService {
    fn handle(&self, req: &mut Request) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("window.set_fullscreen") {
            let flag: bool = req.into_params()?;
            self.proxy.set_fullscreen(flag).map_err(Error::boxed)?;
            response = Some(req.into());
        }
        Ok(response)
    }
}
