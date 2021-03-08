use serde_json::Value;
use wry::WindowProxy;

use json_rpc2::*;

use log::{info, warn, error};

use project::ProjectManifestEntry;

pub struct ServiceData {
    pub window: WindowProxy,
}

pub struct ConsoleService;

impl Service for ConsoleService {
    type Data = ServiceData;
    fn handle(
        &self,
        req: &mut Request,
        _ctx: &Self::Data,
    ) -> Result<Option<Response>> {
        let mut response = None;
        if req.method().starts_with("console.") {
            let params: Vec<Value> = req.deserialize()?;
            let log_value = params
                .into_iter()
                .map(|v| {
                    if let Value::String(s) = v {
                        s
                    } else {
                        serde_json::to_string(&v).unwrap()
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            if req.method().ends_with("log") {
                info!("CONSOLE {}", log_value);
            } else if req.method().ends_with("info") {
                info!("CONSOLE {}", log_value);
            } else if req.method().ends_with("warn") {
                warn!("CONSOLE {}", log_value);
            } else if req.method().ends_with("error") {
                error!("CONSOLE {}", log_value);
            }

            response = Some(req.into());
        }
        Ok(response)
    }
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
        if req.matches("project.list") {
            let manifest = project::list().map_err(Box::from)?;
            let result = serde_json::to_value(manifest).map_err(Box::from)?;
            response = Some((req, result).into());
        } else if req.matches("project.add") {
            let mut params: Vec<ProjectManifestEntry> = req.deserialize()?;
            if !params.is_empty() {
                let entry = params.swap_remove(0);
                project::add(entry).map_err(Box::from)?;
                response = Some(req.into());
            } else {
                return Err((req, "Method expects parameters").into())
            }
        } else if req.matches("project.remove") {
            let mut params: Vec<ProjectManifestEntry> = req.deserialize()?;
            if !params.is_empty() {
                let entry = params.swap_remove(0);
                project::remove(&entry).map_err(Box::from)?;
                response = Some(req.into());
            } else {
                return Err((req, "Method expects parameters").into())
            }
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
