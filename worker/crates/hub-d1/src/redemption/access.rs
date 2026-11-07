use super::{RedemptionStore, failure, integer, prepare};
use hub_application::redemption::access::SkillAccessFacts;
use hub_domain::{account::AccountId, redemption::GuestTokenHash};
use worker::{send::SendFuture, wasm_bindgen::JsValue};

#[derive(serde::Deserialize)]
struct AccessRow {
    allowed: i64,
}

impl SkillAccessFacts for RedemptionStore {
    async fn has_skill_entitlement(
        &self,
        account_id: Option<&AccountId>,
        guest_token_hash: Option<&GuestTokenHash>,
        skill_id: &str,
        now: i64,
    ) -> anyhow::Result<bool> {
        if account_id.is_none() && guest_token_hash.is_none() {
            return Ok(false);
        }
        SendFuture::new(async move {
            let account = account_id.map(|id| JsValue::from_str(id.as_str())).unwrap_or(JsValue::NULL);
            let guest = guest_token_hash.map(|hash| JsValue::from_str(hash.as_str())).unwrap_or(JsValue::NULL);
            // Deliberately no bound_account_id guard on the guest branch: already issued tokens keep their original lifetime.
            let row = prepare(&self.db,
                "SELECT EXISTS(SELECT 1 FROM redemption_codes WHERE skill_id=?1 AND ((?2 IS NOT NULL AND bound_account_id=?2) OR (?3 IS NOT NULL AND guest_token_hash=?3 AND guest_expires_at>?4))) AS allowed",
                &[skill_id.into(), account, guest, integer(now)?])?
                .first::<AccessRow>(None).await.map_err(|error| failure("access lookup", error))?
                .ok_or_else(|| anyhow::anyhow!("D1 access lookup missing result"))?;
            anyhow::ensure!(matches!(row.allowed, 0 | 1), "invalid D1 access result");
            Ok(row.allowed == 1)
        }).await
    }
}
