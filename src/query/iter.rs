use crate::{QueryState, ReadOnlyWorldQuery, WorldQuery};

pub struct QueryIter<'s, Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
    query_state: &'s QueryState<Q, F>,
}
