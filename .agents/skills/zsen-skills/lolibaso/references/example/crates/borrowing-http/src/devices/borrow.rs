use axum::{
    Json,
    body::Bytes,
    extract::{
        Path, State,
        rejection::{BytesRejection, PathRejection},
    },
};
use borrowing_application::borrow_device;
use borrowing_domain::device::BorrowError;
use lolibaso::biz::UseCase;

use super::{parse_device_id, require_empty_body};
use crate::{error::ApiError, identity::DevActor, response::Success};

/// Creates an owned borrowing use case; only the factory is shared.
pub trait Factory: Clone + Send + Sync + 'static {
    type Case: UseCase<
            Input = borrow_device::Input,
            Output = borrow_device::Output,
            Error = borrow_device::Error,
        > + 'static;

    fn create(&self) -> impl Future<Output = anyhow::Result<Self::Case>> + Send;
}

pub(crate) async fn handle<F: Factory>(
    State(state): State<F>,
    DevActor(actor_id): DevActor,
    path: Result<Path<String>, PathRejection>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<Success<()>>, ApiError> {
    let device_id = parse_device_id(path)?;
    require_empty_body(body)?;
    let input = borrow_device::Input {
        device_id,
        actor_id,
    };
    let case = state.create().await?;
    case.execute(input).await??;
    Ok(Json(Success { data: () }))
}

impl From<borrow_device::Error> for ApiError {
    fn from(error: borrow_device::Error) -> Self {
        match error {
            borrow_device::Error::NotFound => Self::DeviceNotFound,
            borrow_device::Error::Rejected(BorrowError::AlreadyBorrowed) => Self::AlreadyBorrowed,
            ...
        }
    }
}
