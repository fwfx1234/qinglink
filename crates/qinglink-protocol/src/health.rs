use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub pc_id: String,
    pub name: String,
    pub version: u8,
    pub port: u16,
}
