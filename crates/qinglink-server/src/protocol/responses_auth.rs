// === Auth responses ===

#[derive(Debug, Serialize)]
pub struct PairResponse {
    pub token: String,
    /// None 表示永不过期
    pub expires_at: Option<i64>,
    /// PC 的 MAC 地址，用于 Wake-on-LAN 魔法封唤醒
    pub mac_address: Option<String>,
}


