use crate::SparseArray;

/// An index into a [`World`](super::world).
///
/// Contains an `index` and a `generation`.
/// When an [`Entity`] is freed, the index is reused,
/// but the generation is incremented.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// An allocator for [`Entity`]s.
#[derive(Clone, Debug, Default)]
pub struct Entities {
    entities: EntitySet,
    free: Vec<Entity>,
    next: u32,
}

impl Entities {
    /// Creates a new [`EntityAllocator`].
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(entity)
    }

    /// Allocates a new [`Entity`].
    ///
    /// If there are any free entities, their ids will be reused.
    #[inline]
    pub fn allocate(&mut self) -> Entity {
        let entity = if let Some(mut entity) = self.free.pop() {
            entity.generation += 1;
            entity
        } else {
            let id = self.next;
            self.next += 1;
            Entity::from_raw_parts(id, 0)
        };

        self.entities.insert(entity);

        entity
    }

    /// Frees an [`Entity`].
    ///
    /// The entity will be reused when allocating a new one.
    #[inline]
    pub fn free(&mut self, entity: Entity) -> bool {
        if self.entities.remove(entity) {
            self.free.push(entity);

            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EntitySet {
    entities: SparseArray<u32>,
}

impl EntitySet {
    #[inline]
    pub fn insert(&mut self, entity: Entity) {
        self.entities
            .insert(entity.index() as usize, entity.generation());
    }

    #[inline]
    pub fn remove(&mut self, entity: Entity) -> bool {
        self.entities.remove(entity.index() as usize).is_some()
    }

    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.get(entity.index() as usize) == Some(&entity.generation())
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities
            .iter()
            .map(|(index, generation)| Entity::from_raw_parts(index as u32, *generation))
    }
}
