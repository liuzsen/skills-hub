use super::{
    ContentRequest, Error, SkillCatalog, artifacts::ArtifactDescriptor, authorized_snapshot,
};
use crate::redemption::access::SkillAccess;
use hub_domain::redemption::SkillOperation;
use lolibaso::biz::{BizResult, UseCase};

pub struct Output {
    pub revision_id: String,
    pub filename: String,
    pub artifact: ArtifactDescriptor,
}
pub struct DownloadLatest<S, A> {
    pub catalog: S,
    pub access: A,
}

impl<S: SkillCatalog, A: SkillAccess> UseCase for DownloadLatest<S, A> {
    type Input = ContentRequest;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: ContentRequest) -> BizResult<Output, Error> {
        let snapshot = match authorized_snapshot(
            &self.catalog,
            &self.access,
            &input,
            SkillOperation::Download,
        )
        .await?
        {
            Ok(snapshot) => snapshot,
            Err(error) => return Ok(Err(error)),
        };
        Ok(Ok(Output {
            revision_id: snapshot.artifacts.revision.revision_id().to_owned(),
            filename: format!("{}.zip", snapshot.summary.id.as_str()),
            artifact: snapshot.artifacts.zip,
        }))
    }
}
