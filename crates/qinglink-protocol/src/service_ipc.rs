use serde::{Deserialize, Serialize};

pub const SERVICE_IPC_VERSION: u32 = 2;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServiceRequest {
    pub version: u32,
    pub request_id: String,
    pub command: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServiceResponse {
    pub version: u32,
    pub request_id: String,
    pub ok: bool,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ServiceEvent {
    pub version: u32,
    pub event: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}
