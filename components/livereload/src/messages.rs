use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum EventType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "notify")]
    Notify,
}

#[derive(Debug, Serialize, Deserialize)]
struct SimpleEvent {
    #[serde(rename = "type")]
    pub event_type: EventType,
}

#[derive(Debug, Serialize, Deserialize)]
struct MessageEvent {
    #[serde(rename = "type")]
    pub event_type: EventType,

    pub message: String,
    pub error: bool,
}

pub fn start() -> impl Serialize + std::fmt::Debug {
    SimpleEvent {
        event_type: EventType::Start,
    }
}

pub fn reload() -> impl Serialize + std::fmt::Debug {
    SimpleEvent {
        event_type: EventType::Reload,
    }
}

pub fn notify(message: String, error: bool) -> impl Serialize + std::fmt::Debug {
    MessageEvent {
        event_type: EventType::Notify,
        message,
        error,
    }
}
