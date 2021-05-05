use crate::aug_node::{self, Node};
use core::iter::FromIterator;
use core::ops::{Range, RangeBounds};

/**
 * An associative map where the keys are `Range` objects. Supports point and
 * range-based queries to iterate over key-value pairs.
 */
#[derive(Clone)]
pub struct IntervalMap<T: Ord + Copy, V> {
    root: Option<Box<Node<T, V>>>,
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
        self.root.as_ref().and_then(|root| root.get(key))
    }

    pub fn get_mut(&mut self, key: &Range<T>) -> Option<&mut V> {
        self.root.as_mut().and_then(|root| root.get_mut(key))
    }

    pub fn insert(&mut self, key: Range<T>, value: V) -> &mut V {
        Node::insert(&mut self.root, key, value)
    }

    pub fn require(&mut self, key: Range<T>) -> &mut V
    where
        V: Default,
    {
        Node::require(&mut self.root, key)
    }

    pub fn remove(&mut self, key: &Range<T>) {
        Node::remove(&mut self.root, key)
    }

    pub fn into_balanced(self) -> Self {
        let mut data: Vec<_> = self.into_sorted().map(Some).collect();
        Self {
            root: Node::from_sorted_slice(&mut data[..]),
        }
    }

    pub fn into_sorted(self) -> impl Iterator<Item = (Range<T>, V)> {
        aug_node::IntoIterInOrder::new(self.root)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Range<T>, &V)> {
        aug_node::Iter::new(&self.root)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Range<T>, &mut V)> {
        aug_node::IterMut::new(&mut self.root)
    }

    pub fn keys(&self) -> impl Iterator<Item = &Range<T>> {
        self.iter().map(|(k, _)| k)
    }

    pub fn query_point(&self, point: T) -> impl Iterator<Item = (&Range<T>, &V)> + '_ {
        aug_node::IterPointQuery::new(&self.root, point)
    }

    pub fn query_range<R: RangeBounds<T>>(
        &self,
        range: R,
    ) -> impl Iterator<Item = (&Range<T>, &V)> {
        aug_node::IterRangeQuery::new(&self.root, range)
    }
}

// ============================================================================
impl<T: Ord + Copy, V> Default for IntervalMap<T, V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
impl<T: Ord + Copy, V> IntoIterator for IntervalMap<T, V> {
    type Item = (Range<T>, V);
    type IntoIter = aug_node::IntoIter<T, V>;

    fn into_iter(self) -> Self::IntoIter {
        aug_node::IntoIter::new(self.root)
    }
}

// ============================================================================
impl<'a, T: Ord + Copy, V> IntoIterator for &'a IntervalMap<T, V> {
    type Item = (&'a Range<T>, &'a V);
    type IntoIter = aug_node::Iter<'a, T, V>;

    fn into_iter(self) -> Self::IntoIter {
        aug_node::Iter::new(&self.root)
    }
}

// ============================================================================
impl<'a, T: Ord + Copy, V> IntoIterator for &'a mut IntervalMap<T, V> {
    type Item = (&'a Range<T>, &'a mut V);
    type IntoIter = aug_node::IterMut<'a, T, V>;

    fn into_iter(self) -> Self::IntoIter {
        aug_node::IterMut::new(&mut self.root)
    }
}

// ============================================================================
impl<T: Ord + Copy, V> FromIterator<(Range<T>, V)> for IntervalMap<T, V> {
    fn from_iter<I: IntoIterator<Item = (Range<T>, V)>>(iter: I) -> Self {
        Self {
            root: Node::from_iter(iter),
        }
    }
}
