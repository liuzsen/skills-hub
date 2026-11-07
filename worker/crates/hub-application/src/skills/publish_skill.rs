use super::{
    Error, SkillCatalog,
    artifacts::{ArtifactRevision, ArtifactStore, StagedArtifacts},
};
use crate::{
    time::Clock,
    tokens::{self, TokenSource},
};
use hub_domain::{
    account::Actor,
    skill::{SkillFiles, SkillId, SkillMetadata},
};
use lolibaso::biz::{BizResult, UseCase};

pub struct Input {
    pub actor: Actor,
    pub skill_id: SkillId,
    pub metadata: SkillMetadata,
    /// None creates a unique skill; Some performs a current-revision compare-and-swap.
    pub expected_revision: Option<String>,
    pub files: SkillFiles,
}

pub struct Publication {
    pub skill_id: SkillId,
    pub metadata: SkillMetadata,
    pub expected_revision: Option<String>,
    pub artifacts: StagedArtifacts,
    pub operation_id: String,
    pub updated_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublishOutcome {
    Published,
    Conflict,
}

pub trait PublicationStore: Send + Sync {
    /// One atomic guarded batch: CAS, revision/manifest insertion, and operation receipt.
    /// A zero-row CAS must not insert revision metadata or report success.
    fn publish_if_current(
        &self,
        publication: Publication,
    ) -> impl Future<Output = anyhow::Result<PublishOutcome>> + Send;
}

pub struct Output {
    pub skill_id: SkillId,
    pub revision_id: String,
    pub updated_at: i64,
}
pub struct PublishSkill<S, R, C, T> {
    pub catalog: S,
    pub artifacts: R,
    pub clock: C,
    pub tokens: T,
}

impl<S: SkillCatalog + PublicationStore, R: ArtifactStore, C: Clock, T: TokenSource> UseCase
    for PublishSkill<S, R, C, T>
{
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: Input) -> BizResult<Output, Error> {
        let Some(account) = input.actor.account() else {
            return Ok(Err(Error::Unauthenticated));
        };
        if !account.capabilities().can_edit_all() {
            return Ok(Err(Error::Forbidden));
        }
        if input
            .expected_revision
            .as_ref()
            .is_some_and(|revision| !tokens::is_opaque(revision))
        {
            return Ok(Err(Error::InvalidInput));
        }
        // Cheap stale-state rejection before staging. This does not replace database CAS.
        let current = self.catalog.snapshot(&input.skill_id).await?;
        let matches = match (&input.expected_revision, &current) {
            (None, None) => true,
            (Some(expected), Some(current)) => expected == current.artifacts.revision.revision_id(),
            _ => false,
        };
        if !matches {
            return Ok(Err(Error::Conflict));
        }
        let revision_id = tokens::opaque(&self.tokens)?;
        let operation_id = tokens::opaque(&self.tokens)?;
        let revision = ArtifactRevision::new(input.skill_id.as_str(), revision_id.clone())
            .map_err(|_| anyhow::anyhow!("generated invalid artifact namespace"))?;
        let artifacts = self.artifacts.stage(revision, input.files).await?;
        let updated_at = self.clock.now();
        let skill_id = input.skill_id;
        let publication = Publication {
            skill_id: skill_id.clone(),
            metadata: input.metadata,
            expected_revision: input.expected_revision,
            artifacts,
            operation_id,
            updated_at,
        };
        match self.catalog.publish_if_current(publication).await? {
            PublishOutcome::Published => Ok(Ok(Output {
                skill_id,
                revision_id,
                updated_at,
            })),
            PublishOutcome::Conflict => Ok(Err(Error::Conflict)),
        }
        // No cleanup here: D1 commit may be unknown after a transport failure.
    }
}
