use crate::{
    hlist::hcons::{HCons, HList, HNil, Plucker, Selector},
    inject::{Inject, InjectAsync, SingletonContainer},
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub struct TypeBuilder<H> {
    instances: H,
    singletons: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl<T, I, H> Plucker<T, I> for TypeBuilder<H>
where
    H: Plucker<T, I>,
{
    type Remainder = TypeBuilder<<H as Plucker<T, I>>::Remainder>;

    fn pluck(self) -> (T, Self::Remainder) {
        let (t, remain) = self.instances.pluck();
        (
            t,
            TypeBuilder {
                instances: remain,
                singletons: self.singletons,
            },
        )
    }
}

impl<T, I, H> Selector<T, I> for TypeBuilder<H>
where
    H: Selector<T, I>,
{
    fn get(&self) -> &T {
        self.instances.get()
    }
}

impl<H> TypeBuilder<H> {
    pub fn build<T, I>(self) -> anyhow::Result<T>
    where
        T: Inject<Self, I>,
    {
        let (t, _) = <T as Inject<Self, I>>::inject(self)?;
        Ok(t)
    }

    pub async fn build_async<T, I>(self) -> anyhow::Result<T>
    where
        T: InjectAsync<Self, I>,
    {
        let (t, _) = <T as InjectAsync<Self, I>>::inject(self).await?;
        Ok(t)
    }
}

impl TypeBuilder<HNil> {
    pub fn new() -> Self {
        Self {
            instances: HNil,
            singletons: HashMap::new(),
        }
    }
}

impl<H> TypeBuilder<H>
where
    H: HList,
{
    pub fn add_instance<T>(self, instance: T) -> TypeBuilder<HCons<T, H>> {
        TypeBuilder {
            instances: self.instances.prepend(instance),
            singletons: self.singletons,
        }
    }
}

impl<H> SingletonContainer for TypeBuilder<H> {
    fn get_singleton<T: 'static>(&self) -> Option<&T> {
        self.singletons
            .get(&TypeId::of::<T>())
            .map(|t| t.downcast_ref().unwrap())
    }

    fn add_singleton<T: 'static + Send>(&mut self, t: T) {
        let _ = self.singletons.insert(t.type_id(), Box::new(t));
    }
}
