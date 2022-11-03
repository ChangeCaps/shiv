use std::marker::PhantomData;

use fixedbitset::FixedBitSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Access<T> {
    read: FixedBitSet,
    write: FixedBitSet,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Access<T> {}
