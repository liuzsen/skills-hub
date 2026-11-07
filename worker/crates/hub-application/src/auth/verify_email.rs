use super::{ChallengePepper, SESSION_TTL};
use crate::{
    time::Clock,
    tokens::{self, TokenSource},
};
use hub_domain::account::{AccountId, EmailAddress};
use lolibaso::biz::{BizResult, UseCase};

pub enum Error {
    ChallengeRejected,
}

pub struct Input {
    pub email: EmailAddress,
    pub challenge_id: String,
    pub code: String,
}

pub struct Consumption {
    pub challenge_hash: String,
    pub email: EmailAddress,
    pub code_mac: String,
    pub operation_id: String,
    pub candidate_account_id: AccountId,
    pub session_hash: String,
    pub now: i64,
    pub session_expires_at: i64,
}

pub trait ChallengeConsumer: Send {
    /// Must atomically consume once, merge a verified account, and create this session.
    /// Zero-row consumption must never register an account or create a session.
    fn consume_and_create_session(
        &self,
        input: Consumption,
    ) -> impl Future<Output = BizResult<(), Error>> + Send;
}

pub struct VerifyEmail<S, C, T> {
    pub store: S,
    pub clock: C,
    pub tokens: T,
    pub pepper: ChallengePepper,
}

/// Raw token may only leave through the HttpOnly cookie boundary.
pub struct Output {
    pub session_token: String,
    pub expires_at: i64,
}

impl<S: ChallengeConsumer, C: Clock, T: TokenSource> UseCase for VerifyEmail<S, C, T> {
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: Input) -> BizResult<Output, Error> {
        if !tokens::is_opaque(&input.challenge_id)
            || input.code.len() != 6
            || !input.code.bytes().all(|b| b.is_ascii_digit())
        {
            return Ok(Err(Error::ChallengeRejected));
        }
        let now = self.clock.now();
        let challenge_hash = tokens::digest(&input.challenge_id);
        let code_mac = self
            .pepper
            .code_mac(&challenge_hash, input.email.as_str(), &input.code);
        let session_token = tokens::opaque(&self.tokens)?;
        let operation_id = tokens::opaque(&self.tokens)?;
        let candidate_account_id = AccountId::parse(tokens::opaque(&self.tokens)?)
            .map_err(|_| anyhow::anyhow!("generated invalid account id"))?;
        let expires_at = now + SESSION_TTL;
        let consumption = Consumption {
            challenge_hash,
            email: input.email,
            code_mac,
            operation_id,
            candidate_account_id,
            session_hash: tokens::digest(&session_token),
            now,
            session_expires_at: expires_at,
        };
        match self.store.consume_and_create_session(consumption).await? {
            Ok(()) => Ok(Ok(Output {
                session_token,
                expires_at,
            })),
            Err(error) => Ok(Err(error)),
        }
    }
}
