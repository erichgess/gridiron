use rayon::prelude::*;
use gridiron::index_space::range2d;
use gridiron::thread_pool::ThreadPool;

// assume 2x logical cores per CPU
const LOGICAL_CORES_PER_CPU: usize = 2;

fn main() {
    let num_cores = core_affinity::get_core_ids().unwrap().len() / LOGICAL_CORES_PER_CPU;
    let t0 = run_with_num_threads(1);
    let t1 = run_with_num_threads(1);
    let t2 = run_with_num_threads(num_cores);
    let t3 = run_with_num_threads(num_cores);
    println!("scaling is {:.3}% to {} threads", 100.0 * (t0 + t1) / (t2 + t3) / num_cores as f64, num_cores);
}

fn _run_with_num_threads(num_threads: usize) -> f64 {
    let num_blocks = (64, 64);
    let block_size = (64, 64);
    let blocks = range2d(0..num_blocks.0 as i64, 0..num_blocks.1 as i64);
    let peers: Vec<_> = blocks.iter().map(|_| Task::new(block_size)).collect();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();

    let start = std::time::Instant::now();
    pool.install(|| {
        let _result: Vec<_> = peers.into_par_iter().map(|task| task.run()).collect();
    });
    let delta = start.elapsed().as_secs_f64();
    println!("num_threads: {}: {:.4}s", num_threads, delta);
    delta
}

fn run_with_num_threads(num_threads: usize) -> f64 {
    let num_blocks = (64, 64);
    let block_size = (64, 64);
    let blocks = range2d(0..num_blocks.0 as i64, 0..num_blocks.1 as i64);
    let peers: Vec<_> = blocks.iter().map(|_| Task::new(block_size)).collect();

    let pool = ThreadPool::new(num_threads);

    let start = std::time::Instant::now();
    for task in peers {
        pool.spawn(move || {
            task.run();
        });
    }
    drop(pool);
    let delta = start.elapsed().as_secs_f64();
    println!("num_threads: {}: {:.4}s", num_threads, delta);
    delta
}

struct Task {
    data: Vec<f64>,
    result: Vec<f64>,
    shape: (usize, usize),
}

impl Task {
    fn new(shape: (usize, usize)) -> Self {
        Self {
            data: range2d(0..shape.0 as i64, 0..shape.1 as i64)
                .iter()
                .map(|(i, j)| i as f64 + j as f64)
                .collect(),
            result: vec![0.0; shape.0 * shape.1],
            shape,
        }
    }

    fn run(self) -> Vec<f64> {
        let Self { data, mut result, shape: (l, m) } = self;

        let ind = |i, j| i * m + j;

        for _ in 0..500 {
            for i in 1..l - 1 {
                for j in 1..m - 1 {
                    let cxl = data[ind(i, j)];
                    let cxr = data[ind(i, j)];
                    let cyl = data[ind(i, j)];
                    let cyr = data[ind(i, j)];
                    result[ind(i, j)] = (cxr - cxl) + (cyr - cyl);
                }
            }
        }
        result
    }
}
