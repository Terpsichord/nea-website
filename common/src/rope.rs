use std::{any::TypeId, fmt, ops::Range};

use egui::TextBuffer;
use serde::{Deserialize, Serialize};

const LEAF_SIZE: usize = 32;

#[derive(Clone)]
struct RopeNode {
    left: Option<Box<RopeNode>>,
    right: Option<Box<RopeNode>>,
    weight: usize,
    value: Option<String>,
}

#[derive(Clone)]
pub struct Rope {
    root: Option<Box<RopeNode>>,
}

impl Rope {
    pub fn new(s: &str) -> Self {
        Self::from_root(Some(Self::build_rope(s)))
    }

    fn from_root(root: Option<Box<RopeNode>>) -> Self {
        let _flat_cache = if let Some(ref r) = root {
            r.to_string()
        } else {
            String::new()
        };

        Self { root, _flat_cache }
    }

    fn build_rope(s: &str) -> Box<RopeNode> {
        if s.len() <= LEAF_SIZE {
            Box::new(RopeNode {
                left: None,
                right: None,
                weight: s.len(),
                value: Some(s.to_string()),
            })
        } else {
            let mid = s.len() / 2;

            let left_node = Self::build_rope(&s[..mid]);
            let right_node = Self::build_rope(&s[mid..]);

            let weight = Self::rope_length_node(&Some(left_node.clone()));

            Box::new(RopeNode {
                left: Some(left_node),
                right: Some(right_node),
                weight,
                value: None,
            })
        }
    }

    fn rope_length_node(node: &Option<Box<RopeNode>>) -> usize {
        match node {
            None => 0,
            Some(n) => {
                if let Some(val) = &n.value {
                    val.len()
                } else {
                    Self::rope_length_node(&n.left)
                        + Self::rope_length_node(&n.right)
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self._flat_cache.len()
    }

    pub fn char_at(&self, index: usize) -> Option<char> {
        self._flat_cache.chars().nth(index)
    }

    pub fn concat(self, other: Rope) -> Rope {
        let weight = Self::rope_length_node(&self.root);

        let new_root = Some(Box::new(RopeNode {
            left: self.root,
            right: other.root,
            weight,
            value: None,
        }));

        Rope::from_root(new_root)
    }

    pub fn split(self, index: usize) -> (Rope, Rope) {
        let (left, right) = Self::split_node(self.root, index);

        (
            Rope::from_root(left),
            Rope::from_root(right),
        )
    }

    fn split_node(
        node: Option<Box<RopeNode>>,
        index: usize,
    ) -> (Option<Box<RopeNode>>, Option<Box<RopeNode>>) {
        match node {
            None => (None, None),
            Some(n) => {
                if let Some(val) = &n.value {
                    let split_index = index.min(val.len());

                    let left_str = &val[..split_index];
                    let right_str = &val[split_index..];

                    let left_node = if left_str.is_empty() {
                        None
                    } else {
                        Some(Self::build_rope(left_str))
                    };

                    let right_node = if right_str.is_empty() {
                        None
                    } else {
                        Some(Self::build_rope(right_str))
                    };

                    (left_node, right_node)
                } else {
                    if index < n.weight {
                        let (l, r) = Self::split_node(n.left, index);

                        let weight = Self::rope_length_node(&r);

                        let new_right = Some(Box::new(RopeNode {
                            left: r,
                            right: n.right,
                            weight,
                            value: None,
                        }));

                        (l, new_right)
                    } else {
                        let (l, r) =
                            Self::split_node(n.right, index - n.weight);

                        let weight = Self::rope_length_node(&n.left);

                        let new_left = Some(Box::new(RopeNode {
                            left: n.left,
                            right: l,
                            weight,
                            value: None,
                        }));

                        (new_left, r)
                    }
                }
            }
        }
    }

    pub fn insert(&mut self, index: usize, s: &str) {
        let current = std::mem::replace(&mut self.root, None);
        let rope = Rope::from_root(current);

        let (left, right) = rope.split(index);
        let middle = Rope::new(s);

        let result = left.concat(middle).concat(right);

        self.root = result.root;
        self._flat_cache = result._flat_cache;
    }

    pub fn delete(&mut self, start: usize, len: usize) {
        let current = std::mem::replace(&mut self.root, None);
        let rope = Rope::from_root(current);

        let (left, rest) = rope.split(start);
        let (_, right) = rest.split(len);

        let result = left.concat(right);

        self.root = result.root;
        self._flat_cache = result._flat_cache;
    }
}

impl fmt::Display for RopeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(val) = &self.value {
            write!(f, "{}", val)
        } else {
            if let Some(left) = &self.left {
                write!(f, "{}", left)?;
            }
            if let Some(right) = &self.right {
                write!(f, "{}", right)?;
            }
            Ok(())
        }
    }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self._flat_cache)
    }
}