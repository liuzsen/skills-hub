use super::{RedemptionStore, failure, prepare};
use hub_application::redemption::bind_code::{BindOutcome, CodeBindingStore};
use hub_domain::account::AccountId;
use worker::send::SendFuture;

#[derive(serde::Deserialize)]
struct BindingRow {
    skill_id: String,
}

impl CodeBindingStore for RedemptionStore {
    async fn bind_once(
        &self,
        code_hash: String,
        account_id: AccountId,
    ) -> anyhow::Result<BindOutcome> {
        SendFuture::new(async move {
            // Same-owner retries return the same receipt in this single statement, without a racy second read.
            let result = prepare(&self.db,
                "UPDATE redemption_codes SET bound_account_id=?2 WHERE code_hash=?1 AND (bound_account_id IS NULL OR bound_account_id=?2) RETURNING skill_id",
                &[code_hash.as_str().into(), account_id.as_str().into()])?
                .all().await.map_err(|error| failure("account bind", error))?;
            anyhow::ensure!(result.success(), "D1 account bind reported failure");
            let mut rows = result.results::<BindingRow>().map_err(|error| failure("binding receipt", error))?;
            anyhow::ensure!(rows.len() <= 1, "D1 binding returned duplicate receipts");
            Ok(match rows.pop() {
                Some(row) => BindOutcome::Bound { skill_id: row.skill_id },
                None => BindOutcome::Unavailable,
            })
        }).await
    }
}
