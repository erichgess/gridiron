use core::ops::Range;
use std::cmp::Ordering::{self, Less, Greater, Equal};
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
     * Create a balanced sub-tree from a sorted slice. If the slice is not
     * sorted, the resulting tree is invalid. No check is done here to ensure
     * the slice is sorted.
     */
    fn from_sorted_slice(slice: &[Range<T>]) -> Option<Box<Self>> {
        if slice.is_empty() {
            None
        } else {
            let mid = slice.len() / 2;
            let key = slice[mid].clone();
            let l = Self::from_sorted_slice(&slice[..mid]);
            let r = Self::from_sorted_slice(&slice[mid + 1..]);
            let max = Self::local_max(key.end, &l, &r);
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
    fn contains(&self, key: &Range<T>) -> bool {
        match Self::compare(key, &self.key) {
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

            match Self::compare(&key, &n.key) {
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
    fn remove(node: &mut Option<Box<Self>>, key: &Range<T>) {
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
    fn take_lmost(mut self: Box<Self>) -> (Option<Box<Self>>, Range<T>) {
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
    fn take_rmost(mut self: Box<Self>) -> (Option<Box<Self>>, Range<T>) {
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
    fn lmost_path(&self) -> Vec<&Self> {
        let mut path = vec![self];

        while let Some(l) = path.last().and_then(|b| b.l.as_ref()) {
            path.push(l)
        }
        path
    }




    /**
     * Consume this node and return a list of nodes forming a path from this
     * node to its leftmost node. This function is to facilitate consuming
     * in-order traversal.
     */
    fn into_lmost_path(self) -> Vec<Self> {
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
    fn validate_max(&self) {
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
    fn validate_order(&self) {
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




    fn compare(a: &Range<T>, b: &Range<T>) -> Ordering {
        (a.start, a.end).cmp(&(b.start, b.end))
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

    pub fn contains(&self, key: &Range<T>) -> bool {
        self.root.as_ref().map_or(false, |root| root.contains(key))
    }

    pub fn insert(&mut self, key: Range<T>) {
        Node::insert(&mut self.root, key)
    }

    pub fn remove(&mut self, key: &Range<T>) {
        Node::remove(&mut self.root, key)
    }

    pub fn max(&self) -> Option<T> {
        self.root.as_ref().map(|root| root.max)
    }

    pub fn into_balanced(self) -> Self {
        let data: Vec<_> = self.into_iter().collect();
        Tree { root: Node::from_sorted_slice(&data[..]) }
    }

    pub fn iter<'a>(&'a self) -> TreeIter<'a, T> {
        TreeIter::new(self)
    }

    pub fn validate_max(&self) {
        if let Some(root) = &self.root {
            root.validate_max()
        }
    }

    pub fn validate_order(&self) {
        if let Some(root) = &self.root {
            root.validate_order()
        }
    }

    fn lmost_path(&self) -> Vec<&Node<T>> {
        self.root.as_ref().map_or(Vec::new(), |root| root.lmost_path())
    }

    fn into_lmost_path(mut self) -> Vec<Node<T>> {
        self.root.take().map_or(Vec::new(), |root| root.into_lmost_path())
    }
}




// ============================================================================
impl<T: Ord + Copy> FromIterator<Range<T>> for Tree<T> {
    fn from_iter<I: IntoIterator<Item = Range<T>>>(iter: I) -> Self {
        let mut values: Vec<_> = iter.into_iter().collect();

        values.sort_by(Node::compare);

        Self {
            root: Node::from_sorted_slice(&values[..])
        }
    }
}




// ============================================================================
pub struct TreeIntoIter<T: Ord + Copy> {
    nodes: Vec<Node<T>>
}

impl<T: Ord + Copy> TreeIntoIter<T> {
    fn new(tree: Tree<T>) -> Self {
        Self {
            nodes: tree.into_lmost_path()
        }
    }
}

impl<T: Ord + Copy> Iterator for TreeIntoIter<T> {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {

       /*
        * Pop the last node on the stack (A).
        *
        * If A has a right child (B) then take B and push it onto the stack,
        * followed by the path to its minimum node.
        *
        * Yield the key of A.
        */

        if let Some(mut a) = self.nodes.pop() {
            if let Some(r) = a.r.take() {
                self.nodes.extend(r.into_lmost_path())
            }
            Some(a.key)
        } else {
            None
        }
    }
}

impl<T: Ord + Copy> IntoIterator for Tree<T> {
    type Item = Range<T>;
    type IntoIter = TreeIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        TreeIntoIter::new(self)
    }
}




// ============================================================================
pub struct TreeIter<'a, T: Ord + Copy> {
    nodes: Vec<&'a Node<T>>
}

impl<'a, T: Ord + Copy> TreeIter<'a, T> {
    fn new(tree: &'a Tree<T>) -> Self {
        Self {
            nodes: tree.lmost_path()
        }
    }
}

impl<'a, T: Ord + Copy> Iterator for TreeIter<'a, T> {
    type Item = &'a Range<T>;

    fn next(&mut self) -> Option<Self::Item> {

       /*
        * Pop the last node on the stack (A).
        *
        * If A has a right child (B) then push B onto the stack, followed by the
        * path to its minimum node.
        *
        * Yield the key of A.
        */

        if let Some(a) = self.nodes.pop() {
            if let Some(b) = &a.r {
                self.nodes.extend(b.lmost_path());
            }
            Some(&a.key)
        } else {
            None
        }
    }
}

impl<'a, T: Ord + Copy> IntoIterator for &'a Tree<T> where T: Ord {
    type Item = &'a Range<T>;
    type IntoIter = TreeIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        TreeIter::new(self)
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use core::ops::Range;
    use crate::aug_bst::Tree;

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
        let tree: Tree<_> = stupid_random_intervals(1000, 666).into_iter().collect();
        tree.validate_max();
        tree.validate_order();
    }

    #[test]
    fn max_value_is_correctly_recorded_for_random_incremental_tree() {
        let mut tree = Tree::new();
        for x in stupid_random_intervals(1000, 12345) {
            tree.insert(x)
        }
        tree.validate_max();
        tree.validate_order();
    }

    #[test]
    fn tree_contains_works() {
        let mut tree = Tree::new();
        tree.insert(-5..0);
        tree.insert(-2..0);
        tree.insert(-8..8);
        tree.insert(-6..2);
        tree.insert(-1..2);
        assert_eq!(tree.len(), 5);
        assert!( tree.contains(&(-1..2)));
        assert!( tree.contains(&(-6..2)));
        assert!(!tree.contains(&(-6..3)));
        tree.validate_max();
        tree.validate_order();
    }

    #[test]
    fn tree_removal_works() {
        for i in 0..100 {
            let intervals = stupid_random_intervals(100, i);
            let mut tree = Tree::new();

            for x in &intervals {
                tree.insert(x.clone())
            }

            let x = &intervals[i];
            assert!(tree.contains(x));
            tree.remove(x);
            assert!(!tree.contains(x));

            tree.validate_max();
            tree.validate_order();
        }
    }

    #[test]
    fn can_balance_tree() {
        let tree: Tree<_> = (0..0).map(|i| i..i + 10).collect();
        assert_eq!(tree.into_balanced().height(), 0);

        let tree: Tree<_> = (0..2047).map(|i| i..i + 10).collect();
        assert_eq!(tree.into_balanced().height(), 11);

        let tree: Tree<_> = (0..2048).map(|i| i..i + 10).collect();
        assert_eq!(tree.into_balanced().height(), 12);
    }

    #[test]
    fn tree_into_iter_works() {
        let mut tree = Tree::new();
        tree.insert(5..12);
        tree.insert(2..12);
        tree.insert(7..12);
        tree.insert(0..12);

        let mut iter = tree.iter();
        assert_eq!(iter.next(), Some(&(0..12)));
        assert_eq!(iter.next(), Some(&(2..12)));
        assert_eq!(iter.next(), Some(&(5..12)));
        assert_eq!(iter.next(), Some(&(7..12)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn tree_iter_works() {
        let mut tree = Tree::new();
        tree.insert(5..12);
        tree.insert(2..12);
        tree.insert(7..12);
        tree.insert(0..12);

        let mut iter = tree.into_iter();
        assert_eq!(iter.next(), Some(0..12));
        assert_eq!(iter.next(), Some(2..12));
        assert_eq!(iter.next(), Some(5..12));
        assert_eq!(iter.next(), Some(7..12));
        assert_eq!(iter.next(), None);
    }
}
