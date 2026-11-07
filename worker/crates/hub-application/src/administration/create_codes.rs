use crate::{
    time::Clock,
    tokens::{self, TokenSource},
};
use hub_domain::account::{AccountId, Actor};
use lolibaso::biz::{BizResult, UseCase};

pub const MAX_CODES_PER_BATCH: u32 = 25;
pub enum Error {
    Unauthenticated,
    Forbidden,
    InvalidCount,
    InvalidSkillId,
    SkillNotFound,
}

pub struct CodeRecord {
    pub id: String,
    pub code_hash: String,
}
pub struct CodeBatch {
    pub batch_id: String,
    pub skill_id: String,
    pub created_by: AccountId,
    pub created_at: i64,
    pub codes: Vec<CodeRecord>,
}

pub trait CodeIssuer: Send {
    /// One atomic batch. False means the skill does not exist and no codes were inserted.
    fn create_codes(&self, batch: CodeBatch) -> impl Future<Output = anyhow::Result<bool>> + Send;
}

pub struct Input {
    pub actor: Actor,
    pub skill_id: String,
    pub count: u32,
}
pub struct CreatedCode {
    pub id: String,
    pub code: String,
}
/// Plaintext exists only in this one-time response, not in the issuer port.
pub struct Output {
    pub skill_id: String,
    pub codes: Vec<CreatedCode>,
}
pub struct CreateCodes<S, C, T> {
    pub store: S,
    pub clock: C,
    pub tokens: T,
}

impl<S: CodeIssuer, C: Clock, T: TokenSource> UseCase for CreateCodes<S, C, T> {
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: Input) -> BizResult<Output, Error> {
        let Some(account) = input.actor.account() else {
            return Ok(Err(Error::Unauthenticated));
        };
        if !account.is_admin() {
            return Ok(Err(Error::Forbidden));
        }
        if !(1..=MAX_CODES_PER_BATCH).contains(&input.count) {
            return Ok(Err(Error::InvalidCount));
        }
        if input.skill_id.is_empty() || input.skill_id.len() > 256 {
            return Ok(Err(Error::InvalidSkillId));
        }
        let mut records = Vec::with_capacity(input.count as usize);
        let mut codes = Vec::with_capacity(input.count as usize);
        for _ in 0..input.count {
            let id = tokens::opaque(&self.tokens)?;
            let code = tokens::opaque(&self.tokens)?;
            records.push(CodeRecord {
                id: id.clone(),
                code_hash: tokens::digest(&code),
            });
            codes.push(CreatedCode { id, code });
        }
        let batch = CodeBatch {
            batch_id: tokens::opaque(&self.tokens)?,
            skill_id: input.skill_id.clone(),
            created_by: account.id().clone(),
            created_at: self.clock.now(),
            codes: records,
        };
        if !self.store.create_codes(batch).await? {
            return Ok(Err(Error::SkillNotFound));
        }
        Ok(Ok(Output {
            skill_id: input.skill_id,
            codes,
        }))
    }
}
