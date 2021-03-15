#![allow(unused)]
use std::cmp::Ordering;
use std::iter::once;
use std::collections::BTreeMap;
use core::ops::{Bound, Range, RangeBounds};




type Interval = Range<i32>;




/**
 * The ways an interval can contain, or not contain, a point.
 */
enum Containment {
    /// The interval lies to the left of a point
    L,
    /// The interval lies to the right of a point
    R,
    /// The interval contains a point
    C,
}




// ============================================================================
impl Containment {
    fn from(interval: &Interval, point: i32) -> Self {
        if interval.end <= point {
            Self::L
        } else if interval.start > point {
            Self::R
        } else if interval.contains(&point) {
            Self::C
        } else {
            unreachable!()
        }
    }
}




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

    fn insert(node: &mut Option<Box<Node>>, interval: Interval) {
        if let Some(n) = node {
            match Containment::from(&interval, n.center) {
                Containment::L => {
                    Node::insert(&mut n.l, interval)
                }
                Containment::R => {
                    Node::insert(&mut n.r, interval)
                }
                Containment::C => {
                    n.cl.insert(interval.start, interval.clone());
                    n.cr.insert(interval.end,   interval.clone());
                }
            }
        } else {
            *node = Some(Box::new(Node::new(interval)))
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

    pub fn insert(&mut self, interval: Interval) {
        Node::insert(&mut self.root, interval)
    }

    pub fn including(&self, point: i32) -> Vec<Interval> {
        self.root.as_ref().map_or(Vec::new(), |root| root.including(point))
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use crate::interval_tree::IntervalTree;

    #[test]
    fn interaval_tree_has_correct_length() {
        let mut tree = IntervalTree::new();
        tree.insert(0..10);
        tree.insert(5..10);
        tree.insert(5..10);
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn interaval_tree_query_works() {
        let mut tree = IntervalTree::new();
        tree.insert( 2..12);
        tree.insert(-2..8);
        tree.insert( 0..10);
        tree.insert( 4..14);
        tree.insert(-4..6);

        assert_eq!(tree.including(-5), vec![]);
        assert_eq!(tree.including(-4), vec![-4..6]);
        assert_eq!(tree.including(-3), vec![-4..6]);
        assert_eq!(tree.including(-2), vec![-4..6, -2..8]);
        assert_eq!(tree.including(12), vec![ 4..14]);
    }
}
