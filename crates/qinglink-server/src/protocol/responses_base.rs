/// 统一的 API 响应包装：成功时携带 `data`，失败时携带 `error`。
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// 请求是否成功。
    pub ok: bool,
    /// 成功时的响应数据，失败时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 失败时的错误详情，成功时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 构造一个携带数据的成功响应。
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// 构造一个携带错误码与错误信息的失败响应。
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

/// API 错误详情。
#[derive(Debug, Serialize)]
pub struct ApiError {
    /// 机器可读的错误码。
    pub code: String,
    /// 面向用户的错误描述。
    pub message: String,
}

