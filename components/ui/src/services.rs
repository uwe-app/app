use std::rc::Rc;
use serde_json::Value;
use wry::WindowProxy;

use json_rpc2::*;

pub struct ServiceData {
    pub window: Rc<WindowProxy>,
}

pub struct ProjectService;

impl Service for ProjectService {
    type Data = ServiceData;
    fn handle(&self, req: &mut Request, _ctx: &Context<Self::Data>) -> Result<Option<Response>> {
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
    type Data = ServiceData;
    fn handle(&self, req: &mut Request, _ctx: &Context<Self::Data>) -> Result<Option<Response>> {
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

pub struct WindowService;

impl Service for WindowService {
    type Data = ServiceData;
    fn handle(&self, req: &mut Request, ctx: &Context<Self::Data>) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("window.set_fullscreen") {
            let flag: bool = req.deserialize()?;
            ctx.data().window.set_fullscreen(flag).map_err(Error::boxed)?;
            response = Some(req.into());
        }
        Ok(response)
    }
}
