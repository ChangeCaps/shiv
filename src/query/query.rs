use crate::{
    system::FilteredAccess,
    world::{ComponentId, Entity, EntityIdSet, World, WorldId},
};

use super::{QueryItem, QueryIter, ReadOnlyQueryItem, ReadOnlyWorldQuery, WorldQuery};

#[derive(Debug)]
pub struct QueryState<Q: WorldQuery, F: ReadOnlyWorldQuery> {
    pub(crate) world_id: WorldId,
    pub(crate) filtered_access: FilteredAccess<ComponentId>,
    pub(crate) query_state: Q::State,
    pub(crate) filter_state: F::State,
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> QueryState<Q, F> {
    #[inline]
    pub fn new(world: &mut World) -> Self {
        let query_state = Q::init_state(world);
        let filter_state = F::init_state(world);

        let mut filtered_access = FilteredAccess::new();
        Q::update_component_access(&query_state, &mut filtered_access);
        F::update_component_access(&filter_state, &mut filtered_access);

        Self {
            world_id: world.id(),
            filtered_access,
            query_state,
            filter_state,
        }
    }

    #[inline]
    pub fn get_entities(&self, world: &World) -> EntityIdSet {
        self.debug_validate_world(world);

        let mut iter = self.filtered_access.iter_with();

        let mut entities = if let Some(id) = iter.next() {
            world.storage.entity_ids(id)
        } else {
            world.entities.entity_ids().clone()
        };

        for id in iter {
            entities.intersect_with(&world.storage.entity_ids(id));
        }

        for id in self.filtered_access.iter_without() {
            entities.difference_with(&world.storage.entity_ids(id));
        }

        entities
    }

    #[inline]
    pub fn matches(&self, world: &World, entity: Entity) -> bool {
        self.debug_validate_world(world);

        for id in self.filtered_access.iter_with() {
            if !world.storage.contains(id, entity) {
                return false;
            }
        }

        for id in self.filtered_access.iter_without() {
            if world.storage.contains(id, entity) {
                return false;
            }
        }

        true
    }

    #[inline]
    pub fn as_readonly(&self) -> &QueryState<Q::ReadOnly, F::ReadOnly> {
        unsafe { self.as_transmuted_state::<Q::ReadOnly, F::ReadOnly>() }
    }

    /// # Safety
    /// - `NQ` must have a subset of the access of `Q`.
    /// - `NF` must have a subset of the access of `F`.
    #[inline]
    pub unsafe fn as_transmuted_state<NQ, NF>(&self) -> &QueryState<NQ, NF>
    where
        NQ: WorldQuery<State = Q::State>,
        NF: ReadOnlyWorldQuery<State = F::State>,
    {
        unsafe { &*(self as *const Self as *const QueryState<NQ, NF>) }
    }

    /// # Safety
    /// - `world` must be the same world that was used to create this [`QueryState`].
    #[inline]
    pub unsafe fn get_unchecked_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Option<Q::Item<'w>> {
        if !self.matches(world, entity) {
            return None;
        }

        let mut fetch =
            unsafe { Q::init_fetch(world, &self.query_state, last_change_tick, change_tick) };
        let mut filter =
            unsafe { F::init_fetch(world, &self.filter_state, last_change_tick, change_tick) };

        if unsafe { F::filter_fetch(&mut filter, entity) } {
            Some(unsafe { Q::fetch(&mut fetch, entity) })
        } else {
            None
        }
    }
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> QueryState<Q, F> {
    #[inline]
    pub fn validate_world(&self, world: &World) {
        if self.world_id != world.id() {
            panic!("QueryState used with a different world");
        }
    }

    #[inline]
    #[allow(unused_variables)]
    pub fn debug_validate_world(&self, world: &World) {
        #[cfg(debug_assertions)]
        self.validate_world(world);
    }

    #[inline]
    pub fn is_empty(&self, world: &World) -> bool {
        self.iter(world).next().is_none()
    }

    #[inline]
    pub fn contains(&self, world: &World, entity: Entity) -> bool {
        self.get(world, entity).is_some()
    }

    #[inline]
    pub fn get<'w>(&self, world: &'w World, entity: Entity) -> Option<ReadOnlyQueryItem<'w, Q>> {
        self.validate_world(world);

        let state = self.as_readonly();
        unsafe {
            state.get_unchecked_manual(world, entity, world.last_change_tick(), world.change_tick())
        }
    }

    #[inline]
    pub fn get_mut<'w>(
        &mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Option<QueryItem<'w, Q>> {
        self.validate_world(world);

        unsafe {
            self.get_unchecked_manual(world, entity, world.last_change_tick(), world.change_tick())
        }
    }

    /// # Safety
    /// - `world` must be the same world that was used to create this [`QueryState`].
    #[inline]
    pub unsafe fn iter_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryIter<'w, 's, Q, F> {
        self.debug_validate_world(world);
        unsafe { QueryIter::new(self, world, last_change_tick, change_tick) }
    }

    #[inline]
    pub fn iter<'w, 's>(&'s self, world: &'w World) -> QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly> {
        self.validate_world(world);
        unsafe {
            self.as_readonly().iter_unchecked_manual(
                world,
                world.last_change_tick(),
                world.change_tick(),
            )
        }
    }

    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, Q, F> {
        self.validate_world(world);
        unsafe { self.iter_unchecked_manual(world, world.last_change_tick(), world.change_tick()) }
    }
}

pub struct Query<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
    world: &'w World,
    state: &'s QueryState<Q, F>,
    last_change_tick: u32,
    change_tick: u32,
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> Query<'w, 's, Q, F> {
    /// # Safety
    /// - `world` must be the same world that was used to create this [`QueryState`].
    #[inline]
    pub unsafe fn new(
        world: &'w World,
        state: &'s mut QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            world,
            state,
            last_change_tick,
            change_tick,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.get(entity).is_some()
    }

    #[inline]
    pub fn get(&self, entity: Entity) -> Option<ReadOnlyQueryItem<'_, Q>> {
        let state = self.state.as_readonly();
        unsafe {
            state.get_unchecked_manual(self.world, entity, self.last_change_tick, self.change_tick)
        }
    }

    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<QueryItem<'_, Q>> {
        unsafe {
            self.state.get_unchecked_manual(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    #[inline]
    pub fn iter(&self) -> QueryIter<'_, 's, Q::ReadOnly, F::ReadOnly> {
        unsafe {
            self.state.as_readonly().iter_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> QueryIter<'_, 's, Q, F> {
        let state = &self.state;
        unsafe { state.iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick) }
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> IntoIterator for &'w Query<'_, 's, Q, F> {
    type Item = ReadOnlyQueryItem<'w, Q>;
    type IntoIter = QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> IntoIterator for &'w mut Query<'_, 's, Q, F> {
    type Item = QueryItem<'w, Q>;
    type IntoIter = QueryIter<'w, 's, Q, F>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
