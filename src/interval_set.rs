use core::ops::{Range, RangeBounds};
use std::iter::FromIterator;
use crate::aug_node::{self, Node};




/**
 * An set type where the keys are `Range` objects. Supports point and
 * range-based queries to iterate over the keys.
 */
#[derive(Clone)]
pub struct IntervalSet<T: Ord + Copy> {
    root: Option<Box<Node<T, ()>>>
}




// ============================================================================
impl<T: Ord + Copy> IntervalSet<T> {

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

    pub fn insert(&mut self, key: Range<T>) {
        Node::insert(&mut self.root, key, ())
    }

    pub fn remove(&mut self, key: &Range<T>) {
        Node::remove(&mut self.root, key)
    }

    pub fn into_balanced(self) -> Self {
        let mut data: Vec<_> = self.into_iter().map(|r| Some((r, ()))).collect();
        Self { root: Node::from_sorted_slice(&mut data[..]) }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Range<T>> {
        aug_node::traversal(&self.root).map(|(r, _)| r)
    }

    pub fn query_point<'a>(&'a self, point: &'a T) -> impl Iterator<Item = &'a Range<T>> {
        aug_node::query_point(&self.root, point).map(|(r, _)| r)
    }

    pub fn query_range<'a, R: RangeBounds<T>>(&'a self, range: &'a R) -> impl Iterator<Item = &'a Range<T>> {
        aug_node::query_range(&self.root, range).map(|(r, _)| r)
    }

    #[cfg(test)]
    fn validate_max(&self) {
        if let Some(root) = &self.root {
            root.validate_max()
        }
    }

    #[cfg(test)]
    fn validate_order(&self) {
        if let Some(root) = &self.root {
            root.validate_order()
        }
    }
}




// ============================================================================
impl<T: Ord + Copy> Default for IntervalSet<T> {
    fn default() -> Self {
        Self::new()
    }
}




// ============================================================================
impl<T: Ord + Copy> IntoIterator for IntervalSet<T> {
    type Item = Range<T>;
    type IntoIter = aug_node::NodeIntoIterKey<T, ()>;

    fn into_iter(self) -> Self::IntoIter {
        aug_node::NodeIntoIterKey::new(self.root)
    }
}




// ============================================================================
impl<T: Ord + Copy> FromIterator<Range<T>> for IntervalSet<T> {
    fn from_iter<I: IntoIterator<Item = Range<T>>>(iter: I) -> Self {
        Self {
            root: Node::from_iter(iter.into_iter().map(|r| (r, ())))
        }
    }
}




// ============================================================================
#[cfg(test)]
mod test {

	use core::ops::Range;
	use super::IntervalSet;

	/**
	 * A simple deterministic linear congruential generator:
	 *
	 * https://en.wikipedia.org/wiki/Linear_congruential_generator
	 */
	fn stupid_random_intervals(len: usize, mut seed: usize) -> Vec<Range<usize>> {
	    let mut values = Vec::new();
	    let a = 1103515245;
	    let c = 12345;
	    let m = 1 << 31;
	    for _ in 0..len {
	        seed = (a * seed + c) % m;
	        values.push(seed..seed + 30)
	    }
	    values
	}

    #[test]
    fn set_contains_works() {
        let mut set = IntervalSet::new();
        set.insert(-5..0);
        set.insert(-2..0);
        set.insert(-8..8);
        set.insert(-6..2);
        set.insert(-1..2);
        assert_eq!(set.len(), 5);
        assert!( set.contains(&(-1..2)));
        assert!( set.contains(&(-6..2)));
        assert!(!set.contains(&(-6..3)));
        set.validate_max();
        set.validate_order();
    }

    #[test]
    fn set_removal_works() {
        for i in 0..100 {
            let intervals = stupid_random_intervals(100, i);
            let mut set = IntervalSet::new();

            for x in &intervals {
                set.insert(x.clone())
            }

            let x = &intervals[i];
            assert!(set.contains(x));
            set.remove(x);
            assert!(!set.contains(x));

            set.validate_max();
            set.validate_order();
        }
    }

    #[test]
    fn can_balance_set() {
        let set: IntervalSet<_> = (0..0).map(|i| i..i + 10).collect();
        assert_eq!(set.into_balanced().height(), 0);

        let set: IntervalSet<_> = (0..2047).map(|i| i..i + 10).collect();
        assert_eq!(set.into_balanced().height(), 11);

        let set: IntervalSet<_> = (0..2048).map(|i| i..i + 10).collect();
        assert_eq!(set.into_balanced().height(), 12);
    }

    #[test]
    fn set_into_iter_works() {
        let mut set = IntervalSet::new();
        set.insert(5..12);
        set.insert(2..12);
        set.insert(7..12);
        set.insert(0..12);

        let mut iter = set.iter();
        assert_eq!(iter.next(), Some(&(0..12)));
        assert_eq!(iter.next(), Some(&(2..12)));
        assert_eq!(iter.next(), Some(&(5..12)));
        assert_eq!(iter.next(), Some(&(7..12)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn set_iter_works() {
        let mut set = IntervalSet::new();
        set.insert(5..12);
        set.insert(2..12);
        set.insert(7..12);
        set.insert(0..12);

        let mut iter = set.into_iter();
        assert_eq!(iter.next(), Some(0..12));
        assert_eq!(iter.next(), Some(2..12));
        assert_eq!(iter.next(), Some(5..12));
        assert_eq!(iter.next(), Some(7..12));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn interval_query_works() {
        let mut set = IntervalSet::new();
        set.insert(0..10);
        set.insert(4..7);
        set.insert(2..3);
        set.insert(8..12);
        set.insert(1..17);
        set.insert(6..9);
        set.validate_max();
        assert!(set.query_point(&-1).count() == 0);
        assert_eq!(set.query_point(&0).collect::<Vec<_>>(), [&(0..10)]);
        assert_eq!(set.query_point(&1).collect::<Vec<_>>(), [&(0..10), &(1..17)]);
        assert_eq!(set.query_point(&2).collect::<Vec<_>>(), [&(0..10), &(1..17), &(2..3)]);
        assert_eq!(set.query_point(&3).collect::<Vec<_>>(), [&(0..10), &(1..17)]);
        assert_eq!(set.query_point(&4).collect::<Vec<_>>(), [&(0..10), &(1..17), &(4..7)]);
        assert_eq!(set.query_point(&11).collect::<Vec<_>>(), [&(1..17), &(8..12)]);
    }

    #[test]
    fn overlap_query_works() {
        let mut set = IntervalSet::new();
        set.insert(0..2);
        set.insert(4..10);
        set.insert(6..12);
        set.insert(2..5);
        assert_eq!(set.query_range(&(5..10)).collect::<Vec<_>>(), vec![&(4..10), &(6..12)]);
    }
}
