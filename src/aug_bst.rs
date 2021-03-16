#![allow(unused)]




use core::ops::Range;
use std::cmp::Ordering::{Less, Greater, Equal};
use std::iter::FromIterator;




/**
 * A node in a binary search tree
 */
struct Node<T: Ord + Copy> {
    key: Range<T>,
    max: T,
    l: Option<Box<Node<T>>>,
    r: Option<Box<Node<T>>>,
}




// ============================================================================
impl<T: Ord + Copy> Node<T> {




    /**
     * Create an empty sub-tree with the given key
     */
    fn new(key: Range<T>) -> Self {
        Self { max: key.end, key, l: None, r: None }
    }




    /**
     * Create a sub-tree from a slice, which is balanced if the slice is sorted.
     */
    fn from_slice(slice: &mut [Option<Range<T>>]) -> Option<Box<Self>> {
        if slice.is_empty() {
            None
        } else {
            let mid = slice.len() / 2;
            let key = slice[mid].take().unwrap();
            let l = Self::from_slice(&mut slice[..mid]);
            let r = Self::from_slice(&mut slice[mid + 1..]);
            let max = Self::local_max(key.end, &l, &r);
            Some(Box::new(Self { key, max, l, r }.validate()))
        }
    }




    /**
     * Ensure a node's children are properly ordered.
     */
    fn validate(self) -> Self {
        let valid = match (&self.l, &self.r) {
            (None, None) => true,
            (Some(l), None) => l.key.start < self.key.start,
            (None, Some(r)) => r.key.start > self.key.start,
            (Some(l), Some(r)) => l.key.start < self.key.start && r.key.start > self.key.start,
        };
        if !valid {
            panic!("unordered node")
        }
        self
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
    fn contains(&self, key: Range<T>) -> bool {
        match key.start.cmp(&self.key.start) {
            Less    => self.l.as_ref().map_or(false, |l| l.contains(key)),
            Greater => self.r.as_ref().map_or(false, |r| r.contains(key)),
            Equal   => true
        }
    }




    /**
     * Insert a node with the given key into this sub-tree.
     */
    fn insert(node: &mut Option<Box<Self>>, key: Range<T>) {
        if let Some(n) = node {

            n.max = key.end.max(n.max);

            match key.start.cmp(&n.key.start) {
                Less    => Self::insert(&mut n.l, key),
                Greater => Self::insert(&mut n.r, key),
                Equal => {}
            }
        } else {
            *node = Some(Box::new(Self::new(key)))
        }
    }




    /**
     * Remove a node with the given key from this sub-tree.
     */
    fn remove(node: &mut Option<Box<Self>>, key: Range<T>) {
        if let Some(n) = node {
            match key.start.cmp(&n.key.start) {
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
            n.max = Self::local_max(n.key.end, &n.l, &n.r);
        }
    }




    /**
     * Return this sub-tree, but with the left-most descendant node removed.
     * Also return the key of that node.
     */
    fn take_min(mut self: Box<Self>) -> (Option<Box<Self>>, Range<T>) {
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
    fn take_max(mut self: Box<Self>) -> (Option<Box<Self>>, Range<T>) {
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
    fn into_min_path(self) -> Vec<Self> {
        let mut path = vec![self];

        while let Some(l) = path.last_mut().and_then(|n| n.l.take()) {
            path.push(*l)
        }
        path
    }




    fn local_max(upper: T, l: &Option<Box<Self>>, r: &Option<Box<Self>>) -> T {
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
pub struct Tree<T: Ord + Copy> {
    root: Option<Box<Node<T>>>
}




// ============================================================================
impl<T: Ord + Copy> Tree<T> {

    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())
    }

    pub fn height(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.height())
    }

    pub fn contains(&self, key: Range<T>) -> bool {
        self.root.as_ref().map_or(false, |root| root.contains(key))
    }

    pub fn insert(&mut self, key: Range<T>) {
        Node::insert(&mut self.root, key)
    }

    pub fn remove(&mut self, key: Range<T>) {
        Node::remove(&mut self.root, key)
    }

    pub fn max(&self) -> Option<T> {
        self.root.as_ref().map(|root| root.max)
    }

    // pub fn into_balanced(self) -> Self {
    //     self.into_iter().collect()
    // }

    // pub fn iter<'a>(&'a self) -> TreeIter<'a, T> {
    //     TreeIter::new(self)
    // }

    fn min_path(&self) -> Vec<&Node<T>> {
        self.root.as_ref().map_or(Vec::new(), |root| root.min_path())
    }

    fn into_min_path(mut self) -> Vec<Node<T>> {
        self.root.take().map_or(Vec::new(), |root| root.into_min_path())
    }
}




// ============================================================================
impl<T: Ord + Copy> FromIterator<Range<T>> for Tree<T> {
    fn from_iter<I: IntoIterator<Item = Range<T>>>(iter: I) -> Self {
        let mut values: Vec<_> = iter.into_iter().map(|v| Some(v)).collect();
        Self {
            root: Node::from_slice(&mut values[..])
        }
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use crate::aug_bst::Tree;

    #[test]
    fn max_value_is_correctly_recorded_if_built_from_insert() {
        let mut tree = Tree::new();
        tree.insert( 0.. 10);
        tree.insert( 2..  3);
        tree.insert( 5..  6);
        tree.insert(-3.. 12);
        tree.insert(-2..  0);
        assert_eq!(tree.max(), Some(12));
    }

    #[test]
    fn max_value_is_correctly_recorded_if_built_from_iter() {
        let tree: Tree<_> = vec![2..3, 4..6, 5..7].iter().cloned().collect();
        assert_eq!(tree.max(), Some(7));

        let tree: Tree<_> = vec![2..3, 3..7, 4..6].iter().cloned().collect();
        assert_eq!(tree.max(), Some(7));

        let tree: Tree<_> = vec![4..7, 5..6, 6..7].iter().cloned().collect();
        assert_eq!(tree.max(), Some(7));
    }
}
