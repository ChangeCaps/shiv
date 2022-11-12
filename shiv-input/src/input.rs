use std::hash::Hash;

use shiv::{hash_map::HashSet, prelude::EventReader, system::ResMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputState {
    Pressed,
    Released,
}

impl InputState {
    #[inline]
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }

    #[inline]
    pub fn is_released(self) -> bool {
        self == Self::Released
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputEvent<T> {
    pub input: T,
    pub state: InputState,
}

impl<T> InputEvent<T> {
    #[inline]
    pub const fn new(input: T, state: InputState) -> Self {
        Self { input, state }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input<T: Hash + Eq + Copy + Send + Sync + 'static> {
    held: HashSet<T>,
    pressed: HashSet<T>,
    released: HashSet<T>,
}

impl<T: Hash + Eq + Copy + Send + Sync + 'static> Default for Input<T> {
    fn default() -> Self {
        Self {
            held: HashSet::default(),
            pressed: HashSet::default(),
            released: HashSet::default(),
        }
    }
}

impl<T: Hash + Eq + Copy + Send + Sync + 'static> Input<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            held: HashSet::default(),
            pressed: HashSet::default(),
            released: HashSet::default(),
        }
    }

    #[inline]
    pub fn press(&mut self, value: T) {
        self.held.insert(value);
        self.pressed.insert(value);
    }

    #[inline]
    pub fn release(&mut self, value: T) {
        self.held.remove(&value);
        self.released.insert(value);
    }

    #[inline]
    pub fn is_held(&self, value: &T) -> bool {
        self.held.contains(value)
    }

    #[inline]
    pub fn is_pressed(&self, value: &T) -> bool {
        self.pressed.contains(value)
    }

    #[inline]
    pub fn is_released(&self, value: &T) -> bool {
        self.released.contains(value)
    }

    #[inline]
    pub fn iter_held(&self) -> impl ExactSizeIterator<Item = &T> {
        self.held.iter()
    }

    #[inline]
    pub fn iter_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.pressed.iter()
    }

    #[inline]
    pub fn iter_released(&self) -> impl ExactSizeIterator<Item = &T> {
        self.released.iter()
    }

    #[inline]
    pub fn update(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }

    #[inline]
    pub fn event_system(mut events: EventReader<InputEvent<T>>, mut input: ResMut<Self>) {
        for event in events.iter() {
            match event.state {
                InputState::Pressed => input.press(event.input),
                InputState::Released => input.release(event.input),
            }
        }
    }

    #[inline]
    pub fn update_system(mut input: ResMut<Self>) {
        input.update();
    }
}
