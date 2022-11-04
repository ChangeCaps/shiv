use std::{
    alloc::{self, Layout},
    ptr::{self, NonNull},
};

#[derive(Debug)]
pub struct BlobVec {
    item_layout: Layout,
    drop: Option<unsafe fn(*mut u8)>,
    capacity: usize,
    len: usize,
    data: NonNull<u8>,
}

impl BlobVec {
    #[inline]
    pub fn new(item_layout: Layout, drop: Option<unsafe fn(*mut u8)>, capacity: usize) -> BlobVec {
        if item_layout.size() == 0 {
            Self {
                item_layout,
                drop,
                capacity: 0,
                len: 0,
                data: NonNull::dangling(),
            }
        } else {
            let mut this = Self {
                item_layout,
                drop,
                capacity: 0,
                len: 0,
                data: NonNull::dangling(),
            };
            this.reserve_exact(capacity);

            this
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub const fn item_layout(&self) -> Layout {
        self.item_layout
    }

    /// # Safety
    /// - `index` must be in bounds
    #[inline]
    pub const unsafe fn get_unchecked(&self, index: usize) -> *mut u8 {
        debug_assert!(index < self.len);

        unsafe { self.data.as_ptr().add(index * self.item_layout.size()) }
    }

    /// # Safety
    /// - `index` must be in bounds
    #[inline]
    pub unsafe fn initialize_unchecked(&mut self, index: usize, value: *mut u8) {
        debug_assert!(index < self.len);

        let ptr = unsafe { self.get_unchecked(index) };
        unsafe { ptr::copy_nonoverlapping(value, ptr, self.item_layout.size()) };
    }

    /// # Safety
    /// - `index` must be in bounds
    /// - the memory at `index` must be initialized matching `self.item_layout`
    #[inline]
    pub unsafe fn replace_unchecked(&mut self, index: usize, value: *mut u8) {
        let len = self.len;
        self.len = 0;

        let ptr = unsafe { self.get_unchecked(index) };
        if let Some(drop) = self.drop {
            unsafe { drop(ptr) };
        }

        unsafe { ptr::copy_nonoverlapping(value, ptr, self.item_layout.size()) };
        self.len = len;
    }

    /// # Safety
    /// - `value` must be a valid this [`BlobVec`]
    #[inline]
    pub unsafe fn push(&mut self, value: *mut u8) {
        self.reserve_exact(1);
        let index = self.len();
        self.len += 1;
        unsafe { self.initialize_unchecked(index, value) };
    }

    /// # Safety
    /// - `index` must be in bounds
    /// - the memory at `index` must be initialized
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize, ptr: *mut u8) {
        debug_assert!(index < self.len);

        let last = unsafe { self.get_unchecked(self.len - 1) };
        let target = unsafe { self.get_unchecked(index) };

        unsafe { ptr::copy_nonoverlapping(last, ptr, self.item_layout.size()) };
        unsafe { ptr::copy(target, last, self.item_layout.size()) };

        self.len -= 1;
    }

    /// # Safety
    /// - `index` must be in bounds
    /// - the memory at `index` must be initialized
    pub unsafe fn swap_remove_and_drop_unchecked(&mut self, index: usize) {
        debug_assert!(index < self.len);

        let last = unsafe { self.get_unchecked(self.len - 1) };
        let target = unsafe { self.get_unchecked(index) };

        if last != target {
            unsafe { ptr::swap_nonoverlapping(last, target, self.item_layout.size()) };
        }

        self.len -= 1;

        if let Some(drop) = self.drop {
            unsafe { drop(last) };
        }
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        let available = self.capacity - self.len;

        if available < additional && self.item_layout.size() != 0 {
            let increment = additional - available;

            // SAFETY: self.item_layout is is not zero-sized.
            unsafe { self.grow_exact(increment) }
        }
    }

    /// # Safety
    /// - `self.layout` must not be zero-sized.
    /// - `additional` must be greater than 0
    #[inline]
    unsafe fn grow_exact(&mut self, additional: usize) {
        let new_capacity = self.capacity + additional;
        let new_layout = array_layout(self.item_layout, new_capacity).unwrap();

        let new_data = if self.capacity == 0 {
            // SAFETY: `self.item_layout` is not zero-sized as per safety requirement.
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = array_layout(self.item_layout, self.capacity).unwrap();

            // SAFETY:
            // - ptr was allocated via this allocator.
            // - the layout is the same as the one used to allocate it.
            // - `self.item_layout.size() > 0` and `new_capacity > 0`, so new_layout is not
            // zero-sized.
            unsafe { alloc::realloc(self.data.as_ptr(), old_layout, new_layout.size()) }
        };

        self.data = NonNull::new(new_data).unwrap_or_else(|| alloc::handle_alloc_error(new_layout));
        self.capacity = new_capacity;
    }

    #[inline]
    pub fn clear(&mut self) {
        let len = self.len;
        self.len = 0;

        if let Some(drop) = self.drop {
            let layout_size = self.item_layout.size();

            for i in 0..len {
                // SAFETY: `i` is in bounds.
                let ptr = unsafe { self.data.as_ptr().add(i * layout_size) };
                unsafe { drop(ptr) }
            }
        }
    }
}

#[inline]
fn array_layout(layout: Layout, len: usize) -> Option<Layout> {
    let padded_size = layout.pad_to_align().size();
    let alloc_size = padded_size.checked_mul(len)?;

    let layout = unsafe { Layout::from_size_align_unchecked(alloc_size, layout.align()) };
    Some(layout)
}

impl Drop for BlobVec {
    #[inline]
    fn drop(&mut self) {
        self.clear();

        let layout = array_layout(self.item_layout, self.capacity).unwrap();
        if layout.size() != 0 {
            // SAFETY:
            // - ptr was allocated via this allocator.
            // - the layout is the same as the one used to allocate it.
            unsafe { alloc::dealloc(self.data.as_ptr(), layout) }
        }
    }
}
