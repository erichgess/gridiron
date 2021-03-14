use std::cmp::Ordering::*;




/**
 * A node in a binary search tree
 */
struct Node<T> {
    value: T,
    l: Option<Box<Node<T>>>,
    r: Option<Box<Node<T>>>,
}




// ============================================================================
impl<T: Ord> Node<T> {




    /**
     * Create an empty sub-tree with the given value
     */
    fn new(value: T) -> Self {
        Self { value, l: None, r: None }
    }




    /**
     * Create a sub-tree from a slice, which is into_balanced if the slice is sorted,
     */
    fn from_slice(slice: &mut [Option<T>]) -> Option<Box<Self>> {
        if slice.is_empty() {
            None
        } else {
            let mid = slice.len() / 2;
            Some(Box::new(Self {
                value: slice[mid].take().unwrap(),
                l: Self::from_slice(&mut slice[..mid]),
                r: Self::from_slice(&mut slice[mid + 1..]),
            }))
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
     * Return true of the given value exists in this sub-tree.
     */
    fn contains(&self, value: T) -> bool {
        match value.cmp(&self.value) {
            Less    => self.l.as_ref().map_or(false, |l| l.contains(value)),
            Greater => self.r.as_ref().map_or(false, |r| r.contains(value)),
            Equal   => true
        }
    }




    /**
     * Insert a node with the given value into this sub-tree.
     */
    fn insert(&mut self, value: T) {
        match value.cmp(&self.value) {
            Less => {
                if let Some(l) = &mut self.l {
                    l.insert(value)
                } else {
                    self.l = Some(Box::new(Node::new(value)))
                }
            }
            Greater => {
                if let Some(r) = &mut self.r {
                    r.insert(value)
                } else {
                    self.r = Some(Box::new(Node::new(value)))
                }
            }
            Equal => {}
        }
    }




    /**
     * Remove a node with the given value from this sub-tree.
     */
    fn remove(node: &mut Option<Box<Node<T>>>, value: T) {
        match node {
            Some(n) => {
                match value.cmp(&n.value) {
                    Less    => Self::remove(&mut n.l, value),
                    Greater => Self::remove(&mut n.r, value),
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
                                n.value = min_r;
                                n.l = Some(l);
                                n.r = new_r;
                            } else {
                                let (new_l, max_l) = l.take_max();
                                n.value = max_l;
                                n.l = new_l;
                                n.r = Some(r);
                            }
                        }
                    }
                }
            }
            None => {}
        }
    }




    /**
     * Return this sub-tree, but with the left-most descendant node removed.
     * Also return the value of that node.
     */
    fn take_min(mut self: Box<Self>) -> (Option<Box<Self>>, T) {
        if let Some(l) = self.l {
            if l.l.is_none() {
                self.l = None;
                (Some(self), l.value)
            } else {
                let (new_l, min) = l.take_min();
                self.l = new_l;
                (Some(self), min)
            }
        } else {
            (None, self.value)
        }
    }




    /**
     * Return this sub-tree, but with the right-most descendant node removed.
     * Also return the value of that node.
     */
    fn take_max(mut self: Box<Self>) -> (Option<Box<Self>>, T) {
        if let Some(r) = self.r {
            if r.r.is_none() {
                self.r = None;
                (Some(self), r.value)
            } else {
                let (new_r, max) = r.take_max();
                self.r = new_r;
                (Some(self), max)
            }
        } else {
            (None, self.value)
        }
    }
}




/**
 * A binary search tree
 */
pub struct Tree<T> {
    root: Option<Box<Node<T>>>
}




// ============================================================================
impl<T: Ord> Tree<T> {

    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())
    }

    pub fn height(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.height())
    }

    pub fn contains(&self, value: T) -> bool {
        self.root.as_ref().map_or(false, |root| root.contains(value))
    }

    pub fn insert(&mut self, value: T) {
        if let Some(root) = &mut self.root {
            root.insert(value)
        } else {
            self.root = Some(Box::new(Node::new(value)))
        }
    }

    pub fn remove(&mut self, value: T) {
        Node::remove(&mut self.root, value)
    }

    pub fn into_balanced(self) -> Self {
        self.into_iter().collect()
    }
}




// ============================================================================
impl<T: Ord> std::iter::FromIterator<T> for Tree<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>
    {
        let mut values: Vec<_> = iter.into_iter().map(|v| Some(v)).collect();
        Self {
            root: Node::from_slice(&mut values[..])
        }
    }
}




// ============================================================================
impl<T> IntoIterator for Tree<T> {
    type Item = T;
    type IntoIter = TreeIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        TreeIntoIter::new(self)
    }
}




// ============================================================================
pub struct TreeIntoIter<T> {
    nodes: Vec<Node<T>>
}

impl<T> TreeIntoIter<T> {
    fn new(tree: Tree<T>) -> Self {
        Self {
            nodes: tree.root.into_iter().map(|root| *root).collect()
        }
    }
}

impl<T> Iterator for TreeIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {

        /*
         * While the last node on the stack (A) has a left child B, push B onto
         * the stack.
         *
         * If B has a right child C, then push C onto the stack.
         *
         * Yield the value of B.
         */

        while let Some(l) = self.nodes.last_mut().and_then(|n| n.l.take()) {
            self.nodes.push(*l)
        }

        if let Some(mut node) = self.nodes.pop() {
            if let Some(r) = node.r.take() {
                self.nodes.push(*r)
            }
            Some(node.value)
        } else {
            None
        }
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use crate::bst::Tree;

    fn ordered_tree() -> Tree<i32> {
        let mut tree = Tree::new();
        tree.insert(-2);
        tree.insert(10);
        tree.insert(11);
        tree.insert(15);
        tree.insert(16);
        tree
    }

    fn random_tree() -> Tree<i32> {
        let mut tree = Tree::new();
        tree.insert(15);
        tree.insert(16);
        tree.insert(10);
        tree.insert(11);
        tree.insert(-2);
        tree
    }

    fn remove_value_works_on(mut tree: Tree<i32>) {
        assert!(tree.contains(-2));
        tree.remove(-2);
        assert!(!tree.contains(-2));

        assert!(tree.contains(15));
        tree.remove(15);
        assert!(!tree.contains(15));

        assert_eq!(tree.len(), 3);
        tree.remove(0);
        assert_eq!(tree.len(), 3);

        assert!(tree.contains(11));
        tree.remove(11);
        assert!(!tree.contains(11));
    }

    #[test]
    fn tree_insertion_works() {
        let tree = ordered_tree();
        assert!(tree.contains(10));
        assert!(tree.contains(11));
        assert!(!tree.contains(12));
    }

    #[test]
    fn tree_len_is_correct() {
        assert_eq!(ordered_tree().len(), 5);
        assert_eq!(random_tree().len(), 5);
    }

    #[test]
    fn tree_height_is_correct() {
        assert_eq!(ordered_tree().height(), 5);
        assert_eq!(random_tree().height(), 3);
    }

    #[test]
    fn remove_value_works_on_ordered_tree() {
        remove_value_works_on(ordered_tree());
    }

    #[test]
    fn remove_value_works_on_random_tree() {
        remove_value_works_on(random_tree());
    }

    #[test]
    fn can_take_min_node() {
        let tree = random_tree();
        let (root, min) = tree.root.unwrap().take_min();

        assert_eq!(min, -2);
        assert_eq!(root.unwrap().len(), 4);
    }

    #[test]
    fn can_iterate_tree() {
        let mut iter = ordered_tree().into_iter();
        assert_eq!(iter.next(), Some(-2));
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.next(), Some(11));
        assert_eq!(iter.next(), Some(15));
        assert_eq!(iter.next(), Some(16));
        assert_eq!(iter.next(), None);

        let mut vec = Vec::new();

        for value in random_tree() {
            vec.push(value)
        }
        assert_eq!(vec, vec![-2, 10, 11, 15, 16]);
    }

    #[test]
    fn can_build_tree_from_iter() {
        let tree: Tree<_> = [-2, 10, 11, 15, 16].iter().collect();
        assert_eq!(tree.len(), 5);
    }

    #[test]
    fn can_balance_tree() {
        let tree: Tree<_> = (0..0).collect();
        assert_eq!(tree.into_balanced().height(), 0);

        let tree: Tree<_> = (0..1).collect();
        assert_eq!(tree.into_balanced().height(), 1);

        let tree: Tree<_> = (0..2).collect();
        assert_eq!(tree.into_balanced().height(), 2);

        let tree: Tree<_> = (0..1024).collect();
        assert_eq!(tree.into_balanced().height(), 11);
    }
}
