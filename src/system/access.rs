use std::marker::PhantomData;

use fixedbitset::FixedBitSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Access<T> {
    read: FixedBitSet,
    write: FixedBitSet,

    read_all: bool,

    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for Access<T> {
    #[inline]
    fn default() -> Self {
        Self {
            read: FixedBitSet::with_capacity(0),
            write: FixedBitSet::with_capacity(0),

            read_all: false,

            _marker: PhantomData,
        }
    }
}

impl<T> Access<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn grow(&mut self, new_capacity: usize) {
        self.read.grow(new_capacity);
        self.write.grow(new_capacity);
    }
}

impl<T> Access<T>
where
    T: Into<usize> + From<usize> + Copy,
{
    #[inline]
    pub fn add_read(&mut self, index: T) {
        let index = index.into();

        self.read.grow(index + 1);
        self.read.insert(index);
    }

    #[inline]
    pub fn add_write(&mut self, index: T) {
        let index = index.into();

        self.read.grow(index + 1);
        self.write.grow(index + 1);

        self.read.insert(index);
        self.write.insert(index);
    }

    #[inline]
    pub fn read_all(&mut self) {
        self.read_all = true;
    }

    #[inline]
    pub fn has_read(&self, index: T) -> bool {
        self.read_all || self.read.contains(index.into())
    }

    #[inline]
    pub fn has_write(&self, index: T) -> bool {
        self.write.contains(index.into())
    }

    #[inline]
    pub fn clear(&mut self) {
        self.read.clear();
        self.write.clear();
        self.read_all = false;
    }

    #[inline]
    pub fn extend(&mut self, other: &Self) {
        self.read.union_with(&other.read);
        self.write.union_with(&other.write);
        self.read_all |= other.read_all;
    }

    #[inline]
    pub fn is_compatible(&self, other: &Self) -> bool {
        if self.read_all {
            return other.write.count_ones(..) == 0;
        }

        if other.read_all {
            return self.write.count_ones(..) == 0;
        }

        self.write.is_disjoint(&other.read) && self.read.is_disjoint(&other.write)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FilteredAccess<T> {
    access: Access<T>,
    with: FixedBitSet,
    without: FixedBitSet,
}

impl<T> Default for FilteredAccess<T> {
    #[inline]
    fn default() -> Self {
        Self {
            access: Access::default(),
            with: FixedBitSet::with_capacity(0),
            without: FixedBitSet::with_capacity(0),
        }
    }
}

impl<T> FilteredAccess<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn grow(&mut self, new_capacity: usize) {
        self.access.grow(new_capacity);
        self.with.grow(new_capacity);
        self.without.grow(new_capacity);
    }
}

impl<T> FilteredAccess<T>
where
    T: Into<usize> + From<usize> + Copy,
{
    #[inline]
    pub fn access(&self) -> &Access<T> {
        &self.access
    }

    #[inline]
    pub fn access_mut(&mut self) -> &mut Access<T> {
        &mut self.access
    }

    #[inline]
    pub fn add_read(&mut self, index: T) {
        self.access.add_read(index);
        self.add_with(index);
    }

    #[inline]
    pub fn add_write(&mut self, index: T) {
        self.access.add_write(index);
        self.add_with(index);
    }

    #[inline]
    pub fn add_with(&mut self, index: T) {
        let index = index.into();

        self.with.grow(index + 1);
        self.with.insert(index);
    }

    #[inline]
    pub fn add_without(&mut self, index: T) {
        let index = index.into();

        self.without.grow(index + 1);
        self.without.insert(index);
    }

    #[inline]
    pub fn compatible(&self, other: &Self) -> bool {
        if self.access().is_compatible(other.access()) {
            true
        } else {
            self.with.intersection(&other.without).next().is_some()
                && self.without.intersection(&other.with).next().is_some()
        }
    }
}
