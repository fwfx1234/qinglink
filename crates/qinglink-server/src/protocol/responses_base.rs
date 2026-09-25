#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}


