use anyhow::Result;
use loso_entity::Entity;

pub trait ExistenceCheck<E, K>: Send
where
    E: Entity,
    K: Sync + ?Sized,
{
    fn exists(&mut self, key: &K) -> impl Future<Output = Result<bool>> + Send;
}
