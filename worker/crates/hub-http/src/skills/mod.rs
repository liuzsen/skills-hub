mod dto;

use crate::{auth, body, policy::HttpPolicy, redemption, response};
use dto::{DetailDto, ListDto, PublishBody, PublishedDto, TextDto};
use hub_application::{
    auth::authenticate_session::{AuthenticateSession, SessionReader},
    redemption::access::SkillAccess,
    skills::{
        ContentRequest, Error, SkillCatalog,
        artifacts::{ArtifactDescriptor, ArtifactStore},
        download_latest::DownloadLatest,
        list_skills::{self, ListSkills},
        publish_skill::{PublicationStore, PublishSkill},
        read_detail::ReadDetail,
        read_text::{self, ReadText},
    },
    time::Clock,
    tokens::TokenSource,
};
use hub_domain::{
    account::Actor,
    skill::{SkillId, SkillPath},
};
use lolibaso::biz::UseCase;
use worker::{Method, Request, Response, ResponseBody, Result};

const MAX_UPLOAD_BODY: usize = 12 * 1024 * 1024;

/// Implement in the composition root using a local wrapper around the R2 adapter.
/// HTTP never depends on hub-r2, and the application never sees ResponseBody.
pub trait DownloadTransport {
    fn open_body(
        &self,
        artifact: &ArtifactDescriptor,
    ) -> impl Future<Output = anyhow::Result<ResponseBody>>;
}

pub struct SkillsDependencies<S, A, R, I, C, T, D> {
    pub catalog: S,
    pub access: A,
    pub artifacts: R,
    pub sessions: I,
    pub clock: C,
    pub tokens: T,
    pub downloads: D,
}

enum Resource {
    Catalog,
    Detail(SkillId),
    Text(SkillId),
    Download(SkillId),
}

pub fn is_route(path: &str) -> bool {
    path == "/api/skills" || path.starts_with("/api/skills/")
}

fn resource(path: &str) -> std::result::Result<Resource, ()> {
    if path == "/api/skills" {
        return Ok(Resource::Catalog);
    }
    let parts: Vec<_> = path
        .strip_prefix("/api/skills/")
        .ok_or(())?
        .split('/')
        .collect();
    let id = SkillId::parse(*parts.first().ok_or(())?).map_err(|_| ())?;
    match parts.as_slice() {
        [_] => Ok(Resource::Detail(id)),
        [_, "text"] => Ok(Resource::Text(id)),
        [_, "download"] => Ok(Resource::Download(id)),
        _ => Err(()),
    }
}

struct ClockRef<'a, C>(&'a C);
impl<C: Clock> Clock for ClockRef<'_, C> {
    fn now(&self) -> i64 {
        self.0.now()
    }
}

pub async fn route<S, A, R, I, C, T, D>(
    request: &mut Request,
    deps: SkillsDependencies<S, A, R, I, C, T, D>,
    policy: &HttpPolicy,
) -> Result<Option<Response>>
where
    S: SkillCatalog + PublicationStore,
    A: SkillAccess,
    R: ArtifactStore,
    I: SessionReader,
    C: Clock,
    T: TokenSource,
    D: DownloadTransport,
{
    if !is_route(&request.path()) {
        return Ok(None);
    }
    let resource = match resource(&request.path()) {
        Ok(resource) => resource,
        Err(()) => return Ok(Some(response::error(404, "not_found")?)),
    };
    let method = request.method();
    let allowed = match &resource {
        Resource::Catalog => matches!(method, Method::Get | Method::Post),
        Resource::Detail(_) => matches!(method, Method::Get | Method::Put),
        Resource::Text(_) | Resource::Download(_) => method == Method::Get,
    };
    if !allowed {
        let allow = match &resource {
            Resource::Catalog => "GET, POST",
            Resource::Detail(_) => "GET, PUT",
            _ => "GET",
        };
        return Ok(Some(response::method_not_allowed(allow)?));
    }
    if matches!(method, Method::Post | Method::Put) && !policy.check_origin(request)? {
        return Ok(Some(response::error(403, "origin_forbidden")?));
    }
    let SkillsDependencies {
        catalog,
        access,
        artifacts,
        sessions,
        clock,
        tokens,
        downloads,
    } = deps;
    if matches!(resource, Resource::Catalog) && method == Method::Get {
        let input = match list_input(request)? {
            Ok(input) => input,
            Err(()) => return Ok(Some(response::error(400, "invalid_request")?)),
        };
        let result = match (ListSkills { catalog }).execute(input).await {
            Ok(Ok(output)) => response::json(&ListDto {
                items: output.items.into_iter().map(Into::into).collect(),
                next_after: output.next_after.map(|id| id.as_str().to_owned()),
            }),
            Ok(Err(error)) => content_error(error),
            Err(error) => response::internal(error),
        }?;
        return Ok(Some(result));
    }
    if !matches!(resource, Resource::Text(_)) && request.url()?.query().is_some() {
        return Ok(Some(response::error(400, "invalid_request")?));
    }
    let guest_token_hash = match redemption::guest_token_hash(request)? {
        Ok(hash) => hash,
        Err(()) => return Ok(Some(response::error(400, "invalid_guest_token")?)),
    };
    let actor = match (AuthenticateSession {
        store: sessions,
        clock: ClockRef(&clock),
    })
    .execute(auth::session_token(request)?)
    .await
    {
        Ok(Ok(account)) => Actor::Account(account),
        Ok(Err(_)) => Actor::Anonymous,
        Err(error) => return Ok(Some(response::internal(error)?)),
    };
    let result = match resource {
        Resource::Catalog => {
            publish(
                request,
                PublishSkill {
                    catalog,
                    artifacts,
                    clock,
                    tokens,
                },
                actor,
                None,
            )
            .await?
        }
        Resource::Detail(id) if method == Method::Put => {
            publish(
                request,
                PublishSkill {
                    catalog,
                    artifacts,
                    clock,
                    tokens,
                },
                actor,
                Some(id),
            )
            .await?
        }
        Resource::Detail(skill_id) => {
            let input = ContentRequest {
                actor,
                guest_token_hash,
                skill_id,
            };
            match (ReadDetail {
                catalog,
                access,
                artifacts,
            })
            .execute(input)
            .await
            {
                Ok(Ok(output)) => response::json(&DetailDto::from(output))?,
                Ok(Err(error)) => content_error(error)?,
                Err(error) => response::internal(error)?,
            }
        }
        Resource::Text(skill_id) => {
            let path = match text_path(request)? {
                Ok(path) => path,
                Err(()) => return Ok(Some(response::error(400, "invalid_file_path")?)),
            };
            let input = read_text::Input {
                content: ContentRequest {
                    actor,
                    guest_token_hash,
                    skill_id,
                },
                path,
            };
            match (ReadText {
                catalog,
                access,
                artifacts,
            })
            .execute(input)
            .await
            {
                Ok(Ok(output)) => response::json(&TextDto::from(output))?,
                Ok(Err(error)) => content_error(error)?,
                Err(error) => response::internal(error)?,
            }
        }
        Resource::Download(skill_id) => {
            let input = ContentRequest {
                actor,
                guest_token_hash,
                skill_id,
            };
            match (DownloadLatest { catalog, access }).execute(input).await {
                Ok(Ok(output)) => match downloads.open_body(&output.artifact).await {
                    Ok(body) => {
                        let mut result = response::finish(Response::from_body(body)?)?;
                        result
                            .headers_mut()
                            .set("Content-Type", "application/zip")?;
                        result.headers_mut().set(
                            "Content-Disposition",
                            &format!("attachment; filename=\"{}\"", output.filename),
                        )?;
                        result
                            .headers_mut()
                            .set("Content-Length", &output.artifact.byte_len.to_string())?;
                        result
                            .headers_mut()
                            .set("X-Content-Type-Options", "nosniff")?;
                        result
                            .headers_mut()
                            .set("X-Skill-Revision", &output.revision_id)?;
                        result
                    }
                    Err(error) => response::internal(error)?,
                },
                Ok(Err(error)) => content_error(error)?,
                Err(error) => response::internal(error)?,
            }
        }
    };
    Ok(Some(result))
}

async fn publish<S, R, C, T>(
    request: &mut Request,
    case: PublishSkill<S, R, C, T>,
    actor: Actor,
    update_id: Option<SkillId>,
) -> Result<Response>
where
    S: SkillCatalog + PublicationStore,
    R: ArtifactStore,
    C: Clock,
    T: TokenSource,
{
    // Admission control must precede body reads, JSON allocation and base64 decoding.
    // The use case independently retains its authoritative editing-permission check.
    let Some(account) = actor.account() else {
        return content_error(Error::Unauthenticated);
    };
    if !account.capabilities().can_edit_all() {
        return content_error(Error::Forbidden);
    }
    let creating = update_id.is_none();
    let upload = match body::json::<PublishBody>(request, MAX_UPLOAD_BODY).await? {
        Ok(upload) => upload,
        Err(response) => return Ok(response),
    };
    let input = match upload.into_input(actor, update_id) {
        Ok(input) => input,
        Err(error) => return response::error(error.status, error.code),
    };
    match case.execute(input).await {
        Ok(Ok(output)) => Ok(response::json(&PublishedDto {
            id: output.skill_id.as_str().to_owned(),
            revision_id: output.revision_id,
            updated_at: output.updated_at,
        })?
        .with_status(if creating { 201 } else { 200 })),
        Ok(Err(error)) => content_error(error),
        Err(error) => response::internal(error),
    }
}

fn list_input(request: &Request) -> Result<std::result::Result<list_skills::Input, ()>> {
    let mut after = None;
    let mut limit = None;
    for (key, value) in request.url()?.query_pairs() {
        match key.as_ref() {
            "after" if after.is_none() => {
                let Ok(id) = SkillId::parse(value.into_owned()) else {
                    return Ok(Err(()));
                };
                after = Some(id);
            }
            "limit" if limit.is_none() => {
                let Ok(parsed) = value.parse::<u32>() else {
                    return Ok(Err(()));
                };
                limit = Some(parsed);
            }
            _ => return Ok(Err(())),
        }
    }
    Ok(Ok(list_skills::Input {
        after,
        limit: limit.unwrap_or(50),
    }))
}

fn text_path(request: &Request) -> Result<std::result::Result<SkillPath, ()>> {
    let url = request.url()?;
    let pairs: Vec<_> = url.query_pairs().collect();
    if pairs.len() != 1 || pairs[0].0 != "path" {
        return Ok(Err(()));
    }
    Ok(SkillPath::parse(pairs[0].1.to_string()).map_err(|_| ()))
}

fn content_error(error: Error) -> Result<Response> {
    let (status, code) = match error {
        Error::Unauthenticated => (401, "unauthenticated"),
        Error::Forbidden => (403, "skill_forbidden"),
        Error::NotFound => (404, "not_found"),
        Error::NotText => (415, "not_text_file"),
        Error::Conflict => (409, "revision_conflict"),
        Error::InvalidInput => (400, "invalid_request"),
    };
    response::error(status, code)
}
