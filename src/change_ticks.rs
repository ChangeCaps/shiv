use std::ops::{Deref, DerefMut};

pub const CHECK_TICK_THRESHOLD: u32 = 518_400_000;

pub const MAX_CHANGE_AGE: u32 = u32::MAX - (2 * CHECK_TICK_THRESHOLD - 1);

pub struct Ticks<'w> {
    pub(crate) ticks: &'w mut ChangeTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'w> Ticks<'w> {
    #[inline]
    pub fn set_changed(&mut self) {
        self.ticks.set_changed(self.change_tick);
    }

    #[inline]
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }

    #[inline]
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }
}

pub struct Mut<'w, T> {
    pub(crate) value: &'w mut T,
    pub(crate) ticks: Ticks<'w>,
}

impl<'w, T> Mut<'w, T> {
    /// Sets the value but only marks is as changed if `self.value != value`.
    #[inline]
    pub fn set(&mut self, value: T)
    where
        T: PartialEq,
    {
        if *self.value != value {
            self.ticks.set_changed();
        }

        *self.value = value;
    }

    #[inline]
    pub fn get(&self) -> &T {
        self.value
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.ticks.set_changed();
        self.value
    }
}

impl<'w, T> Deref for Mut<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> DerefMut for Mut<'w, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChangeTicks {
    added: u32,
    changed: u32,
}

impl ChangeTicks {
    #[inline]
    pub fn is_added(&self, last_change_tick: u32, change_tick: u32) -> bool {
        let ticks_since_insert = change_tick.wrapping_sub(self.added).min(MAX_CHANGE_AGE);
        let ticks_since_system = change_tick
            .wrapping_sub(last_change_tick)
            .min(MAX_CHANGE_AGE);

        ticks_since_system > ticks_since_insert
    }

    #[inline]
    pub fn is_changed(&self, last_change_tick: u32, change_tick: u32) -> bool {
        let ticks_since_change = change_tick.wrapping_sub(self.changed).min(MAX_CHANGE_AGE);
        let ticks_since_system = change_tick
            .wrapping_sub(last_change_tick)
            .min(MAX_CHANGE_AGE);

        ticks_since_system > ticks_since_change
    }

    #[inline]
    pub fn new(change_tick: u32) -> Self {
        Self {
            added: change_tick,
            changed: change_tick,
        }
    }

    #[inline]
    pub fn set_changed(&mut self, change_tick: u32) {
        self.changed = change_tick;
    }

    #[inline]
    pub fn check_ticks(&mut self, change_tick: u32) {
        Self::check_tick(&mut self.added, change_tick);
        Self::check_tick(&mut self.changed, change_tick);
    }

    fn check_tick(tick: &mut u32, change_tick: u32) {
        let age = change_tick.wrapping_sub(*tick);

        if age > MAX_CHANGE_AGE {
            *tick = change_tick;
        }
    }
}
