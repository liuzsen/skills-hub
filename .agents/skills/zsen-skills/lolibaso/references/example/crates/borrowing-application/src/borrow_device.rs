use borrowing_domain::device::{ActorId, BorrowError, DeviceId};
use lolibaso::{
    biz::{BizResult, UseCase, biz_try},
    biz_err, biz_ok, ensure_exist,
    repository::{EntitySnapshot, UpdateOutcome},
};

use crate::CirculationRepository;

pub struct Input {
    pub device_id: DeviceId,
    pub actor_id: ActorId,
}

pub type Output = ();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotFound,
    Rejected(BorrowError),
    ConcurrentChange,
}

impl From<BorrowError> for Error {
    fn from(error: BorrowError) -> Self {
        Self::Rejected(error)
    }
}

#[derive(lolibaso::inject::Inject)]
pub struct BorrowDevice<R> {
    pub repository: R,
}

impl<R: CirculationRepository> UseCase for BorrowDevice<R> {
    type Input = Input;
    type Output = Output;
    type Error = Error;

    async fn execute(mut self, input: Input) -> BizResult<Output, Error> {
        self.repository
            .transaction(move |repo| Box::pin(borrow(repo, input)))
            .await
    }
}

async fn borrow<R: CirculationRepository>(repo: &mut R, input: Input) -> BizResult<Output, Error> {
    let mut snapshot = ensure_exist!(repo.find(&input.device_id).await?, Error::NotFound);
    let movement = biz_try!(snapshot.entity_mut().borrow(input.actor_id));
    match repo.update(snapshot).await? {
        UpdateOutcome::Updated(_) => {}
        UpdateOutcome::StaleState => return biz_err!(Error::ConcurrentChange),
    }
    repo.append_movement(movement).await?;
    biz_ok!(())
}
