use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    Access, ChangeTicks, CommandQueue, Commands, ComponentId, Query, QueryState,
    ReadOnlyWorldQuery, Resource, SystemMeta, World, WorldQuery,
};

pub unsafe trait ReadOnlySystemParamFetch: for<'w, 's> SystemParamFetch<'w, 's> {}

pub trait SystemParam: Sized {
    type Fetch: for<'w, 's> SystemParamFetch<'w, 's>;
}

pub type SystemParamItem<'w, 's, P> = <<P as SystemParam>::Fetch as SystemParamFetch<'w, 's>>::Item;

pub unsafe trait SystemParamState: Send + Sync + 'static {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self;

    #[inline]
    fn apply(&mut self, _world: &mut World) {}
}

pub trait SystemParamFetch<'w, 's>: SystemParamState {
    type Item: SystemParam<Fetch = Self>;

    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item;
}

unsafe impl SystemParamState for CommandQueue {
    fn init(_world: &mut World, _meta: &mut SystemMeta) -> Self {
        Self::default()
    }

    fn apply(&mut self, world: &mut World) {
        self.apply(world);
    }
}

impl<'w, 's> SystemParamFetch<'w, 's> for CommandQueue {
    type Item = Commands<'w, 's>;

    unsafe fn get_param(
        &'s mut self,
        _meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        Commands::new(self, world)
    }
}

unsafe impl ReadOnlySystemParamFetch for CommandQueue {}

impl<'w, 's> SystemParam for Commands<'w, 's> {
    type Fetch = CommandQueue;
}

unsafe impl<Q, F> SystemParamState for QueryState<Q, F>
where
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    #[inline]
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        let state = QueryState::new(world);

        assert_access_compatibility(
            std::any::type_name::<Q>(),
            std::any::type_name::<F>(),
            state.filtered_access.access(),
            meta,
            world,
        );

        meta.access.extend(state.filtered_access.access());

        state
    }
}

impl<'w, 's, Q, F> SystemParamFetch<'w, 's> for QueryState<Q, F>
where
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Item = Query<'w, 's, Q, F>;

    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        unsafe { Query::new(world, self, meta.last_change_tick, change_tick) }
    }
}

impl<'w, 's, Q, F> SystemParam for Query<'w, 's, Q, F>
where
    Q: WorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    type Fetch = QueryState<Q, F>;
}

unsafe impl<Q, F> ReadOnlySystemParamFetch for QueryState<Q, F>
where
    Q: ReadOnlyWorldQuery + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
}

fn assert_access_compatibility(
    query_type: &'static str,
    filter_type: &'static str,
    access: &Access<ComponentId>,
    meta: &SystemMeta,
    world: &World,
) {
    let conflicts = access.get_conflicts(&meta.access);

    if conflicts.is_empty() {
        return;
    }

    let messages = conflicts
        .into_iter()
        .map(|id| {
            let info = world.components.get(id).unwrap();
            info.name()
        })
        .collect::<Vec<_>>();

    let message = messages.join(", ");

    panic!(
        "Query<{}, {}> in system {} access component(s) {} that conflict with previous system parameters.",
        query_type,
        filter_type,
        meta.name(),
        message,
    );
}

pub struct Res<'w, T> {
    value: &'w T,
    ticks: &'w ChangeTicks,
    last_change_tick: u32,
    change_tick: u32,
}

impl<'w, T> Res<'w, T> {
    #[inline]
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    #[inline]
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }
}

impl<'w, T> Deref for Res<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> AsRef<T> for Res<'w, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.value
    }
}

pub struct ResState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

unsafe impl<T: Resource> SystemParamState for ResState<T> {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        let component_id = world.components.init_resource::<T>();

        meta.access.add_read(component_id);

        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for ResState<T> {
    type Item = Res<'w, T>;

    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let (ptr, ticks) = world
            .resources
            .get_with_ticks(self.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by system {} does not exist: {}.",
                    meta.name(),
                    std::any::type_name::<T>()
                )
            });

        Res {
            value: unsafe { &*ptr.cast::<T>() },
            ticks: unsafe { &*ticks },
            last_change_tick: meta.last_change_tick,
            change_tick,
        }
    }
}

impl<'w, T: Resource> SystemParam for Res<'w, T> {
    type Fetch = ResState<T>;
}

unsafe impl<T: Resource> ReadOnlySystemParamFetch for ResState<T> {}

pub struct ResMut<'w, T> {
    value: &'w mut T,
    ticks: &'w mut ChangeTicks,
    last_change_tick: u32,
    change_tick: u32,
}

impl<'w, T> ResMut<'w, T> {
    #[inline]
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    #[inline]
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }
}

impl<'w, T> Deref for ResMut<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> DerefMut for ResMut<'w, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ticks.set_changed(self.change_tick);
        self.value
    }
}

pub struct ResMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

unsafe impl<T: Resource> SystemParamState for ResMutState<T> {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        let component_id = world.components.init_resource::<T>();

        meta.access.add_write(component_id);

        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for ResMutState<T> {
    type Item = ResMut<'w, T>;

    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let (ptr, ticks) = world
            .resources
            .get_with_ticks(self.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by system {} does not exist: {}.",
                    meta.name(),
                    std::any::type_name::<T>()
                )
            });

        ResMut {
            value: unsafe { &mut *ptr.cast::<T>() },
            ticks: unsafe { &mut *ticks },
            last_change_tick: meta.last_change_tick,
            change_tick,
        }
    }
}

impl<'w, T: Resource> SystemParam for ResMut<'w, T> {
    type Fetch = ResMutState<T>;
}

macro_rules! impl_system_param {
    (@ $($param:ident),*) => {
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type Fetch = ($($param::Fetch,)*);
        }

        #[allow(non_snake_case, unused)]
        unsafe impl<$($param: SystemParamState),*> SystemParamState for ($($param,)*) {
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
        impl<'w, 's, $($param: SystemParamFetch<'w, 's>),*> SystemParamFetch<'w, 's> for ($($param,)*) {
            type Item = ($($param::Item,)*);

            unsafe fn get_param(
                &'s mut self,
                meta: &SystemMeta,
                world: &'w World,
                change_tick: u32,
            ) -> Self::Item {
                let ($($param,)*) = self;
                unsafe { ($($param.get_param(meta, world, change_tick),)*) }
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

impl_system_param!(A, B, C, D, E, F, G, H, I, J, K, L);
