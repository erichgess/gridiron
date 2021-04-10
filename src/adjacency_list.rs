use std::collections::HashMap;
use core::hash::Hash;




/**
 * A minimal directed graph structure that stores only edges
 */
pub struct AdjacencyList<K> {
    outgoing: HashMap<K, Vec<K>>,
    incoming: HashMap<K, Vec<K>>,
}




// ============================================================================
impl<K> AdjacencyList<K> where K: Hash + Eq + Clone {


	/**
	 * Insert an edge from a -> b. Duplicate edges are allowed.
	 */
    pub fn insert(&mut self, a0: K, b0: K) {
        let a1 = a0.clone();
        let b1 = b0.clone();
        self.outgoing.entry(a0).or_default().push(b0);
        self.incoming.entry(b1).or_default().push(a1);
    }


    /**
     * Determine whether the given edge exists.
     */
    pub fn contains(&mut self, a: K, b: K) -> bool {
    	self.outgoing
    		.get(&a)
    		.and_then(|edges| edges.iter().find(|&k| k == &b))
    		.is_some()
    }


    /**
     * Remove an edge if it exists.
     */
    pub fn remove(&mut self, a0: K, b0: K) {
        let a1 = a0.clone();
        let b1 = b0.clone();
        self.outgoing.entry(a0).and_modify(|edges| edges.retain(|k| k != &b0));
        self.incoming.entry(b1).and_modify(|edges| edges.retain(|k| k != &a1));
    }


    /**
     * Return an iterator over the vertices with edges emanating from the given
     * vertex.
     */
    pub fn outgoing_edges(&self, a: &K) -> impl Iterator<Item = &K> {
    	self.outgoing.get(a).into_iter().flat_map(|edges| edges.iter())
    }


    /**
     * Return an iterator over the vertices with edges pointing to the given
     * vertex.
     */
    pub fn incoming_edges(&self, b: &K) -> impl Iterator<Item = &K> {
    	self.incoming.get(b).into_iter().flat_map(|edges| edges.iter())
    }
}
