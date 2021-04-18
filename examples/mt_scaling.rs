use clap::{AppSettings, Clap};
use gridiron::thread_pool::ThreadPool;
use rayon::prelude::*;

#[derive(Debug, Clap)]
#[clap(version = "1.0", author = "J. Zrake <jzrake@clemson.edu>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short = 't', long, default_value = "1")]
    num_threads: usize,

    #[clap(short = 'n', long, default_value = "1000")]
    num_jobs: usize,

    #[clap(short = 'w', long, default_value = "100000")]
    work_per_job: usize,
}

fn main() {
    let opts = Opts::parse();
    println!("{:?}", opts);

    let duration = {
        let pool = ThreadPool::new(opts.num_threads);
        let work = opts.work_per_job;
        let start = std::time::Instant::now();

        for _ in 0..opts.num_jobs {
            pool.spawn(move || {
                let _: f64 = (0..work).map(|n| n as f64).sum();
            });
        }
        drop(pool);
        start.elapsed().as_secs_f64()
    };
    println!();
    println!("gridiron::ThreadPool");
    println!("total ................. {}s", duration);
    println!(
        "cpu-s ................. {}",
        duration * opts.num_threads as f64
    );
    println!(
        "cpu-ns / job / work ... {}",
        duration * opts.num_threads as f64 / opts.num_jobs as f64 / opts.work_per_job as f64 * 1e9
    );

    let duration = {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(opts.num_threads)
            .build()
            .unwrap();
        let work = opts.work_per_job;
        let start = std::time::Instant::now();
        pool.scope(|scope| {
            for _ in 0..opts.num_jobs {
                scope.spawn(|_| {
                    let _: f64 = (0..work).map(|n| n as f64).sum();
                });
            }
        });
        start.elapsed().as_secs_f64()
    };
    println!();
    println!("rayon::ThreadPool");
    println!("total ................. {}s", duration);
    println!(
        "cpu-s ................. {}",
        duration * opts.num_threads as f64
    );
    println!(
        "cpu-ns / job / work ... {}",
        duration * opts.num_threads as f64 / opts.num_jobs as f64 / opts.work_per_job as f64 * 1e9
    );

    let duration = {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(opts.num_threads)
            .build()
            .unwrap();
        let work = opts.work_per_job;
        let data: Vec<_> = (0..opts.num_jobs).collect();
        let start = std::time::Instant::now();
        pool.install(|| {
            data.par_iter().for_each(|_| {
                let _: f64 = (0..work).map(|n| n as f64).sum();
            });
        });
        start.elapsed().as_secs_f64()
    };
    println!();
    println!("rayon::par_iter");
    println!("total ................. {}s", duration);
    println!(
        "cpu-s ................. {}",
        duration * opts.num_threads as f64
    );
    println!(
        "cpu-ns / job / work ... {}",
        duration * opts.num_threads as f64 / opts.num_jobs as f64 / opts.work_per_job as f64 * 1e9
    );
}
