use super::{ContentRequest, Error, SkillCatalog, artifacts::ArtifactStore, authorized_snapshot};
use crate::redemption::access::SkillAccess;
use hub_domain::{
    redemption::SkillOperation,
    skill::{MAX_FILE_BYTES, SkillPath},
};
use lolibaso::biz::{BizResult, UseCase};

pub struct Input {
    pub content: ContentRequest,
    pub path: SkillPath,
}
pub struct Output {
    pub revision_id: String,
    pub path: SkillPath,
    pub content: String,
}
pub struct ReadText<S, A, R> {
    pub catalog: S,
    pub access: A,
    pub artifacts: R,
}

impl<S: SkillCatalog, A: SkillAccess, R: ArtifactStore> UseCase for ReadText<S, A, R> {
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: Input) -> BizResult<Output, Error> {
        let snapshot = match authorized_snapshot(
            &self.catalog,
            &self.access,
            &input.content,
            SkillOperation::Read,
        )
        .await?
        {
            Ok(snapshot) => snapshot,
            Err(error) => return Ok(Err(error)),
        };
        let Some(file) = snapshot
            .artifacts
            .files
            .iter()
            .find(|file| file.path == input.path)
        else {
            return Ok(Err(Error::NotFound));
        };
        if !file.is_utf8 {
            return Ok(Err(Error::NotText));
        }
        let bytes = self
            .artifacts
            .read(&file.artifact, MAX_FILE_BYTES as u64)
            .await?;
        let content = String::from_utf8(bytes)
            .map_err(|_| anyhow::anyhow!("published file UTF-8 metadata mismatch"))?;
        Ok(Ok(Output {
            revision_id: snapshot.artifacts.revision.revision_id().to_owned(),
            path: input.path,
            content,
        }))
    }
}
