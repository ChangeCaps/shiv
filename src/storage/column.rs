use std::{alloc::Layout, cell::UnsafeCell};

use crate::{change_detection::ChangeTicks, world::ComponentDescriptor};

use super::BlobVec;

#[derive(Debug)]
pub struct Column {
    data: BlobVec,
    ticks: Vec<UnsafeCell<ChangeTicks>>,
}

unsafe impl Sync for Column {}
unsafe impl Send for Column {}

impl Column {
    #[inline]
    pub fn with_capacity(descriptor: &ComponentDescriptor, capacity: usize) -> Self {
        Self {
            data: BlobVec::new(descriptor.layout(), descriptor.drop(), capacity),
            ticks: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn item_layout(&self) -> Layout {
        self.data.item_layout()
    }

    /// # Safety
    /// - Data at `index` must already be allocated
    #[inline]
    pub unsafe fn initialize(&mut self, index: usize, data: *mut u8, ticks: ChangeTicks) {
        unsafe { self.data.initialize_unchecked(index, data) };
        self.ticks.push(UnsafeCell::new(ticks));
    }

    /// # Safety
    /// - Data at `index` must already be allocated
    #[inline]
    pub unsafe fn replace(&mut self, index: usize, data: *mut u8, change_tick: u32) {
        debug_assert!(index < self.len());
        unsafe { self.data.replace_unchecked(index, data) };
        let ticks = unsafe { self.ticks.get_unchecked_mut(index) };
        ticks.get_mut().set_changed(change_tick);
    }

    /// # Safety
    /// - Data at `index` must already be allocated
    #[inline]
    pub unsafe fn replace_untracked(&mut self, index: usize, data: *mut u8) {
        debug_assert!(index < self.len());
        unsafe { self.data.replace_unchecked(index, data) };
    }

    /// # Safety
    /// - `data` must point to valid data of column's component type
    #[inline]
    pub unsafe fn push(&mut self, data: *mut u8, change_ticks: ChangeTicks) {
        unsafe { self.data.push(data) };
        self.ticks.push(UnsafeCell::new(change_ticks));
    }

    /// # Safety
    /// - `index` must be in bounds
    /// - `data` must point to valid data of column's component type
    #[inline]
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize, component: *mut u8) {
        debug_assert!(index < self.len());
        unsafe { self.data.swap_remove_unchecked(index, component) };
        self.ticks.swap_remove(index);
    }

    /// # Safety
    /// - `index` must be in bounds
    #[inline]
    pub unsafe fn swap_remove_and_drop_unchecked(&mut self, index: usize) {
        unsafe { self.data.swap_remove_and_drop_unchecked(index) };
        self.ticks.swap_remove(index);
    }

    #[inline]
    pub fn get_data(&self, index: usize) -> Option<*mut u8> {
        if index < self.len() {
            unsafe { Some(self.data.get_unchecked(index)) }
        } else {
            None
        }
    }
    /// # Safety
    /// - `index` must be in bounds
    #[inline]
    pub unsafe fn get_data_unchecked(&self, index: usize) -> *mut u8 {
        unsafe { self.data.get_unchecked(index) }
    }

    #[inline]
    pub fn get_ticks(&self, index: usize) -> Option<&UnsafeCell<ChangeTicks>> {
        self.ticks.get(index)
    }

    /// # Safety
    /// - `index` must be in bounds
    #[inline]
    pub unsafe fn get_ticks_unchecked(&self, index: usize) -> &UnsafeCell<ChangeTicks> {
        unsafe { self.ticks.get_unchecked(index) }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
        self.ticks.clear();
    }

    #[inline]
    pub fn check_change_ticks(&mut self, change_tick: u32) {
        for ticks in self.ticks.iter_mut() {
            ticks.get_mut().check_ticks(change_tick);
        }
    }
}
