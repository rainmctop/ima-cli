//! Error types and result aliases for IMA CLI

use thiserror::Error;
use serde::Serialize;
use std::fmt;

/// Programmatic error code (equivalent to -100 in Node.js version)
pub const ERR_PROGRAMMATIC: i32 = -100;

/// Update available error code (equivalent to -200 in Node.js version)
pub const ERR_UPDATE_AVAILABLE: i32 = -200;

/// Result type alias using internal Error enum
pub type Result<T> = std::result::Result<T, Error>;

/// CLI error structure for JSON output
#[derive(Debug, Serialize)]
pub struct CliError {
    pub code: i32,
    pub msg: String,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.msg)
    }
}

/// Internal error enum
#[derive(Error, Debug)]
pub enum Error {
    #[error("Missing credentials: {0}")]
    MissingCredentials(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("API request failed: {0}")]
    ApiRequestFailed(String),

    #[error("API response error (code={0}): {1}")]
    ApiResponseError(i32, String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("File validation failed: {0}")]
    FileValidationFailed(String),

    #[error("COS upload failed: {0}")]
    CosUploadFailed(String),

    #[error("Update available: {0}")]
    UpdateAvailable(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl Error {
    /// Get the error code for this error
    pub fn code(&self) -> i32 {
        match self {
            Error::MissingCredentials(_) => ERR_PROGRAMMATIC,
            Error::InvalidConfig(_) => ERR_PROGRAMMATIC,
            Error::InvalidArgument(_) => ERR_PROGRAMMATIC,
            Error::FileValidationFailed(_) => ERR_PROGRAMMATIC,
            Error::NotFound(_) => ERR_PROGRAMMATIC,
            Error::PermissionDenied(_) => 110030,
            Error::RateLimited(_) => 110021,
            Error::SecurityViolation(_) => 110020,
            Error::NetworkError(_) => 110010,
            Error::ApiRequestFailed(_) => 110011,
            Error::ApiResponseError(code, _) => *code,
            Error::CosUploadFailed(_) => ERR_PROGRAMMATIC,
            Error::IoError(_) => ERR_PROGRAMMATIC,
            Error::JsonError(_) => ERR_PROGRAMMATIC,
            Error::UpdateAvailable(_) => ERR_UPDATE_AVAILABLE,
            Error::InternalError(_) => ERR_PROGRAMMATIC,
        }
    }

    /// Get a human-readable message for this error
    pub fn message(&self) -> String {
        match self {
            Error::MissingCredentials(msg) => format!("未找到 IMA 凭证。{}", msg),
            Error::InvalidConfig(msg) => format!("配置错误：{}", msg),
            Error::InvalidArgument(msg) => format!("参数错误：{}", msg),
            Error::FileValidationFailed(msg) => format!("文件验证失败：{}", msg),
            Error::NotFound(msg) => format!("未找到：{}", msg),
            Error::PermissionDenied(msg) => format!("无权限：{}", msg),
            Error::RateLimited(msg) => format!("请求频控：{}", msg),
            Error::SecurityViolation(msg) => format!("安全打击：{}", msg),
            Error::NetworkError(e) => format!("网络错误：{}", e),
            Error::ApiRequestFailed(msg) => format!("API 请求失败：{}", msg),
            Error::ApiResponseError(_, msg) => msg.clone(),
            Error::CosUploadFailed(msg) => format!("COS 上传失败：{}", msg),
            Error::IoError(e) => format!("IO 错误：{}", e),
            Error::JsonError(e) => format!("JSON 错误：{}", e),
            Error::UpdateAvailable(msg) => msg.clone(),
            Error::InternalError(msg) => format!("内部错误：{}", msg),
        }
    }
}

impl From<Error> for CliError {
    fn from(err: Error) -> Self {
        CliError {
            code: err.code(),
            msg: err.message(),
        }
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError {
            code: ERR_PROGRAMMATIC,
            msg: format!("JSON error: {}", err),
        }
    }
}

// Helper functions for creating errors
pub fn missing_credentials(detail: impl Into<String>) -> Error {
    Error::MissingCredentials(detail.into())
}

pub fn invalid_config(detail: impl Into<String>) -> Error {
    Error::InvalidConfig(detail.into())
}

pub fn api_request_failed(detail: impl Into<String>) -> Error {
    Error::ApiRequestFailed(detail.into())
}

pub fn api_error(code: i32, msg: impl Into<String>) -> Error {
    Error::ApiResponseError(code, msg.into())
}

pub fn file_validation_failed(detail: impl Into<String>) -> Error {
    Error::FileValidationFailed(detail.into())
}

pub fn cos_upload_failed(detail: impl Into<String>) -> Error {
    Error::CosUploadFailed(detail.into())
}

pub fn update_available(msg: impl Into<String>) -> Error {
    Error::UpdateAvailable(msg.into())
}

pub fn invalid_argument(detail: impl Into<String>) -> Error {
    Error::InvalidArgument(detail.into())
}

pub fn not_found(detail: impl Into<String>) -> Error {
    Error::NotFound(detail.into())
}

pub fn permission_denied(detail: impl Into<String>) -> Error {
    Error::PermissionDenied(detail.into())
}

pub fn rate_limited(detail: impl Into<String>) -> Error {
    Error::RateLimited(detail.into())
}

pub fn security_violation(detail: impl Into<String>) -> Error {
    Error::SecurityViolation(detail.into())
}

pub fn network_error(e: reqwest::Error) -> Error {
    Error::NetworkError(e)
}

pub fn io_error(e: std::io::Error) -> Error {
    Error::IoError(e)
}

pub fn json_error(e: serde_json::Error) -> Error {
    Error::JsonError(e)
}
