use serde_json::Value;
use wry::WindowProxy;

use json_rpc2::*;

pub struct ServiceData {
    pub window: WindowProxy,
}

pub struct ProjectService;

impl Service for ProjectService {
    type Data = ServiceData;
    fn handle(
        &self,
        req: &mut Request,
        _ctx: &Self::Data,
    ) -> Result<Option<Response>> {
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
    fn handle(
        &self,
        req: &mut Request,
        _ctx: &Self::Data,
    ) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("folder.open") {
            let params: Vec<String> = req.deserialize()?;
            if let Some(title) = params.get(0) {
                let folder = tinyfiledialogs::select_folder_dialog(title, "");
                response = if let Some(ref path) = folder {
                    Some((req, Value::String(path.to_string())).into())
                } else {
                    Some(req.into())
                }
            } else {
                response = Some(req.into());
            }
        }
        Ok(response)
    }
}

pub struct WindowService;

impl Service for WindowService {
    type Data = ServiceData;
    fn handle(
        &self,
        req: &mut Request,
        ctx: &Self::Data,
    ) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("window.set_fullscreen") {
            let mut params: Vec<bool> = req.deserialize()?;
            let flag = if params.get(0).is_none() {
                None
            } else { Some(params.swap_remove(0)) };
            if let Some(flag) = flag {
                ctx.window.set_fullscreen(flag).map_err(Box::from)?;
            }
            response = Some(req.into());
        }
        Ok(response)
    }
}
