use crate::{
    query::{QueryState, ReadOnlyWorldQuery, WorldQuery},
    world::{FromWorld, World},
};

use super::{Local, SystemMeta, SystemParam, SystemState};

pub type ExclusiveSystemParamItem<'w, T> =
    <<T as ExclusiveSystemParam>::Fetch as ExclusiveSystemParamFetch<'w>>::Item;

pub trait ExclusiveSystemParam {
    type Fetch: for<'s> ExclusiveSystemParamFetch<'s>;
}

pub trait ExclusiveSystemParamState: Send + Sync {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self;

    #[inline]
    fn apply(&mut self, _world: &mut World) {}
}

pub trait ExclusiveSystemParamFetch<'s>: ExclusiveSystemParamState {
    type Item: ExclusiveSystemParam<Fetch = Self>;

    fn get_param(&'s mut self, meta: &SystemMeta) -> Self::Item;
}

impl<'s, Q, F> ExclusiveSystemParam for &'s mut QueryState<Q, F>
where
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Fetch = QueryState<Q, F>;
}

impl<Q, F> ExclusiveSystemParamState for QueryState<Q, F>
where
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    #[inline]
    fn init(world: &mut World, _meta: &mut SystemMeta) -> Self {
        QueryState::new(world)
    }
}

impl<'s, Q, F> ExclusiveSystemParamFetch<'s> for QueryState<Q, F>
where
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Item = &'s mut QueryState<Q, F>;

    #[inline]
    fn get_param(&'s mut self, _meta: &SystemMeta) -> Self::Item {
        self
    }
}

impl<'s, P: SystemParam + 's> ExclusiveSystemParam for &'s mut SystemState<P> {
    type Fetch = SystemState<P>;
}

impl<P: SystemParam> ExclusiveSystemParamState for SystemState<P> {
    #[inline]
    fn init(world: &mut World, _meta: &mut SystemMeta) -> Self {
        SystemState::new(world)
    }
}

impl<'s, P: SystemParam + 's> ExclusiveSystemParamFetch<'s> for SystemState<P> {
    type Item = &'s mut SystemState<P>;

    #[inline]
    fn get_param(&'s mut self, _meta: &SystemMeta) -> Self::Item {
        self
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ExclusiveLocalState<T: Send + Sync + 'static> {
    value: T,
}

impl<'s, T: FromWorld + Send + Sync + 'static> ExclusiveSystemParam for Local<'s, T> {
    type Fetch = ExclusiveLocalState<T>;
}

impl<T: FromWorld + Send + Sync + 'static> ExclusiveSystemParamState for ExclusiveLocalState<T> {
    #[inline]
    fn init(world: &mut World, _meta: &mut SystemMeta) -> Self {
        ExclusiveLocalState {
            value: T::from_world(world),
        }
    }
}

impl<'s, T> ExclusiveSystemParamFetch<'s> for ExclusiveLocalState<T>
where
    T: FromWorld + Send + Sync + 'static,
{
    type Item = Local<'s, T>;

    #[inline]
    fn get_param(&'s mut self, _meta: &SystemMeta) -> Self::Item {
        Local {
            value: &mut self.value,
        }
    }
}

macro_rules! impl_system_param {
    (@ $($param:ident),*) => {
        impl<$($param: ExclusiveSystemParam),*> ExclusiveSystemParam for ($($param,)*) {
            type Fetch = ($($param::Fetch,)*);
        }

        #[allow(non_snake_case, unused)]
        impl<$($param: ExclusiveSystemParamState),*> ExclusiveSystemParamState for ($($param,)*) {
            #[inline]
            fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
                ($($param::init(world, meta),)*)
            }

            #[inline]
            fn apply(&mut self, world: &mut World) {
                let ($($param,)*) = self;
                $($param.apply(world);)*
            }
        }

        #[allow(non_snake_case, unused)]
        impl<'s, $($param: ExclusiveSystemParamFetch<'s>),*> ExclusiveSystemParamFetch<'s> for ($($param,)*) {
            type Item = ($($param::Item,)*);

            fn get_param(&'s mut self, meta: &SystemMeta) -> Self::Item {
                let ($($param,)*) = self;
                unsafe { ($($param.get_param(meta),)*) }
            }
        }
    };
    ($start:ident $(,$ident:ident)*) => {
        impl_system_param!(@ $start $(,$ident)*);
        impl_system_param!($($ident),*);
    };
    () => {
        impl_system_param!(@);
    };
}

impl_system_param!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
