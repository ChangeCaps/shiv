use crate::{Access, Component, ComponentId, Entity, Mut, Storage, StorageSet, Ticks, World};

pub unsafe trait WorldQuery {
    type Item<'w>;
    type Fetch<'w>;
    type State: Send + Sync + Sized;
    type ReadOnly: ReadOnlyWorldQuery<State = Self::State>;

    unsafe fn init_fetch<'w>(
        world: &'w World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self::Fetch<'w>;
    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w>;
    unsafe fn filter_fetch<'w>(_fetch: &mut Self::Fetch<'w>, _entity: Entity) -> bool {
        true
    }

    fn init_state(world: &mut World) -> Self::State;
    fn update_component_access(state: &Self::State, access: &mut Access<ComponentId>);
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

    unsafe fn init_fetch<'w>(
        _world: &'w World,
        _state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self::Fetch<'w> {
    }

    unsafe fn fetch<'w>(_fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        entity
    }

    fn init_state(_world: &mut World) -> Self::State {}
    fn update_component_access(_state: &Self::State, _access: &mut Access<ComponentId>) {}
}

unsafe impl ReadOnlyWorldQuery for Entity {}

pub struct ReadFetch<'w, T: Component> {
    storage: &'w T::Storage,
}

unsafe impl<T: Component> WorldQuery for &T {
    type Item<'w> = &'w T;
    type Fetch<'w> = ReadFetch<'w, T>;
    type State = ComponentId;
    type ReadOnly = Self;

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

    unsafe fn fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> Self::Item<'w> {
        unsafe { &*(fetch.storage.get_unchecked(entity) as *mut T) }
    }

    unsafe fn filter_fetch<'w>(fetch: &mut Self::Fetch<'w>, entity: Entity) -> bool {
        fetch.storage.contains(entity)
    }

    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    fn update_component_access(&state: &Self::State, access: &mut Access<ComponentId>) {
        access.add_read(state);
    }
}

unsafe impl<T: Component> ReadOnlyWorldQuery for &T {}

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

    fn init_state(world: &mut World) -> Self::State {
        world.init_component::<T>()
    }

    fn update_component_access(&state: &Self::State, access: &mut Access<ComponentId>) {
        access.add_write(state);
    }
}

macro_rules! impl_world_query {
    (@ $($ident:ident),*) => {
        #[allow(non_snake_case, unused)]
        unsafe impl<$($ident: WorldQuery),*> WorldQuery for ($($ident,)*) {
            type Item<'w> = ($($ident::Item<'w>,)*);
            type Fetch<'w> = ($($ident::Fetch<'w>,)*);
            type State = ($($ident::State,)*);
            type ReadOnly = ($($ident::ReadOnly,)*);

            unsafe fn init_fetch<'w>(
                world: &'w World,
                ($($ident,)*): &Self::State,
                last_change_tick: u32,
                change_tick: u32,
            ) -> Self::Fetch<'w> {
                unsafe { ($($ident::init_fetch(world, $ident, last_change_tick, change_tick),)*) }
            }

            unsafe fn fetch<'w>(
                ($($ident,)*): &mut Self::Fetch<'w>,
                entity: Entity,
            ) -> Self::Item<'w> {
                unsafe { ($($ident::fetch($ident, entity),)*) }
            }

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

            fn init_state(world: &mut World) -> Self::State {
                ($($ident::init_state(world),)*)
            }

            fn update_component_access(state: &Self::State, access: &mut Access<ComponentId>) {
                let ($($ident,)*) = state;

                $(
                    $ident::update_component_access($ident, access);
                )*
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
