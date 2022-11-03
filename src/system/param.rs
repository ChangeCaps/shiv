use crate::{SystemMeta, World};

pub trait SystemParam {
    type Fetch: for<'w, 's> SystemParamFetch<'w, 's>;
}

pub trait SystemParamFetch<'w, 's> {
    type Item: SystemParam<Fetch = Self>;

    unsafe fn get_param(&'s mut self, meta: &SystemMeta, world: &'w World) -> Self::Item;
}
