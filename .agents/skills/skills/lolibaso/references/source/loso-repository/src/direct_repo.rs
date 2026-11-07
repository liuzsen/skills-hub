use anyhow::Result;
use loso_entity::Entity;

pub trait DirectDelete<E, K>: Send
where
    E: Entity,
    K: Send,
{
    fn direct_delete(&mut self, key: K)
    -> impl Future<Output = Result<DirectDeleteOutcome>> + Send;
}

pub enum DirectDeleteOutcome {
    Deleted,
    NotFound,
}

pub trait DirectPatch<P: Send>: Send {
    fn apply_patch(&mut self, patch: P) -> impl Future<Output = Result<PatchOutcome>> + Send;
}

pub enum PatchOutcome {
    Updated,
    NotFound,
}
