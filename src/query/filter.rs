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

pub struct WithFetch<'w, T: Component> {
    storage: &'w T::Storage,
}

unsafe impl<T: Component> WorldQuery for With<T> {
    type Item<'w> = ();
    type Fetch<'w> = WithFetch<'w, T>;
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

        WithFetch {
            storage: storage_sets.get(state).unwrap(),
        }
    }

    #[inline]
    fn contains<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    #[inline]
    unsafe fn fetch<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> Self::Item<'w> {}

    #[inline]
    unsafe fn filter_fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        fetch.storage.contains(entity)
    }

    #[inline]
    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    #[inline]
    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        access.add_with(state);
    }

    #[inline]
    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state == id
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for With<T> {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Without<T> {
    _marker: PhantomData<T>,
}

pub struct WithoutFetch<'w, T: Component> {
    storage: &'w T::Storage,
}

unsafe impl<T: Component> WorldQuery for Without<T> {
    type Item<'w> = ();
    type Fetch<'w> = WithoutFetch<'w, T>;
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

        WithoutFetch {
            storage: storage_sets.get(state).unwrap(),
        }
    }

    #[inline]
    fn contains<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    #[inline]
    unsafe fn fetch<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> Self::Item<'w> {}

    #[inline]
    unsafe fn filter_fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        !fetch.storage.contains(entity)
    }

    #[inline]
    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    #[inline]
    fn update_component_access(&state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        access.add_without(state);
    }

    #[inline]
    fn matches_component_set(&state: &Self::State, id: ComponentId) -> bool {
        state != id
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for Without<T> {}

pub struct Or<T>(T);

#[doc(hidden)]
#[derive(Debug)]
pub struct OrFetch<'w, T: WorldQuery> {
    fetch: T::Fetch<'w>,
}

macro_rules! impl_or_world_query {
    (@ $($ident:ident),*) => {
        #[allow(non_snake_case, unused)]
        unsafe impl<$($ident: WorldQuery),*> WorldQuery for Or<($($ident,)*)> {
            type Item<'w> = bool;
            type Fetch<'w> = ($(OrFetch<'w, $ident>,)*);
            type State = ($($ident::State,)*);
            type ReadOnly = Or<($($ident::ReadOnly,)*)>;

            #[inline]
            unsafe fn init_fetch<'w>(
                world: &'w World,
                ($($ident,)*): &Self::State,
                last_change_tick: u32,
                change_tick: u32,
            ) -> Self::Fetch<'w> {
                unsafe { ($(OrFetch {
                    fetch: $ident::init_fetch(world, $ident, last_change_tick, change_tick),
                },)*) }
            }

            #[inline]
            fn contains<'w>(($($ident,)*): &mut Self::Fetch<'w>, entity: Entity) -> bool {
                $($ident::contains(&mut $ident.fetch, entity) &&)* true
            }

            #[inline]
            unsafe fn fetch<'w>(
                ($($ident,)*): &mut Self::Fetch<'w>,
                entity: Entity,
            ) -> Self::Item<'w> {
                unsafe { $($ident::contains(&mut $ident.fetch, entity) &&
                    $ident::filter_fetch(&mut $ident.fetch, entity) ||)* false }
            }

            #[inline]
            unsafe fn filter_fetch<'w>(
                fetch: &mut Self::Fetch<'w>,
                entity: Entity,
            ) -> bool {
                unsafe { Self::fetch(fetch, entity) }
            }

            #[inline]
            fn init_state(world: &mut World) -> Self::State {
                ($($ident::init_state(world),)*)
            }

            #[inline]
            fn update_component_access(($($ident,)*): &Self::State, access: &mut FilteredAccess<ComponentId>) {
                let mut _access = FilteredAccess::default();
                let mut _is_first = true;
                $(
                    if _is_first {
                        _is_first = false;

                        $ident::update_component_access($ident, &mut _access);
                    } else {
                        let mut intermediate = FilteredAccess::default();
                        $ident::update_component_access($ident, &mut intermediate);
                        _access.extend_intersect(&intermediate);
                    }
                )*

                access.extend(&_access);
            }

            #[inline]
            fn matches_component_set(($($ident,)*): &Self::State, id: ComponentId) -> bool {
                $($ident::matches_component_set($ident, id) ||)* false
            }
        }

        unsafe impl<$($ident: ReadOnlyWorldQuery),*> ReadOnlyWorldQuery for Or<($($ident,)*)> {}
    };
    ($start:ident $(,$ident:ident)*) => {
        impl_or_world_query!(@ $start $(,$ident)*);
        impl_or_world_query!($($ident),*);
    };
    () => {
        impl_or_world_query!(@);
    }
}

impl_or_world_query!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Added<T> {
    _marker: PhantomData<T>,
}

#[doc(hidden)]
#[derive(Debug)]
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

    #[inline]
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

    #[inline]
    fn contains<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        fetch.storage.contains(entity)
    }

    #[inline]
    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        if !Self::contains(fetch, entity) {
            return false;
        }

        let ticks = unsafe { fetch.storage.get_ticks_unchecked(entity) };
        let ticks = unsafe { &*ticks.get() };

        ticks.is_added(fetch.last_change_tick, fetch.change_tick)
    }

    #[inline]
    unsafe fn filter_fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        unsafe { Self::fetch(fetch, entity) }
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

unsafe impl<T: Component> ReadOnlyWorldQuery for Added<T> {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Changed<T> {
    _marker: PhantomData<T>,
}

#[doc(hidden)]
#[derive(Debug)]
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

    #[inline]
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

    #[inline]
    fn contains<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        fetch.storage.contains(entity)
    }

    #[inline]
    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        if !Self::contains(fetch, entity) {
            return false;
        }

        let ticks = unsafe { fetch.storage.get_ticks_unchecked(entity) };
        let ticks = unsafe { &*ticks.get() };

        ticks.is_changed(fetch.last_change_tick, fetch.change_tick)
    }

    #[inline]
    unsafe fn filter_fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        unsafe { Self::fetch(fetch, entity) }
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

unsafe impl<T: Component> ReadOnlyWorldQuery for Changed<T> {}
