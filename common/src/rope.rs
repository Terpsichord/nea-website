use std::fmt;

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
        Self {
            root: Some(Self::build_rope(s)),
        }
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

            Box::new(RopeNode {
                left: Some(left_node.clone()),
                right: Some(right_node),
                weight: Self::rope_length_node(&Some(left_node)),
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
        Self::rope_length_node(&self.root)
    }

    pub fn char_at(&self, index: usize) -> Option<char> {
        Self::char_at_node(&self.root, index)
    }

    fn char_at_node(node: &Option<Box<RopeNode>>, index: usize) -> Option<char> {
        match node {
            None => None,
            Some(n) => {
                if let Some(val) = &n.value {
                    val.chars().nth(index)
                } else if index < n.weight {
                    Self::char_at_node(&n.left, index)
                } else {
                    Self::char_at_node(&n.right, index - n.weight)
                }
            }
        }
    }

    pub fn concat(self, other: Rope) -> Rope {
        let weight = Self::rope_length_node(&self.root);

        Rope {
            root: Some(Box::new(RopeNode {
                left: self.root,
                right: other.root,
                weight,
                value: None,
            })),
        }
    }

    pub fn split(self, index: usize) -> (Rope, Rope) {
        let (left, right) = Self::split_node(self.root, index);
        (
            Rope { root: left },
            Rope { root: right },
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
                    let left_str = &val[..index.min(val.len())];
                    let right_str = &val[index.min(val.len())..];

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
                } else if index < n.weight {
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

    pub fn insert(&mut self, index: usize, s: &str) {
        let current = self.root.take();
        let rope = Rope { root: current };

        let (left, right) = rope.split(index);
        let middle = Rope::new(s);

        self.root = left.concat(middle).concat(right).root;
    }

    pub fn delete(&mut self, start: usize, len: usize) {
        let current = self.root.take();
        let rope = Rope { root: current };

        let (left, rest) = rope.split(start);
        let ( _ , right) = rest.split(len);

        self.root = left.concat(right).root;
    }
}

impl fmt::Display for RopeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(val) = &self.value {
            write!(f, "{}", val)
        } else if let Some(left) = &self.left && let Some(right) = &self.right {
            write!(f, "{}{}", left, right)
        } else {
            unreachable!();
        }
    }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(root) = &self.root {
            write!(f, "{}", root)
        } else {
            write!(f, "")
        }
    }
}