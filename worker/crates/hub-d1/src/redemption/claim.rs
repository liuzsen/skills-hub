use super::{RedemptionStore, failure, integer, prepare};
use hub_application::redemption::claim_guest::{Claim, Claimed, GuestClaimStore};
use worker::send::SendFuture;

#[derive(serde::Deserialize)]
struct ClaimRow {
    skill_id: String,
    guest_expires_at: i64,
}

impl GuestClaimStore for RedemptionStore {
    async fn claim_guest_once(&self, claim: Claim) -> anyhow::Result<Option<Claimed>> {
        SendFuture::new(async move {
            let result = prepare(&self.db,
                "UPDATE redemption_codes SET guest_token_hash=?2, guest_claimed_at=?3, guest_expires_at=?4 WHERE code_hash=?1 AND bound_account_id IS NULL AND guest_claimed_at IS NULL RETURNING skill_id, guest_expires_at",
                &[claim.code_hash.as_str().into(), claim.guest_token_hash.as_str().into(), integer(claim.now)?, integer(claim.expires_at)?])?
                .all().await.map_err(|error| failure("guest claim", error))?;
            anyhow::ensure!(result.success(), "D1 guest claim reported failure");
            let mut rows = result.results::<ClaimRow>().map_err(|error| failure("guest receipt", error))?;
            anyhow::ensure!(rows.len() <= 1, "D1 guest claim returned duplicate receipts");
            Ok(rows.pop().map(|row| Claimed { skill_id: row.skill_id, expires_at: row.guest_expires_at }))
        }).await
    }
}
