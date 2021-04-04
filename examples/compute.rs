use std::sync::Arc;
use gridiron::compute;
use gridiron::index_space::range2d;




// ============================================================================
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

    divergence_example();
    divergence_example();
    divergence_example();
    divergence_example();
}




// ============================================================================
fn divergence_example() {
    let num_blocks = (64, 64);
    let block_size = (64, 64);
    let blocks = range2d(0..num_blocks.0 as i64, 0..num_blocks.1 as i64);
    let peers: Vec<_> = blocks.into_iter().map(|ij| DivergenceStencil::new(block_size, num_blocks, ij)).collect();
    let start = std::time::Instant::now();

    for (_key, _result) in compute::exec_with_crossbeam_channel(peers) {

    }
    println!("elapsed: {:.4}s", start.elapsed().as_secs_f64());
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




#[derive(Clone)]

/**
 * Example to compute the divergence of a 2D field, using the '+' stencil, so
 * based on four neighbors.
 */
struct DivergenceStencil {
    data: Arc<Vec<f64>>,
    shape: (usize, usize),
    block: (usize, usize),
    index: (i64, i64),
}

impl DivergenceStencil {
    fn new(shape: (usize, usize), block: (usize, usize), index: (i64, i64)) -> Self {
        Self {
            data: Arc::new(vec![0.0; shape.0 * shape.1]),
            shape,
            block,
            index,
        }
    }
}




// ============================================================================
impl compute::Compute for DivergenceStencil {

    type Key = (i64, i64);

    type Value = Vec<f64>;

    fn key(&self) -> Self::Key {
        self.index
    }

    fn peer_keys(&self) -> Vec<Self::Key> {
        let (i, j) = self.index;
        let (l, m) = self.block;
        let (l, m) = (l as i64, m as i64);

        vec![
            ((i + l - 1) % l, (j + m) % m),
            ((i + l + 1) % l, (j + m) % m),
            ((i + l) % l, (j + m - 1) % m),
            ((i + l) % l, (j + m + 1) % m),
        ]
    }

    fn run(&self, peers: Vec<Self>) -> Self::Value {
        let b00 = &self.data;
        let bxl = &peers[0].data;
        let bxr = &peers[1].data;
        let byl = &peers[2].data;
        let byr = &peers[3].data;

        let (l, m) = self.shape;

        let ind = |i, j| {
            i * m + j
        };
        let mut result = vec![0.0; self.data.len()];

        for i in 0..l {
            for j in 0..m {
                // This amount of work seems to be the minimum to attain
                // perfect scaling on the Mac Pro.
                for _ in 0..500 {
                    let cxl = if i == 0 { bxl[ind(l - 1, j)] } else { b00[ind(i, j)] };
                    let cxr = if i == l - 1 { bxr[ind(0, j)] } else { b00[ind(i, j)] };
                    let cyl = if j == 0 { byl[ind(i, m - 1)] } else { b00[ind(i, j)] };
                    let cyr = if j == m - 1 { byr[ind(i, 0)] } else { b00[ind(i, j)] };
                    result[ind(i, j)] = (cxr - cxl) + (cyr - cyl);
                }
            }
        }
        result
    }
}
