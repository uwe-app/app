use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "reload")]
    Reload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleEvent {
    #[serde(rename = "type")]
    pub event_type: EventType,
}

pub fn start() -> impl Serialize + std::fmt::Debug {
    SimpleEvent {event_type: EventType::Start}
}

pub fn reload() -> impl Serialize + std::fmt::Debug {
    SimpleEvent {event_type: EventType::Reload}
}
