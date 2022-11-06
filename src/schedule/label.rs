use std::any::TypeId;

pub use shiv_macro::{StageLabel, SystemLabel};

macro_rules! define_label {
    (
        $(#[$trait_meta:meta])*
        $trait:ident,

        $(#[$id_meta:meta])*
        $id:ident $(,)?
    ) => {
        $(#[$trait_meta])*
        pub trait $trait {
            fn label(self) -> $id;
        }

        impl<T: $trait + Copy> $trait for &T {
            fn label(self) -> $id {
                (*self).label()
            }
        }

        impl $trait for $id {
            fn label(self) -> $id {
                self
            }
        }

        $(#[$id_meta])*
        #[derive(Clone, Copy, Debug)]
        pub struct $id {
            type_id: TypeId,
            name: &'static str,
            variant: u32,
        }

        impl $id {
            #[inline]
            pub fn from_raw_parts<T: 'static>(name: &'static str, variant: u32) -> Self {
                Self {
                    type_id: TypeId::of::<T>(),
                    name,
                    variant
                }
            }
        }

        impl PartialEq for $id {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.type_id == other.type_id && self.variant == other.variant
            }
        }

        impl Eq for $id {}

        impl ::std::hash::Hash for $id {
            #[inline]
            fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                self.type_id.hash(state);
                self.variant.hash(state);
            }
        }

        impl ::std::fmt::Display for $id {
            #[inline]
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.name)
            }
        }
    };
}

define_label!(
    /// A label that can be used to identify a schedule stage.
    StageLabel,
    /// A unique identifier for a schedule stage.
    StageLabelId,
);
define_label!(
    /// A label that can be used to identify a system.
    SystemLabel,
    /// A unique identifier for a system.
    SystemLabelId
);
