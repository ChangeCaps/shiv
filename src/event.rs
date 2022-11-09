use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
};

use crate::{
    system::{
        Local, LocalState, ReadOnlySystemParamFetch, Res, ResMut, ResMutState, ResState,
        SystemMeta, SystemParam, SystemParamFetch, SystemParamState,
    },
    world::World,
};

pub trait Event: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Event for T {}

pub struct EventId<E: Event> {
    pub id: usize,
    _marker: PhantomData<E>,
}

impl<E: Event> EventId<E> {
    #[inline]
    pub const fn new(id: usize) -> Self {
        EventId {
            id,
            _marker: PhantomData,
        }
    }
}

impl<E: Event> Clone for EventId<E> {
    #[inline]
    fn clone(&self) -> Self {
        EventId {
            id: self.id,
            _marker: PhantomData,
        }
    }
}

impl<E: Event> Copy for EventId<E> {}

impl<E: Event> PartialEq for EventId<E> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E: Event> Eq for EventId<E> {}

impl<E: Event> PartialOrd for EventId<E> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl<E: Event> Ord for EventId<E> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<E: Event> Hash for EventId<E> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<E: Event> std::fmt::Debug for EventId<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventId<{}>({})", std::any::type_name::<E>(), self.id)
    }
}

#[derive(Debug)]
struct EventInstance<E: Event> {
    id: EventId<E>,
    event: E,
}

#[derive(Debug)]
struct EventSequence<E: Event> {
    events: Vec<EventInstance<E>>,
    start_event_count: usize,
}

impl<E: Event> Default for EventSequence<E> {
    #[inline]
    fn default() -> Self {
        EventSequence {
            events: Vec::new(),
            start_event_count: 0,
        }
    }
}

impl<E: Event> Deref for EventSequence<E> {
    type Target = Vec<EventInstance<E>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl<E: Event> DerefMut for EventSequence<E> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

#[derive(Debug)]
pub struct Events<E: Event> {
    events_a: EventSequence<E>,
    events_b: EventSequence<E>,
    event_count: usize,
}

impl<E: Event> Default for Events<E> {
    #[inline]
    fn default() -> Self {
        Events {
            events_a: EventSequence::default(),
            events_b: EventSequence::default(),
            event_count: 0,
        }
    }
}

impl<E: Event> Events<E> {
    #[inline]
    pub fn oldest_event_count(&self) -> usize {
        let a = self.events_a.start_event_count;
        let b = self.events_b.start_event_count;
        usize::min(a, b)
    }

    #[inline]
    pub fn send(&mut self, event: E) {
        let id = EventId::new(self.event_count);
        let instance = EventInstance { id, event };

        self.events_b.push(instance);
        self.event_count += 1;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.events_a.len() + self.events_b.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn update(&mut self) {
        mem::swap(&mut self.events_a, &mut self.events_b);
        self.events_b.clear();
        self.events_b.start_event_count = self.event_count;
    }

    #[inline]
    pub fn update_system(mut events: ResMut<Self>) {
        events.update();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.reset();
        self.events_a.clear();
        self.events_b.clear();
    }

    #[inline]
    fn reset(&mut self) {
        self.events_a.start_event_count = self.event_count;
        self.events_b.start_event_count = self.event_count;
    }

    #[inline]
    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.reset();

        self.events_a
            .drain(..)
            .chain(self.events_b.drain(..))
            .map(|e| e.event)
    }
}

#[derive(Debug)]
pub struct EventReader<'w, 's, E: Event> {
    reader: Local<'s, ManualEventReader<E>>,
    events: Res<'w, Events<E>>,
}

impl<'w, 's, E: Event> EventReader<'w, 's, E> {
    #[inline]
    pub fn iter(&mut self) -> impl DoubleEndedIterator<Item = &E> {
        self.reader.iter(&self.events)
    }

    #[inline]
    pub fn iter_with_id(&mut self) -> impl DoubleEndedIterator<Item = (EventId<E>, &E)> {
        self.reader.iter_with_id(&self.events)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.reader.len(&self.events)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.reader.is_empty(&self.events)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.iter().last();
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct EventReaderState<E: Event> {
    reader: LocalState<ManualEventReader<E>>,
    events: ResState<Events<E>>,
}

unsafe impl<E: Event> ReadOnlySystemParamFetch for EventReaderState<E> {}

unsafe impl<E: Event> SystemParamState for EventReaderState<E> {
    #[inline]
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        world.init_resource::<Events<E>>();

        Self {
            reader: LocalState::init(world, meta),
            events: ResState::init(world, meta),
        }
    }
}

impl<'w, 's, E: Event> SystemParamFetch<'w, 's> for EventReaderState<E> {
    type Item = EventReader<'w, 's, E>;

    #[inline]
    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        EventReader {
            reader: unsafe { self.reader.get_param(meta, world, change_tick) },
            events: unsafe { self.events.get_param(meta, world, change_tick) },
        }
    }
}

impl<'w, 's, E: Event> SystemParam for EventReader<'w, 's, E> {
    type Fetch = EventReaderState<E>;
}

#[derive(Debug)]
pub struct EventWriter<'w, E: Event> {
    events: ResMut<'w, Events<E>>,
}

impl<'w, E: Event> EventWriter<'w, E> {
    #[inline]
    pub fn send(&mut self, event: E) {
        self.events.send(event);
    }

    #[inline]
    pub fn send_default(&mut self)
    where
        E: Default,
    {
        self.events.send(E::default());
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct EventWriterState<E: Event> {
    events: ResMutState<Events<E>>,
}

unsafe impl<E: Event> SystemParamState for EventWriterState<E> {
    #[inline]
    fn init(world: &mut World, meta: &mut SystemMeta) -> Self {
        world.init_resource::<Events<E>>();

        Self {
            events: ResMutState::init(world, meta),
        }
    }
}

impl<'w, 's, E: Event> SystemParamFetch<'w, 's> for EventWriterState<E> {
    type Item = EventWriter<'w, E>;

    #[inline]
    unsafe fn get_param(
        &'s mut self,
        meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        EventWriter {
            events: unsafe { self.events.get_param(meta, world, change_tick) },
        }
    }
}

impl<'w, E: Event> SystemParam for EventWriter<'w, E> {
    type Fetch = EventWriterState<E>;
}

#[derive(Debug)]
pub struct ManualEventReader<E: Event> {
    last_event_count: usize,
    _marker: PhantomData<E>,
}

impl<E: Event> Default for ManualEventReader<E> {
    #[inline]
    fn default() -> Self {
        ManualEventReader {
            last_event_count: 0,
            _marker: PhantomData,
        }
    }
}

impl<E: Event> ManualEventReader<E> {
    #[inline]
    pub fn iter<'a>(&'a mut self, events: &'a Events<E>) -> impl DoubleEndedIterator<Item = &'a E> {
        self.iter_with_id(events).map(|(_, event)| event)
    }

    #[inline]
    pub fn iter_with_id<'a>(
        &'a mut self,
        events: &'a Events<E>,
    ) -> impl DoubleEndedIterator<Item = (EventId<E>, &'a E)> {
        let a_index = (self.last_event_count).saturating_sub(events.events_a.start_event_count);
        let b_index = (self.last_event_count).saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();

        self.last_event_count = events.event_count - unread_count;

        let iterator = a.iter().chain(b.iter());
        iterator
            .map(move |e| (e.id, &e.event))
            .inspect(move |(id, _)| {
                self.last_event_count = usize::max(id.id + 1, self.last_event_count)
            })
    }

    #[inline]
    pub fn missed_events(&self, events: &Events<E>) -> usize {
        events
            .oldest_event_count()
            .saturating_sub(self.last_event_count)
    }

    #[inline]
    pub fn len(&self, events: &Events<E>) -> usize {
        events
            .event_count
            .saturating_sub(self.last_event_count)
            .min(events.len())
    }

    #[inline]
    pub fn is_empty(&self, events: &Events<E>) -> bool {
        self.len(events) == 0
    }
}
