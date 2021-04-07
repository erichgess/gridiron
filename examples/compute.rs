use gridiron::compute;




fn main() {

    let group_size = 10;
    let stage = (0..group_size).map(|index| StringConcat { index, group_size });

    println!("\n--------------------------------------------");
    for (key, result) in compute::exec_with_mpsc_channel(stage.clone()) {
        println!("{} -> {}", key, result);
    }

    println!("\n--------------------------------------------");
    for (key, result) in compute::exec_with_serial_iterator(stage.clone()) {
        println!("{} -> {}", key, result);
    }

    println!("\n--------------------------------------------");
    for (key, result) in compute::exec_with_parallel_iterator(stage.clone()) {
        println!("{} -> {}", key, result);
    }
}




#[derive(Clone)]

/**
 * Example to concatenate nearest neighbor strings in a 1D grid of strings.
 */
struct StringConcat {
    index: usize,
    group_size: usize,
}




// ============================================================================
impl compute::Compute for StringConcat {

    type Key = usize;
    type Value = String;

    fn key(&self) -> Self::Key {
        self.index
    }

    fn peer_keys(&self) -> Vec<Self::Key> {
        vec![(self.index + self.group_size - 1) % self.group_size, (self.index + 1) % self.group_size]
    }

    fn run(&self, peers: Vec<Self>) -> Self::Value {
        format!("{} {} {}", peers[0].index, self.index, peers[1].index)
    }
}
