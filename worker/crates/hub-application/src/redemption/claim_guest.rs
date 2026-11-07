use crate::{
    time::Clock,
    tokens::{self, TokenSource},
};
use hub_domain::redemption::GUEST_TTL_SECONDS;
use lolibaso::biz::{BizResult, UseCase};

pub enum Error {
    InvalidCode,
    Unavailable,
}

pub struct Claim {
    pub code_hash: String,
    pub guest_token_hash: String,
    pub now: i64,
    pub expires_at: i64,
}

pub struct Claimed {
    pub skill_id: String,
    pub expires_at: i64,
}

pub trait GuestClaimStore: Send {
    fn claim_guest_once(
        &self,
        claim: Claim,
    ) -> impl Future<Output = anyhow::Result<Option<Claimed>>> + Send;
}

/// Secret token is returned only on the successful first claim. Never Debug/Serialize implicitly.
pub struct Output {
    pub token: String,
    pub skill_id: String,
    pub expires_at: i64,
}

pub struct ClaimGuest<S, C, T> {
    pub store: S,
    pub clock: C,
    pub tokens: T,
}

impl<S: GuestClaimStore, C: Clock, T: TokenSource> UseCase for ClaimGuest<S, C, T> {
    type Input = String;
    type Output = Output;
    type Error = Error;

    async fn execute(self, code: String) -> BizResult<Output, Error> {
        if !tokens::is_opaque(&code) {
            return Ok(Err(Error::InvalidCode));
        }
        let token = tokens::opaque(&self.tokens)?;
        let now = self.clock.now();
        let expires_at = now
            .checked_add(GUEST_TTL_SECONDS)
            .ok_or_else(|| anyhow::anyhow!("guest expiry overflow"))?;
        let claim = Claim {
            code_hash: tokens::digest(&code),
            guest_token_hash: tokens::digest(&token),
            now,
            expires_at,
        };
        let Some(claimed) = self.store.claim_guest_once(claim).await? else {
            return Ok(Err(Error::Unavailable));
        };
        Ok(Ok(Output {
            token,
            skill_id: claimed.skill_id,
            expires_at: claimed.expires_at,
        }))
    }
}
