#![allow(unused)]




use core::ops::Range;
use std::cmp::Ordering::{Less, Greater, Equal};
use std::iter::FromIterator;




pub trait Interval: Ord {
    type Scalar: Ord + Copy;
    fn new(a: Self::Scalar, b: Self::Scalar) -> Self;
    fn lower(&self) -> Self::Scalar;
    fn upper(&self) -> Self::Scalar;
}




/**
 * A node in a binary search tree
 */
struct Node<K: Interval> {
    key: K,
    max: K::Scalar,
    l: Option<Box<Node<K>>>,
    r: Option<Box<Node<K>>>,
}




// ============================================================================
impl<K: Interval> Node<K> {




    /**
     * Create an empty sub-tree with the given key
     */
    fn new(key: K) -> Self {
        Self { max: key.upper(), key, l: None, r: None }
    }




    /**
     * Create a sub-tree from a slice, which is balanced if the slice is sorted.
     */
    fn from_slice(slice: &mut [Option<K>]) -> Option<Box<Self>> {
        if slice.is_empty() {
            None
        } else {
            let mid = slice.len() / 2;
            let key = slice[mid].take().unwrap();
            let l = Self::from_slice(&mut slice[..mid]);
            let r = Self::from_slice(&mut slice[mid + 1..]);
            let max = Self::local_max(key.upper(), &l, &r);
            Some(Box::new(Self { key, max, l, r }))
        }
    }




    /**
     * Return the number of nodes contained in this sub-tree (including self).
     */
    fn len(&self) -> usize {
        self.l.as_ref().map_or(0, |l| l.len()) +
        self.r.as_ref().map_or(0, |r| r.len()) + 1
    }




    /**
     * Return the height of this sub-tree
     */
    fn height(&self) -> usize {
        self.l.as_ref().map_or(0, |l| l.height()).max(
        self.r.as_ref().map_or(0, |r| r.height())) + 1
    }




    /**
     * Return true of the given key exists in this sub-tree.
     */
    fn contains(&self, key: &K) -> bool {
        match key.cmp(&self.key) {
            Less    => self.l.as_ref().map_or(false, |l| l.contains(key)),
            Greater => self.r.as_ref().map_or(false, |r| r.contains(key)),
            Equal   => true
        }
    }




    /**
     * Insert a node with the given key into this sub-tree.
     */
    fn insert(node: &mut Option<Box<Node<K>>>, key: K) {
        if let Some(n) = node {

            n.max = key.upper().max(n.max);

            match key.cmp(&n.key) {
                Less    => Self::insert(&mut n.l, key),
                Greater => Self::insert(&mut n.r, key),
                Equal => {}
            }
        } else {
            *node = Some(Box::new(Node::new(key)))
        }
    }




    /**
     * Remove a node with the given key from this sub-tree.
     */
    fn remove(node: &mut Option<Box<Node<K>>>, key: &K) {
        if let Some(n) = node {
            match key.cmp(&n.key) {
                Less    => Self::remove(&mut n.l, key),
                Greater => Self::remove(&mut n.r, key),
                Equal   => match (n.l.take(), n.r.take()) {
                    (None, None) => {
                        *node = None
                    }
                    (Some(l), None) => {
                        *node = Some(l)
                    }
                    (None, Some(r)) => {
                        *node = Some(r)
                    }
                    (Some(l), Some(r)) => {
                        if r.len() > l.len() {
                            let (new_r, min_r) = r.take_min();
                            n.key = min_r;
                            n.l = Some(l);
                            n.r = new_r;
                        } else {
                            let (new_l, max_l) = l.take_max();
                            n.key = max_l;
                            n.l = new_l;
                            n.r = Some(r);
                        }
                    }
                }
            }
        }
        if let Some(n) = node {
            n.max = Self::local_max(n.key.upper(), &n.l, &n.r);
        }
    }




    /**
     * Return this sub-tree, but with the left-most descendant node removed.
     * Also return the key of that node.
     */
    fn take_min(mut self: Box<Self>) -> (Option<Box<Self>>, K) {
        if let Some(l) = self.l {
            if l.l.is_none() {
                self.l = None;
                (Some(self), l.key)
            } else {
                let (new_l, min) = l.take_min();
                self.l = new_l;
                (Some(self), min)
            }
        } else {
            (None, self.key)
        }
    }




    /**
     * Return this sub-tree, but with the right-most descendant node removed.
     * Also return the key of that node.
     */
    fn take_max(mut self: Box<Self>) -> (Option<Box<Self>>, K) {
        if let Some(r) = self.r {
            if r.r.is_none() {
                self.r = None;
                (Some(self), r.key)
            } else {
                let (new_r, max) = r.take_max();
                self.r = new_r;
                (Some(self), max)
            }
        } else {
            (None, self.key)
        }
    }




    /**
     * Return a list of node references forming a path from this node to its
     * leftmost node.
     */
    fn min_path(&self) -> Vec<&Self> {
        let mut path = vec![self];

        while let Some(l) = path.last().and_then(|b| b.l.as_ref()) {
            path.push(l)
        }
        path
    }




    /**
     * Consume this node and return a list of nodes forming a path from this
     * node to its leftmost node.
     */
    fn into_min_path(self) -> Vec<Node<K>> {
        let mut path = vec![self];

        while let Some(l) = path.last_mut().and_then(|n| n.l.take()) {
            path.push(*l)
        }
        path
    }




    fn local_max(upper: K::Scalar, l: &Option<Box<Self>>, r: &Option<Box<Self>>) -> K::Scalar {
        match (&l, &r) {
            (Some(l), Some(r)) => l.max.max(r.max),
            (Some(l), None) => l.max,
            (None, Some(r)) => r.max,
            (None, None) => upper,
        }.max(upper)
    }
}




/**
 * An augmented binary search tree
 */
pub struct Tree<T: Interval> {
    root: Option<Box<Node<T>>>
}




// ============================================================================
impl<K: Interval> Tree<K> {

    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())
    }

    pub fn height(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.height())
    }

    pub fn contains(&self, key: K) -> bool {
        self.root.as_ref().map_or(false, |root| root.contains(&key))
    }

    // pub fn insert(&mut self, key: Range<K::Scalar>) {
    //     Node::insert(&mut self.root, K::new(key.start, key.end))
    // }

    pub fn insert(&mut self, key: K) {
        Node::insert(&mut self.root, key)
    }

    pub fn remove(&mut self, key: K) {
        Node::remove(&mut self.root, &key)
    }

    pub fn max(&self) -> Option<K::Scalar> {
        self.root.as_ref().map(|root| root.max)
    }

    // pub fn into_balanced(self) -> Self {
    //     self.into_iter().collect()
    // }

    // pub fn iter<'a>(&'a self) -> TreeIter<'a, T> {
    //     TreeIter::new(self)
    // }

    fn min_path(&self) -> Vec<&Node<K>> {
        self.root.as_ref().map_or(Vec::new(), |root| root.min_path())
    }

    fn into_min_path(mut self) -> Vec<Node<K>> {
        self.root.take().map_or(Vec::new(), |root| root.into_min_path())
    }
}




// ============================================================================
impl<K: Interval> FromIterator<K> for Tree<K> {
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        let mut values: Vec<_> = iter.into_iter().map(|v| Some(v)).collect();
        Self {
            root: Node::from_slice(&mut values[..])
        }
    }
}




// ============================================================================
impl<T> Interval for (T, T) where T: Ord + Copy {
    type Scalar = T;
    fn new(a: T, b: T) -> Self { (a, b) }
    fn lower(&self) -> T { self.0 }
    fn upper(&self) -> T { self.1 }
}




// ============================================================================
#[cfg(test)]
mod test {

    use crate::aug_bst::Tree;

    #[test]
    fn max_value_is_correctly_recorded_if_built_from_insert() {
        let mut tree = Tree::new();
        tree.insert(( 0, 10));
        tree.insert(( 2,  3));
        tree.insert(( 5,  6));
        tree.insert((-3, 12));
        tree.insert((-2,  0));
        assert_eq!(tree.max(), Some(12));
    }

    #[test]
    fn max_value_is_correctly_recorded_if_built_from_iter() {
        let tree: Tree<_> = vec![(2, 3), (4, 6), (5, 7)].iter().cloned().collect();
        assert_eq!(tree.max(), Some(7));

        let tree: Tree<_> = vec![(2, 3), (5, 7), (4, 6)].iter().cloned().collect();
        assert_eq!(tree.max(), Some(7));

        let tree: Tree<_> = vec![(5, 7), (4, 6), (2, 3)].iter().cloned().collect();
        assert_eq!(tree.max(), Some(7));
    }
}
