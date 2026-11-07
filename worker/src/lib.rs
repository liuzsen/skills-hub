mod downloads;
mod runtime;

use hub_application::{
    auth::{ChallengePepper, authenticate_session::AuthenticateSession},
    redemption::access::CheckSkillAccess,
};
use hub_d1::{
    AuthStore, administration::AdministrationStore, redemption::RedemptionStore,
    skills::SkillsStore,
};
use hub_http::{
    administration::{self, AdministrationDependencies},
    auth::{self, AuthDependencies},
    policy::HttpPolicy,
    redemption::{self, RedemptionDependencies},
    response,
    skills::{self, SkillsDependencies},
};
use hub_r2::R2ArtifactStore;
use serde::Serialize;
use worker::{Context, D1Database, Env, Method, Request, Response, Result, event};

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
    runtime: &'static str,
}

#[event(fetch)]
pub async fn main(mut request: Request, env: Env, _context: Context) -> Result<Response> {
    match dispatch(&mut request, &env).await {
        Ok(response) => response::finish(response),
        Err(_) => {
            worker::console_error!("backend HTTP adapter failed (SDK details redacted)");
            response::error(500, "internal_error")
        }
    }
}

async fn dispatch(request: &mut Request, env: &Env) -> Result<Response> {
    let path = request.path();
    if path == "/api/health" {
        return if request.method() == Method::Get {
            response::json(&Health {
                status: "ok",
                service: "skills-worker",
                runtime: "cloudflare-workers",
            })
        } else {
            response::method_not_allowed("GET")
        };
    }
    if !(auth::is_route(&path)
        || redemption::is_route(&path)
        || administration::is_route(&path)
        || skills::is_route(&path))
    {
        return response::error(404, "not_found");
    }
    // Unconfigured environments cannot serve business data, even with bindings present.
    let app_env = variable(env, "APP_ENV");
    if !matches!(app_env.as_str(), "local" | "production") {
        return response::error(
            503,
            if auth::is_route(&path) {
                "auth_unavailable"
            } else {
                "service_unavailable"
            },
        );
    }
    let local = cfg!(feature = "local-mail") && app_env == "local";
    // The local config must not enable insecure cookies without its compile-time gate.
    if app_env == "local" && !local {
        return response::error(503, "auth_unavailable");
    }
    let policy = match HttpPolicy::new(variable(env, "APP_ORIGIN"), local) {
        Ok(policy) => policy,
        Err(_) => return response::error(503, "auth_unavailable"),
    };
    let db = match env.d1("DB") {
        Ok(db) => db,
        Err(_) => return response::internal(anyhow::anyhow!("DB binding unavailable")),
    };
    // Only the matched adapter's dependencies are constructed. Auth/redemption/admin
    // do not require R2; only auth reads the challenge pepper.
    let result = if auth::is_route(&path) {
        auth_request(request, env, db, &policy, local).await?
    } else if redemption::is_route(&path) {
        redemption::route(
            request,
            RedemptionDependencies {
                store: RedemptionStore::new(env.d1("DB")?),
                authenticate: AuthenticateSession {
                    store: AuthStore::new(db),
                    clock: runtime::SystemClock,
                },
                clock: runtime::SystemClock,
                tokens: runtime::CryptoTokens,
            },
            &policy,
        )
        .await?
    } else if administration::is_route(&path) {
        administration::route(
            request,
            AdministrationDependencies {
                store: AdministrationStore::new(env.d1("DB")?),
                authenticate: AuthenticateSession {
                    store: AuthStore::new(db),
                    clock: runtime::SystemClock,
                },
                clock: runtime::SystemClock,
                tokens: runtime::CryptoTokens,
            },
            &policy,
        )
        .await?
    } else {
        skills_request(request, env, db, &policy).await?
    };
    match result {
        Some(result) => Ok(result),
        None => response::error(404, "not_found"),
    }
}

async fn auth_request(
    request: &mut Request,
    env: &Env,
    db: D1Database,
    policy: &HttpPolicy,
    local: bool,
) -> Result<Option<Response>> {
    let pepper = match env.secret("AUTH_PEPPER") {
        Ok(value) => match ChallengePepper::from_hex(&value.to_string()) {
            Ok(pepper) => Some(pepper),
            Err(_) => return Ok(Some(response::error(503, "auth_unavailable")?)),
        },
        Err(_) => None,
    };
    let deps = AuthDependencies {
        store: AuthStore::new(db),
        clock: runtime::SystemClock,
        tokens: runtime::CryptoTokens,
        pepper,
        mail_enabled: local && variable(env, "MAIL_TRANSPORT") == "d1-outbox",
    };
    auth::route(request, deps, policy).await
}

async fn skills_request(
    request: &mut Request,
    env: &Env,
    db: D1Database,
    policy: &HttpPolicy,
) -> Result<Option<Response>> {
    let bucket = match env.bucket("SKILLS") {
        Ok(bucket) => bucket,
        Err(_) => {
            return Ok(Some(response::internal(anyhow::anyhow!(
                "SKILLS binding unavailable"
            ))?));
        }
    };
    let deps = SkillsDependencies {
        catalog: SkillsStore::new(env.d1("DB")?),
        access: CheckSkillAccess {
            store: RedemptionStore::new(env.d1("DB")?),
            clock: runtime::SystemClock,
        },
        artifacts: R2ArtifactStore::new(bucket.clone()),
        sessions: AuthStore::new(db),
        clock: runtime::SystemClock,
        tokens: runtime::CryptoTokens,
        downloads: downloads::R2Downloads(R2ArtifactStore::new(bucket)),
    };
    skills::route(request, deps, policy).await
}

fn variable(env: &Env, name: &str) -> String {
    env.var(name)
        .map(|value| value.to_string())
        .unwrap_or_default()
}
