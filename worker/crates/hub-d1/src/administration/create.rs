use super::{AdministrationStore, failure, prepare};
use hub_application::administration::create_codes::{CodeBatch, CodeIssuer, MAX_CODES_PER_BATCH};
use worker::{D1Result, send::SendFuture, wasm_bindgen::JsValue};

#[derive(serde::Deserialize)]
struct CountRow {
    issued: u32,
}

impl CodeIssuer for AdministrationStore {
    async fn create_codes(&self, batch: CodeBatch) -> anyhow::Result<bool> {
        SendFuture::new(async move {
            anyhow::ensure!(!batch.codes.is_empty() && batch.codes.len() <= MAX_CODES_PER_BATCH as usize, "invalid code batch size at D1 boundary");
            anyhow::ensure!((0..=9_007_199_254_740_991).contains(&batch.created_at), "D1 timestamp out of safe range");
            let expected = batch.codes.len();
            let mut statements = Vec::with_capacity(expected + 1);
            for code in batch.codes {
                // Missing skill inserts nothing. All statements share the same DB snapshot in this atomic batch.
                statements.push(prepare(&self.db,
                    "INSERT INTO redemption_codes(id, code_hash, batch_id, skill_id, created_by, created_at) SELECT ?1, ?2, ?3, s.id, ?5, ?6 FROM skills s WHERE s.id=?4",
                    &[code.id.as_str().into(), code.code_hash.as_str().into(), batch.batch_id.as_str().into(), batch.skill_id.as_str().into(), batch.created_by.as_str().into(), JsValue::from_f64(batch.created_at as f64)])?);
            }
            statements.push(prepare(&self.db, "SELECT COUNT(*) AS issued FROM redemption_codes WHERE batch_id=?1", &[batch.batch_id.as_str().into()])?);
            let results = self.db.batch(statements).await.map_err(|error| failure("issue batch", error))?;
            anyhow::ensure!(results.len() == expected + 1 && results.iter().all(D1Result::success), "D1 code batch reported failure");
            let rows = results.last().ok_or_else(|| anyhow::anyhow!("missing code batch receipt"))?
                .results::<CountRow>().map_err(|error| failure("issue receipt", error))?;
            anyhow::ensure!(rows.len() == 1, "invalid code batch receipt");
            let issued = rows[0].issued as usize;
            anyhow::ensure!(issued == 0 || issued == expected, "partial code batch receipt");
            Ok(issued == expected)
        }).await
    }
}
