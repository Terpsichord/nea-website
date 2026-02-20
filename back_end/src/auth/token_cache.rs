use std::{borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc};

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use crate::github::GithubUser;

/// Period of time after which the access token expires
//pub const ACCESS_EXPIRY: Duration = Duration::hours(8);
// TODO: i think this was safe to remove (double check tho)

#[derive(Clone, Debug)]
struct Node<K, V> {
    key: Option<K>,
    value: Option<V>,
    prev: usize,
    next: usize,
}

#[derive(Debug)]
pub struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, usize>,
    nodes: Vec<Node<K, V>>,
    head: usize,
    tail: usize,
}

impl<K: Eq + Hash + Clone, V: Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        let nodes = vec![
            // head
            Node {
                key: None,
                value: None,
                prev: 0,
                next: 1,
            },
            // tail
            Node {
                key: None,
                value: None,
                prev: 0,
                next: 1,
            },
        ];

        Self {
            capacity,
            map: HashMap::new(),
            nodes,
            head: 0,
            tail: 1,
        }
    }

    fn remove(&mut self, idx: usize) {
        let (prev, next) = {
            let n = &self.nodes[idx];
            (n.prev, n.next)
        };

        self.nodes[prev].next = next;
        self.nodes[next].prev = prev;
    }

    fn insert_to_front(&mut self, idx: usize) {
        let first = self.nodes[self.head].next;

        self.nodes[idx].next = first;
        self.nodes[idx].prev = self.head;

        self.nodes[first].prev = idx;
        self.nodes[self.head].next = idx;
    }

    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(&idx) = self.map.get(key) {
            self.remove(idx);
            self.insert_to_front(idx);

            self.nodes[idx].value.as_ref()
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        if let Some(&idx) = self.map.get(&key) {
            // update existing node
            self.nodes[idx].value = Some(value);
            self.remove(idx);
            self.insert_to_front(idx);
            return;
        }

        // evict LRU
        if self.map.len() == self.capacity {
            let lru = self.nodes[self.tail].prev;
            let lru_key = self.nodes[lru].key.clone().unwrap();

            self.remove(lru);
            self.map.remove(&lru_key);
        }

        // create new node
        let idx = self.nodes.len();
        self.nodes.push(Node {
            key: Some(key.clone()),
            value: Some(value),
            prev: 0,
            next: 0,
        });

        self.map.insert(key, idx);
        self.insert_to_front(idx);
    }
}

#[derive(Copy, Clone, Debug)]
/// A struct containing information associated with a given access token
pub struct TokenInfo {
    /// Github ID of the user that the access token belongs to
    pub github_id: i32,
    /// Date by which the token expires (and must be refreshed)
    pub expiry_date: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
/// Table of `TokenInfo` cached with encrypted access tokens
pub struct TokenCache(Arc<Mutex<LruCache<String, TokenInfo>>>);

impl TokenCache {
    const DEFAULT_CAPACITY: usize = 50;

    pub async fn cache_user_token(
        &self,
        user: &GithubUser,
        encrypted_token: String,
        expiry_date: Option<DateTime<Utc>>,
    ) {
        self.0.lock().await.put(
            encrypted_token,
            TokenInfo {
                github_id: user.id,
                expiry_date,
            },
        );
    }

    /// Gets the stored token info for the given token
    ///
    /// Returns None if the token can't be found
    pub async fn get_token_info(&self, token: &str) -> Option<TokenInfo> {
        self.0.lock().await.get(token).copied()
    }
}

impl Default for TokenCache {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(LruCache::new(
            Self::DEFAULT_CAPACITY,
        ))))
    }
}
