use hub_domain::account::{AccountId, Actor};
use lolibaso::biz::{BizResult, UseCase};

#[derive(Clone, Copy)]
pub enum Capability {
    ViewAll,
    DownloadAll,
    UploadEdit,
}
pub enum Error {
    Unauthenticated,
    Forbidden,
    AccountNotFound,
}

pub trait CapabilityGranter: Send {
    /// Grants just one flag, preserves the other two. Never creates an account or admin role.
    fn grant(
        &self,
        target: AccountId,
        capability: Capability,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;
}

pub struct Input {
    pub actor: Actor,
    pub target: AccountId,
    pub capability: Capability,
}
pub struct GrantCapabilities<S> {
    pub store: S,
}

impl<S: CapabilityGranter> UseCase for GrantCapabilities<S> {
    type Input = Input;
    type Output = ();
    type Error = Error;

    async fn execute(self, input: Input) -> BizResult<(), Error> {
        let Some(account) = input.actor.account() else {
            return Ok(Err(Error::Unauthenticated));
        };
        if !account.is_admin() {
            return Ok(Err(Error::Forbidden));
        }
        Ok(if self.store.grant(input.target, input.capability).await? {
            Ok(())
        } else {
            Err(Error::AccountNotFound)
        })
    }
}
