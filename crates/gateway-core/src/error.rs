use serde::Serialize;
use thiserror::Error;

pub type GatewayResult<T> = Result<T, GatewayError>;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("authentication failed")]
    Authentication,
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("upstream Cypht error: {0}")]
    Upstream(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("cryptographic error")]
    Crypto,
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
}

impl GatewayError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Authentication => "authentication_failed",
            Self::PermissionDenied(_) => "permission_denied",
            Self::NotFound(_) => "not_found",
            Self::InvalidRequest(_) => "invalid_request",
            Self::Upstream(_) => "upstream_error",
            Self::Storage(_) => "storage_error",
            Self::Crypto => "crypto_error",
            Self::Configuration(_) => "configuration_error",
            Self::Internal(_) => "internal_error",
        }
    }
}
