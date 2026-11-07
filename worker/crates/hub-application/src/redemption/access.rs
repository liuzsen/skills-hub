use crate::{time::Clock, tokens};
use hub_domain::{
    account::{AccountId, Actor},
    redemption::{GuestTokenHash, SkillOperation, has_global_access},
};
use lolibaso::biz::{BizResult, UseCase};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessDenied {
    Forbidden,
}

pub fn guest_token_hash(raw: &str) -> Option<GuestTokenHash> {
    if !tokens::is_opaque(raw) {
        return None;
    }
    GuestTokenHash::parse(tokens::digest(raw))
}

/// Shared by content use cases. No HTTP or persistence types cross this port.
pub trait SkillAccess: Send + Sync {
    fn authorize(
        &self,
        actor: &Actor,
        guest_token_hash: Option<&GuestTokenHash>,
        skill_id: &str,
        operation: SkillOperation,
    ) -> impl Future<Output = BizResult<(), AccessDenied>> + Send;
}

/// Facts only: SQL must scope both binding and guest token to the requested skill.
pub trait SkillAccessFacts: Send + Sync {
    fn has_skill_entitlement(
        &self,
        account_id: Option<&AccountId>,
        guest_token_hash: Option<&GuestTokenHash>,
        skill_id: &str,
        now: i64,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;
}

pub struct CheckSkillAccess<S, C> {
    pub store: S,
    pub clock: C,
}

pub struct Input {
    pub actor: Actor,
    pub guest_token_hash: Option<GuestTokenHash>,
    pub skill_id: String,
    pub operation: SkillOperation,
}

impl<S: SkillAccessFacts, C: Clock> SkillAccess for CheckSkillAccess<S, C> {
    async fn authorize(
        &self,
        actor: &Actor,
        guest_token_hash: Option<&GuestTokenHash>,
        skill_id: &str,
        operation: SkillOperation,
    ) -> BizResult<(), AccessDenied> {
        if has_global_access(actor, operation) {
            return Ok(Ok(()));
        }
        let account_id = actor.account().map(|account| account.id());
        let allowed = self
            .store
            .has_skill_entitlement(account_id, guest_token_hash, skill_id, self.clock.now())
            .await?;
        Ok(if allowed {
            Ok(())
        } else {
            Err(AccessDenied::Forbidden)
        })
    }
}

impl<S: SkillAccessFacts, C: Clock> UseCase for CheckSkillAccess<S, C> {
    type Input = Input;
    type Output = ();
    type Error = AccessDenied;

    async fn execute(self, input: Input) -> BizResult<(), AccessDenied> {
        self.authorize(
            &input.actor,
            input.guest_token_hash.as_ref(),
            &input.skill_id,
            input.operation,
        )
        .await
    }
}
