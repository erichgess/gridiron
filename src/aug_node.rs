use core::ops::Range;
use std::cmp::Ordering::{self, Less, Greater, Equal};
use crate::overlap::Overlap;




/**
 * A node in a binary search tree
 */
#[derive(Clone)]
pub struct Node<T: Ord + Copy, V> {
    key: Range<T>,
    value: V,
    max: T,
    l: Option<Box<Node<T, V>>>,
    r: Option<Box<Node<T, V>>>,
}




// ============================================================================
impl<T: Ord + Copy, V> Node<T, V> {




    /**
     * Create an empty sub-tree with the given key.
     */
    pub(crate) fn new(key: Range<T>, value: V) -> Self {
        Self { max: key.end, key, value, l: None, r: None }
    }




    /**
     * Create a balanced sub-tree from a sorted slice. If the slice is not
     * sorted, the resulting tree is invalid. No check is done here to ensure
     * the slice is sorted.
     */
    pub(crate) fn from_sorted_slice(slice: &mut [Option<(Range<T>, V)>]) -> Option<Box<Self>> {
        if slice.is_empty() {
            None
        } else {
            let mid = slice.len() / 2;
            let (key, value) = slice[mid].take().unwrap();
            let l = Self::from_sorted_slice(&mut slice[..mid]);
            let r = Self::from_sorted_slice(&mut slice[mid + 1..]);
            let max = Self::local_max(key.end, &l, &r);
            Some(Box::new(Self { key, value, max, l, r }))
        }
    }




    /**
     * Create a balanced sub-tree from an unsorted iterator.
     */
    pub(crate) fn from_iter<I: IntoIterator<Item = (Range<T>, V)>>(iter: I) -> Option<Box<Self>> {
        let mut values: Vec<_> = iter.into_iter().map(Some).collect();

        values.sort_by(Node::compare_key_val);

        Self::from_sorted_slice(&mut values[..])
    }




    /**
     * Return the number of nodes contained in this sub-tree (including self).
     */
    pub(crate) fn len(&self) -> usize {
        self.l.as_ref().map_or(0, |l| l.len()) +
        self.r.as_ref().map_or(0, |r| r.len()) + 1
    }




    /**
     * Return the height of this sub-tree.
     */
    pub(crate) fn height(&self) -> usize {
        self.l.as_ref().map_or(0, |l| l.height()).max(
        self.r.as_ref().map_or(0, |r| r.height())) + 1
    }




    /**
     * Return true of the given key exists in this sub-tree.
     */
    pub(crate) fn contains(&self, key: &Range<T>) -> bool {
        self.get(key).is_some()
    }




    /**
     * Return an immutable reference to this node's value.
     */
    pub(crate) fn get(&self, key: &Range<T>) -> Option<&V> {
        match Self::compare(key, &self.key) {
            Less    => self.l.as_ref().map_or(None, |l| l.get(key)),
            Greater => self.r.as_ref().map_or(None, |r| r.get(key)),
            Equal   => Some(&self.value)
        }       
    }




    /**
     * Return a mutable reference to this node's value.
     */
    pub(crate) fn get_mut(&mut self, key: &Range<T>) -> Option<&mut V> {
        match Self::compare(key, &self.key) {
            Less    => self.l.as_mut().map_or(None, |l| l.get_mut(key)),
            Greater => self.r.as_mut().map_or(None, |r| r.get_mut(key)),
            Equal   => Some(&mut self.value)
        }       
    }




    /**
     * Insert a node with the given key into this sub-tree.
     */
    pub(crate) fn insert(node: &mut Option<Box<Self>>, key: Range<T>, value: V) {
        if let Some(n) = node {

            n.max = key.end.max(n.max);

            match Self::compare(&key, &n.key) {
                Less    => Self::insert(&mut n.l, key, value),
                Greater => Self::insert(&mut n.r, key, value),
                Equal   => {}
            }
        } else {
            *node = Some(Box::new(Self::new(key, value)))
        }
    }




    /**
     * Remove a node with the given key from this sub-tree.
     */
    pub(crate) fn remove(node: &mut Option<Box<Self>>, key: &Range<T>) {
        if let Some(n) = node {
            match Self::compare(key, &n.key) {
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
                            let (new_r, r_key) = r.take_lmost();
                            n.key = r_key;
                            n.l = Some(l);
                            n.r = new_r;
                        } else {
                            let (new_l, l_key) = l.take_rmost();
                            n.key = l_key;
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
    pub(crate) fn take_lmost(mut self: Box<Self>) -> (Option<Box<Self>>, Range<T>) {
        if let Some(l) = self.l {
            if l.l.is_none() {
                self.l = None;
                self.max = Self::local_max(self.key.end, &self.l, &self.r);
                (Some(self), l.key)
            } else {
                let (new_l, l_key) = l.take_lmost();
                self.l = new_l;
                self.max = Self::local_max(self.key.end, &self.l, &self.r);
                (Some(self), l_key)
            }
        } else {
            (None, self.key)
        }
    }




    /**
     * Return this sub-tree, but with the right-most descendant node removed.
     * Also return the key of that node.
     */
    pub(crate) fn take_rmost(mut self: Box<Self>) -> (Option<Box<Self>>, Range<T>) {
        if let Some(r) = self.r {
            if r.r.is_none() {
                self.r = None;
                self.max = Self::local_max(self.key.end, &self.l, &self.r);
                (Some(self), r.key)
            } else {
                let (new_r, r_key) = r.take_rmost();
                self.r = new_r;
                self.max = Self::local_max(self.key.end, &self.l, &self.r);
                (Some(self), r_key)
            }
        } else {
            (None, self.key)
        }
    }




    /**
     * Return a list of node references forming a path from this node to its
     * leftmost node. This function is to facilitate non-consuming in-order
     * traversal.
     */
    pub(crate) fn lmost_path(&self) -> Vec<&Self> {
        self.lmost_path_while(&(|_| true))
    }




    /**
     * Like `lmost_path`, except the path only descends left only while the
     * given predicate is satisfied. The final node in the path is the last one
     * to satisfy the predicate.
     */
    pub(crate) fn lmost_path_while<F: Fn(&Self) -> bool>(&self, predicate: &F) -> Vec<&Self> {
        let mut path = vec![self];

        while let Some(l) = path.last().and_then(|b| b.l.as_ref()) {
            if !predicate(l) {
                break
            }
            path.push(l)
        }
        path
    }




    /**
     * Consume this node and return a list of nodes forming a path from this
     * node to its leftmost node. This function is to facilitate consuming
     * in-order traversal.
     */
    pub(crate) fn into_lmost_path(self) -> Vec<Self> {
        let mut path = vec![self];

        while let Some(l) = path.last_mut().and_then(|n| n.l.take()) {
            path.push(*l)
        }
        path
    }




    /**
     * Panic unless a node is storing the maximum endpoint of its subtree. This
     * function is for testing purposes.
     */
    pub(crate) fn validate_max(&self) {
        if self.max != self.compute_max() {
            panic!("stored maximum endpoint out of sync with subtree");
        }
        if let Some(l) = &self.l {
            l.validate_max()
        }
        if let Some(r) = &self.r {
            r.validate_max()
        }
    }




    /**
     * Panic unless a node and its entire subtree is properly ordered. This
     * function is for testing purposes.
     */
    pub(crate) fn validate_order(&self) {
        if self.l.as_ref().map_or(Less,    |l| Self::compare(&l.key, &self.key)) != Less ||
           self.r.as_ref().map_or(Greater, |r| Self::compare(&r.key, &self.key)) != Greater {
            panic!("unordered node")
        }
        if let Some(l) = &self.l {
            l.validate_order()
        }
        if let Some(r) = &self.r {
            r.validate_order()
        }
    }




    /**
     * Return the maximum upper bound on this sub-tree. This *should* be the
     * same as the `max` data member on the node, but this function can be
     * useful to test validity of this augmented data.
     */
    fn compute_max(&self) -> T {
        match (&self.l, &self.r) {
            (Some(l), Some(r)) => l.compute_max().max(r.compute_max()),
            (Some(l), None) => l.compute_max(),
            (None, Some(r)) => r.compute_max(),
            (None, None) => self.key.end,
        }.max(self.key.end)
    }




    /**
     * Determine the maximum upper bound based on the given endpoint, and two
     * other maybe-nodes. The result is correct as two maybe-nodes have
     * correctly stored max upper bounds.
     */
    fn local_max(upper: T, l: &Option<Box<Self>>, r: &Option<Box<Self>>) -> T {
        match (&l, &r) {
            (Some(l), Some(r)) => l.max.max(r.max),
            (Some(l), None) => l.max,
            (None, Some(r)) => r.max,
            (None, None) => upper,
        }.max(upper)
    }




    /**
     * Utility function to dictionary-compare two range objects.
     */
    pub(crate) fn compare(a: &Range<T>, b: &Range<T>) -> Ordering {
        (a.start, a.end).cmp(&(b.start, b.end))
    }




    /**
     * Utility function to dictionary-compare two Option<(Range<T>, V)> objects.
     */
    pub(crate) fn compare_key_val(a: &Option<(Range<T>, V)>, b: &Option<(Range<T>, V)>) -> Ordering {
        Self::compare(&a.as_ref().unwrap().0, &b.as_ref().unwrap().0)
    }




    /**
     * Utility function enabling in-order consuming traversals, for use by
     * iterators.
     */
    fn next(stack: &mut Vec<Self>) -> Option<Self> {

        /*
         * Pop the last node on the stack (A).
         *
         * If A has a right child (B) then take B and push it onto the stack,
         * followed by the path to its minimum node.
         *
         * Yield the key of A.
         */

         if let Some(mut a) = stack.pop() {
             if let Some(r) = a.r.take() {
                 stack.extend(r.into_lmost_path())
             }
             Some(a)
         } else {
             None
         }
    }




    /**
     * Utility function enabling in-order by-reference traversals with querying,
     * for use by iterators.
     */
    fn next_query<'a, F, G, H>(
        stack: &mut Vec<&'a Self>,
        descend_l: &F,
        descend_r: &G,
        predicate: &H) -> Option<&'a Self>
    where
        F: Fn(&Node<T, V>) -> bool,
        G: Fn(&Node<T, V>) -> bool,
        H: Fn(&Node<T, V>) -> bool,
    {
        while let Some(a) = stack.pop() {
            if let Some(b) = &a.r {
                if descend_r(a) {
                    stack.extend(b.lmost_path_while(&descend_l))
                }
            }
            if predicate(&a) {
                return Some(a)
            }
        }
        None
    }
}




/**
 * Consuming iterator that traveres an entire sub-tree in-order, returning
 * key-value pairs.
 */
pub struct NodeIntoIter<T: Ord + Copy, V> {
    pub(crate) nodes: Vec<Node<T, V>>
}

impl<T: Ord + Copy, V> NodeIntoIter<T, V> {
    pub(crate) fn new(node: Option<Box<Node<T, V>>>) -> Self {
        Self {
            nodes: node.map_or(Vec::new(), |node| node.into_lmost_path())
        }
    }
}

impl<T: Ord + Copy, V> Iterator for NodeIntoIter<T, V> {
    type Item = (Range<T>, V);

    fn next(&mut self) -> Option<Self::Item> {
        Node::next(&mut self.nodes).map(|n| (n.key, n.value))
    }
}




/**
 * Consuming iterator that traveres an entire sub-tree in-order, returning
 * key only the keys.
 */
pub struct NodeIntoIterKey<T: Ord + Copy, V> {
    pub(crate) nodes: Vec<Node<T, V>>
}

impl<T: Ord + Copy, V> NodeIntoIterKey<T, V> {
    pub(crate) fn new(node: Option<Box<Node<T, V>>>) -> Self {
        Self {
            nodes: node.map_or(Vec::new(), |node| node.into_lmost_path())
        }
    }
}

impl<T: Ord + Copy, V> Iterator for NodeIntoIterKey<T, V> {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {
        Node::next(&mut self.nodes).map(|n| n.key)
    }
}




/**
 * By-reference iterator that traverses a subset of the tree. Used for point and
 * range-bounds queries.
 */
pub struct NodeQueryIter<'a, T: Ord + Copy, V, F, G, H> {
    pub(crate) nodes: Vec<&'a Node<T, V>>,
    pub(crate) descend_l: F,
    pub(crate) descend_r: G,
    pub(crate) predicate: H,
}

impl<'a, T, V, F, G, H> Iterator for NodeQueryIter<'a, T, V, F, G, H>
where
    T: Ord + Copy,
    F: Fn(&Node<T, V>) -> bool,
    G: Fn(&Node<T, V>) -> bool,
    H: Fn(&Node<T, V>) -> bool,
{
    type Item = (&'a Range<T>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        Node::next_query(&mut self.nodes, &self.descend_l, &self.descend_r, &self.predicate).map(|n| (&n.key, &n.value))
    }
}




/**
 * Return an iterator that traverses the whole tree by reference.
 */
pub(crate) fn traversal<'a, T, V>(
    node: &'a Option<Box<Node<T, V>>>) -> impl Iterator<Item = (&'a Range<T>, &'a V)>
where
    T: Ord + Copy
{
    NodeQueryIter {
        nodes: node.as_ref().map_or(Vec::new(), |node| node.lmost_path()),
        descend_l: |_: &Node<T, V>| true,
        descend_r: |_: &Node<T, V>| true,
        predicate: |_: &Node<T, V>| true,
    }
}




/**
 * Return an iterator that visits (by reference) only those key-value pairs for
 * which the interval contains the given point.
 */
pub(crate) fn query_point<'a, T, V>(
    node: &'a Option<Box<Node<T, V>>>,
    point: &'a T) -> impl Iterator<Item = (&'a Range<T>, &'a V)>
where
    T: Ord + Copy
{
    let descend_l = move |a: &Node<T, V>| point < &a.max;
    let descend_r = move |a: &Node<T, V>| point >= &a.key.start;
    let predicate = move |a: &Node<T, V>| a.key.contains(point);

    NodeQueryIter {
        nodes: node.as_ref().map_or(Vec::new(), |node| node.lmost_path_while(&descend_l)),
        descend_l,
        descend_r,
        predicate,
    }
}




/**
 * Return an iterator that visits (by reference) only those key-value pairs for
 * which the interval overlaps the given range bounds object.
 */
pub(crate) fn query_range<'a, T, V, R: Overlap<T>>(
    node: &'a Option<Box<Node<T, V>>>,
    range: &'a R) -> impl Iterator<Item = (&'a Range<T>, &'a V)>
where
    T: Ord + Copy
{
    let descend_l = move |a: &Node<T, V>| range.overlaps(&(..a.max));
    let descend_r = move |a: &Node<T, V>| range.overlaps(&(a.key.start..));
    let predicate = move |a: &Node<T, V>| range.overlaps(&a.key);

    NodeQueryIter {
        nodes: node.as_ref().map_or(Vec::new(), |node| node.lmost_path_while(&descend_l)),
        descend_l,
        descend_r,
        predicate,
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use core::ops::Range;
    use super::Node;

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
    fn max_value_is_correctly_recorded_for_random_collected_tree() {
        let node: Node<_, ()> = *Node::from_iter(stupid_random_intervals(1000, 666).into_iter().map(|x| (x, ()))).unwrap();
        node.validate_max();
        node.validate_order();
    }

    #[test]
    fn max_value_is_correctly_recorded_for_random_incremental_tree() {
        let mut node = Some(Box::new(Node::new(0..10, ())));
        for x in stupid_random_intervals(1000, 12345) {
            Node::insert(&mut node, x, ());
        }
        node.as_ref().unwrap().validate_max();
        node.as_ref().unwrap().validate_order();
    }
}
