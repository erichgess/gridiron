#![allow(unused)]
use std::cmp::Ordering;
use std::iter::once;
use std::collections::BTreeMap;
use core::ops::{Bound, Range, RangeBounds};




type Interval = Range<i32>;




/**
 * A node in an interval tree
 */
struct Node {
    center: i32,
    cl: BTreeMap<i32, Interval>,
    cr: BTreeMap<i32, Interval>,
    l: Option<Box<Node>>,
    r: Option<Box<Node>>,
}




// ============================================================================
impl Node {

    fn new(interval: Interval) -> Self {

        assert!(!interval.is_empty(), "cannot hold empty intervals");

        Self {
            center: (interval.start + interval.end) / 2,
            cl: once((interval.start, interval.clone())).collect(),
            cr: once((interval.end,   interval.clone())).collect(),
            l: None,
            r: None,
        }
    }

    fn len(&self) -> usize {
        self.l.as_ref().map_or(0, |l| l.len()) +
        self.r.as_ref().map_or(0, |r| r.len()) + self.cl.len()
    }

    fn is_empty(&self) -> bool {
        self.cl.is_empty() &&
        self.l.as_ref().map_or(true, |l| l.is_empty()) &&
        self.r.as_ref().map_or(true, |r| r.is_empty())
    }

    fn insert(node: &mut Option<Box<Node>>, interval: Interval) {
        if let Some(n) = node {
            if interval.end <= n.center {
                Node::insert(&mut n.l, interval)
            }
            else if interval.start > n.center {
                Node::insert(&mut n.r, interval)
            }
            else {
                n.cl.insert(interval.start, interval.clone());
                n.cr.insert(interval.end,   interval.clone());
            }
        } else {
            *node = Some(Box::new(Node::new(interval)))
        }
    }

    fn remove(node: &mut Option<Box<Self>>, interval: &Interval) {
        if let Some(n) = node {
            if interval.end <= n.center {
                Node::remove(&mut n.l, interval)
            }
            else if interval.start > n.center {
                Node::remove(&mut n.r, interval)
            }
            else {
                n.cl.remove(&interval.start);
                n.cr.remove(&interval.end);
            }

            if n.is_empty() {
                *node = None
            }
        }
    }

    fn including(&self, point: i32) -> Vec<Interval> {
        let mut result = Vec::new();

        if point < self.center {
            if let Some(l) = &self.l {
                result.extend(l.including(point))
            }
            result.extend(self.cl.range(..point + 1).map(|e| e.1.clone()));
        } else {
            if let Some(r) = &self.r {
                result.extend(r.including(point))
            }
            result.extend(self.cr.range(point + 1..).map(|e| e.1.clone()));
        }
        result
    }
}




/**
 * An interval tree
 */
struct IntervalTree {
    root: Option<Box<Node>>
}




// ============================================================================
impl IntervalTree {

    pub fn new() -> Self {
        Self {
            root: None
        }
    }

    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())
    }

    pub fn is_empty(&self) -> bool {
        self.root.as_ref().map_or(true, |root| root.is_empty())
    }

    pub fn insert(&mut self, interval: Interval) {
        Node::insert(&mut self.root, interval)
    }

    pub fn including(&self, point: i32) -> Vec<Interval> {
        self.root.as_ref().map_or(Vec::new(), |root| root.including(point))
    }

    pub fn remove(&mut self, interval: &Interval) {
        Node::remove(&mut self.root, interval)
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use crate::interval_tree::IntervalTree;

    #[test]
    fn interval_tree_has_correct_length() {
        let mut ranges = IntervalTree::new();
        ranges.insert(0..10);
        ranges.insert(5..10);
        ranges.insert(5..10);
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    #[should_panic]
    fn interval_tree_panics_on_empty_interval() {
        let mut ranges = IntervalTree::new();
        ranges.insert(0..0);
    }

    #[test]
    fn interval_tree_query_works() {
        let mut ranges = IntervalTree::new();
        ranges.insert( 2..12);
        ranges.insert(-2..8);
        ranges.insert( 0..10);
        ranges.insert( 4..14);
        ranges.insert(-4..6);

        assert_eq!(ranges.including(-5), vec![]);
        assert_eq!(ranges.including(-4), vec![-4..6]);
        assert_eq!(ranges.including(-3), vec![-4..6]);
        assert_eq!(ranges.including(-2), vec![-4..6, -2..8]);
        assert_eq!(ranges.including(12), vec![ 4..14]);
    }


    #[test]
    fn interval_tree_can_remove_interval() {
        let mut ranges = IntervalTree::new();

        ranges.insert(-2..8);
        ranges.insert( 0..10);
        ranges.insert( 4..14);

        assert!(!ranges.including(-2).is_empty());
        ranges.remove(&(-2..8));
        assert!(ranges.including(-2).is_empty());

        assert!(!ranges.including(13).is_empty());
        ranges.remove(&(4..14));
        assert!(ranges.including(13).is_empty());
    }
}
