use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub enum RequestEvent {
    #[serde(rename = "enter-fullscreen")]
    EnterFullScreen,
    #[serde(rename = "exit-fullscreen")]
    ExitFullScreen,
    #[serde(rename = "open-folder")]
    OpenFolder,
}

#[derive(Debug, Serialize)]
pub enum ResponseEvent {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "open-folder")]
    OpenFolder { path: String },
    #[serde(rename = "dialog-cancel")]
    DialogCancel,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    #[serde(skip)]
    pub id: i32,
    pub event: RequestEvent,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub id: i32,
    pub event: ResponseEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Response {
    pub fn ok(id: i32) -> Self {
        Self {
            id,
            event: ResponseEvent::Ok,
            ..Default::default()
        }
    }

    pub fn into_error(id: i32, e: impl std::error::Error) -> Self {
        Self {
            id,
            event: ResponseEvent::Error,
            message: Some(e.to_string()),
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            event: ResponseEvent::Error,
            id: 0,
            message: None,
        }
    }
}

