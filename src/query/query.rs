use crate::{Entity, QueryItem, ReadOnlyQueryItem, ReadOnlyWorldQuery, World, WorldId, WorldQuery};

pub struct QueryState<Q: WorldQuery, F: ReadOnlyWorldQuery> {
    world_id: WorldId,
    fetch_state: Q::State,
    filter_state: F::State,
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> QueryState<Q, F> {
    #[inline]
    pub fn new(world: &mut World) -> Self {
        Self {
            world_id: world.id(),
            fetch_state: Q::init_state(world),
            filter_state: F::init_state(world),
        }
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
    ) -> Option<Q::Item<'w>> {
        let mut fetch = unsafe { Q::init_fetch(world, &self.fetch_state) };
        let mut filter = unsafe { F::init_fetch(world, &self.filter_state) };

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
    pub fn get<'w>(&self, world: &'w World, entity: Entity) -> Option<ReadOnlyQueryItem<'w, Q>> {
        self.validate_world(world);

        let state = self.as_readonly();
        unsafe { state.get_unchecked_manual(world, entity) }
    }

    #[inline]
    pub fn get_mut<'w>(&self, world: &'w mut World, entity: Entity) -> Option<QueryItem<'w, Q>> {
        self.validate_world(world);

        unsafe { self.get_unchecked_manual(world, entity) }
    }
}

pub struct Query<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
    world: &'w World,
    state: &'s mut QueryState<Q, F>,
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
    pub fn get(&self, entity: Entity) -> Option<ReadOnlyQueryItem<'_, Q>> {
        let state = self.state.as_readonly();
        unsafe { state.get_unchecked_manual(self.world, entity) }
    }

    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<QueryItem<'_, Q>> {
        unsafe { self.state.get_unchecked_manual(self.world, entity) }
    }
}
