use crate::tokens;
use hub_domain::account::{AccountId, Actor};
use lolibaso::biz::{BizResult, UseCase};

pub enum Error {
    Unauthenticated,
    InvalidCode,
    Unavailable,
}

pub enum BindOutcome {
    Bound { skill_id: String },
    Unavailable,
}

pub trait CodeBindingStore: Send {
    /// Atomically sets an empty owner; an existing identical owner is an idempotent success.
    fn bind_once(
        &self,
        code_hash: String,
        account_id: AccountId,
    ) -> impl Future<Output = anyhow::Result<BindOutcome>> + Send;
}

pub struct Input {
    pub actor: Actor,
    pub code: String,
}
pub struct Output {
    pub skill_id: String,
}
pub struct BindCode<S> {
    pub store: S,
}

impl<S: CodeBindingStore> UseCase for BindCode<S> {
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: Input) -> BizResult<Output, Error> {
        let Some(account) = input.actor.account() else {
            return Ok(Err(Error::Unauthenticated));
        };
        if !tokens::is_opaque(&input.code) {
            return Ok(Err(Error::InvalidCode));
        }
        let result = self
            .store
            .bind_once(tokens::digest(&input.code), account.id().clone())
            .await?;
        Ok(match result {
            BindOutcome::Bound { skill_id } => Ok(Output { skill_id }),
            BindOutcome::Unavailable => Err(Error::Unavailable),
        })
    }
}
