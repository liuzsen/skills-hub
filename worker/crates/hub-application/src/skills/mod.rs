pub mod artifacts;
pub mod download_latest;
pub mod list_skills;
pub mod publish_skill;
pub mod read_detail;
pub mod read_text;

use crate::redemption::access::SkillAccess;
use artifacts::StagedArtifacts;
use hub_domain::{
    account::Actor,
    redemption::{GuestTokenHash, SkillOperation},
    skill::{SkillId, SkillMetadata},
};
use lolibaso::biz::BizResult;

#[derive(Clone, Debug)]
pub struct SkillSummary {
    pub id: SkillId,
    pub metadata: SkillMetadata,
    pub updated_at: i64,
}

/// A single immutable revision snapshot. No subsequent current-pointer reads are needed.
#[derive(Clone, Debug)]
pub struct RevisionSnapshot {
    pub summary: SkillSummary,
    pub artifacts: StagedArtifacts,
}

pub struct ContentRequest {
    pub actor: Actor,
    pub guest_token_hash: Option<GuestTokenHash>,
    pub skill_id: SkillId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unauthenticated,
    Forbidden,
    NotFound,
    NotText,
    Conflict,
    InvalidInput,
}

/// Public queries deliberately cannot return a file manifest or object key.
pub trait SkillCatalog: Send + Sync {
    fn list(
        &self,
        after: Option<&SkillId>,
        limit: u32,
    ) -> impl Future<Output = anyhow::Result<Vec<SkillSummary>>> + Send;
    fn snapshot(
        &self,
        id: &SkillId,
    ) -> impl Future<Output = anyhow::Result<Option<RevisionSnapshot>>> + Send;
}

async fn authorized_snapshot<S: SkillCatalog, A: SkillAccess>(
    catalog: &S,
    access: &A,
    input: &ContentRequest,
    operation: SkillOperation,
) -> BizResult<RevisionSnapshot, Error> {
    if access
        .authorize(
            &input.actor,
            input.guest_token_hash.as_ref(),
            input.skill_id.as_str(),
            operation,
        )
        .await?
        .is_err()
    {
        return Ok(Err(Error::Forbidden));
    }
    Ok(catalog
        .snapshot(&input.skill_id)
        .await?
        .ok_or(Error::NotFound))
}
