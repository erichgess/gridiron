#![allow(unused)]

use std::cmp::Ordering;




struct Node {
    value: i32,
    l: Option<usize>,
    r: Option<usize>,
}

impl Node {
    fn new(value: i32) -> Self {
        Self {
            value: value,
            l: None,
            r: None,
        }
    }
}




struct Tree {
    root: Option<usize>,
    nodes: Vec<Node>,
}




impl Tree {




    // ========================================================================
    fn new() -> Self {
        Self {
            root: None,
            nodes: Vec::new(),
        }
    }

    fn insert(&mut self, value: i32) {
        self.root = self.insert_node(self.root, value)
    }

    pub fn contains(&self, value: i32) -> bool {
        self.contains_node(self.root, value)
    }




    // ========================================================================
    fn contains_node(&self, id: Option<usize>, value: i32) -> bool {
        if let Some(node) = id.map(|id| &self.nodes[id]) {
            match value.cmp(&node.value) {
                Ordering::Less    => self.contains_node(node.l, value),
                Ordering::Greater => self.contains_node(node.r, value),
                Ordering::Equal   => true
            }
        } else {
            false
        }
    }

    fn insert_node(&mut self, id: Option<usize>, value: i32) -> Option<usize> {
        if let Some(id) = id {

            let l = self.nodes[id].l;
            let r = self.nodes[id].r;

            match value.cmp(&self.nodes[id].value) {
                Ordering::Less    => { self.nodes[id].l = self.insert_node(l, value); self.nodes[id].l }
                Ordering::Greater => { self.nodes[id].r = self.insert_node(r, value); self.nodes[id].r }
                Ordering::Equal   => Some(id),
            }
        } else {
            self.nodes.push(Node::new(value));
            Some(self.nodes.len() - 1)
        }
    }
}
