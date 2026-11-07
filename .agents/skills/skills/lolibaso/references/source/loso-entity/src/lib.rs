pub use loso_macros::Entity;

pub trait Entity: Send + Sync + 'static {
    type Id: EntityId;

    type Readonly<'a>
    where
        Self: 'a;

    fn id(&self) -> &Self::Id;

    fn readonly(&self) -> Self::Readonly<'_>;
}

/// Consumes an entity, moving out its identity and owned state.
pub trait IntoParts: Entity + Sized {
    type State;

    fn into_parts(self) -> (Self::Id, Self::State);
}

pub trait EntityId: Eq + Clone + std::fmt::Debug + std::hash::Hash + Send + Sync + 'static {}

pub trait IdGenerator {
    type Id: EntityId;

    fn generate(&mut self) -> Self::Id;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` must define how persisted state is restored",
    note = "implement Reconstitute for {Self} with the generated State required by #[derive(Entity)], or opt out with #[entity(no_state)]"
)]
pub trait Reconstitute: Entity + Sized {
    type State;
    type Error;

    fn reconstitute(id: Self::Id, state: Self::State) -> Result<Self, Self::Error>;
}
