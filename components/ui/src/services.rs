use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use wry::WindowProxy;

use json_rpc2::{futures::*, Request, Response, Result};
use async_trait::async_trait;

use log::{info, warn, error};

use project::{ProjectList, ProjectManifestEntry};

pub struct ServiceData {}

pub struct ConsoleService;

#[async_trait]
impl Service for ConsoleService {
    type Data = ServiceData;
    async fn handle(
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    projects: ProjectList,
    preferences: AppPreferences,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppPreferences {
    project: ProjectPreferences,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectPreferences {
    target: Option<PathBuf>,
}

pub struct AppService;

#[async_trait]
impl Service for AppService {
    type Data = ServiceData;
    async fn handle(
        &self,
        req: &mut Request,
        _ctx: &Self::Data,
    ) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("app.boot") {
            let projects = project::list().map_err(Box::from)?;
            // TODO: get project base from preferences
            let target = dirs::home_dir();
            let info = AppInfo {
                projects,
                preferences: AppPreferences {
                    project: ProjectPreferences {target}
                }
            };
            let result = serde_json::to_value(info).map_err(Box::from)?;
            response = Some((req, result).into());
        }
        Ok(response)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectCreateRequest {
    name: String,
    target: PathBuf,
    source: String,
}

pub struct ProjectService;

#[async_trait]
impl Service for ProjectService {
    type Data = ServiceData;
    async fn handle(
        &self,
        req: &mut Request,
        _ctx: &Self::Data,
    ) -> Result<Option<Response>> {
        let mut response = None;
        if req.matches("project.find") {
            let mut params: Vec<String> = req.deserialize()?;
            if !params.is_empty() {
                let id = params.swap_remove(0);
                let entry = project::find(&id).map_err(Box::from)?;
                let value = if let Some(entry) = entry {
                    serde_json::to_value(entry).map_err(Box::from)?
                } else {
                    Value::Null
                };
                response = Some((req, value).into());
            } else {
                return Err((req, "Method expects parameters").into())
            }
        } else if req.matches("project.create") {
            let mut params: Vec<ProjectCreateRequest> = req.deserialize()?;
            if !params.is_empty() {
                let project_request = params.swap_remove(0);

                let target = project_request.target.join(&project_request.name);
                let opts = project::ProjectOptions {
                    source: Some(project_request.source),
                    target: target.to_path_buf(),
                    ..Default::default()
                };

                let entry = project::create(opts).await.map_err(Box::from)?;
                let value = serde_json::to_value(entry).map_err(Box::from)?;
                response = Some((req, value).into());
            } else {
                return Err((req, "Method expects parameters").into())
            }
        } else if req.matches("project.list") {
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

#[async_trait]
impl Service for DialogService {
    type Data = ServiceData;
    async fn handle(
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

impl json_rpc2::Service for WindowService {
    type Data = WindowProxy;
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
                ctx.set_fullscreen(flag).map_err(Box::from)?;
            }
            response = Some(req.into());
        }
        Ok(response)
    }
}
