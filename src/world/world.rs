use std::sync::atomic::{AtomicU32, Ordering};

use crate::{
    Component, ComponentId, ComponentStorage, Components, Entities, Entity, EntityMut, EntityRef,
    Mut, QueryState, ReadOnlyWorldQuery, Resource, Resources, Storage, Ticks, WorldQuery,
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
    pub(crate) resources: Resources,
    pub(crate) change_tick: AtomicU32,
    pub(crate) last_change_tick: u32,
}

unsafe impl Send for World {}
unsafe impl Sync for World {}

impl Default for World {
    #[inline]
    fn default() -> Self {
        Self {
            id: WorldId::new(),
            entities: Entities::default(),
            storage: ComponentStorage::default(),
            components: Components::default(),
            resources: Resources::default(),
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
    pub fn reserve_entity(&self) -> Entity {
        self.entities.reserve()
    }

    #[inline]
    pub fn flush(&mut self) {
        self.entities.flush();
    }

    #[inline]
    pub fn len(&self) -> u32 {
        self.entities.len()
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
    pub fn despawn(&mut self, entity: Entity) -> bool {
        self.storage.remove(entity);
        self.entities.free(entity)
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
    pub fn contains_resource<T: Resource>(&self) -> bool {
        if let Some(id) = self.components.get_resource::<T>() {
            self.resources.contains(id)
        } else {
            false
        }
    }

    #[inline]
    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        let id = self.components.init_resource::<T>();

        unsafe {
            self.resources
                .insert(id, Box::new(resource), self.change_tick())
        };
    }

    #[inline]
    pub fn init_resource<T: Resource + Default>(&mut self) {
        if !self.contains_resource::<T>() {
            self.insert_resource(T::default());
        }
    }

    #[inline]
    pub fn remove_resource<T: Resource>(&mut self) -> Option<T> {
        let id = self.components.init_resource::<T>();
        let resource = self.resources.remove(id)?;
        unsafe { Some(*Box::from_raw(resource as *mut T)) }
    }

    #[inline]
    pub fn get_resource<T: Resource>(&self) -> Option<&T> {
        let id = self.components.get_resource::<T>()?;
        let resource = self.resources.get(id)?;
        unsafe { Some(&*(resource as *mut T)) }
    }

    #[inline]
    pub fn get_resource_mut<T: Resource>(&mut self) -> Option<Mut<T>> {
        let id = self.components.get_resource::<T>()?;
        let (resource, change_ticks) = self.resources.get_with_ticks(id)?;

        Some(Mut {
            value: unsafe { &mut *(resource as *mut T) },
            ticks: Ticks {
                ticks: unsafe { &mut *change_ticks },
                last_change_tick: self.last_change_tick(),
                change_tick: self.change_tick(),
            },
        })
    }

    #[inline]
    #[track_caller]
    pub fn resource<T: Resource>(&self) -> &T {
        self.get_resource().unwrap_or_else(|| {
            panic!(
                "resource `{}` does not exist in world",
                std::any::type_name::<T>(),
            )
        })
    }

    #[inline]
    #[track_caller]
    pub fn resource_mut<T: Resource>(&mut self) -> Mut<T> {
        self.get_resource_mut().unwrap_or_else(|| {
            panic!(
                "resource `{}` does not exist in world",
                std::any::type_name::<T>(),
            )
        })
    }
}

impl World {
    #[inline]
    pub fn spawn(&mut self) -> EntityMut<'_> {
        let entity = self.entities.alloc();
        EntityMut::new(self, entity)
    }

    #[inline]
    pub fn get_or_spawn(&mut self, entity: Entity) -> EntityMut<'_> {
        if self.contains_entity(entity) {
            EntityMut::new(self, entity)
        } else {
            self.spawn()
        }
    }

    #[inline]
    pub fn get_entity(&self, entity: Entity) -> Option<EntityRef<'_>> {
        if self.entities.contains(entity) {
            Some(EntityRef::new(self, entity))
        } else {
            None
        }
    }

    #[inline]
    pub fn get_entity_mut(&mut self, entity: Entity) -> Option<EntityMut<'_>> {
        if self.entities.contains(entity) {
            Some(EntityMut::new(self, entity))
        } else {
            None
        }
    }

    #[inline]
    #[track_caller]
    pub fn entity(&self, entity: Entity) -> EntityRef<'_> {
        self.get_entity(entity).unwrap_or_else(|| {
            panic!(
                "Attempting to create EntityRef for entity {}, which does not exist.",
                entity,
            )
        })
    }

    #[inline]
    #[track_caller]
    pub fn entity_mut(&mut self, entity: Entity) -> EntityMut<'_> {
        self.get_entity_mut(entity).unwrap_or_else(|| {
            panic!(
                "Attempting to create EntityMut for entity {}, which does not exist.",
                entity,
            )
        })
    }
}

impl World {
    #[inline]
    pub fn check_change_ticks(&mut self) {
        let change_tick = self.change_tick();

        self.storage.check_change_ticks(change_tick);
    }

    pub fn clear_trackers(&mut self) {
        self.last_change_tick = self.change_tick();
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
        let entity = world.spawn().insert(2i32).insert(false).entity();

        assert_eq!(*world.entity(entity).get::<i32>().unwrap(), 2);
        assert_eq!(*world.entity(entity).get::<bool>().unwrap(), false);
    }

    #[test]
    fn despawn() {
        let mut world = World::new();
        let entity = world.spawn().insert(2i32).insert(false).entity();

        assert!(world.despawn(entity));
        assert!(!world.despawn(entity));
        assert!(!world.contains_entity(entity));

        let new_entity = world.spawn().entity();

        assert!(world.contains_entity(new_entity));
        assert!(!world.contains_entity(entity));

        world.entity_mut(new_entity).insert(2i32);

        assert_eq!(*world.entity(new_entity).get::<i32>().unwrap(), 2);
        assert!(!world.contains_entity(entity));
    }

    #[test]
    fn multiple_entities() {
        let mut world = World::new();
        let entity1 = world.spawn().insert(2i32).entity();
        let entity2 = world.spawn().insert(false).entity();

        assert_eq!(*world.entity(entity1).get::<i32>().unwrap(), 2);
        assert_eq!(*world.entity(entity2).get::<bool>().unwrap(), false);
    }

    #[test]
    fn query_get() {
        let mut world = World::new();
        let entity1 = world.spawn().insert(2i32).entity();
        let entity2 = world.spawn().insert(3i32).entity();
        let entity3 = world.spawn().entity();

        let mut query = world.query::<&mut i32>();

        assert_eq!(query.get(&world, entity1).unwrap(), &2);
        assert_eq!(query.get(&world, entity2).unwrap(), &3);
        assert!(query.get(&world, entity3).is_none());

        *query.get_mut(&mut world, entity1).unwrap() *= 2;

        assert_eq!(query.get(&world, entity1).unwrap(), &4);
    }

    #[test]
    fn query_filter() {
        let mut world = World::new();

        let entity1 = world.spawn().insert(2i32).entity();
        let entity2 = world.spawn().insert(3i32).insert(false).entity();

        let query = world.query_filtered::<&i32, Without<bool>>();

        assert_eq!(query.get(&world, entity1).unwrap(), &2);
        assert!(query.get(&world, entity2).is_none());

        let query = world.query_filtered::<&i32, With<bool>>();

        assert!(query.get(&world, entity1).is_none());
        assert_eq!(query.get(&world, entity2).unwrap(), &3);
    }

    #[test]
    fn query_iter() {
        let mut world = World::new();

        let entity1 = world.spawn().insert(2i32).entity();
        let entity2 = world.spawn().entity();
        let entity3 = world.spawn().insert(3i32).entity();

        let mut query = world.query::<(Entity, &mut i32)>();

        let mut iter = query.iter(&world);
        assert_eq!(iter.next().unwrap(), (entity1, &2));
        assert_eq!(iter.next().unwrap(), (entity3, &3));
        assert!(iter.next().is_none());

        let mut iter = query.iter_mut(&mut world);
        *iter.next().unwrap().1 *= 2;
        *iter.next().unwrap().1 *= 3;

        let mut iter = query.iter(&world);
        assert_eq!(iter.next().unwrap(), (entity1, &4));
        assert_eq!(iter.next().unwrap(), (entity3, &9));
        assert!(iter.next().is_none());

        let query = world.query::<Entity>();

        let mut iter = query.iter(&world);
        assert_eq!(iter.next().unwrap(), entity1);
        assert_eq!(iter.next().unwrap(), entity2);
        assert_eq!(iter.next().unwrap(), entity3);
    }

    #[test]
    fn query_iter_filter() {
        let mut world = World::new();

        let entity1 = world.spawn().insert(2i32).entity();
        let entity2 = world.spawn().insert(3i32).insert(false).entity();
        let entity3 = world.spawn().insert(4i32).entity();

        let query = world.query_filtered::<(Entity, &i32), Without<bool>>();

        let mut iter = query.iter(&world);

        assert_eq!(iter.next().unwrap(), (entity1, &2));
        assert_eq!(iter.next().unwrap(), (entity3, &4));
        assert!(iter.next().is_none());

        let query = world.query_filtered::<(Entity, &i32), With<bool>>();

        let mut iter = query.iter(&world);

        assert_eq!(iter.next().unwrap(), (entity2, &3));
        assert!(iter.next().is_none());
    }
}
