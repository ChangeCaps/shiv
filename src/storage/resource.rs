pub trait Resource: Send + Sync + 'static {}

pub struct ResourceData {
    data: *mut dyn Resource,
}

impl ResourceData {
    #[inline]
    pub fn new<T: Resource>(data: T) -> Self {
        Self {
            data: Box::into_raw(Box::new(data)),
        }
    }
}

impl Drop for ResourceData {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: `self.data` was crated from a Box.
        unsafe { Box::from_raw(self.data) };
    }
}
