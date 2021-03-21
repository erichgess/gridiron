use core::ops::{Range, RangeBounds};
use std::iter::FromIterator;
use crate::aug_node::{self, Node};




/**
 * An associative map where the keys are `Range` objects. Supports point and
 * range-based queries to iterate over key-value pairs.
 */
#[derive(Clone)]
pub struct IntervalMap<T: Ord + Copy, V> {
    root: Option<Box<Node<T, V>>>
}




// ============================================================================
impl<T: Ord + Copy, V> IntervalMap<T, V> {

    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())
    }

    pub fn height(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.height())
    }

    pub fn contains(&self, key: &Range<T>) -> bool {
        self.root.as_ref().map_or(false, |root| root.contains(key))
    }

    pub fn get(&self, key: &Range<T>) -> Option<&V> {
        self.root.as_ref().map_or(None, |root| root.get(key))
    }

    pub fn get_mut(&mut self, key: &Range<T>) -> Option<&mut V> {
        self.root.as_mut().map_or(None, |root| root.get_mut(key))
    }

    pub fn insert(&mut self, key: Range<T>, value: V) {
        Node::insert(&mut self.root, key, value)
    }

    pub fn remove(&mut self, key: &Range<T>) {
        Node::remove(&mut self.root, key)
    }

    pub fn into_balanced(self) -> Self {
        let mut data: Vec<_> = self.into_iter().map(Some).collect();
        Self { root: Node::from_sorted_slice(&mut data[..]) }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Range<T>, &V)> {
        aug_node::traversal(&self.root)
    }

    pub fn query_point<'a>(&'a self, point: &'a T) -> impl Iterator<Item = (&'a Range<T>, &'a V)> {
        aug_node::query_point(&self.root, point)
    }

    pub fn query_range<'a, R: RangeBounds<T>>(&'a self, range: &'a R) -> impl Iterator<Item = (&'a Range<T>, &'a V)> {
        aug_node::query_range(&self.root, range)
    }
}




// ============================================================================
impl<T: Ord + Copy, V> IntoIterator for IntervalMap<T, V> {
    type Item = (Range<T>, V);
    type IntoIter = aug_node::NodeIntoIter<T, V>;

    fn into_iter(self) -> Self::IntoIter {
        aug_node::NodeIntoIter::new(self.root)
    }
}




// ============================================================================
impl<T: Ord + Copy, V> FromIterator<(Range<T>, V)> for IntervalMap<T, V> {
    fn from_iter<I: IntoIterator<Item = (Range<T>, V)>>(iter: I) -> Self {
        let mut values: Vec<_> = iter.into_iter().map(Some).collect();

        values.sort_by(Node::compare_key_val);

        Self {
            root: Node::from_sorted_slice(&mut values[..])
        }
    }
}
