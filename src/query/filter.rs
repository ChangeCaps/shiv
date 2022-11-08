use std::{any::type_name, marker::PhantomData};

use crate::{
    storage::ComponentStorage,
    system::FilteredAccess,
    world::{Component, ComponentId, Entity, Storage, World},
};

use super::{ReadOnlyWorldQuery, WorldQuery};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct With<T> {
    _marker: PhantomData<T>,
}

unsafe impl<T: Component> WorldQuery for With<T> {
    type Item<'w> = ();
    type Fetch<'w> = ();
    type State = ComponentId;
    type ReadOnly = Self;

    unsafe fn init_fetch<'w>(
        _world: &'w World,
        _state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self::Fetch<'w> {
    }

    fn contains<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    unsafe fn fetch<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> Self::Item<'w> {}

    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        access.add_with(state);
    }

    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state == id
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for With<T> {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Without<T> {
    _marker: PhantomData<T>,
}

unsafe impl<T: Component> WorldQuery for Without<T> {
    type Item<'w> = ();
    type Fetch<'w> = ();
    type State = ComponentId;
    type ReadOnly = Self;

    unsafe fn init_fetch<'w>(
        _world: &'w World,
        _state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self::Fetch<'w> {
    }

    fn contains<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    unsafe fn fetch<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> Self::Item<'w> {}

    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        access.add_without(state);
    }

    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state != id
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for Without<T> {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Added<T> {
    _marker: PhantomData<T>,
}

pub struct AddedFetch<'w, T: Component> {
    storage: &'w T::Storage,
    last_change_tick: u32,
    change_tick: u32,
}

unsafe impl<T: Component> WorldQuery for Added<T> {
    type Item<'w> = bool;
    type Fetch<'w> = AddedFetch<'w, T>;
    type State = ComponentId;
    type ReadOnly = Self;

    unsafe fn init_fetch<'w>(
        world: &'w World,
        &state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self::Fetch<'w> {
        let storage_sets = <T::Storage as Storage>::get(&world.storage);
        let storage = unsafe { storage_sets.get_unchecked(state) };

        AddedFetch {
            storage,
            last_change_tick,
            change_tick,
        }
    }

    fn contains<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        let ticks = unsafe { fetch.storage.get_ticks_unchecked(entity) };
        let ticks = unsafe { &*ticks.get() };

        ticks.is_changed(fetch.last_change_tick, fetch.change_tick)
    }

    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        Self::contains(fetch, entity)
    }

    unsafe fn filter_fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        Self::contains(fetch, entity)
    }

    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.has_write(state),
            "&{} conflicts with previous access in this query. Shared access cannot coexist with exclusive access.", 
            type_name::<T>(),
        );

        access.add_read(state);
    }

    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state == id
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for Added<T> {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Changed<T> {
    _marker: PhantomData<T>,
}

pub struct ChangedFetch<'w, T: Component> {
    storage: &'w T::Storage,
    last_change_tick: u32,
    change_tick: u32,
}

unsafe impl<T: Component> WorldQuery for Changed<T> {
    type Item<'w> = bool;
    type Fetch<'w> = ChangedFetch<'w, T>;
    type State = ComponentId;
    type ReadOnly = Self;

    unsafe fn init_fetch<'w>(
        world: &'w World,
        &state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self::Fetch<'w> {
        let storage_sets = <T::Storage as Storage>::get(&world.storage);
        let storage = unsafe { storage_sets.get_unchecked(state) };

        ChangedFetch {
            storage,
            last_change_tick,
            change_tick,
        }
    }

    fn contains<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        let ticks = unsafe { fetch.storage.get_ticks_unchecked(entity) };
        let ticks = unsafe { &*ticks.get() };

        ticks.is_changed(fetch.last_change_tick, fetch.change_tick)
    }

    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        Self::contains(fetch, entity)
    }

    unsafe fn filter_fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        Self::contains(fetch, entity)
    }

    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.has_write(state),
            "&{} conflicts with previous access in this query. Shared access cannot coexist with exclusive access.", 
            type_name::<T>(),
        );

        access.add_read(state);
    }

    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state == id
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for Changed<T> {}
