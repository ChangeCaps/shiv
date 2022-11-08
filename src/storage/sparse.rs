#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SparseArray<T> {
    data: Vec<Option<T>>,
}

impl<T> Default for SparseArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SparseArray<T> {
    #[inline]
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        self.data.get(index).map_or(false, Option::is_some)
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)?.as_ref()
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)?.as_mut()
    }

    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        unsafe { self.data.get_unchecked(index).as_ref().unwrap_unchecked() }
    }

    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        unsafe {
            self.data
                .get_unchecked_mut(index)
                .as_mut()
                .unwrap_unchecked()
        }
    }

    #[inline]
    pub fn insert(&mut self, index: usize, value: T) {
        if index >= self.data.len() {
            self.data.resize_with(index + 1, Default::default);
        }

        self.data[index] = Some(value)
    }

    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<T> {
        self.data.get_mut(index)?.take()
    }

    #[inline]
    pub unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        unsafe { self.data.get_unchecked_mut(index).take().unwrap_unchecked() }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(index, value)| value.as_ref().map(|value| (index, value)))
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        self.data
            .iter_mut()
            .enumerate()
            .filter_map(|(index, value)| value.as_mut().map(|value| (index, value)))
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.clear()
    }
}
