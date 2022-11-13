use std::any::type_name;

use crate::{
    change_detection::{Mut, Ticks},
    storage::ComponentStorage,
    system::FilteredAccess,
    world::{Component, ComponentId, Entity, Storage, World},
};

pub unsafe trait WorldQuery {
    type Item<'w>;
    type Fetch<'w>;
    type State: Send + Sync + Sized;
    type ReadOnly: ReadOnlyWorldQuery<State = Self::State>;

    /// # Safety
    /// - `state` must be the result of [`WorldQuery::init_state`] with the same `world`.
    /// - This function does not check borrow rules, so it's up to the caller to ensure that access
    /// is valid.
    unsafe fn init_fetch<'w>(
        world: &'w World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self::Fetch<'w>;

    fn contains<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool;

    /// Fetch a single item from the given `fetch` and `entity`.
    ///
    /// # Safety
    /// - `fetch` must be the result of [`WorldQuery::init_fetch`].
    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w>;

    /// Fetches the filter for this query.
    ///
    /// # Safety
    /// - `fetch` must be the result of [`WorldQuery::init_fetch`].
    #[inline]
    unsafe fn filter_fetch<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    /// Initialize the state required to fetch this query.
    fn init_state(world: &mut World) -> Self::State;

    /// Update the component access for this query.
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>);

    /// Update the component access for this query.
    fn matches_component_set(state: &Self::State, id: ComponentId) -> bool;
}

pub unsafe trait ReadOnlyWorldQuery: WorldQuery<ReadOnly = Self> {}

pub type QueryItem<'w, Q> = <Q as WorldQuery>::Item<'w>;
pub type QueryFetch<'w, Q> = <Q as WorldQuery>::Fetch<'w>;
pub type ReadOnlyQueryItem<'w, Q> = QueryItem<'w, <Q as WorldQuery>::ReadOnly>;
pub type ReadOnlyQueryFetch<'w, Q> = QueryFetch<'w, <Q as WorldQuery>::ReadOnly>;

unsafe impl WorldQuery for Entity {
    type Item<'w> = Entity;
    type Fetch<'w> = ();
    type State = ();
    type ReadOnly = Self;

    #[inline]
    unsafe fn init_fetch<'w>(
        _world: &'w World,
        _state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self::Fetch<'w> {
    }

    #[inline]
    fn contains<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    #[inline]
    unsafe fn fetch<'w>(_fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        entity
    }

    #[inline]
    fn init_state(_world: &mut World) -> Self::State {}

    #[inline]
    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    #[inline]
    fn matches_component_set(_state: &Self::State, _id: ComponentId) -> bool {
        true
    }
}

unsafe impl ReadOnlyWorldQuery for Entity {}

#[doc(hidden)]
#[derive(Debug)]
pub struct ReadFetch<'w, T: Component> {
    storage: &'w T::Storage,
}

unsafe impl<T: Component> WorldQuery for &T {
    type Item<'w> = &'w T;
    type Fetch<'w> = ReadFetch<'w, T>;
    type State = ComponentId;
    type ReadOnly = Self;

    #[inline]
    unsafe fn init_fetch<'w>(
        world: &'w World,
        &state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self::Fetch<'w> {
        let storage_sets = <T::Storage as Storage>::get(&world.storage);

        ReadFetch {
            storage: storage_sets.get(state).unwrap(),
        }
    }

    #[inline]
    fn contains<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        fetch.storage.contains(entity)
    }

    #[inline]
    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        unsafe { &*(fetch.storage.get_unchecked(entity) as *mut T) }
    }

    #[inline]
    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    #[inline]
    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.has_write(state),
            "&{} conflicts with previous access in this query. Shared access cannot coexist with exclusive access.", 
            type_name::<T>(),
        );

        access.add_read(state);
    }

    #[inline]
    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state == id
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for &T {}

#[doc(hidden)]
#[derive(Debug)]
pub struct WriteFetch<'w, T: Component> {
    storage: &'w T::Storage,
    last_change_tick: u32,
    change_tick: u32,
}

unsafe impl<'a, T: Component> WorldQuery for &'a mut T {
    type Item<'w> = Mut<'w, T>;
    type Fetch<'w> = WriteFetch<'w, T>;
    type State = ComponentId;
    type ReadOnly = &'a T;

    #[inline]
    unsafe fn init_fetch<'w>(
        world: &'w World,
        &state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self::Fetch<'w> {
        let storage_sets = <T::Storage as Storage>::get(&world.storage);

        WriteFetch {
            storage: storage_sets.get(state).unwrap(),
            last_change_tick,
            change_tick,
        }
    }

    #[inline]
    fn contains<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        fetch.storage.contains(entity)
    }

    #[inline]
    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        let (value, ticks) = unsafe { fetch.storage.get_with_ticks_unchecked(entity) };

        Mut {
            value: unsafe { &mut *(value as *mut T) },
            ticks: Ticks {
                ticks: unsafe { &mut *ticks.get() },
                last_change_tick: fetch.last_change_tick,
                change_tick: fetch.change_tick,
            },
        }
    }

    #[inline]
    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    #[inline]
    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.has_read(state),
            "&mut {} conflicts with previous access in this query. Mutable component access must be unique.", 
            type_name::<T>(),
        );

        access.add_write(state);
    }

    #[inline]
    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state == id
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct OptionFetch<'w, T: WorldQuery> {
    fetch: T::Fetch<'w>,
}

unsafe impl<T: WorldQuery> WorldQuery for Option<T> {
    type Item<'w> = Option<T::Item<'w>>;
    type Fetch<'w> = OptionFetch<'w, T>;
    type State = T::State;
    type ReadOnly = Option<T::ReadOnly>;

    #[inline]
    unsafe fn init_fetch<'w>(
        world: &'w World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self::Fetch<'w> {
        OptionFetch {
            fetch: unsafe { T::init_fetch(world, state, last_change_tick, change_tick) },
        }
    }

    #[inline]
    fn contains<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    #[inline]
    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        if T::contains(&mut fetch.fetch, entity) {
            Some(unsafe { T::fetch(&mut fetch.fetch, entity) })
        } else {
            None
        }
    }

    #[inline]
    fn init_state(world: &mut World) -> Self::State {
        T::init_state(world)
    }

    #[inline]
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        let mut intermediate = access.clone();
        T::update_component_access(state, &mut intermediate);
        access.access_mut().extend(intermediate.access());
    }

    #[inline]
    fn matches_component_set(_state: &Self::State, _id: ComponentId) -> bool {
        true
    }
}

unsafe impl<T: ReadOnlyWorldQuery> ReadOnlyWorldQuery for Option<T> {}

macro_rules! impl_world_query {
    (@ $($ident:ident),*) => {
        #[allow(non_snake_case, unused)]
        unsafe impl<$($ident: WorldQuery),*> WorldQuery for ($($ident,)*) {
            type Item<'w> = ($($ident::Item<'w>,)*);
            type Fetch<'w> = ($($ident::Fetch<'w>,)*);
            type State = ($($ident::State,)*);
            type ReadOnly = ($($ident::ReadOnly,)*);

            #[inline]
            unsafe fn init_fetch<'w>(
                world: &'w World,
                ($($ident,)*): &Self::State,
                last_change_tick: u32,
                change_tick: u32,
            ) -> Self::Fetch<'w> {
                unsafe { ($($ident::init_fetch(world, $ident, last_change_tick, change_tick),)*) }
            }

            #[inline]
            fn contains<'w>(($($ident,)*): &mut Self::Fetch<'w>, entity: Entity) -> bool {
                $($ident::contains($ident, entity) &&)* true
            }

            #[inline]
            unsafe fn fetch<'w>(
                ($($ident,)*): &mut Self::Fetch<'w>,
                entity: Entity,
            ) -> Self::Item<'w> {
                unsafe { ($($ident::fetch($ident, entity),)*) }
            }

            #[inline]
            unsafe fn filter_fetch<'w>(
                ($($ident,)*): &mut Self::Fetch<'w>,
                entity: Entity,
            ) -> bool {
                $(
                    if unsafe { !$ident::filter_fetch($ident, entity) } {
                        return false;
                    }
                )*

                true
            }

            #[inline]
            fn init_state(world: &mut World) -> Self::State {
                ($($ident::init_state(world),)*)
            }

            #[inline]
            fn update_component_access(($($ident,)*): &Self::State, access: &mut FilteredAccess<ComponentId>) {
                $($ident::update_component_access($ident, access);)*
            }

            #[inline]
            fn matches_component_set(($($ident,)*): &Self::State, id: ComponentId) -> bool {
                $($ident::matches_component_set($ident, id) ||)* false
            }
        }

        unsafe impl<$($ident: ReadOnlyWorldQuery),*> ReadOnlyWorldQuery for ($($ident,)*) {}
    };
    ($start:ident $(,$ident:ident)*) => {
        impl_world_query!(@ $start $(,$ident)*);
        impl_world_query!($($ident),*);
    };
    () => {
        impl_world_query!(@);
    }
}

impl_world_query!(A, B, C, D, E, F, G, H, I, J, K, L);
