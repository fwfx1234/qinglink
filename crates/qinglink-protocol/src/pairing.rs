use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WakeNetworkAdapter {
    pub mac_address: String,
    pub ipv4_address: String,
    pub prefix_length: u8,
    pub broadcast_address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingRequest {
    pub device_id: String,
    pub device_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingRequestResponse {
    pub request_id: Uuid,
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingConfirm {
    pub request_id: Uuid,
    pub pin: String,
    pub device_name: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingTokenResponse {
    pub token: String,
    pub pc_id: String,
    pub pc_name: String,
    pub mac_address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake_networks: Option<Vec<WakeNetworkAdapter>>,
}
