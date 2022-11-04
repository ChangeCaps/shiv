use std::{
    mem::MaybeUninit,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    Component, ComponentId, ComponentStorage, Components, Entities, Entity, QueryState,
    ReadOnlyWorldQuery, Storage, StorageSet, WorldQuery,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldId(usize);

impl WorldId {
    #[inline]
    pub fn new() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::AcqRel) as usize)
    }
}

pub struct World {
    id: WorldId,
    pub(crate) entities: Entities,
    pub(crate) storage: ComponentStorage,
    pub(crate) components: Components,
    pub(crate) change_tick: AtomicU32,
    pub(crate) last_change_tick: u32,
}

impl Default for World {
    #[inline]
    fn default() -> Self {
        Self {
            id: WorldId::new(),
            entities: Entities::default(),
            storage: ComponentStorage::default(),
            components: Components::default(),
            change_tick: AtomicU32::new(1),
            last_change_tick: 0,
        }
    }
}

impl World {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn id(&self) -> WorldId {
        self.id
    }

    #[inline]
    pub fn reserve_entity(&mut self) -> Entity {
        self.entities.allocate()
    }

    #[inline]
    pub fn contains_entity(&self, entity: Entity) -> bool {
        self.entities.contains(entity)
    }

    #[inline]
    pub fn init_component<T: Component>(&mut self) -> ComponentId {
        let id = self.components.init_component::<T>();
        let info = unsafe { self.components.get_unchecked(id) };

        let storage_sets = <T::Storage as Storage>::get_mut(&mut self.storage);
        storage_sets.initialize(info);

        id
    }

    #[inline]
    pub fn insert_component<T: Component>(&mut self, entity: Entity, mut component: T) {
        let id = self.init_component::<T>();

        let change_tick = self.change_tick();

        let storage_sets = <T::Storage as Storage>::get_mut(&mut self.storage);
        let storage = unsafe { storage_sets.get_unchecked_mut(id) };

        unsafe { storage.insert(entity, &mut component as *mut T as *mut u8, change_tick) };
    }

    #[inline]
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let id = self.init_component::<T>();

        let storage_sets = <T::Storage as Storage>::get_mut(&mut self.storage);
        let storage = unsafe { storage_sets.get_unchecked_mut(id) };

        let mut component = MaybeUninit::<T>::uninit();

        unsafe { storage.remove_unchecked(entity, component.as_mut_ptr() as *mut u8) }

        Some(unsafe { component.assume_init() })
    }

    #[inline]
    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.storage.remove(entity);
        self.entities.free(entity)
    }

    #[inline]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        todo!()
    }

    #[inline]
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        todo!()
    }

    #[inline]
    pub fn get_component_raw<T: Component>(&self, entity: Entity) -> Option<*mut T> {
        todo!()
    }
}

impl World {
    #[inline]
    pub fn query<Q: WorldQuery>(&mut self) -> QueryState<Q, ()> {
        QueryState::new(self)
    }

    #[inline]
    pub fn query_filtered<Q: WorldQuery, F: ReadOnlyWorldQuery>(&mut self) -> QueryState<Q, F> {
        QueryState::new(self)
    }
}

impl World {
    #[inline]
    pub fn increment_change_tick(&self) -> u32 {
        self.change_tick.fetch_add(1, Ordering::AcqRel)
    }

    #[inline]
    pub fn change_tick(&self) -> u32 {
        self.change_tick.load(Ordering::Acquire)
    }

    #[inline]
    pub fn set_last_change_tick(&mut self, tick: u32) {
        self.last_change_tick = tick;
    }

    #[inline]
    pub fn last_change_tick(&self) -> u32 {
        self.last_change_tick
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    impl Component for i32 {
        type Storage = SparseStorage;
    }

    impl Component for bool {
        type Storage = SparseStorage;
    }

    #[test]
    fn components() {
        let mut world = World::new();
        let entity = world.reserve_entity();
        world.insert_component(entity, 2i32);
        world.insert_component(entity, false);

        assert_eq!(*world.get_component::<i32>(entity).unwrap(), 2);
        assert_eq!(*world.get_component::<bool>(entity).unwrap(), false);
    }

    #[test]
    fn despawn() {
        let mut world = World::new();
        let entity = world.reserve_entity();
        world.insert_component(entity, 2i32);
        world.insert_component(entity, false);

        assert!(world.despawn(entity));
        assert!(!world.despawn(entity));
        assert!(!world.contains_entity(entity));
    }

    #[test]
    fn multiple_entities() {
        let mut world = World::new();
        let entity1 = world.reserve_entity();
        let entity2 = world.reserve_entity();
        world.insert_component(entity1, 2i32);
        world.insert_component(entity2, false);

        assert_eq!(*world.get_component::<i32>(entity1).unwrap(), 2);
        assert_eq!(*world.get_component::<bool>(entity2).unwrap(), false);
    }

    #[test]
    fn query() {
        let mut world = World::new();
        let entity1 = world.reserve_entity();
        let entity2 = world.reserve_entity();

        world.insert_component(entity1, 2i32);
        world.insert_component(entity2, 3i32);

        let query = world.query::<&i32>();

        assert_eq!(query.get(&world, entity1).unwrap(), &2);
        assert_eq!(query.get(&world, entity2).unwrap(), &3);
    }
}
