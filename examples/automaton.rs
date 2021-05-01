use std::collections::HashMap;

use gridiron::{
    automaton::{execute_par, Automaton, Status},
    message::comm::Communicator,
};

struct ConcatenateNearestNeighbors {
    key: u32,
    group_size: u32,
    neighbors: Vec<String>,
}

impl ConcatenateNearestNeighbors {
    fn new(key: u32, group_size: u32) -> Self {
        Self {
            key,
            group_size,
            neighbors: Vec::new(),
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
        vec![(il, format!("{}", self.key)), (ir, format!("{}", self.key))]
    }

    fn receive(&mut self, message: Self::Message) -> Status {
        self.neighbors.push(message);
        Status::eligible_if(self.neighbors.len() == 2)
    }

    fn value(self) -> Self::Value {
        let Self { mut neighbors, .. } = self;
        neighbors.sort();
        format!("{} {} {}", neighbors[0], self.key, neighbors[1])
    }
}

struct FakeComm {}

impl Communicator for FakeComm {
    fn rank(&self) -> usize {
        todo!()
    }

    fn size(&self) -> usize {
        todo!()
    }

    fn send(&self, rank: usize, message: Vec<u8>) {
        todo!()
    }

    fn recv(&self) -> Vec<u8> {
        todo!()
    }
}

fn main() {
    let group_size = 10;

    rayon::scope_fifo(|scope| {
        let group = (0..group_size).map(|n| ConcatenateNearestNeighbors::new(n, group_size));
        let fc = FakeComm {};
        let router: HashMap<u32, usize> = HashMap::new();

        assert_eq! {
            group_size as usize,
            execute_par(scope, group, &fc, &router)
            .inspect(|result| println!("{}", result))
            .count()
        };
    });
}
