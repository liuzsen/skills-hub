pub mod borrow;
pub mod register;
// 省略查询、归还、注销 handler 及其模块声明；共享解析函数供借用使用。

use axum::{
    body::Bytes,
    extract::{
        Path,
        rejection::{BytesRejection, PathRejection},
    },
};
use borrowing_domain::device::DeviceId;
use uuid::Uuid;

use super::error::ApiError;

fn parse_device_id(path: Result<Path<String>, PathRejection>) -> Result<DeviceId, ApiError> {
    let Path(id) = path.map_err(|_| ApiError::InvalidDeviceId)?;
    Uuid::parse_str(&id)
        .map(DeviceId)
        .map_err(|_| ApiError::InvalidDeviceId)
}

fn require_empty_body(body: Result<Bytes, BytesRejection>) -> Result<(), ApiError> {
    if !body?.is_empty() {
        return Err(ApiError::InvalidRequest);
    }
    Ok(())
}
