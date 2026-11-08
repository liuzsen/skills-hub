use super::{
    ContentRequest, Error, SkillCatalog, SkillSummary, artifacts::ArtifactStore,
    authorized_snapshot,
};
use crate::redemption::access::SkillAccess;
use hub_domain::{
    redemption::SkillOperation,
    skill::{MAX_FILE_BYTES, SkillPath},
};
use lolibaso::biz::{BizResult, UseCase};

pub struct FileEntry {
    pub path: SkillPath,
    pub byte_len: u64,
    pub is_utf8: bool,
}
pub struct Readme {
    pub path: SkillPath,
    pub content: String,
}
pub struct Output {
    pub summary: SkillSummary,
    pub revision_id: String,
    pub files: Vec<FileEntry>,
    pub readme: Option<Readme>,
}
pub struct ReadDetail<S, A, R> {
    pub catalog: S,
    pub access: A,
    pub artifacts: R,
}

impl<S: SkillCatalog, A: SkillAccess, R: ArtifactStore> UseCase for ReadDetail<S, A, R> {
    type Input = ContentRequest;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: ContentRequest) -> BizResult<Output, Error> {
        let snapshot =
            match authorized_snapshot(&self.catalog, &self.access, &input, SkillOperation::Read)
                .await?
            {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(Err(error)),
            };
        let readme = match snapshot
            .artifacts
            .files
            .iter()
            .find(|file| file.is_utf8 && file.path.as_str().eq_ignore_ascii_case("README.md"))
        {
            Some(file) => {
                let bytes = self
                    .artifacts
                    .read(&file.artifact, MAX_FILE_BYTES as u64)
                    .await?;
                let content = String::from_utf8(bytes)
                    .map_err(|_| anyhow::anyhow!("published README UTF-8 metadata mismatch"))?;
                Some(Readme {
                    path: file.path.clone(),
                    content,
                })
            }
            None => None,
        };
        let files = snapshot
            .artifacts
            .files
            .into_iter()
            .map(|file| FileEntry {
                path: file.path,
                byte_len: file.artifact.byte_len,
                is_utf8: file.is_utf8,
            })
            .collect();
        Ok(Ok(Output {
            summary: snapshot.summary,
            revision_id: snapshot.artifacts.revision.revision_id().to_owned(),
            files,
            readme,
        }))
    }
}
