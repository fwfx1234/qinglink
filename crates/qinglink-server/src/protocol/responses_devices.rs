// === Device management responses ===

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub device_name: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub permanent: bool,
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct DeviceListResponse {
    pub devices: Vec<DeviceInfo>,
}


