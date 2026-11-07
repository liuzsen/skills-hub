use crate::{time::Clock, tokens};
use lolibaso::biz::{BizResult, UseCase};
use std::convert::Infallible;

pub trait SessionRevoker: Send {
    fn revoke(
        &self,
        token_hash: String,
        now: i64,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub struct Logout<S, C> {
    pub store: S,
    pub clock: C,
}

impl<S: SessionRevoker, C: Clock> UseCase for Logout<S, C> {
    type Input = Option<String>;
    type Output = ();
    type Error = Infallible;

    async fn execute(self, token: Option<String>) -> BizResult<(), Infallible> {
        if let Some(token) = token.filter(|token| tokens::is_opaque(token)) {
            self.store
                .revoke(tokens::digest(&token), self.clock.now())
                .await?;
        }
        Ok(Ok(()))
    }
}
