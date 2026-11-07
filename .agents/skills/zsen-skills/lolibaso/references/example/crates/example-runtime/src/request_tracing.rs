use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{HeaderName, HeaderValue, Request, Response},
    middleware::{self, Next},
};
use tower_http::{
    request_id::{PropagateRequestIdLayer, RequestId},
    trace::TraceLayer,
};
use tracing::Span;

async fn assign_request_id(mut request: Request<Body>, next: Next) -> Response<Body> {
    let id = HeaderValue::from_str(&uuid::Uuid::new_v4().to_string())
        .expect("UUID is a valid header value");
    request.headers_mut().insert("x-request-id", id.clone());
    request.extensions_mut().insert(RequestId::new(id));
    next.run(request).await
}

pub fn with_request_tracing(router: Router) -> Router {
    let header = HeaderName::from_static("x-request-id");
    router
        .layer(PropagateRequestIdLayer::new(header.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<Body>| {
                    let request_id = request
                        .extensions()
                        .get::<RequestId>()
                        .and_then(|id| id.header_value().to_str().ok())
                        .unwrap_or("unknown");
                    // Keep request context available to errors even with RUST_LOG=warn.
                    // A span establishes context; it does not emit an error event.
                    tracing::error_span!(
                        "http_request",
                        request_id,
                        method = %request.method(),
                        path = request.uri().path(),
                    )
                })
                .on_request(())
                .on_response(
                    |response: &Response<Body>, latency: Duration, span: &Span| {
                        tracing::info!(
                            parent: span,
                            status = response.status().as_u16(),
                            response_latency_ms = latency.as_secs_f64() * 1000.0,
                            "HTTP request completed"
                        );
                    },
                )
                // The HTTP error boundary already logs technical failure causes.
                .on_failure(()),
        )
        // Router layers run outside-in: replace untrusted IDs before tracing.
        .layer(middleware::from_fn(assign_request_id))
}
