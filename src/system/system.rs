use std::borrow::Cow;

use crate::{Access, ComponentId, World};

pub struct SystemMeta {
    pub name: Cow<'static, str>,
    pub access: Access<ComponentId>,
}

pub trait System {
    type In;
    type Out;

    fn meta(&self) -> SystemMeta;
    unsafe fn run(&mut self, input: Self::In, world: &World) -> Self::Out;
}
