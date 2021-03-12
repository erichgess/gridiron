#![allow(unused)]



#[derive(Clone, Copy)]
struct Node {
    value: i32,
    l: Option<usize>,
    r: Option<usize>,
}




impl Node {
    fn new(value: i32) -> Self {
        Self {
            value,
            l: None,
            r: None,
        }
    }

    fn insert(&self, tree: &mut BinarySearchTree, value: i32) {
        if value < self.value {

        }
    }
}




struct BinarySearchTree {
    root: Option<usize>,
}




impl BinarySearchTree {

    fn new() -> Self {
        Self {
            root: None,
        }
    }

    fn insert(&mut self, data: &mut Vec<Option<Node>>, value: i32) {
        if let Some(root) = self.root {
            Self::node(data, root).insert(self, value);
        } else {
            self.root = Some(self.allocate_node(data, value))
        }
    }

    fn first_free_index(&self, data: &Vec<Option<Node>>) -> Option<usize> {
        data
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_none())
        .map(|(index, _)| index)
        .next()
    }

    fn allocate_node(&mut self, data: &mut Vec<Option<Node>>, value: i32) -> usize {
        let node = Node::new(value);
        if let Some(index) = self.first_free_index(data) {
            data[index] = Some(node);
            index
        } else {
            data.push(Some(node));
            data.len() - 1
        }
    }

    fn node<'a>(data: &'a Vec<Option<Node>>, index: usize) -> &'a Node {
        data[index].as_ref().unwrap()
    }
}




mod test {
    use crate::bst::Node;
    use crate::bst::BinarySearchTree;

    #[test]
    fn tree_can_be_constructed() {
        let data: Vec<Option<Node>> = Vec::new();
        let tree = BinarySearchTree::new();
    }
}
