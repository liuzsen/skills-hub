use crate::{time::Clock, tokens};
use hub_domain::account::Account;
use lolibaso::biz::{BizResult, UseCase};

pub enum Error {
    Unauthenticated,
}

pub trait SessionReader: Send {
    fn find_active(
        &self,
        token_hash: String,
        now: i64,
    ) -> impl Future<Output = anyhow::Result<Option<Account>>> + Send;
}

pub struct AuthenticateSession<S, C> {
    pub store: S,
    pub clock: C,
}

impl<S: SessionReader, C: Clock> UseCase for AuthenticateSession<S, C> {
    type Input = Option<String>;
    type Output = Account;
    type Error = Error;

    async fn execute(self, token: Option<String>) -> BizResult<Account, Error> {
        let Some(token) = token.filter(|token| tokens::is_opaque(token)) else {
            return Ok(Err(Error::Unauthenticated));
        };
        Ok(self
            .store
            .find_active(tokens::digest(&token), self.clock.now())
            .await?
            .ok_or(Error::Unauthenticated))
    }
}
