use axum::{extract::FromRequestParts, http::request::Parts};
use borrowing_domain::device::ActorId;
use uuid::Uuid;

use super::error::ApiError;

/// Local demonstration only. Anyone can impersonate these users through the header.
pub(super) struct DevActor(pub ActorId);

impl<S: Send + Sync> FromRequestParts<S> for DevActor {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut users = parts.headers.get_all("x-dev-user").iter();
        let user = users.next().and_then(|value| value.to_str().ok());
        if users.next().is_some() {
            return Err(ApiError::InvalidDevUser);
        }
        let actor = match user {
            Some("alice") => ActorId(Uuid::from_u128(1)),
            Some("bob") => ActorId(Uuid::from_u128(2)),
            _ => return Err(ApiError::InvalidDevUser),
        };
        Ok(Self(actor))
    }
}
