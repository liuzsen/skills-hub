use borrowing_domain::device::{Device, DeviceId, InvalidNumber};
use lolibaso::{
    biz::{BizResult, UseCase, biz_try},
    biz_err, biz_ok,
    entity::IdGenerator,
    repository::{InsertOutcome, Repository},
};

pub struct Input {
    pub number: String,
}

pub type Output = DeviceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidNumber,
    NumberTaken,
}

impl From<InvalidNumber> for Error {
    fn from(_: InvalidNumber) -> Self {
        Self::InvalidNumber
    }
}

/// Only a number uniqueness violation is a business conflict. ID collisions and
/// storage failures are technical errors. Uniqueness includes deregistered devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertConflict {
    NumberTaken,
}

#[derive(lolibaso::inject::Inject)]
pub struct Register<R, G> {
    pub repository: R,
    pub ids: G,
}

impl<R, G> UseCase for Register<R, G>
where
    R: Repository<Device, InsertConflict = InsertConflict>,
    G: IdGenerator<Id = DeviceId> + Send,
{
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(mut self, input: Input) -> BizResult<Output, Error> {
        let id = self.ids.generate();
        let device = biz_try!(Device::register(id, input.number));
        match self.repository.insert(&device).await? {
            InsertOutcome::Saved => biz_ok!(id),
            InsertOutcome::Conflict(InsertConflict::NumberTaken) => biz_err!(Error::NumberTaken),
        }
    }
}
