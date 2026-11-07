use axum::{
    Json,
    extract::rejection::{BytesRejection, JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use lolibaso::http::HttpErrorCode;

use super::response::ErrorResponse;

#[derive(HttpErrorCode)]
#[repr(u16)]
pub(super) enum ApiError {
    #[http(code = "invalid_request", status = 400)]
    InvalidRequest,
    #[http(code = "invalid_device_id", status = 400)]
    InvalidDeviceId,
    #[http(code = "invalid_dev_user", status = 401)]
    InvalidDevUser,
    #[http(code = "device_not_found", status = 404)]
    DeviceNotFound,
    #[http(code = "route_not_found", status = 404)]
    RouteNotFound,
    #[http(code = "method_not_allowed", status = 405)]
    MethodNotAllowed,
    #[http(code = "number_taken", status = 409)]
    NumberTaken,
    #[http(code = "already_borrowed", status = 409)]
    AlreadyBorrowed,
    #[http(code = "device_deregistered", status = 409)]
    DeviceDeregistered,
    #[http(code = "device_under_maintenance", status = 409)]
    DeviceUnderMaintenance,
    // 省略仅供归还和注销 handler 使用的错误码。
    #[http(code = "concurrent_change", status = 409)]
    ConcurrentChange,
    #[http(code = "payload_too_large", status = 413)]
    PayloadTooLarge,
    #[http(code = "unsupported_media_type", status = 415)]
    UnsupportedMediaType,
    #[http(code = "invalid_number", status = 422)]
    InvalidNumber,
    #[http(code = "internal_error", status = 500)]
    InternalError,
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        tracing::error!(error = %format_args!("{error:#}"), "HTTP request failed");
        Self::InternalError
    }
}

impl From<JsonRejection> for ApiError {
    fn from(error: JsonRejection) -> Self {
        match error.status() {
            StatusCode::UNSUPPORTED_MEDIA_TYPE => Self::UnsupportedMediaType,
            StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge,
            _ => Self::InvalidRequest,
        }
    }
}

impl From<BytesRejection> for ApiError {
    fn from(error: BytesRejection) -> Self {
        match error.status() {
            StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge,
            _ => Self::InvalidRequest,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // HttpErrorCode validates 100..=999, the same range accepted by StatusCode.
        let status = StatusCode::from_u16(self.http_status())
            .expect("HTTP error catalog status must be valid");
        (status, Json(ErrorResponse::new(self.code()))).into_response()
    }
}
