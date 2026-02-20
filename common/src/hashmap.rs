use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
struct Entry<K, V> {
    key: Option<K>,
    value: Option<V>,
    occupied: bool,
}

pub struct HashMap<K, V> {
    entries: Vec<Entry<K, V>>,
    size: usize,
    capacity: usize,
}

impl<K: Eq + Hash + Clone, V: Clone> HashMap<K, V> {
    pub fn new(cap: usize) -> Self {
        let mut entries = Vec::with_capacity(cap);
        for _ in 0..cap {
            entries.push(Entry {
                key: None,
                value: None,
                occupied: false,
            });
        }
        HashMap {
            entries,
            size: 0,
            capacity: cap,
        }
    }

    fn hash<Q>(key: &Q) -> usize
    where
        K: std::borrow::Borrow<Q>,
        Q: ?Sized + Hash,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    fn resize(&mut self) {
        let new_capacity = self.capacity * 2;
        let mut new_entries = Vec::with_capacity(new_capacity);
        for _ in 0..new_capacity {
            new_entries.push(Entry {
                key: None,
                value: None,
                occupied: false,
            });
        }

        for entry in &self.entries {
            if entry.occupied {
                let k = entry.key.as_ref().unwrap().clone();
                let v = entry.value.as_ref().unwrap().clone();
                Self::insert_in_entries(&mut new_entries, new_capacity, k, v);
            }
        }

        self.entries = new_entries;
        self.capacity = new_capacity;
    }

    fn insert_in_entries(entries: &mut [Entry<K, V>], cap: usize, key: K, value: V) {
        let mut idx = Self::hash(&key);
        loop {
            if !entries[idx].occupied {
                entries[idx].key = Some(key);
                entries[idx].value = Some(value);
                entries[idx].occupied = true;
                return;
            } else if entries[idx].key.as_ref().unwrap() == &key {
                entries[idx].value = Some(value);
                return;
            } else {
                idx = (idx + 1) % cap;
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.size >= self.capacity / 2 {
            self.resize();
        }
        Self::insert_in_entries(&mut self.entries, self.capacity, key, value);
        self.size += 1;
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let mut idx = Self::hash(key) % self.capacity;
        let start_idx = idx;

        loop {
            if !self.entries[idx].occupied {
                return None;
            }
            if self.entries[idx].key.as_ref().unwrap() == key {
                return self.entries[idx].value.as_ref();
            }
            idx = (idx + 1) % self.capacity;
            if idx == start_idx {
                break;
            }
        }

        None
    }

    pub fn remove(&mut self, key: &K) {
        let mut idx = Self::hash(key) % self.capacity;
        let start_idx = idx;

        loop {
            if !self.entries[idx].occupied {
                return;
            }
            if self.entries[idx].key.as_ref().unwrap() == key {
                self.entries[idx].occupied = false;
                self.entries[idx].key = None;
                self.entries[idx].value = None;
                self.size -= 1;

                let mut next_idx = (idx + 1) % self.capacity;
                while self.entries[next_idx].occupied {
                    let k = self.entries[next_idx].key.take().unwrap();
                    let v = self.entries[next_idx].value.take().unwrap();
                    self.entries[next_idx].occupied = false;
                    self.size -= 1;
                    self.insert(k, v);
                    next_idx = (next_idx + 1) % self.capacity;
                }

                return;
            }
            idx = (idx + 1) % self.capacity;
            if idx == start_idx {
                break;
            }
        }
    }
}
