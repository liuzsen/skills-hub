use crate::{auth::session_token, body, policy::HttpPolicy, response};
use hub_application::{
    auth::authenticate_session::{AuthenticateSession, SessionReader},
    redemption::{
        access,
        bind_code::{self, BindCode, CodeBindingStore},
        claim_guest::{self, ClaimGuest, GuestClaimStore},
    },
    time::Clock,
    tokens::TokenSource,
};
use hub_domain::{account::Actor, redemption::GuestTokenHash};
use lolibaso::biz::UseCase;
use serde::Deserialize;
use worker::{Method, Request, Response, Result};

pub struct RedemptionDependencies<S, A, C, T, AC> {
    pub store: S,
    pub authenticate: AuthenticateSession<A, AC>,
    pub clock: C,
    pub tokens: T,
}

pub fn is_route(path: &str) -> bool {
    matches!(path, "/api/redemptions/guest" | "/api/redemptions/bind")
}

/// Reads a dedicated bearer header, never a cookie or query parameter.
/// Invalid syntax is a caller-visible 400; absent header is a normal anonymous request.
pub fn guest_token_hash(
    request: &Request,
) -> Result<std::result::Result<Option<GuestTokenHash>, ()>> {
    Ok(match request.headers().get("X-Guest-Token")? {
        None => Ok(None),
        Some(raw) => access::guest_token_hash(&raw).map(Some).ok_or(()),
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodeBody {
    code: String,
}

pub async fn route<S, A, C, T, AC>(
    request: &mut Request,
    deps: RedemptionDependencies<S, A, C, T, AC>,
    policy: &HttpPolicy,
) -> Result<Option<Response>>
where
    S: GuestClaimStore + CodeBindingStore,
    A: SessionReader,
    C: Clock,
    T: TokenSource,
    AC: Clock,
{
    let path = request.path();
    if !is_route(&path) {
        return Ok(None);
    }
    if request.method() != Method::Post {
        return Ok(Some(response::method_not_allowed("POST")?));
    }
    if !policy.check_origin(request)? {
        return Ok(Some(response::error(403, "origin_forbidden")?));
    }
    if path == "/api/redemptions/guest" {
        let input = match body::json::<CodeBody>(request, 4096).await? {
            Ok(value) => value,
            Err(response) => return Ok(Some(response)),
        };
        let result = (ClaimGuest {
            store: deps.store,
            clock: deps.clock,
            tokens: deps.tokens,
        })
        .execute(input.code)
        .await;
        return Ok(Some(match result {
            Ok(Ok(output)) => response::json(&serde_json::json!({ "token": output.token, "skill_id": output.skill_id, "expires_at": output.expires_at }))?.with_status(201),
            Ok(Err(claim_guest::Error::InvalidCode)) => response::error(400, "invalid_code")?,
            Ok(Err(claim_guest::Error::Unavailable)) => response::error(409, "code_unavailable")?,
            Err(error) => response::internal(error)?,
        }));
    }
    let account = match deps.authenticate.execute(session_token(request)?).await {
        Ok(Ok(account)) => account,
        Ok(Err(_)) => return Ok(Some(response::error(401, "unauthenticated")?)),
        Err(error) => return Ok(Some(response::internal(error)?)),
    };
    let input = match body::json::<CodeBody>(request, 4096).await? {
        Ok(value) => value,
        Err(response) => return Ok(Some(response)),
    };
    let result = (BindCode { store: deps.store })
        .execute(bind_code::Input {
            actor: Actor::Account(account),
            code: input.code,
        })
        .await;
    Ok(Some(match result {
        Ok(Ok(output)) => {
            response::json(&serde_json::json!({ "skill_id": output.skill_id, "bound": true }))?
        }
        Ok(Err(bind_code::Error::Unauthenticated)) => response::error(401, "unauthenticated")?,
        Ok(Err(bind_code::Error::InvalidCode)) => response::error(400, "invalid_code")?,
        Ok(Err(bind_code::Error::Unavailable)) => response::error(409, "code_unavailable")?,
        Err(error) => response::internal(error)?,
    }))
}
