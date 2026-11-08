use crate::{body, policy::HttpPolicy, response};
use cookie::{Cookie, SameSite, time::Duration};
use hub_application::{
    auth::{
        ChallengePepper, SESSION_TTL,
        authenticate_session::{AuthenticateSession, SessionReader},
        issue_challenge::{self, ChallengeIssuer, IssueChallenge},
        logout::{Logout, SessionRevoker},
        verify_email::{self, ChallengeConsumer, VerifyEmail},
    },
    time::Clock,
    tokens::{self, TokenSource},
};
use hub_domain::account::EmailAddress;
use lolibaso::biz::UseCase;
use serde::{Deserialize, Serialize};
use worker::{Method, Request, Response, Result};

const SESSION_COOKIE: &str = "hub_session";

pub struct AuthDependencies<S, C, T> {
    pub store: S,
    pub clock: C,
    pub tokens: T,
    pub pepper: Option<ChallengePepper>,
    pub mail_enabled: bool,
}

pub fn is_route(path: &str) -> bool {
    matches!(
        path,
        "/api/auth/email/challenges" | "/api/auth/email/verify" | "/api/me" | "/api/auth/logout"
    )
}

pub async fn route<S, C, T>(
    request: &mut Request,
    deps: AuthDependencies<S, C, T>,
    policy: &HttpPolicy,
) -> Result<Option<Response>>
where
    S: ChallengeIssuer + ChallengeConsumer + SessionReader + SessionRevoker,
    C: Clock,
    T: TokenSource,
{
    let path = request.path();
    if !is_route(&path) {
        return Ok(None);
    }
    let method = if path == "/api/me" {
        Method::Get
    } else {
        Method::Post
    };
    if request.method() != method {
        return Ok(Some(response::method_not_allowed(
            if method == Method::Get { "GET" } else { "POST" },
        )?));
    }
    if method == Method::Post && !policy.check_origin(request)? {
        return Ok(Some(response::error(403, "origin_forbidden")?));
    }
    let response = match path.as_str() {
        "/api/auth/email/challenges" => issue(request, deps).await?,
        "/api/auth/email/verify" => verify(request, deps, policy).await?,
        "/api/me" => me(request, deps).await?,
        "/api/auth/logout" => logout(request, deps, policy).await?,
        _ => unreachable!(),
    };
    Ok(Some(response))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IssueBody {
    email: String,
}

#[derive(Serialize)]
struct ChallengeDto {
    challenge_id: String,
    expires_at: i64,
    resend_after: i64,
}

async fn issue<S: ChallengeIssuer, C: Clock, T: TokenSource>(
    request: &mut Request,
    deps: AuthDependencies<S, C, T>,
) -> Result<Response> {
    if !deps.mail_enabled {
        return response::error(503, "mail_unavailable");
    }
    let Some(pepper) = deps.pepper else {
        return response::error(503, "mail_unavailable");
    };
    let input = match body::json::<IssueBody>(request, 4096).await? {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let email = match EmailAddress::parse(&input.email) {
        Ok(email) => email,
        Err(_) => return response::error(400, "invalid_email"),
    };
    match (IssueChallenge {
        store: deps.store,
        clock: deps.clock,
        tokens: deps.tokens,
        pepper,
    })
    .execute(email)
    .await
    {
        Ok(Ok(output)) => Ok(response::json(&ChallengeDto {
            challenge_id: output.challenge_id,
            expires_at: output.expires_at,
            resend_after: output.resend_after,
        })?
        .with_status(202)),
        Ok(Err(issue_challenge::Error::ResendCooldown)) => {
            let mut response = response::error(429, "resend_cooldown")?;
            response.headers_mut().set("Retry-After", "60")?;
            Ok(response)
        }
        Ok(Err(issue_challenge::Error::DeliveryUnavailable)) => {
            response::error(503, "mail_unavailable")
        }
        Err(error) => response::internal(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyBody {
    email: String,
    challenge_id: String,
    code: String,
}

async fn verify<S: ChallengeConsumer, C: Clock, T: TokenSource>(
    request: &mut Request,
    deps: AuthDependencies<S, C, T>,
    policy: &HttpPolicy,
) -> Result<Response> {
    let Some(pepper) = deps.pepper else {
        return response::error(503, "auth_unavailable");
    };
    let input = match body::json::<VerifyBody>(request, 4096).await? {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let email = match EmailAddress::parse(&input.email) {
        Ok(email) => email,
        Err(_) => return response::error(400, "invalid_email"),
    };
    let case = VerifyEmail {
        store: deps.store,
        clock: deps.clock,
        tokens: deps.tokens,
        pepper,
    };
    match case
        .execute(verify_email::Input {
            email,
            challenge_id: input.challenge_id,
            code: input.code,
        })
        .await
    {
        Ok(Ok(output)) => {
            let mut response =
                response::json(&serde_json::json!({ "expires_at": output.expires_at }))?;
            set_cookie(&mut response, output.session_token, SESSION_TTL, policy)?;
            Ok(response)
        }
        Ok(Err(verify_email::Error::ChallengeRejected)) => {
            response::error(401, "challenge_rejected")
        }
        Err(error) => response::internal(error),
    }
}

#[derive(Serialize)]
struct AccountDto<'a> {
    id: &'a str,
    email: &'a str,
    is_admin: bool,
    capabilities: CapabilitiesDto,
}
#[derive(Serialize)]
struct CapabilitiesDto {
    view_all: bool,
    download_all: bool,
    upload_edit: bool,
}

async fn me<S: SessionReader, C: Clock, T>(
    request: &Request,
    deps: AuthDependencies<S, C, T>,
) -> Result<Response> {
    let token = session_token(request)?;
    match (AuthenticateSession {
        store: deps.store,
        clock: deps.clock,
    })
    .execute(token)
    .await
    {
        Ok(Ok(account)) => {
            let caps = account.capabilities();
            response::json(&AccountDto {
                id: account.id().as_str(),
                email: account.email().as_str(),
                is_admin: account.is_admin(),
                capabilities: CapabilitiesDto {
                    view_all: caps.view_all,
                    download_all: caps.download_all,
                    upload_edit: caps.upload_edit,
                },
            })
        }
        Ok(Err(_)) => response::error(401, "unauthenticated"),
        Err(error) => response::internal(error),
    }
}

async fn logout<S: SessionRevoker, C: Clock, T>(
    request: &Request,
    deps: AuthDependencies<S, C, T>,
    policy: &HttpPolicy,
) -> Result<Response> {
    match (Logout {
        store: deps.store,
        clock: deps.clock,
    })
    .execute(session_token(request)?)
    .await
    {
        Ok(Ok(())) => {
            let mut response = response::json(&serde_json::json!({ "ok": true }))?;
            set_cookie(&mut response, String::new(), 0, policy)?;
            Ok(response)
        }
        Ok(Err(never)) => match never {},
        Err(error) => response::internal(error),
    }
}

/// Duplicate or malformed cookies are never accepted as an identity.
pub fn session_token(request: &Request) -> Result<Option<String>> {
    let Some(header) = request.headers().get("Cookie")? else {
        return Ok(None);
    };
    let mut token = None;
    for part in header.split(';') {
        let Ok(cookie) = Cookie::parse(part.trim()) else {
            return Ok(None);
        };
        if cookie.name() == SESSION_COOKIE {
            if token.is_some() || !tokens::is_opaque(cookie.value()) {
                return Ok(None);
            }
            token = Some(cookie.value().to_owned());
        }
    }
    Ok(token)
}

fn set_cookie(
    response: &mut Response,
    token: String,
    max_age: i64,
    policy: &HttpPolicy,
) -> Result<()> {
    let cookie = Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .secure(policy.secure_cookie())
        .max_age(Duration::seconds(max_age))
        .build();
    response
        .headers_mut()
        .append("Set-Cookie", &cookie.to_string())
}
