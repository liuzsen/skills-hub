use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Success<T> {
    pub data: T,
}

#[derive(Serialize)]
pub(super) struct ErrorResponse {
    error: &'static str,
}

impl ErrorResponse {
    pub(super) fn new(code: &'static str) -> Self {
        Self { error: code }
    }
}
