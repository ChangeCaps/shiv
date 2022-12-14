use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    change_detection::ChangeTicks,
    query::{Query, QueryState, ReadOnlyWorldQuery, WorldQuery},
    storage::Resource,
    world::{ComponentId, FromWorld, World},
};

use super::{CommandQueue, Commands, FilteredAccess, SystemMeta};

/// Derive macro for [`SystemParam`].
///
/// For more information, see [`SystemParam`].
pub use shiv_macro::SystemParam;

pub unsafe trait ReadOnlySystemParamFetch: for<'w, 's> SystemParamFetch<'w, 's> {}

/// A trait that allows a type to be used as a parameter to a system.
///
/// This trait can be derived using the `#[derive(SystemParam)]` attribute.
///
/// # Examples
/// ```rust
/// # use shiv::prelude::*;
/// // only 'w and 's are allowed as lifetime parameters
/// #[derive(SystemParam)]
/// struct MyParam<'w, 's> {
///     local: Local<'s, u32>,
///     resource: Res<'w, String>,
/// }
/// ```
pub trait SystemParam: Sized {
    type Fetch: for<'w, 's> SystemParamFetch<'w, 's>;
}

/// The item fetched by a [`SystemParam`].
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

#[doc(hidden)]
#[derive(Debug)]
pub struct WorldFetch;

impl SystemParam for &World {
    type Fetch = WorldFetch;
}

unsafe impl SystemParamState for WorldFetch {
    fn init(_world: &mut World, meta: &mut SystemMeta) -> Self {
        if meta.access.write_any() {
            panic!("&World cannot be used as a parameter to a system that writes to any component or resource");
        }

        meta.access.read_all();

        WorldFetch
    }
}

impl<'w, 's> SystemParamFetch<'w, 's> for WorldFetch {
    type Item = &'w World;

    unsafe fn get_param(
        &'s mut self,
        _meta: &SystemMeta,
        world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        world
    }
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
            &state.filtered_access,
            meta,
            world,
        );

        meta.access.extend(&state.filtered_access);

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
    access: &FilteredAccess<ComponentId>,
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

#[derive(Debug)]
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

    #[inline]
    pub fn into_inner(self) -> &'w T {
        self.value
    }

    pub fn ticks(&self) -> &ChangeTicks {
        self.ticks
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

#[doc(hidden)]
#[derive(Debug)]
pub struct ResState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

unsafe impl<T: Resource> SystemParamState for ResState<T> {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        let component_id = world.components.init_resource::<T>();

        assert!(
            !meta.access.has_write(component_id),
            "Res<{}> in system {} conflicts with previous access in this query. Shared access cannot coexist with exclusive access.", 
            std::any::type_name::<T>(),
            meta.name(),
        );

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
            .storage
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

#[derive(Debug)]
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

    #[inline]
    pub fn into_inner(self) -> &'w mut T {
        self.value
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

#[doc(hidden)]
#[derive(Debug)]
pub struct ResMutState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

unsafe impl<T: Resource> SystemParamState for ResMutState<T> {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        let component_id = world.components.init_resource::<T>();

        assert!(
            !meta.access.has_read(component_id),
            "ResMut<{}> in system {} conflicts with previous system parameters. Mutable resource access must be unique.",
            std::any::type_name::<T>(),
            meta.name(),
        );

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
            .storage
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

#[derive(Debug)]
pub struct ResInit<'w, T> {
    value: &'w T,
    ticks: &'w ChangeTicks,
    last_change_tick: u32,
    change_tick: u32,
}

impl<'w, T> ResInit<'w, T> {
    #[inline]
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    #[inline]
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }

    #[inline]
    pub fn into_inner(self) -> &'w T {
        self.value
    }
}

impl<'w, T> Deref for ResInit<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> AsRef<T> for ResInit<'w, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.value
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ResInitState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

unsafe impl<T: Resource + FromWorld> SystemParamState for ResInitState<T> {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        world.init_resource::<T>();

        let component_id = world.components.init_resource::<T>();

        assert!(
            !meta.access.has_write(component_id),
            "Res<{}> in system {} conflicts with previous access in this query. Shared access cannot coexist with exclusive access.", 
            std::any::type_name::<T>(),
            meta.name(),
        );

        meta.access.add_read(component_id);

        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'w, 's, T: Resource + FromWorld> SystemParamFetch<'w, 's> for ResInitState<T> {
    type Item = ResInit<'w, T>;

    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let (ptr, ticks) = world
            .storage
            .resources
            .get_with_ticks(self.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by system {} does not exist: {}.",
                    meta.name(),
                    std::any::type_name::<T>()
                )
            });

        ResInit {
            value: unsafe { &*ptr.cast::<T>() },
            ticks: unsafe { &*ticks },
            last_change_tick: meta.last_change_tick,
            change_tick,
        }
    }
}

impl<'w, T: Resource + FromWorld> SystemParam for ResInit<'w, T> {
    type Fetch = ResInitState<T>;
}

unsafe impl<T: Resource + FromWorld> ReadOnlySystemParamFetch for ResInitState<T> {}

#[derive(Debug)]
pub struct ResMutInit<'w, T> {
    value: &'w mut T,
    ticks: &'w mut ChangeTicks,
    last_change_tick: u32,
    change_tick: u32,
}

impl<'w, T> ResMutInit<'w, T> {
    #[inline]
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    #[inline]
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }

    #[inline]
    pub fn into_inner(self) -> &'w mut T {
        self.value
    }
}

impl<'w, T> Deref for ResMutInit<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> DerefMut for ResMutInit<'w, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ticks.set_changed(self.change_tick);
        self.value
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ResMutInitState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

unsafe impl<T: Resource + FromWorld> SystemParamState for ResMutInitState<T> {
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        world.init_resource::<T>();

        let component_id = world.components.init_resource::<T>();

        assert!(
            !meta.access.has_read(component_id),
            "ResMut<{}> in system {} conflicts with previous system parameters. Mutable resource access must be unique.",
            std::any::type_name::<T>(),
            meta.name(),
        );

        meta.access.add_write(component_id);

        Self {
            component_id,
            marker: PhantomData,
        }
    }
}

impl<'w, 's, T: Resource + FromWorld> SystemParamFetch<'w, 's> for ResMutInitState<T> {
    type Item = ResMutInit<'w, T>;

    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let (ptr, ticks) = world
            .storage
            .resources
            .get_with_ticks(self.component_id)
            .unwrap_or_else(|| {
                panic!(
                    "Resource requested by system {} does not exist: {}.",
                    meta.name(),
                    std::any::type_name::<T>()
                )
            });

        ResMutInit {
            value: unsafe { &mut *ptr.cast::<T>() },
            ticks: unsafe { &mut *ticks },
            last_change_tick: meta.last_change_tick,
            change_tick,
        }
    }
}

impl<'w, T: Resource + FromWorld> SystemParam for ResMutInit<'w, T> {
    type Fetch = ResMutInitState<T>;
}

#[doc(hidden)]
#[derive(Debug)]
pub struct OptionResState<T>(ResState<T>);

unsafe impl<T: Resource> ReadOnlySystemParamFetch for OptionResState<T> {}

unsafe impl<T: Resource> SystemParamState for OptionResState<T> {
    #[inline]
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        Self(ResState::init(world, meta))
    }
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for OptionResState<T> {
    type Item = Option<Res<'w, T>>;

    #[inline]
    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        if world.storage.resources.contains(self.0.component_id) {
            Some(unsafe { ResState::get_param(&mut self.0, meta, world, change_tick) })
        } else {
            None
        }
    }
}

impl<'w, T: Resource> SystemParam for Option<Res<'w, T>> {
    type Fetch = OptionResState<T>;
}

#[doc(hidden)]
#[derive(Debug)]
pub struct OptionResMutState<T>(ResMutState<T>);

unsafe impl<T: Resource> SystemParamState for OptionResMutState<T> {
    #[inline]
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        Self(ResMutState::init(world, meta))
    }
}

impl<'w, 's, T: Resource> SystemParamFetch<'w, 's> for OptionResMutState<T> {
    type Item = Option<ResMut<'w, T>>;

    #[inline]
    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        if world.storage.resources.contains(self.0.component_id) {
            Some(unsafe { ResMutState::get_param(&mut self.0, meta, world, change_tick) })
        } else {
            None
        }
    }
}

impl<'w, T: Resource> SystemParam for Option<ResMut<'w, T>> {
    type Fetch = OptionResMutState<T>;
}

#[derive(Debug)]
pub struct Local<'s, T: FromWorld + Send + 'static> {
    pub(crate) value: &'s mut T,
}

impl<'s, T> Deref for Local<'s, T>
where
    T: FromWorld + Send + 'static,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'s, T> DerefMut for Local<'s, T>
where
    T: FromWorld + Send + 'static,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct LocalState<T: Send + 'static> {
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for LocalState<T> {}

unsafe impl<T: FromWorld + Send + 'static> ReadOnlySystemParamFetch for LocalState<T> {}

unsafe impl<T: FromWorld + Send + 'static> SystemParamState for LocalState<T> {
    #[inline]
    fn init(world: &mut World, _meta: &mut SystemMeta) -> Self {
        Self {
            value: UnsafeCell::new(T::from_world(world)),
        }
    }
}

impl<'w, 's, T: FromWorld + Send + 'static> SystemParamFetch<'w, 's> for LocalState<T> {
    type Item = Local<'s, T>;

    #[inline]
    unsafe fn get_param(
        &'s mut self,
        _meta: &SystemMeta,
        _world: &'w World,
        _change_tick: u32,
    ) -> Self::Item {
        Local {
            value: unsafe { &mut *self.value.get() },
        }
    }
}

impl<'w, T: FromWorld + Send + 'static> SystemParam for Local<'w, T> {
    type Fetch = LocalState<T>;
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

impl_system_param!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
