pub type RandomState = ahash::RandomState;

pub type HashMap<K, V> = hashbrown::HashMap<K, V, RandomState>;
