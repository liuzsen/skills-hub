use super::AuthStore;
use hub_application::auth::issue_challenge::{ChallengeIssuer, Error, Issuance};

impl ChallengeIssuer for AuthStore {
    async fn issue(&self, input: Issuance) -> anyhow::Result<Result<(), Error>> {
        #[cfg(not(feature = "local-mail"))]
        {
            let _ = input;
            Ok(Err(Error::DeliveryUnavailable))
        }
        #[cfg(feature = "local-mail")]
        {
            worker::send::SendFuture::new(async move {
                use super::{batch, integer, prepare, receipt};
                let email = input.email.as_str().into();
                let id = input.challenge_hash.as_str().into();
                let now = integer(input.now)?;
                let statements = vec![
                    prepare(&self.db, "INSERT INTO email_issuance(email_key, current_challenge_hash, next_send_at) VALUES (?1, ?2, ?3) ON CONFLICT(email_key) DO UPDATE SET current_challenge_hash=excluded.current_challenge_hash, next_send_at=excluded.next_send_at WHERE email_issuance.next_send_at <= ?4", &[email, id, integer(input.next_send_at)?, now])?,
                    prepare(&self.db, "INSERT INTO email_challenges(id, email_key, code_mac, created_at, expires_at) SELECT ?1, ?2, ?3, ?4, ?5 FROM email_issuance WHERE email_key=?2 AND current_challenge_hash=?1", &[input.challenge_hash.as_str().into(), input.email.as_str().into(), input.code_mac.as_str().into(), integer(input.now)?, integer(input.expires_at)?])?,
                    prepare(&self.db, "DELETE FROM local_mail_outbox WHERE recipient=?1 AND EXISTS(SELECT 1 FROM email_issuance WHERE email_key=?1 AND current_challenge_hash=?2)", &[input.email.as_str().into(), input.challenge_hash.as_str().into()])?,
                    prepare(&self.db, "INSERT INTO local_mail_outbox(challenge_id, recipient, code, created_at, expires_at) SELECT id, email_key, ?2, created_at, expires_at FROM email_challenges WHERE id=?1", &[input.challenge_hash.as_str().into(), input.local_code.as_str().into()])?,
                    prepare(&self.db, "SELECT challenge_id AS receipt FROM local_mail_outbox WHERE challenge_id=?1", &[input.challenge_hash.as_str().into()])?,
                ];
                let results = batch(&self.db, statements).await?;
                Ok(if receipt(&results, &input.challenge_hash)? { Ok(()) } else { Err(Error::ResendCooldown) })
            }).await
        }
    }
}
