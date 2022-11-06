use crate::world::{Entities, EntityIdSet, World};

use super::{QueryItem, QueryState, ReadOnlyWorldQuery, WorldQuery};

#[allow(dead_code)]
pub struct QueryIter<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
    pub(crate) query_state: &'s QueryState<Q, F>,
    pub(crate) cursor: QueryIterationCursor<'w, Q, F>,
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> QueryIter<'w, 's, Q, F> {
    #[inline]
    pub unsafe fn new(
        query_state: &'s QueryState<Q, F>,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            query_state,
            cursor: unsafe {
                QueryIterationCursor::new(query_state, world, last_change_tick, change_tick)
            },
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> Iterator for QueryIter<'w, 's, Q, F> {
    type Item = QueryItem<'w, Q>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next()
    }
}

pub(crate) struct QueryIterationCursor<'w, Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
    pub(crate) entity_ids: EntityIdSet,
    pub(crate) current_index: usize,
    pub(crate) entities: &'w Entities,
    pub(crate) fetch: Q::Fetch<'w>,
    pub(crate) filter: F::Fetch<'w>,
}

impl<'w, Q: WorldQuery, F: ReadOnlyWorldQuery> QueryIterationCursor<'w, Q, F> {
    /// # Safety
    /// - `world` must be the world that `query_state` was created from.
    #[inline]
    unsafe fn new(
        query_state: &QueryState<Q, F>,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        query_state.debug_validate_world(world);

        let fetch = unsafe {
            Q::init_fetch(
                world,
                &query_state.query_state,
                last_change_tick,
                change_tick,
            )
        };
        let filter = unsafe {
            F::init_fetch(
                world,
                &query_state.filter_state,
                last_change_tick,
                change_tick,
            )
        };

        Self {
            entity_ids: query_state.get_entities(world),
            current_index: 0,
            entities: &world.entities,
            fetch,
            filter,
        }
    }

    #[inline]
    fn next(&mut self) -> Option<QueryItem<'w, Q>> {
        loop {
            if self.current_index >= self.entity_ids.len() {
                return None;
            }

            if self.entity_ids.contains(self.current_index) {
                let entity = unsafe { self.entities.get_unchecked(self.current_index) };
                if unsafe { F::filter_fetch(&mut self.filter, entity) } {
                    self.current_index += 1;

                    return Some(unsafe { Q::fetch(&mut self.fetch, entity) });
                }
            }

            self.current_index += 1;
        }
    }
}
