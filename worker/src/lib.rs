use serde::Serialize;
use worker::{Context, Env, Method, Request, Response, Result, event};

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
    runtime: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
    NotFound,
    MethodNotAllowed,
}

#[derive(Serialize)]
struct ApiError {
    error: ErrorCode,
}

#[event(fetch)]
pub async fn main(request: Request, _env: Env, _context: Context) -> Result<Response> {
    let mut response = match (request.path().as_str(), request.method()) {
        ("/api/health", Method::Get) => Response::from_json(&Health {
            status: "ok",
            service: "skills-worker",
            runtime: "cloudflare-workers",
        })?,
        ("/api/health", _) => {
            let mut response = Response::from_json(&ApiError {
                error: ErrorCode::MethodNotAllowed,
            })?
            .with_status(405);
            response.headers_mut().set("Allow", "GET")?;
            response
        }
        _ => Response::from_json(&ApiError {
            error: ErrorCode::NotFound,
        })?
        .with_status(404),
    };
    response.headers_mut().set("Cache-Control", "no-store")?;
    Ok(response)
}
