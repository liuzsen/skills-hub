pub trait Inject<Context, Index>: Sized {
    type OutContext;

    fn inject(context: Context) -> anyhow::Result<(Self, Self::OutContext)>;
}

pub trait InjectAsync<Context, Index>: Sized + Send {
    type OutContext;

    fn inject(
        context: Context,
    ) -> impl Future<Output = anyhow::Result<(Self, Self::OutContext)>> + Send;
}

pub trait SingletonContainer {
    fn get_singleton<T: 'static>(&self) -> Option<&T>;

    fn add_singleton<T: 'static + Send>(&mut self, t: T);
}

macro_rules! impl_inject_for_tuple {
    ($($T:ident, $Idx:ident),+) => {
        impl_inject_for_tuple!(@impl Context; []; []; $($T, $Idx),+);
        impl_inject_async_for_tuple!(@impl Context; []; []; $($T, $Idx),+);
    };

    (@impl $ctx:ty; [$($prev_T:ident, $prev_Idx:ident),*]; [$($where_bound:tt)*]; $last_T:ident, $last_Idx:ident) => {
        #[allow(non_snake_case)]
        impl<Context, $($prev_T, $prev_Idx,)* $last_T, $last_Idx>
            Inject<Context, ($($prev_Idx,)* $last_Idx,)>
            for ($($prev_T,)* $last_T,)
        where
            $($where_bound)*
            $last_T: Inject<$ctx, $last_Idx>,
        {
            type OutContext = $last_T::OutContext;
            fn inject(context: Context) -> anyhow::Result<(Self, Self::OutContext)> {
                $(let ($prev_T, context) = <$prev_T as Inject<_, $prev_Idx>>::inject(context)?;)*
                let ($last_T, context) = <$last_T as Inject<_, $last_Idx>>::inject(context)?;
                Ok((($($prev_T,)* $last_T,), context))
            }
        }
    };

    (@impl $ctx:ty; [$($prev_T:ident, $prev_Idx:ident),*]; [$($where_bound:tt)*]; $T:ident, $Idx:ident, $($rest_T:ident, $rest_Idx:ident),+) => {
        impl_inject_for_tuple!(
            @impl <$T as Inject<$ctx, $Idx>>::OutContext;
            [$($prev_T, $prev_Idx,)* $T, $Idx];
            [$($where_bound)* $T: Inject<$ctx, $Idx>,];
            $($rest_T, $rest_Idx),+
        );
    };
}

macro_rules! impl_inject_async_for_tuple {
    (@impl $ctx:ty; [$($prev_T:ident, $prev_Idx:ident),*]; [$($where_bound:tt)*]; $last_T:ident, $last_Idx:ident) => {
        #[allow(non_snake_case)]
        impl<Context, $($prev_T, $prev_Idx,)* $last_T, $last_Idx>
            InjectAsync<Context, ($($prev_Idx,)* $last_Idx,)>
            for ($($prev_T,)* $last_T,)
        where
            $($where_bound)*
            $last_T: InjectAsync<$ctx, $last_Idx>,
            Context: Send,
        {
            type OutContext = $last_T::OutContext;

            async fn inject(context: Context) -> anyhow::Result<(Self, Self::OutContext)> {
                $(let ($prev_T, context) = <$prev_T as InjectAsync<_, $prev_Idx>>::inject(context).await?;)*
                let ($last_T, context) = <$last_T as InjectAsync<_, $last_Idx>>::inject(context).await?;
                Ok((($($prev_T,)* $last_T,), context))
            }
        }
    };

    (@impl $ctx:ty; [$($prev_T:ident, $prev_Idx:ident),*]; [$($where_bound:tt)*]; $T:ident, $Idx:ident, $($rest_T:ident, $rest_Idx:ident),+) => {
        impl_inject_async_for_tuple!(
            @impl <$T as InjectAsync<$ctx, $Idx>>::OutContext;
            [$($prev_T, $prev_Idx,)* $T, $Idx];
            [$($where_bound)* $T: InjectAsync<$ctx, $Idx> + Send,];
            $($rest_T, $rest_Idx),+
        );
    };
}

impl_inject_for_tuple!(T0, Idx0);
impl_inject_for_tuple!(T0, Idx0, T1, Idx1);
impl_inject_for_tuple!(T0, Idx0, T1, Idx1, T2, Idx2);
impl_inject_for_tuple!(T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3);
impl_inject_for_tuple!(T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4);
impl_inject_for_tuple!(T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4, T5, Idx5);
impl_inject_for_tuple!(
    T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4, T5, Idx5, T6, Idx6
);
impl_inject_for_tuple!(
    T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4, T5, Idx5, T6, Idx6, T7, Idx7
);
impl_inject_for_tuple!(
    T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4, T5, Idx5, T6, Idx6, T7, Idx7, T8, Idx8
);
impl_inject_for_tuple!(
    T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4, T5, Idx5, T6, Idx6, T7, Idx7, T8, Idx8, T9,
    Idx9
);
impl_inject_for_tuple!(
    T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4, T5, Idx5, T6, Idx6, T7, Idx7, T8, Idx8, T9,
    Idx9, T10, Idx10
);
impl_inject_for_tuple!(
    T0, Idx0, T1, Idx1, T2, Idx2, T3, Idx3, T4, Idx4, T5, Idx5, T6, Idx6, T7, Idx7, T8, Idx8, T9,
    Idx9, T10, Idx10, T11, Idx11
);
