use super::{SkillsStore, failure, integer, prepare, rows::encode_files};
use hub_application::{
    skills::publish_skill::{Publication, PublicationStore, PublishOutcome},
    tokens,
};
use worker::{D1Result, send::SendFuture};

#[derive(serde::Deserialize)]
struct Receipt {
    receipt: String,
}

impl PublicationStore for SkillsStore {
    async fn publish_if_current(&self, publication: Publication) -> anyhow::Result<PublishOutcome> {
        SendFuture::new(async move {
            let id = publication.skill_id.as_str();
            let artifacts = &publication.artifacts;
            let revision = artifacts.revision.revision_id();
            let operation = publication.operation_id.as_str();
            anyhow::ensure!(artifacts.revision.skill_id() == id && tokens::is_opaque(revision)
                && tokens::is_opaque(operation), "invalid publication identity");
            let tags = serde_json::to_string(publication.metadata.tags())
                .map_err(|_| anyhow::anyhow!("encode publication tags failed"))?;
            let files = encode_files(artifacts)?;
            let cas = match &publication.expected_revision {
                None => prepare(&self.db,
                    "INSERT INTO skills(id,current_revision_id,publish_op) VALUES(?1,?2,?3) ON CONFLICT(id) DO NOTHING",
                    &[id.into(), revision.into(), operation.into()])?,
                Some(expected) => prepare(&self.db,
                    "UPDATE skills SET current_revision_id=?2,publish_op=?3 WHERE id=?1 AND current_revision_id=?4",
                    &[id.into(), revision.into(), operation.into(), expected.as_str().into()])?,
            };
            // The CAS is first. Every later mutation is guarded by this operation + revision.
            // If it affects zero rows the INSERT also selects zero rows; batch success alone
            // is not evidence that publication succeeded.
            let insert = prepare(&self.db,
                "INSERT INTO skill_revisions(id,skill_id,publish_op,name,description,tags_json,updated_at,files_json,manifest_key,manifest_size,zip_key,zip_size,total_bytes) SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13 FROM skills s WHERE s.id=?2 AND s.current_revision_id=?1 AND s.publish_op=?3",
                &[revision.into(), id.into(), operation.into(), publication.metadata.name().into(),
                    publication.metadata.description().into(), tags.as_str().into(), integer(publication.updated_at)?,
                    files.as_str().into(), artifacts.manifest.key.as_storage_key().into(), size(artifacts.manifest.byte_len)?,
                    artifacts.zip.key.as_storage_key().into(), size(artifacts.zip.byte_len)?, size(artifacts.total_bytes)?])?;
            let receipt = prepare(&self.db,
                "SELECT r.id AS receipt FROM skills s JOIN skill_revisions r ON r.id=s.current_revision_id AND r.skill_id=s.id WHERE s.id=?1 AND s.current_revision_id=?2 AND s.publish_op=?3 AND r.publish_op=?3",
                &[id.into(), revision.into(), operation.into()])?;
            let results = self.db.batch(vec![cas, insert, receipt]).await
                .map_err(|error| failure("publish batch", error))?;
            anyhow::ensure!(results.len() == 3 && results.iter().all(D1Result::success), "content publication batch failed");
            let receipts = results.last().ok_or_else(|| anyhow::anyhow!("publication receipt missing"))?
                .results::<Receipt>().map_err(|error| failure("publish receipt decode", error))?;
            anyhow::ensure!(receipts.len() <= 1, "duplicate publication receipt");
            Ok(if receipts.first().is_some_and(|row| row.receipt == revision) {
                PublishOutcome::Published
            } else { PublishOutcome::Conflict })
        }).await
    }
}

fn size(value: u64) -> anyhow::Result<worker::wasm_bindgen::JsValue> {
    integer(i64::try_from(value).map_err(|_| anyhow::anyhow!("publication size out of range"))?)
}
