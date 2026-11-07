use super::{AuthStore, batch, integer, prepare, receipt};
use hub_application::auth::{
    MAX_ATTEMPTS,
    verify_email::{ChallengeConsumer, Consumption, Error},
};
use worker::send::SendFuture;

impl ChallengeConsumer for AuthStore {
    async fn consume_and_create_session(
        &self,
        input: Consumption,
    ) -> anyhow::Result<Result<(), Error>> {
        SendFuture::new(async move {
            // Every later write is guarded by this invocation's unique consumed_op.
            // A successful batch containing zero-row UPDATEs must not create an account/session.
            let active = "id=?1 AND email_key=?2 AND consumed_op IS NULL AND expires_at>?4 AND attempts<?5 AND EXISTS(SELECT 1 FROM email_issuance i WHERE i.email_key=email_challenges.email_key AND i.current_challenge_hash=email_challenges.id)";
            let wrong = format!("UPDATE email_challenges SET attempts=attempts+1 WHERE {active} AND code_mac<>?3");
            let consume = format!("UPDATE email_challenges SET consumed_op=?6 WHERE {active} AND code_mac=?3");
            let args = [input.challenge_hash.as_str().into(), input.email.as_str().into(), input.code_mac.as_str().into(), integer(input.now)?, integer(MAX_ATTEMPTS)?, input.operation_id.as_str().into()];
            let statements = vec![
                prepare(&self.db, &wrong, &args[..5])?,
                prepare(&self.db, &consume, &args)?,
                prepare(&self.db, "INSERT INTO accounts(id, email_key, verified_at, created_at) SELECT ?1, email_key, ?2, ?2 FROM email_challenges WHERE consumed_op=?3 ON CONFLICT(email_key) DO NOTHING", &[input.candidate_account_id.as_str().into(), integer(input.now)?, input.operation_id.as_str().into()])?,
                prepare(&self.db, "INSERT INTO sessions(token_hash, account_id, challenge_id, created_at, expires_at) SELECT ?1, a.id, c.id, ?2, ?3 FROM email_challenges c JOIN accounts a ON a.email_key=c.email_key WHERE c.consumed_op=?4", &[input.session_hash.as_str().into(), integer(input.now)?, integer(input.session_expires_at)?, input.operation_id.as_str().into()])?,
                prepare(&self.db, "DELETE FROM local_mail_outbox WHERE challenge_id IN (SELECT id FROM email_challenges WHERE consumed_op=?1)", &[input.operation_id.as_str().into()])?,
                prepare(&self.db, "SELECT s.token_hash AS receipt FROM sessions s JOIN email_challenges c ON c.id=s.challenge_id WHERE c.consumed_op=?1 AND s.token_hash=?2", &[input.operation_id.as_str().into(), input.session_hash.as_str().into()])?,
            ];
            let results = batch(&self.db, statements).await?;
            Ok(if receipt(&results, &input.session_hash)? { Ok(()) } else { Err(Error::ChallengeRejected) })
        }).await
    }
}
