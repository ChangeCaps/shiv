use std::marker::PhantomData;

use fixedbitset::FixedBitSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Access<T> {
    read: FixedBitSet,
    write: FixedBitSet,

    read_all: bool,
    entities: bool,

    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for Access<T> {
    #[inline]
    fn default() -> Self {
        Self {
            read: FixedBitSet::with_capacity(0),
            write: FixedBitSet::with_capacity(0),

            read_all: false,
            entities: false,

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
    pub fn read_entities(&mut self) {
        self.entities = true;
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
    pub fn is_entities(&self) -> bool {
        self.read.is_empty() && !self.read_all && self.entities
    }

    #[inline]
    pub fn get_conflicts(&self, other: &Self) -> Vec<T> {
        let mut conflicts = Vec::new();

        for read in self.iter_read() {
            if other.has_write(read) {
                conflicts.push(read);
            }
        }

        for write in self.iter_write() {
            if other.has_read(write) {
                conflicts.push(write);
            }
        }

        conflicts
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

    #[inline]
    pub fn iter_read(&self) -> impl Iterator<Item = T> + '_ {
        self.read.ones().map(T::from)
    }

    #[inline]
    pub fn iter_write(&self) -> impl Iterator<Item = T> + '_ {
        self.write.ones().map(T::from)
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
    pub fn read_all(&mut self) {
        self.access.read_all();
    }

    #[inline]
    pub fn read_entities(&mut self) {
        self.access.read_entities();
    }

    #[inline]
    pub fn has_read(&self, index: T) -> bool {
        self.access.has_read(index)
    }

    #[inline]
    pub fn has_write(&self, index: T) -> bool {
        self.access.has_write(index)
    }

    #[inline]
    pub fn has_with(&self, index: T) -> bool {
        self.with.contains(index.into())
    }

    #[inline]
    pub fn has_without(&self, index: T) -> bool {
        self.without.contains(index.into())
    }

    #[inline]
    pub fn extend(&mut self, other: &Self) {
        self.access.extend(&other.access);
        self.with.union_with(&other.with);
        self.without.union_with(&other.without);
    }

    #[inline]
    pub fn is_entities(&self) -> bool {
        self.access.is_entities()
    }

    #[inline]
    pub fn get_conflicts(&self, other: &Self) -> Vec<T> {
        self.access.get_conflicts(&other.access)
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

    #[inline]
    pub fn iter_read(&self) -> impl Iterator<Item = T> + '_ {
        self.access.iter_read()
    }

    #[inline]
    pub fn iter_write(&self) -> impl Iterator<Item = T> + '_ {
        self.access.iter_write()
    }

    #[inline]
    pub fn iter_with(&self) -> impl Iterator<Item = T> + '_ {
        self.with.ones().map(T::from)
    }

    #[inline]
    pub fn iter_without(&self) -> impl Iterator<Item = T> + '_ {
        self.without.ones().map(T::from)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.access.clear();
        self.with.clear();
        self.without.clear();
    }
}
