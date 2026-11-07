use crate::{auth::session_token, body, policy::HttpPolicy, response};
use hub_application::{
    administration::{
        create_codes::{self, CodeIssuer, CreateCodes},
        grant_capabilities::{self, Capability, CapabilityGranter, GrantCapabilities},
    },
    auth::authenticate_session::{AuthenticateSession, SessionReader},
    time::Clock,
    tokens::TokenSource,
};
use hub_domain::account::{AccountId, Actor};
use lolibaso::biz::UseCase;
use serde::{Deserialize, Serialize};
use worker::{Method, Request, Response, Result};

pub struct AdministrationDependencies<S, A, C, T, AC> {
    pub store: S,
    pub authenticate: AuthenticateSession<A, AC>,
    pub clock: C,
    pub tokens: T,
}

pub fn is_route(path: &str) -> bool {
    matches!(
        path,
        "/api/admin/redemption-codes" | "/api/admin/capabilities"
    )
}

pub async fn route<S, A, C, T, AC>(
    request: &mut Request,
    deps: AdministrationDependencies<S, A, C, T, AC>,
    policy: &HttpPolicy,
) -> Result<Option<Response>>
where
    S: CodeIssuer + CapabilityGranter,
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
    let account = match deps.authenticate.execute(session_token(request)?).await {
        Ok(Ok(account)) => account,
        Ok(Err(_)) => return Ok(Some(response::error(401, "unauthenticated")?)),
        Err(error) => return Ok(Some(response::internal(error)?)),
    };
    let actor = Actor::Account(account);
    let response = if path == "/api/admin/redemption-codes" {
        create(request, deps.store, deps.clock, deps.tokens, actor).await?
    } else {
        grant(request, deps.store, actor).await?
    };
    Ok(Some(response))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateBody {
    skill_id: String,
    count: u32,
}

#[derive(Serialize)]
struct CodeDto {
    id: String,
    code: String,
}

async fn create<S: CodeIssuer, C: Clock, T: TokenSource>(
    request: &mut Request,
    store: S,
    clock: C,
    tokens: T,
    actor: Actor,
) -> Result<Response> {
    let input = match body::json::<CreateBody>(request, 4096).await? {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let result = (CreateCodes {
        store,
        clock,
        tokens,
    })
    .execute(create_codes::Input {
        actor,
        skill_id: input.skill_id,
        count: input.count,
    })
    .await;
    match result {
        Ok(Ok(output)) => {
            let codes: Vec<CodeDto> = output
                .codes
                .into_iter()
                .map(|code| CodeDto {
                    id: code.id,
                    code: code.code,
                })
                .collect();
            Ok(
                response::json(
                    &serde_json::json!({ "skill_id": output.skill_id, "codes": codes }),
                )?
                .with_status(201),
            )
        }
        Ok(Err(create_codes::Error::Unauthenticated)) => response::error(401, "unauthenticated"),
        Ok(Err(create_codes::Error::Forbidden)) => response::error(403, "admin_required"),
        Ok(Err(create_codes::Error::InvalidCount)) => response::error(400, "invalid_count"),
        Ok(Err(create_codes::Error::InvalidSkillId)) => response::error(400, "invalid_skill_id"),
        Ok(Err(create_codes::Error::SkillNotFound)) => response::error(404, "skill_not_found"),
        Err(error) => response::internal(error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CapabilityDto {
    ViewAll,
    DownloadAll,
    UploadEdit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantBody {
    account_id: String,
    capability: CapabilityDto,
}

async fn grant<S: CapabilityGranter>(
    request: &mut Request,
    store: S,
    actor: Actor,
) -> Result<Response> {
    let input = match body::json::<GrantBody>(request, 4096).await? {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let target = match AccountId::parse(input.account_id) {
        Ok(target) => target,
        Err(_) => return response::error(400, "invalid_account_id"),
    };
    let capability = match input.capability {
        CapabilityDto::ViewAll => Capability::ViewAll,
        CapabilityDto::DownloadAll => Capability::DownloadAll,
        CapabilityDto::UploadEdit => Capability::UploadEdit,
    };
    match (GrantCapabilities { store })
        .execute(grant_capabilities::Input {
            actor,
            target,
            capability,
        })
        .await
    {
        Ok(Ok(())) => response::json(&serde_json::json!({ "ok": true })),
        Ok(Err(grant_capabilities::Error::Unauthenticated)) => {
            response::error(401, "unauthenticated")
        }
        Ok(Err(grant_capabilities::Error::Forbidden)) => response::error(403, "admin_required"),
        Ok(Err(grant_capabilities::Error::AccountNotFound)) => {
            response::error(404, "account_not_found")
        }
        Err(error) => response::internal(error),
    }
}
