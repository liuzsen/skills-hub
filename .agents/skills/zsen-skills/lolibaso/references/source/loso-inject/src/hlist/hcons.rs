use crate::hlist::indices::{Here, There};

#[derive(PartialEq, Debug, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct HCons<H, T> {
    pub head: H,
    pub tail: T,
}

#[derive(PartialEq, Debug, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub struct HNil;

pub trait Plucker<Target, Index> {
    type Remainder;

    fn pluck(self) -> (Target, Self::Remainder);
}

pub trait Selector<S, I> {
    fn get(&self) -> &S;
}

pub trait HList: Sized {
    fn prepend<H>(self, h: H) -> HCons<H, Self> {
        HCons {
            head: h,
            tail: self,
        }
    }
}

impl HList for HNil {}

impl<H, T: HList> HList for HCons<H, T> {}

impl<T, Tail> Plucker<T, Here> for HCons<T, Tail> {
    type Remainder = Tail;

    fn pluck(self) -> (T, Self::Remainder) {
        (self.head, self.tail)
    }
}

impl<Head, Tail, FromTail, TailIndex> Plucker<FromTail, There<TailIndex>> for HCons<Head, Tail>
where
    Tail: Plucker<FromTail, TailIndex>,
{
    type Remainder = HCons<Head, <Tail as Plucker<FromTail, TailIndex>>::Remainder>;

    fn pluck(self) -> (FromTail, Self::Remainder) {
        let (target, tail_remainder): (
            FromTail,
            <Tail as Plucker<FromTail, TailIndex>>::Remainder,
        ) = <Tail as Plucker<FromTail, TailIndex>>::pluck(self.tail);
        (
            target,
            HCons {
                head: self.head,
                tail: tail_remainder,
            },
        )
    }
}

impl<T, Tail> Selector<T, Here> for HCons<T, Tail> {
    fn get(&self) -> &T {
        &self.head
    }
}

impl<Head, Tail, FromTail, TailIndex> Selector<FromTail, There<TailIndex>> for HCons<Head, Tail>
where
    Tail: Selector<FromTail, TailIndex>,
{
    fn get(&self) -> &FromTail {
        self.tail.get()
    }
}
