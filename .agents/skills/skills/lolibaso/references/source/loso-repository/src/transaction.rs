pub use scoped_futures::ScopedBoxFuture;

use crate::biz_repo::Repository;
use loso_biz::BizResult;
use loso_entity::Entity;

pub trait TransactionalRepository<E: Entity>: Repository<E> {
    fn transaction<'a, Tx, T, Err>(
        &'a mut self,
        tx: Tx,
    ) -> impl Future<Output = BizResult<T, Err>> + Send
    where
        for<'repo> Tx:
            Send + FnOnce(&'repo mut Self) -> ScopedBoxFuture<'a, 'repo, BizResult<T, Err>>,
        T: Send,
        Err: Send;
}

pub trait RepositoryFactory: Send {
    type Repository: Send;

    fn build(&self) -> anyhow::Result<Self::Repository>;
}
