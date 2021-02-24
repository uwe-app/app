use serde_json::Value;
use std::rc::Rc;
use wry::WindowProxy;

use json_rpc2::*;

pub struct ProjectService;

impl<T> Service<T> for ProjectService {
    fn handle(&self, req: &mut Request, _ctx: &Context<T>) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("project.open") {
            //println!("Got project open!");
            response = Some(req.into());
        }
        Ok(response)
    }
}

pub struct DialogService;

impl<T> Service<T> for DialogService {
    fn handle(&self, req: &mut Request, _ctx: &Context<T>) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("folder.open") {
            let title: String = req.deserialize()?;
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

impl<T> Service<T> for WindowService {
    fn handle(&self, req: &mut Request, _ctx: &Context<T>) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("window.set_fullscreen") {
            let flag: bool = req.deserialize()?;
            self.proxy.set_fullscreen(flag).map_err(Error::boxed)?;
            response = Some(req.into());
        }
        Ok(response)
    }
}
