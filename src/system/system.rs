use std::borrow::Cow;

use crate::{
    change_detection::MAX_CHANGE_AGE,
    storage::SparseArray,
    world::{ComponentId, World, WorldId},
};

use super::{
    FilteredAccess, ReadOnlySystemParamFetch, SystemParam, SystemParamFetch, SystemParamItem,
    SystemParamState,
};

pub type BoxedSystem<In, Out> = Box<dyn System<In = In, Out = Out>>;

#[derive(Debug)]
pub struct SystemMeta {
    pub name: Cow<'static, str>,
    pub access: FilteredAccess<ComponentId>,
    pub last_change_tick: u32,
}

impl SystemMeta {
    #[inline]
    pub fn new<T>() -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            access: FilteredAccess::default(),
            last_change_tick: 0,
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct SystemState<Param: SystemParam + 'static> {
    meta: SystemMeta,
    param_state: <Param as SystemParam>::Fetch,
    last_change_ticks: SparseArray<u32>,
    world_id: WorldId,
}

impl<Param: SystemParam + 'static> SystemState<Param> {
    #[inline]
    pub fn new(world: &mut World) -> Self {
        let mut meta = SystemMeta::new::<Param>();
        meta.last_change_tick = world.change_tick().wrapping_sub(MAX_CHANGE_AGE);
        let param_state = <Param::Fetch as SystemParamState>::init(world, &mut meta);
        let world_id = world.id();

        let mut last_change_ticks = SparseArray::new();
        last_change_ticks.insert(world_id.index(), meta.last_change_tick);

        Self {
            meta,
            param_state,
            last_change_ticks,
            world_id,
        }
    }

    #[inline]
    fn init(&mut self, world: &mut World) {
        if self.world_id == world.id() {
            return;
        }

        self.store_last_change_tick();
        self.meta.last_change_tick = self.get_last_change_tick(world);

        self.meta.access.clear();
        <Param::Fetch as SystemParamState>::init(world, &mut self.meta);

        self.world_id = world.id();
    }

    #[inline]
    fn store_last_change_tick(&mut self) {
        let index = self.world_id.index();
        (self.last_change_ticks).insert(index, self.meta.last_change_tick);
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

    #[inline]
    pub fn meta(&self) -> &SystemMeta {
        &self.meta
    }

    #[inline]
    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    #[inline]
    pub fn get<'w, 's>(&'s mut self, world: &'w World) -> SystemParamItem<'w, 's, Param>
    where
        Param::Fetch: ReadOnlySystemParamFetch,
    {
        self.validate_world(world);

        unsafe { self.get_unchecked_manual(world) }
    }

    #[inline]
    pub fn get_mut<'w, 's>(&'s mut self, world: &'w mut World) -> SystemParamItem<'w, 's, Param> {
        if self.world_id != world.id() {
            self.init(world);
        }

        unsafe { self.get_unchecked_manual(world) }
    }

    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        self.param_state.apply(world);
    }

    #[inline]
    pub fn matches_world(&self, world: &World) -> bool {
        self.world_id == world.id()
    }

    #[inline]
    fn validate_world(&mut self, world: &World) {
        if !self.matches_world(world) {
            panic!(
                "World mismatch: expected {:?}, got {:?}",
                self.world_id,
                world.id()
            );
        }
    }

    #[inline]
    pub unsafe fn get_unchecked_manual<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> <Param::Fetch as SystemParamFetch<'w, 's>>::Item {
        let change_tick = world.increment_change_tick();
        let param = unsafe {
            <Param::Fetch as SystemParamFetch>::get_param(
                &mut self.param_state,
                &self.meta,
                world,
                change_tick,
            )
        };
        self.meta.last_change_tick = change_tick;
        param
    }
}

pub trait System: Send + Sync + 'static {
    type In;
    type Out;

    fn meta(&self) -> &SystemMeta;

    /// # Safety
    /// - `SystemMeta::access` must not be modified
    unsafe fn meta_mut(&mut self) -> &mut SystemMeta;

    #[inline]
    fn is_exclusive(&self) -> bool {
        false
    }

    #[inline]
    fn init(&mut self, _world: &mut World) {}

    /// # Safety
    /// - `world` must be the same world that was used to [`System::init`] this system.
    /// - This doesn't check borrow rules, so it's up to the caller to that access is correct.
    unsafe fn run_unchecked(&mut self, input: Self::In, world: &World) -> Self::Out;

    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out;

    #[inline]
    fn apply(&mut self, _world: &mut World) {}

    #[inline]
    fn check_change_tick(&mut self, change_tick: u32) {
        // SAFETY: `SystemMeta::access` is not modified
        let meta = unsafe { self.meta_mut() };

        let age = change_tick.wrapping_sub(meta.last_change_tick);

        if age > MAX_CHANGE_AGE {
            meta.last_change_tick = change_tick.wrapping_sub(MAX_CHANGE_AGE);
        }
    }

    #[inline]
    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        // SAFETY: `SystemMeta::access` is not modified
        let meta = unsafe { self.meta_mut() };

        meta.last_change_tick = last_change_tick;
    }
}

impl<In: 'static, Out: 'static> std::fmt::Debug for dyn System<In = In, Out = Out> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System")
            .field("name", &self.meta().name)
            .finish()
    }
}
