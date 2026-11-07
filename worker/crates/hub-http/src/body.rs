use crate::response;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use worker::{Request, Response, Result};

/// Enforces a streaming cap, including requests without Content-Length.
pub async fn json<T: DeserializeOwned>(
    request: &mut Request,
    limit: usize,
) -> Result<std::result::Result<T, Response>> {
    let content_type = request.headers().get("Content-Type")?.unwrap_or_default();
    if content_type.split(';').next().unwrap_or_default().trim() != "application/json" {
        return Ok(Err(response::error(415, "unsupported_media_type")?));
    }
    let mut bytes = Vec::new();
    let mut stream = match request.stream() {
        Ok(stream) => stream,
        Err(_) => return Ok(Err(response::error(400, "invalid_request")?)),
    };
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return Ok(Err(response::error(400, "invalid_request")?)),
        };
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Ok(Err(response::error(413, "payload_too_large")?));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(match serde_json::from_slice(&bytes) {
        Ok(value) => Ok(value),
        Err(_) => Err(response::error(400, "invalid_request")?),
    })
}
