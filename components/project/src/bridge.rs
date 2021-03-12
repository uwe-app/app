use serde::{Deserialize, Serialize};

use config::server::ConnectionInfo;

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionBridge {
    id: String,
    connection: ConnectionInfo,
}

impl ConnectionBridge {
    pub fn new(id: String, connection: ConnectionInfo) -> Self {
        Self { id, connection }
    }
}
