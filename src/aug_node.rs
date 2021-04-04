use core::ops::{Range, RangeBounds};
use core::cmp::Ordering::{self, Less, Greater, Equal};
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
            Less    => self.l.as_ref().and_then(|l| l.get(key)),
            Greater => self.r.as_ref().and_then(|r| r.get(key)),
            Equal   => Some(&self.value)
        }       
    }




    /**
     * Return a mutable reference to this node's value.
     */
    pub(crate) fn get_mut(&mut self, key: &Range<T>) -> Option<&mut V> {
        match Self::compare(key, &self.key) {
            Less    => self.l.as_mut().and_then(|l| l.get_mut(key)),
            Greater => self.r.as_mut().and_then(|r| r.get_mut(key)),
            Equal   => Some(&mut self.value)
        }       
    }




    /**
     * Insert a node with the given key into this sub-tree. If a node with that
     * key already exists, the value is overwritten.
     */
    pub(crate) fn insert(node: &mut Option<Box<Self>>, key: Range<T>, value: V) -> &mut V {
        if let Some(n) = node {

            n.max = key.end.max(n.max);

            match Self::compare(&key, &n.key) {
                Less    => Self::insert(&mut n.l, key, value),
                Greater => Self::insert(&mut n.r, key, value),
                Equal   => {
                    n.value = value;
                    &mut n.value
                }
            }
        } else {
            *node = Some(Box::new(Self::new(key, value)));
            &mut node.as_mut().unwrap().value
        }
    }




    /**
     * Return a mutable reference to the value with the given key if it exists.
     * If the key does not exist, then create it with the default value and
     * return a mutable reference to that.
     */
    pub(crate) fn require(node: &mut Option<Box<Self>>, key: Range<T>) -> &mut V
    where
        V: Default
    {
        if let Some(n) = node {

            n.max = key.end.max(n.max);

            match Self::compare(&key, &n.key) {
                Less    => Self::require(&mut n.l, key),
                Greater => Self::require(&mut n.r, key),
                Equal   => &mut n.value
            }
        } else {
            *node = Some(Box::new(Self::new(key, V::default())));
            &mut node.as_mut().unwrap().value
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
    #[cfg(test)]
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
    #[cfg(test)]
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
    #[cfg(test)]
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
    fn compare(a: &Range<T>, b: &Range<T>) -> Ordering {
        (a.start, a.end).cmp(&(b.start, b.end))
    }




    /**
     * Utility function to dictionary-compare two Option<(Range<T>, V)> objects.
     */
    fn compare_key_val(a: &Option<(Range<T>, V)>, b: &Option<(Range<T>, V)>) -> Ordering {
        Self::compare(&a.as_ref().unwrap().0, &b.as_ref().unwrap().0)
    }




    /**
     * Utility function enabling in-order consuming traversals, for use by
     * the sonsuming in-order traversal iterator.
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
}




/**
 * Consuming iterator that traveres an entire sub-tree in-order, returning
 * key-value pairs.
 */
pub (crate) struct IntoIterInOrder<T: Ord + Copy, V> {
    stack: Vec<Node<T, V>>
}

impl<T: Ord + Copy, V> IntoIterInOrder<T, V> {
    pub (crate) fn new(node: Option<Box<Node<T, V>>>) -> Self {
        Self {
            stack: node.map_or(Vec::new(), |node| node.into_lmost_path())
        }
    }
}

impl<T: Ord + Copy, V> Iterator for IntoIterInOrder<T, V> {
    type Item = (Range<T>, V);

    fn next(&mut self) -> Option<Self::Item> {
        Node::next(&mut self.stack).map(|n| (n.key, n.value))
    }
}




/**
 * Consuming iterator that does a pre-order traveral of the sub-tree, returning
 * key-value pairs.
 */
pub struct IntoIter<T: Ord + Copy, V> {
    stack: Vec<Node<T, V>>
}

impl<T: Ord + Copy, V> IntoIter<T, V> {
    pub(crate) fn new(node: Option<Box<Node<T, V>>>) -> Self {
        Self {
            stack: node.into_iter().map(|n| *n).collect()
        }
    }
}

impl<T: Ord + Copy, V> Iterator for IntoIter<T, V> {
    type Item = (Range<T>, V);

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        if let Some(r) = node.r {
            self.stack.push(*r)
        }
        if let Some(l) = node.l {
            self.stack.push(*l)
        }
        Some((node.key, node.value))
    }
}




/**
 * Consuming iterator that does a pre-order traveral of the sub-tree, returning
 * only the keys.
 */
pub struct IntoIterKey<T: Ord + Copy, V> {
    stack: Vec<Node<T, V>>
}

impl<T: Ord + Copy, V> IntoIterKey<T, V> {
    pub(crate) fn new(node: Option<Box<Node<T, V>>>) -> Self {
        Self {
            stack: node.into_iter().map(|n| *n).collect()
        }
    }
}

impl<T: Ord + Copy, V> Iterator for IntoIterKey<T, V> {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        if let Some(r) = node.r {
            self.stack.push(*r)
        }
        if let Some(l) = node.l {
            self.stack.push(*l)
        }
        Some(node.key)
    }
}




/**
 * Iterator over immutable values in this sub-tree. The traversal is pre-order.
 */
pub struct Iter<'a, T: Ord + Copy, V> {
    stack: Vec<&'a Node<T, V>>
}

impl<'a, T: Ord + Copy, V> Iter<'a, T, V> {
    pub(crate) fn new(node: &'a Option<Box<Node<T, V>>>) -> Self {
        Self {
            stack: node.iter().map(|n| &**n).collect()
        }
    }
}

impl<'a, T: Ord + Copy, V> Iterator for Iter<'a, T, V> {
    type Item = (&'a Range<T>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        if let Some(r) = &node.r {
            self.stack.push(r)
        }
        if let Some(l) = &node.l {
            self.stack.push(l)
        }
        Some((&node.key, &node.value))
    }
}




/**
 * Iterator over mutable values in this sub-tree. The traversal is pre-order.
 */
pub struct IterMut<'a, T: Ord + Copy, V> {
    stack: Vec<&'a mut Node<T, V>>
}

impl<'a, T: Ord + Copy, V> IterMut<'a, T, V> {
    pub(crate) fn new(node: &'a mut Option<Box<Node<T, V>>>) -> Self {
        Self {
            stack: node.iter_mut().map(|n| &mut **n).collect()
        }
    }
}

impl<'a, T: Ord + Copy, V> Iterator for IterMut<'a, T, V> {
    type Item = (&'a Range<T>, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        if let Some(r) = &mut node.r {
            self.stack.push(r)
        }
        if let Some(l) = &mut node.l {
            self.stack.push(l)
        }
        Some((&node.key, &mut node.value))
    }
}




/**
 * Iterator that visits, by reference in pre-order, only those key-value pairs
 * for which the interval contains the given point.
 */
pub (crate) struct IterPointQuery<'a, T: Ord + Copy, V> {
    stack: Vec<&'a Node<T, V>>,
    point: T
}

impl<'a, T: Ord + Copy, V> IterPointQuery<'a, T, V> {
    pub(crate) fn new(node: &'a Option<Box<Node<T, V>>>, point: T) -> Self {
        Self {
            stack: node.iter().map(|n| &**n).collect(),
            point,
        }
    }
}

impl<'a, T: Ord + Copy, V> Iterator for IterPointQuery<'a, T, V> {
    type Item = (&'a Range<T>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.stack.pop()?;

            if let Some(r) = &node.r {
                if self.point >= node.key.start {
                    self.stack.push(r)
                }
            }
            if let Some(l) = &node.l {
                if self.point < node.max {
                    self.stack.push(l)
                }
            }
            if node.key.contains(&self.point) {
                return Some((&node.key, &node.value))
            }
        }
    }
}




/**
 * Iterator that visits, by reference in pre-order, only those key-value pairs
 * for which the interval intersects the given range boudns object.
 */
pub (crate) struct IterRangeQuery<'a, T: Ord + Copy, V, R: RangeBounds<T>> {
    stack: Vec<&'a Node<T, V>>,
    range: R,
}

impl<'a, T: Ord + Copy, V, R: RangeBounds<T>> IterRangeQuery<'a, T, V, R> {
    pub(crate) fn new(node: &'a Option<Box<Node<T, V>>>, range: R) -> Self {
        Self {
            stack: node.iter().map(|n| &**n).collect(),
            range,
        }
    }
}

impl<'a, T: Ord + Copy, V, R: RangeBounds<T>> Iterator for IterRangeQuery<'a, T, V, R> {
    type Item = (&'a Range<T>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.stack.pop()?;

            if let Some(r) = &node.r {
                if self.range.overlaps(&(node.key.start..)) {
                    self.stack.push(r)
                }
            }
            if let Some(l) = &node.l {
                if self.range.overlaps(&(..node.max)) {
                    self.stack.push(l)
                }
            }
            if self.range.overlaps(&node.key) {
                return Some((&node.key, &node.value))
            }
        }
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
