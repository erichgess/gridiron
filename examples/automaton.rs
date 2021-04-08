use gridiron::automaton::{Automaton, Receipt, execute_par};




struct ConcatenateNearestNeighbors {
    key: u32,
    group_size: u32,
    neighbors: Vec<(u32, String)>,
}




impl ConcatenateNearestNeighbors {

    fn new(key: u32, group_size: u32) -> Self {
        Self {
            key, group_size, neighbors: Vec::new()
        }
    }

    fn neighbor_indexes(&self) -> (u32, u32) {
        let il = (self.key + self.group_size - 1) % self.group_size;
        let ir = (self.key + self.group_size + 1) % self.group_size;
        (il, ir)        
    }
}




impl Automaton for ConcatenateNearestNeighbors {
    type Key = u32;

    type Message = String;

    type Value = String;

    fn key(&self) -> Self::Key {
        self.key
    }

    fn messages(&self) -> Vec<(Self::Key, Self::Message)> {
        let (il, ir) = self.neighbor_indexes();
        vec![
            (il, format!("{}", self.key)),
            (ir, format!("{}", self.key))]
    }

    fn receive(&mut self, message: (Self::Key, Self::Message)) -> Receipt<Self::Key> {
        let (key, data) = message;
        self.neighbors.push((key.clone(), data));

        if self.neighbors.len() == 2 {
            Receipt::Eligible
        } else {
            Receipt::Ineligible(key)
        }
    }

    fn value(self) -> Self::Value {
        let Self { mut neighbors, .. } = self;
        neighbors.sort();
        format!("{} {} {}", neighbors[0].1, self.key, neighbors[1].1)
    }
}




fn main() {

    let group_size = 10;

    rayon::scope(|scope| {
        let group = (0..group_size).map(|n| ConcatenateNearestNeighbors::new(n, group_size));

        assert_eq!{
            group_size as usize,
            execute_par(scope, group)
            .inspect(|result| println!("{}", result))
            .count()
        };
    });
}
