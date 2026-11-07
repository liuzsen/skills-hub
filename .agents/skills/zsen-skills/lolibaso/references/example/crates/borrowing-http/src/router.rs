use axum::{Router, routing::post};

use crate::{devices, error::ApiError};

pub fn router<F>(factory: F) -> Router
where
    F: devices::register::Factory + devices::borrow::Factory,
{
    Router::new()
        .route("/devices", post(devices::register::handle::<F>))
        .route(
            "/devices/{device_id}/borrow",
            post(devices::borrow::handle::<F>),
        )
        // 省略查询、归还、注销路由及对应 Factory 约束。
        .fallback(|| async { ApiError::RouteNotFound })
        .method_not_allowed_fallback(|| async { ApiError::MethodNotAllowed })
        .with_state(factory)
}
