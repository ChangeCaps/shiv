use std::sync::atomic::{AtomicIsize, Ordering};

use fixedbitset::FixedBitSet;

/// An index into a [`World`](super::World).
///
/// Contains an `index` and a `generation`.
/// When an [`Entity`] is freed, the index is reused,
/// but the generation is incremented.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    #[inline]
    pub const fn from_raw_parts(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns the `index` of the entity.
    #[inline]
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// Returns the `generation` of the entity.
    #[inline]
    pub const fn generation(&self) -> u32 {
        self.generation
    }
}

impl std::fmt::Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}v{}", self.index, self.generation)
    }
}

impl std::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}v{}", self.index, self.generation)
    }
}

#[derive(Clone, Debug, Default)]
pub struct EntityIdSet {
    entities: FixedBitSet,
}

impl EntityIdSet {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    #[inline]
    pub fn resize(&mut self, len: usize, value: bool) {
        let old_len = self.entities.len();
        self.entities.grow(len);

        if len > old_len {
            self.entities.set_range(old_len.., value);
        } else {
            self.entities.set_range(len.., value);
        }
    }

    #[inline]
    pub fn insert(&mut self, index: usize) {
        self.entities.grow(index + 1);
        self.entities.insert(index);
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> bool {
        let has = self.entities.contains(index);
        self.entities.set(index, false);
        has
    }

    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        self.entities.contains(index)
    }

    #[inline]
    pub fn union_with(&mut self, other: &Self) {
        self.entities.union_with(&other.entities);
    }

    #[inline]
    pub fn intersect_with(&mut self, other: &Self) {
        self.entities.intersect_with(&other.entities);
    }

    #[inline]
    pub fn difference_with(&mut self, other: &Self) {
        self.entities.difference_with(&other.entities);
    }

    #[inline]
    pub fn iter(&self) -> fixedbitset::Ones<'_> {
        self.entities.ones()
    }
}

impl FromIterator<usize> for EntityIdSet {
    #[inline]
    fn from_iter<I: IntoIterator<Item = usize>>(iter: I) -> Self {
        Self {
            entities: iter.into_iter().collect(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityMeta {
    pub generation: u32,
    pub is_empty: bool,
}

impl EntityMeta {
    pub const EMPTY: Self = Self::new(0, true);

    #[inline]
    pub const fn new(generation: u32, is_empty: bool) -> Self {
        Self {
            generation,
            is_empty,
        }
    }
}

#[derive(Debug, Default)]
pub struct Entities {
    pub(crate) meta: Vec<EntityMeta>,
    pub(crate) entity_id_set: EntityIdSet,
    pending: Vec<u32>,
    free_cursor: AtomicIsize,
    len: u32,
}

impl Entities {
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        if let Some(meta) = self.meta.get(entity.index() as usize) {
            meta.generation == entity.generation() && !meta.is_empty
        } else {
            false
        }
    }

    #[inline]
    pub fn entity_ids(&self) -> &EntityIdSet {
        &self.entity_id_set
    }

    #[inline]
    pub fn len(&self) -> u32 {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn reserve(&self) -> Entity {
        let n = self.free_cursor.fetch_sub(1, Ordering::Relaxed);
        if n > 0 {
            let index = self.pending[n as usize - 1];
            let meta = &self.meta[index as usize];

            Entity {
                index,
                generation: meta.generation,
            }
        } else {
            Entity {
                index: u32::try_from(self.meta.len() as isize - n).expect("too many entities"),
                generation: 0,
            }
        }
    }

    #[inline]
    pub fn alloc(&mut self) -> Entity {
        self.flush();
        self.len += 1;
        if let Some(index) = self.pending.pop() {
            let new_free_cursor = self.pending.len() as isize;
            *self.free_cursor.get_mut() = new_free_cursor;

            self.entity_id_set.insert(index as usize);
            let meta = &mut self.meta[index as usize];
            meta.is_empty = false;

            Entity {
                index,
                generation: meta.generation,
            }
        } else {
            let index = u32::try_from(self.meta.len()).expect("too many entities");

            self.entity_id_set.insert(index as usize);
            self.meta.push(EntityMeta::default());

            Entity {
                index,
                generation: 0,
            }
        }
    }

    #[inline]
    pub fn alloc_at(&mut self, entity: Entity) -> bool {
        self.flush();

        let contains;
        if entity.index() as usize >= self.meta.len() {
            self.pending.extend(self.meta.len() as u32..entity.index());

            let new_free_cursor = self.pending.len() as isize;
            *self.free_cursor.get_mut() = new_free_cursor;

            self.meta
                .resize(entity.index() as usize + 1, EntityMeta::EMPTY);

            self.entity_id_set.insert(entity.index() as usize);
            self.len += 1;

            contains = false;
        } else if let Some(index) = self.pending.iter().position(|&i| i == entity.index()) {
            self.pending.swap_remove(index);

            let new_free_cursor = self.pending.len() as isize;
            *self.free_cursor.get_mut() = new_free_cursor;

            self.entity_id_set.insert(entity.index() as usize);
            self.len += 1;

            contains = false;
        } else {
            contains = true;
        }

        self.meta[entity.index() as usize] = EntityMeta::new(entity.generation(), false);

        contains
    }

    #[inline]
    pub fn free(&mut self, entity: Entity) -> bool {
        self.flush();

        let meta = if let Some(meta) = self.meta.get_mut(entity.index() as usize) {
            meta
        } else {
            return false;
        };

        if meta.generation != entity.generation() {
            return false;
        }
        meta.generation += 1;
        meta.is_empty = true;

        self.entity_id_set.remove(entity.index() as usize);

        self.pending.push(entity.index());

        let new_free_cursor = self.pending.len() as isize;
        *self.free_cursor.get_mut() = new_free_cursor;

        self.len -= 1;

        true
    }

    #[inline]
    pub fn needs_flush(&mut self) -> bool {
        *self.free_cursor.get_mut() != self.pending.len() as isize
    }

    #[inline]
    pub fn flush(&mut self) {
        if !self.needs_flush() {
            return;
        }

        let free_cursor = self.free_cursor.get_mut();
        let current = *free_cursor;

        let new_free_cursor = if current >= 0 {
            current as usize
        } else {
            let current = -current as usize;

            let old_meta_len = self.meta.len();
            let new_meta_len = old_meta_len + current;

            self.meta.resize(new_meta_len, EntityMeta::default());
            self.entity_id_set.resize(new_meta_len, true);

            self.len += current as u32;

            *free_cursor = 0;
            0
        };

        self.len += (self.pending.len() - new_free_cursor) as u32;
        for index in self.pending.drain(new_free_cursor..) {
            self.meta[index as usize].is_empty = false;
            self.entity_id_set.insert(index as usize);
        }
    }

    /// # Safety
    /// - `self.meta` must contain `index`
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> Entity {
        let generation = unsafe { self.meta.get_unchecked(index).generation };

        Entity {
            index: index as u32,
            generation,
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<Entity> {
        let meta = self.meta.get(index)?;

        if meta.is_empty {
            return None;
        }

        Some(Entity {
            index: index as u32,
            generation: meta.generation,
        })
    }
}
