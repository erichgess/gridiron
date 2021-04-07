#![feature(test)]
extern crate test;

use std::sync::Arc;
use gridiron::compute;
use gridiron::index_space::range2d;




#[bench]
fn compute_divergence_stencil_with_crossbeam_channel(b: &mut test::Bencher) {
    b.iter(|| {
        let num_blocks = (64, 64);
        let block_size = (64, 64);
        let blocks = range2d(0..num_blocks.0 as i64, 0..num_blocks.1 as i64);
        let peers: Vec<_> = blocks.iter().map(|ij| DivergenceStencil::new(block_size, num_blocks, ij)).collect();
        compute::exec_with_crossbeam_channel(peers).for_each(|_| {})
    });
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

    fn run(self, peers: Vec<Self>) -> Self::Value {
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
                // 500 iterations seems to be the minimum amount of work to
                // attain perfect scaling on the Mac Pro. With #[bench] the
                // time is too long, so for now we just run a single
                // iteration.

                // for _ in 0..500 {
                    let cxl = if i == 0 { bxl[ind(l - 1, j)] } else { b00[ind(i, j)] };
                    let cxr = if i == l - 1 { bxr[ind(0, j)] } else { b00[ind(i, j)] };
                    let cyl = if j == 0 { byl[ind(i, m - 1)] } else { b00[ind(i, j)] };
                    let cyr = if j == m - 1 { byr[ind(i, 0)] } else { b00[ind(i, j)] };
                    result[ind(i, j)] = (cxr - cxl) + (cyr - cyl);
                // }
            }
        }
        result
    }
}
