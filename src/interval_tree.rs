#![allow(unused)]
use std::cmp::Ordering;
use std::iter::once;
use core::ops::Range;
use crate::bst;





#[derive(PartialEq, Eq)]
struct IntervalL(Range<i32>);

#[derive(PartialEq, Eq)]
struct IntervalR(Range<i32>);




type Interval = Range<i32>;




// ============================================================================
impl From<Interval> for IntervalL {
    fn from(interval: Interval) -> Self {
        Self(interval)
    }
}

impl PartialOrd for IntervalL {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.start.cmp(&other.0.start))
    }
}

impl Ord for IntervalL {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.start.cmp(&other.0.start)
    }
}




// ============================================================================
impl From<Interval> for IntervalR {
    fn from(interval: Interval) -> Self {
        Self(interval)
    }
}

impl PartialOrd for IntervalR {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.end.cmp(&other.0.end))
    }
}

impl Ord for IntervalR {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.end.cmp(&other.0.end)
    }
}




/**
 * A location of an interval with respect to a point
 */
enum Location {
    /// The interval lies to the left of a point
    L,
    /// The interval lies to the right of a point
    R,
    /// The interval contains a point
    C,
}




// ============================================================================
impl Location {
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
    sorted_l: bst::Tree<IntervalL>,
    sorted_r: bst::Tree<IntervalR>,
    l: Option<Box<Node>>,
    r: Option<Box<Node>>,
}




// ============================================================================
impl Node {

    fn new(interval: Interval) -> Self {
        Self {
            center: (interval.start + interval.end) / 2,
            sorted_l: once(interval.clone().into()).collect(),
            sorted_r: once(interval.clone().into()).collect(),
            l: None,
            r: None,
        }
    }

    fn len(&self) -> usize {
        self.l.as_ref().map_or(0, |l| l.len()) +
        self.r.as_ref().map_or(0, |r| r.len()) + self.sorted_l.len()
    }

    fn insert(node: &mut Option<Box<Node>>, interval: Interval) {
        if let Some(n) = node {
            match Location::from(&interval, n.center) {
                Location::L => {
                    Node::insert(&mut n.l, interval)
                }
                Location::R => {
                    Node::insert(&mut n.r, interval)
                }
                Location::C => {
                    n.sorted_l.insert(interval.clone().into());
                    n.sorted_r.insert(interval.clone().into());
                }
            }
        } else {
            *node = Some(Box::new(Node::new(interval)))
        }
    }

    fn find(&self, point: i32) -> Option<Interval> {
        if point < self.center {
            None
        } else if point >= self.center {
            None
        } else {
            None
        }
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

    fn new() -> Self {
        Self {
            root: None
        }
    }

    fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())        
    }

    fn insert(&mut self, interval: Interval) {
        Node::insert(&mut self.root, interval)
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use crate::interval_tree::IntervalTree;

    #[test]
    fn can_build_interval_tree() {
        let mut tree = IntervalTree::new();
        tree.insert(0..10);
        tree.insert(5..10);
        assert_eq!(tree.len(), 2);
    }
}
