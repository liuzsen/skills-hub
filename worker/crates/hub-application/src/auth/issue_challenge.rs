use super::{CHALLENGE_TTL, ChallengePepper, RESEND_COOLDOWN};
use crate::{
    time::Clock,
    tokens::{self, TokenSource},
};
use hub_domain::account::EmailAddress;
use lolibaso::biz::{BizResult, UseCase};

pub enum Error {
    ResendCooldown,
    DeliveryUnavailable,
}

/// Contains plaintext only for the explicitly local outbox adapter; never log or serialize.
pub struct Issuance {
    pub challenge_hash: String,
    pub email: EmailAddress,
    pub code_mac: String,
    pub local_code: String,
    pub now: i64,
    pub expires_at: i64,
    pub next_send_at: i64,
}

pub trait ChallengeIssuer: Send {
    fn issue(&self, issuance: Issuance) -> impl Future<Output = BizResult<(), Error>> + Send;
}

pub struct IssueChallenge<S, C, T> {
    pub store: S,
    pub clock: C,
    pub tokens: T,
    pub pepper: ChallengePepper,
}

pub struct Output {
    pub challenge_id: String,
    pub expires_at: i64,
    pub resend_after: i64,
}

impl<S: ChallengeIssuer, C: Clock, T: TokenSource> UseCase for IssueChallenge<S, C, T> {
    type Input = EmailAddress;
    type Output = Output;
    type Error = Error;

    async fn execute(self, email: EmailAddress) -> BizResult<Output, Error> {
        let now = self.clock.now();
        let challenge_id = tokens::opaque(&self.tokens)?;
        let challenge_hash = tokens::digest(&challenge_id);
        let code = generate_code(&self.tokens)?;
        let code_mac = self.pepper.code_mac(&challenge_hash, email.as_str(), &code);
        let expires_at = now + CHALLENGE_TTL;
        let next_send_at = now + RESEND_COOLDOWN;
        match self
            .store
            .issue(Issuance {
                challenge_hash,
                email,
                code_mac,
                local_code: code,
                now,
                expires_at,
                next_send_at,
            })
            .await?
        {
            Ok(()) => Ok(Ok(Output {
                challenge_id,
                expires_at,
                resend_after: RESEND_COOLDOWN,
            })),
            Err(error) => Ok(Err(error)),
        }
    }
}

fn generate_code(source: &impl TokenSource) -> anyhow::Result<String> {
    let mut code = String::with_capacity(6);
    while code.len() < 6 {
        let mut bytes = [0; 16];
        source.fill(&mut bytes)?;
        for byte in bytes {
            if byte < 250 && code.len() < 6 {
                code.push(char::from(b'0' + byte % 10));
            }
        }
    }
    Ok(code)
}
