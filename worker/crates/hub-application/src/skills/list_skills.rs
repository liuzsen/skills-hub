use super::{Error, SkillCatalog, SkillSummary};
use hub_domain::skill::SkillId;
use lolibaso::biz::{BizResult, UseCase};

pub struct Input {
    pub after: Option<SkillId>,
    pub limit: u32,
}
pub struct Output {
    pub items: Vec<SkillSummary>,
    pub next_after: Option<SkillId>,
}
pub struct ListSkills<S> {
    pub catalog: S,
}

impl<S: SkillCatalog> UseCase for ListSkills<S> {
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(self, input: Input) -> BizResult<Output, Error> {
        if !(1..=100).contains(&input.limit) {
            return Ok(Err(Error::InvalidInput));
        }
        let mut items = self
            .catalog
            .list(input.after.as_ref(), input.limit + 1)
            .await?;
        let next_after = if items.len() > input.limit as usize {
            items.truncate(input.limit as usize);
            items.last().map(|item| item.id.clone())
        } else {
            None
        };
        Ok(Ok(Output { items, next_after }))
    }
}
