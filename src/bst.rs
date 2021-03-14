#![allow(unused)]

use std::cmp::Ordering::*;




struct Node {
    value: i32,
    l: Option<Box<Node>>,
    r: Option<Box<Node>>,
}




// ============================================================================
impl Node {




    fn new(value: i32) -> Self {
        Self { value, l: None, r: None }
    }

    fn len(&self) -> usize {
        self.l.as_ref().map_or(0, |l| l.len()) +
        self.r.as_ref().map_or(0, |r| r.len()) + 1
    }

    fn contains(&self, value: i32) -> bool {
        match value.cmp(&self.value) {
            Less    => self.l.as_ref().map_or(false, |l| l.contains(value)),
            Greater => self.r.as_ref().map_or(false, |r| r.contains(value)),
            Equal   => true
        }
    }

    fn insert(&mut self, value: i32) {
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




    // ========================================================================
    fn remove(node: &mut Option<Box<Node>>, value: i32) {
        match node {
            Some(n) => {
                match value.cmp(&n.value) {
                    Less    => Self::remove(&mut n.l, value),
                    Greater => Self::remove(&mut n.r, value),
                    Equal => {
                        *node = match (n.l.take(), n.r.take()) {
                            (None, None) => {
                                None
                            }
                            (Some(l), None) => {
                                Some(l)
                            }
                            (None, Some(r)) => {
                                Some(r)
                            }
                            (Some(l), Some(r)) => {
                                let (new_r, min_r) = Self::take_min(r);
                                let new_node = Node {
                                    value: min_r,
                                    l: Some(l),
                                    r: new_r,
                                };
                                Some(Box::new(new_node))
                            }
                        };
                    }
                }
            }
            None => {}
        }
    }




    /**
     * Return this node with its minimum value removed. The node must have a
     * left child.
     */
    fn take_min(mut node: Box<Self>) -> (Option<Box<Self>>, i32) {
        if node.l.as_ref().is_none() {
            let Node { value, .. } = *node;
            (None, value)
        }
        else if node.l.as_ref().unwrap().l.is_none() {
            let Node { value, l, r } = *node.l.unwrap();
            let new_node = Node {
                value: node.value,
                l: None,
                r: node.r
            };
            (Some(Box::new(new_node)), value)
        } else {
            let (new_l, min) = Self::take_min(node.l.take().unwrap());
            node.l = new_l;
            (Some(node), min)
        }
    }

}




struct Tree {
    root: Option<Box<Node>>
}




// ============================================================================
impl Tree {

    pub fn new() -> Self {
        Self { root: None }
    }

    fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |root| root.len())
    }

    pub fn contains(&self, value: i32) -> bool {
        self.root.as_ref().map_or(false, |root| root.contains(value))
    }

    pub fn insert(&mut self, value: i32) {
        if let Some(root) = &mut self.root {
            root.insert(value)
        } else {
            self.root = Some(Box::new(Node::new(value)))
        }
    }

    pub fn remove(&mut self, value: i32) {
        Node::remove(&mut self.root, value)
    }
}




mod test {

    use crate::bst::{Tree, Node};

    fn ordered_tree() -> Tree {
        let mut tree = Tree::new();
        tree.insert(-2);
        tree.insert(10);
        tree.insert(11);
        tree.insert(15);
        tree.insert(16);
        tree        
    }

    fn random_tree() -> Tree {
        let mut tree = Tree::new();
        tree.insert(15);
        tree.insert(16);
        tree.insert(10);
        tree.insert(11);
        tree.insert(-2);
        tree        
    }

    #[test]
    fn tree_insertion_works() {
        let tree = ordered_tree();
        assert!(tree.contains(10));
        assert!(tree.contains(11));
        assert!(!tree.contains(12));
    }

    fn remove_value_works_on(mut tree: Tree) {
        assert!(tree.contains(-2));
        tree.remove(-2);
        assert!(!tree.contains(-2));

        assert!(tree.contains(15));
        tree.remove(15);
        assert!(!tree.contains(15));

        tree.remove(0);
        assert_eq!(tree.len(), 3);
    }

    #[test]
    fn tree_len_is_correct() {
        assert_eq!(ordered_tree().len(), 5);
        assert_eq!(random_tree().len(), 5);
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
        let (root, min) = Node::take_min(tree.root.unwrap());

        assert_eq!(min, -2);
        assert_eq!(root.unwrap().len(), 4);
    }
}
