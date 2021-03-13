#![allow(unused)]

use std::cmp::Ordering::*;




// #[derive(Debug)]
// enum RemoveError {
//     DidNotExist,
//     WasEqual,
//     ParentHadTwoChildren,
// }




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
                        match (n.l.take(), n.r.take()) {
                            (None, None) => {
                                *node = None;
                            }
                            (Some(l), None) => {
                                *node = Some(l)
                            }
                            (None, Some(r)) => {
                                *node = Some(r)
                            }
                            _ => {}
                        }
                    }
                }
            }
            None => todo!()
        }
    }




    // ========================================================================
    fn successor(&self, value: i32) -> Option<&Node> {
        match value.cmp(&self.value) {
            Greater|Equal => self.r.as_ref().and_then(|r| r.successor(value)),
            Less          => self.l.as_ref().and_then(|l| l.successor(value)).or(Some(self))
        }
    }

    fn predecessor(&self, value: i32) -> Option<&Node> {
        match value.cmp(&self.value) {
            Less|Equal    => self.l.as_ref().and_then(|l| l.predecessor(value)),
            Greater       => self.r.as_ref().and_then(|r| r.predecessor(value)).or(Some(self))
        }
    }

    fn min(&self) -> &Node {
        self.l.as_ref().map_or(self, |l| l.min())
    }

    fn max(&self) -> &Node {
        self.r.as_ref().map_or(self, |r| r.max())
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




    // ========================================================================
    fn successor(&self, value: i32) -> Option<&Node> {
        self.root.as_ref().and_then(|root| root.successor(value))
    }
    fn predecessor(&self, value: i32) -> Option<&Node> {
        self.root.as_ref().and_then(|root| root.predecessor(value))
    }
}




mod test {

    use crate::bst::Tree;

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

    fn successor_works_on(tree: Tree) {
        assert_eq!(tree.successor(-3).unwrap().value, -2);
        assert_eq!(tree.successor(10).unwrap().value, 11);
        assert_eq!(tree.successor(12).unwrap().value, 15);
        assert!(tree.successor(16).is_none());
    }

    fn predecessor_works_on(tree: Tree) {
        assert_eq!(tree.predecessor(10).unwrap().value, -2);
        assert_eq!(tree.predecessor(11).unwrap().value, 10);
        assert_eq!(tree.predecessor(15).unwrap().value, 11);
        assert!(tree.predecessor(-2).is_none());
    }

    fn remove_value_works_on(mut tree: Tree) {
        assert!(tree.contains(-2));
        tree.remove(-2);
        assert!(!tree.contains(-2));

        assert!(tree.contains(15));
        tree.remove(15);
        assert!(!tree.contains(15));
    }

    #[test]
    fn tree_successor_works_on_ordered_tree() {
        successor_works_on(ordered_tree());
    }

    #[test]
    fn tree_successor_works_on_random_tree() {
        successor_works_on(random_tree());
    }

    #[test]
    fn tree_predecessor_works_on_ordered_tree() {
        predecessor_works_on(ordered_tree());
    }

    #[test]
    fn tree_predecessor_works_on_random_tree() {
        predecessor_works_on(random_tree());
    }

    #[test]
    fn remove_value_works_on_ordered_tree() {
        remove_value_works_on(ordered_tree());
    }
}
