//! Provides the [`World`] type, which stores all data in the ECS.

mod component;
mod entity;
mod entity_ref;
mod world;

pub use component::*;
pub use entity::*;
pub use entity_ref::*;
pub use world::*;
