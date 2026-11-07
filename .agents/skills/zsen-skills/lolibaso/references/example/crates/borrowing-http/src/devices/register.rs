use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use borrowing_application::register;
use lolibaso::biz::UseCase;
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, identity::DevActor, response::Success};

/// Creates an owned registration use case; only the factory is shared.
pub trait Factory: Clone + Send + Sync + 'static {
    type Case: UseCase<Input = register::Input, Output = register::Output, Error = register::Error>
        + 'static;

    fn create(&self) -> impl Future<Output = anyhow::Result<Self::Case>> + Send;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterRequest {
    number: String,
}

#[derive(Serialize)]
struct RegisteredDevice {
    id: String,
}

pub(crate) async fn handle<F: Factory>(
    State(state): State<F>,
    DevActor(_actor_id): DevActor,
    body: Result<Json<RegisterRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body?;
    let input = register::Input {
        number: body.number,
    };
    let case = state.create().await?;
    let result = case.execute(input).await?;
    match result {
        Ok(id) => {
            // 保留上游资源身份响应；本摘录省略该 Location 对应的 GET handler。
            let location = format!("/devices/{}", id.0);
            Ok((
                StatusCode::CREATED,
                [(header::LOCATION, location)],
                Json(Success {
                    data: RegisteredDevice {
                        id: id.0.to_string(),
                    },
                }),
            )
                .into_response())
        }
        Err(register::Error::InvalidNumber) => Err(ApiError::InvalidNumber),
        Err(register::Error::NumberTaken) => Err(ApiError::NumberTaken),
    }
}
