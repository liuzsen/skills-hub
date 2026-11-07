use std::marker::PhantomData;

pub struct Here {
    _priv: (),
}

pub struct There<T> {
    _marker: PhantomData<T>,
}
