use serde::Serialize;
use worker::{Response, Result};

pub fn finish(mut response: Response) -> Result<Response> {
    response.headers_mut().set("Cache-Control", "no-store")?;
    Ok(response)
}

pub fn json<T: Serialize>(value: &T) -> Result<Response> {
    finish(Response::from_json(value)?)
}

pub fn error(status: u16, code: &'static str) -> Result<Response> {
    finish(Response::from_json(&serde_json::json!({ "error": code }))?.with_status(status))
}

pub fn method_not_allowed(allow: &str) -> Result<Response> {
    let mut response = error(405, "method_not_allowed")?;
    response.headers_mut().set("Allow", allow)?;
    Ok(response)
}

pub fn internal(error: anyhow::Error) -> Result<Response> {
    // Adapters must strip SDK diagnostics containing SQL/bound values before this boundary.
    worker::console_error!("backend request failed: {error:#}");
    self::error(500, "internal_error")
}
