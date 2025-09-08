use std::collections::hash_map::Entry;
use std::hash::{BuildHasher, Hash, RandomState};

use num_traits::NumCast;
use std::collections::HashMap;

#[derive(Debug)]
pub struct IndexMap<V, I, S = RandomState> {
    pub map: HashMap<V, I, S>,
    pub list: Vec<V>,
}

impl<V, I, S> IndexMap<V, I, S> {
    pub const fn with_hasher(hash_builder: S) -> IndexMap<V, I, S> {
        Self {
            map: HashMap::with_hasher(hash_builder),
            list: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl<V, I, S> Default for IndexMap<V, I, S>
where
    S: Default,
{
    fn default() -> IndexMap<V, I, S> {
        IndexMap::with_hasher(S::default())
    }
}

impl<V, I, S> IndexMap<V, I, S>
where
    I: NumCast,
    V: Hash + Eq,
    S: BuildHasher,
{
    // Return palette indices with a lifetime attached to
    // prevent jumbled indices when the palette is mutated.

    pub fn index(&self, value: &V) -> Option<&I> {
        self.map.get(value)
    }

    pub fn value(&self, index: I) -> Option<&V> {
        let index = index.to_usize()?;
        self.list.get(index)
    }

    pub fn entry(&mut self, key: V) -> Entry<'_, V, I> {
        self.map.entry(key)
    }

    pub fn index_or_add(&mut self, key: V) -> (&I, bool) {
        let next_index = self.get_next_index();
        match self.map.entry(key) {
            Entry::Occupied(occupied) => (occupied.into_mut(), false),
            Entry::Vacant(vacant) => (vacant.insert(next_index), true),
        }
    }

    pub fn get_next_index(&self) -> I {
        // TODO: replace panic with Result+error
        I::from(self.len() - 1).expect("index overflow")
    }
}
