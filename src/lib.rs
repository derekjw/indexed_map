#[macro_use]

extern crate downcast_rs;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::cmp::Eq;
use std::marker::PhantomData;
use std::clone::Clone;
use std::ops::Deref;
use std::any::TypeId;
use downcast_rs::Downcast;

pub struct IndexedMap<K, V>
where
    K: Eq + Hash,
{
    inner: HashMap<K, V>,
    indices: HashMap<String, HashMap<TypeId, Box<IndexUpdater<K, V>>>>,
}

pub struct IndexId<A> {
    name: String,
    _value: PhantomData<A>,
}

impl<K, V> IndexedMap<K, V>
where
    K: 'static + Eq + Hash + Clone,
    V: 'static + Clone,
{
    pub fn new() -> IndexedMap<K, V> {
        IndexedMap {
            inner: HashMap::new(),
            indices: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.indices
            .values_mut()
            .flat_map(|x| x.values_mut())
            .for_each(|updater| updater.insert(&key, &value));
        self.inner.insert(key, value)
    }

    pub fn add_index<A, F>(&mut self, name: String, index_fn: F) -> IndexId<A>
    where
        A: 'static + Eq + Hash + Clone,
        F: 'static + Fn(&K, &V) -> Vec<A>,
    {
        let mut index_state = IndexState::<K, V, A>::empty(index_fn);
        for (key, value) in &self.inner {
            index_state.insert(key, value)
        }
        self.indices
            .entry(name.clone())
            .or_insert(HashMap::with_capacity(1))
            .insert(TypeId::of::<A>(), Box::new(index_state));
        IndexId {
            name,
            _value: PhantomData,
        }
    }

    fn get_index_state<A>(&self, index_id: &IndexId<A>) -> Option<&IndexState<K, V, A>>
    where
        A: 'static + Eq + Hash + Clone,
    {
        self.indices
            .get(&index_id.name)
            .and_then(|x| x.get(&TypeId::of::<A>()))
            .and_then(|x| x.downcast_ref::<IndexState<K, V, A>>())
    }

    pub fn get_index<A>(&self, index_id: &IndexId<A>) -> Option<&HashMap<A, HashSet<K>>>
    where
        A: 'static + Eq + Hash + Clone,
    {
        self.get_index_state(index_id).map(|x| &x.index)
    }

    pub fn filter_by_index<A>(
        &self,
        index_id: &IndexId<A>,
        index_key: &A,
    ) -> Option<HashMap<&K, &V>>
    where
        A: 'static + Eq + Hash + Clone,
    {
        self.get_index(index_id)
            .and_then(|x| x.get(&index_key))
            .map(|keys| {
                keys.iter()
                    .flat_map(|k| self.inner.get(&k).map(|v| (k, v)).into_iter())
                    .collect()
            })
    }

    pub fn keys_by_index<A>(&self, index_id: &IndexId<A>, index_key: &A) -> Option<&HashSet<K>>
    where
        A: 'static + Eq + Hash + Clone,
    {
        self.get_index(index_id).and_then(|x| x.get(&index_key))
    }
}

impl<K, V> Deref for IndexedMap<K, V>
where
    K: Eq + Hash,
{
    type Target = HashMap<K, V>;

    fn deref(&self) -> &HashMap<K, V> {
        &self.inner
    }
}

struct IndexState<K, V, A> {
    index_fn: Box<Fn(&K, &V) -> Vec<A>>,
    index: HashMap<A, HashSet<K>>,
    indexed: HashMap<K, HashSet<A>>,
}

impl<K, V, A> IndexState<K, V, A>
where
    K: Eq + Hash + Clone,
    V: Clone,
    A: Eq + Hash + Clone,
{
    fn empty<F>(index_fn: F) -> IndexState<K, V, A>
    where
        F: 'static + Fn(&K, &V) -> Vec<A>,
    {
        IndexState::new(index_fn, HashMap::new(), HashMap::new())
    }

    fn new<F>(
        index_fn: F,
        index: HashMap<A, HashSet<K>>,
        indexed: HashMap<K, HashSet<A>>,
    ) -> IndexState<K, V, A>
    where
        F: 'static + Fn(&K, &V) -> Vec<A>,
    {
        IndexState {
            index_fn: Box::new(index_fn),
            index,
            indexed,
        }
    }

    fn insert(&mut self, key: &K, value: &V) {
        let mut indexed_values: HashSet<A> = HashSet::new();
        (self.index_fn)(key, value).into_iter().for_each(|a| {
            self.index
                .entry(a.clone())
                .or_insert(HashSet::new())
                .insert(key.clone());
            indexed_values.insert(a);
        });
        self.indexed.insert(key.clone(), indexed_values);
    }
}

trait IndexUpdater<K, V>: Downcast {
    fn insert(&mut self, key: &K, value: &V);
}

impl_downcast!(IndexUpdater<K, V>);

impl<K, V, A> IndexUpdater<K, V> for IndexState<K, V, A>
where
    K: 'static + Eq + Hash + Clone,
    V: 'static + Clone,
    A: 'static + Eq + Hash + Clone,
{
    fn insert(&mut self, key: &K, value: &V) {
        IndexState::insert(self, key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_do_stuff() {
        let mut m = IndexedMap::<&str, &str>::new();
        m.insert("foo", "str1");
        let index_id = m.add_index("length".to_string(), |_, &v| vec![v.len()]);
        m.insert("foo2", "str2");
        m.insert("foo3", "string");
        let index = m.get_index(&index_id);
        let filtered = m.filter_by_index(&index_id, &4);
        println!("{:?}", index);
        println!("{:?}", filtered);
    }
}
