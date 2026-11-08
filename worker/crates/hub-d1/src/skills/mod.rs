mod publication;
mod rows;

use hub_application::skills::{RevisionSnapshot, SkillCatalog, SkillSummary};
use hub_domain::skill::SkillId;
use rows::{SnapshotRow, SummaryRow};
use worker::{D1Database, D1PreparedStatement, send::SendFuture, wasm_bindgen::JsValue};

pub struct SkillsStore {
    db: D1Database,
}

impl SkillsStore {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

fn failure(operation: &'static str, _error: worker::Error) -> anyhow::Error {
    anyhow::anyhow!("D1 content {operation} failed (SDK details redacted)")
}

fn prepare(db: &D1Database, sql: &str, values: &[JsValue]) -> anyhow::Result<D1PreparedStatement> {
    db.prepare(sql)
        .bind(values)
        .map_err(|error| failure("bind", error))
}

fn integer(value: i64) -> anyhow::Result<JsValue> {
    anyhow::ensure!(
        (0..=9_007_199_254_740_991).contains(&value),
        "content integer out of safe range"
    );
    Ok(JsValue::from_f64(value as f64))
}

impl SkillCatalog for SkillsStore {
    async fn list(&self, after: Option<&SkillId>, limit: u32) -> anyhow::Result<Vec<SkillSummary>> {
        SendFuture::new(async move {
            anyhow::ensure!((1..=101).contains(&limit), "invalid catalog query limit");
            // This SELECT never loads private file manifests, ZIP metadata or object keys.
            let result = prepare(&self.db,
                "SELECT s.id AS skill_id,r.name,r.description,r.tags_json,r.updated_at FROM skills s JOIN skill_revisions r ON r.id=s.current_revision_id AND r.skill_id=s.id WHERE s.id>?1 ORDER BY s.id LIMIT ?2",
                &[after.map(SkillId::as_str).unwrap_or("").into(), integer(i64::from(limit))?])?
                .all().await.map_err(|error| failure("catalog query", error))?;
            anyhow::ensure!(result.success(), "content catalog query failed");
            result.results::<SummaryRow>().map_err(|error| failure("catalog decode", error))?
                .into_iter().map(SummaryRow::into_summary).collect()
        }).await
    }

    async fn snapshot(&self, id: &SkillId) -> anyhow::Result<Option<RevisionSnapshot>> {
        SendFuture::new(async move {
            // One query chooses the revision and all metadata; no later pointer reread.
            let row = prepare(&self.db,
                "SELECT s.id AS skill_id,r.id AS revision_id,r.name,r.description,r.tags_json,r.updated_at,r.files_json,r.manifest_key,r.manifest_size,r.zip_key,r.zip_size,r.total_bytes FROM skills s JOIN skill_revisions r ON r.id=s.current_revision_id AND r.skill_id=s.id WHERE s.id=?1",
                &[id.as_str().into()])?
                .first::<SnapshotRow>(None).await.map_err(|error| failure("snapshot query", error))?;
            row.map(SnapshotRow::into_snapshot).transpose()
        }).await
    }
}
