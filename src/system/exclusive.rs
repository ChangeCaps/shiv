use std::marker::PhantomData;

use crate::{
    change_detection::MAX_CHANGE_AGE,
    storage::SparseArray,
    world::{World, WorldId},
};

use super::{
    ExclusiveSystemParam, ExclusiveSystemParamFetch, ExclusiveSystemParamItem,
    ExclusiveSystemParamState, In, InputMarker, IntoSystem, System, SystemMeta,
};

pub struct ExclusiveFunctionSystem<In, Out, Param, Marker, F>
where
    Param: ExclusiveSystemParam,
{
    func: F,
    param_state: Option<Param::Fetch>,
    meta: SystemMeta,
    last_change_ticks: SparseArray<u32>,
    world_id: Option<WorldId>,
    _marker: PhantomData<fn() -> (In, Out, Marker)>,
}

impl<In, Out, Param, Marker, F> ExclusiveFunctionSystem<In, Out, Param, Marker, F>
where
    Param: ExclusiveSystemParam,
{
    #[inline]
    fn store_last_change_tick(&mut self) {
        if let Some(index) = self.world_id.map(|id| id.index()) {
            (self.last_change_ticks).insert(index, self.meta.last_change_tick);
        }
    }

    #[inline]
    fn get_last_change_tick(&self, world: &World) -> u32 {
        let index = world.id().index();
        if let Some(&last_change_tick) = self.last_change_ticks.get(index) {
            last_change_tick
        } else {
            world.change_tick().wrapping_sub(MAX_CHANGE_AGE)
        }
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct IsExclusive;

impl<In, Out, Param, Marker, F> IntoSystem<In, Out, (Param, Marker, IsExclusive)> for F
where
    In: 'static,
    Out: 'static,
    Param: ExclusiveSystemParam + 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<In, Out, Param, Marker>,
{
    type System = ExclusiveFunctionSystem<In, Out, Param, Marker, F>;

    fn into_system(self) -> Self::System {
        ExclusiveFunctionSystem {
            func: self,
            param_state: None,
            meta: SystemMeta::new::<Self>(),
            last_change_ticks: SparseArray::new(),
            world_id: None,
            _marker: PhantomData,
        }
    }
}

impl<In, Out, Param, Marker, F> System for ExclusiveFunctionSystem<In, Out, Param, Marker, F>
where
    In: 'static,
    Out: 'static,
    Param: ExclusiveSystemParam + 'static,
    Marker: 'static,
    F: ExclusiveSystemParamFunction<In, Out, Param, Marker>,
{
    type In = In;
    type Out = Out;

    #[inline]
    fn meta(&self) -> &SystemMeta {
        &self.meta
    }

    #[inline]
    unsafe fn meta_mut(&mut self) -> &mut SystemMeta {
        &mut self.meta
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        true
    }

    #[inline]
    fn init(&mut self, world: &mut World) {
        self.store_last_change_tick();
        self.meta.last_change_tick = self.get_last_change_tick(world);

        self.param_state = Some(<Param::Fetch as ExclusiveSystemParamState>::init(
            world,
            &mut self.meta,
        ));

        self.world_id = Some(world.id());
    }

    #[inline]
    unsafe fn run_unchecked(&mut self, _input: Self::In, _world: &World) -> Self::Out {
        panic!();
    }

    #[inline]
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        if self.world_id != Some(world.id()) {
            self.init(world);
        }

        let param = <Param as ExclusiveSystemParam>::Fetch::get_param(
            self.param_state.as_mut().unwrap(),
            &self.meta,
        );

        self.func.run(input, world, param)
    }

    #[inline]
    fn apply(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().unwrap();
        param_state.apply(world);
    }
}

pub trait ExclusiveSystemParamFunction<In, Out, Param, Marker>: Send + Sync + 'static
where
    Param: ExclusiveSystemParam,
{
    fn run(&mut self, input: In, world: &mut World, item: ExclusiveSystemParamItem<Param>) -> Out;
}

macro_rules! impl_system_param_function {
    (@ $($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<Out, Func, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<(), Out, ($($param,)*), ()> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            Func: FnMut(&mut World, $($param),*) -> Out,
            Func: FnMut(&mut World, $(ExclusiveSystemParamItem<$param>),*) -> Out,
        {
            #[inline]
            fn run(&mut self, _input: (), world: &mut World, item: ExclusiveSystemParamItem<($($param,)*)>) -> Out {
                let ($($param,)*) = item;
                (self)(world, $($param),*)
            }
        }

        #[allow(non_snake_case)]
        impl<Input, Out, Func, $($param: ExclusiveSystemParam),*> ExclusiveSystemParamFunction<Input, Out, ($($param,)*), InputMarker> for Func
        where
            Out: 'static,
            Func: Send + Sync + 'static,
            Func: FnMut(In<Input>, &mut World, $($param),*) -> Out,
            Func: FnMut(In<Input>, &mut World, $(ExclusiveSystemParamItem<$param>),*) -> Out,
        {
            #[inline]
            fn run(&mut self, input: Input, world: &mut World, item: ExclusiveSystemParamItem<($($param,)*)>) -> Out {
                let ($($param,)*) = item;
                (self)(In::new(input), world, $($param),*)
            }
        }
    };
    ($start:ident $(,$ident:ident)*) => {
        impl_system_param_function!(@ $start $(,$ident)*);
        impl_system_param_function!($($ident),*);
    };
    () => {
        impl_system_param_function!(@);
    };
}

impl_system_param_function!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);
