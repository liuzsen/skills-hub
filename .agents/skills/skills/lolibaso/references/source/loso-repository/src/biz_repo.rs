use anyhow::Result;
use loso_entity::Entity;

pub trait Repository<E: Entity>: Send {
    type Snapshot: EntitySnapshot<E>;

    type InsertConflict: Send;

    fn find(&mut self, id: &E::Id) -> impl Future<Output = Result<Option<Self::Snapshot>>> + Send;

    fn update(
        &mut self,
        snapshot: Self::Snapshot,
    ) -> impl Future<Output = Result<UpdateOutcome<Self::Snapshot>>> + Send;

    fn insert(
        &mut self,
        entity: &E,
    ) -> impl Future<Output = Result<InsertOutcome<Self::InsertConflict>>> + Send;
}

pub trait EntitySnapshot<T>: Send {
    fn entity(&self) -> &T;

    fn entity_mut(&mut self) -> &mut T;

    fn into_entity(self) -> T;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "the insert outcome must be handled"]
pub enum InsertOutcome<C> {
    Saved,
    Conflict(C),
}

pub enum UpdateOutcome<T> {
    Updated(T),
    StaleState,
}

pub trait FindBy<E, K>: Repository<E>
where
    E: Entity,
    K: Send,
{
    fn find_by(&mut self, key: K) -> impl Future<Output = Result<Option<Self::Snapshot>>> + Send;
}

pub trait DeleteEntity<E>: Repository<E>
where
    E: Entity,
{
    fn delete(
        &mut self,
        snapshot: Self::Snapshot,
    ) -> impl Future<Output = Result<DeleteOutcome>> + Send;
}

pub trait FindBatch<E, C>: Repository<E>
where
    E: Entity,
    C: Send,
{
    fn find_batch(
        &mut self,
        condition: C,
    ) -> impl Future<Output = Result<Vec<Self::Snapshot>>> + Send;
}

pub enum DeleteOutcome {
    Deleted,
    StaleState,
}
