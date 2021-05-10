use std::{collections::HashMap, net::SocketAddr};

use clap::{AppSettings, Clap};
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;

use gridiron::hydro::euler2d::Primitive;
use gridiron::index_space::range2d;
use gridiron::rect_map::RectangleMap;
use gridiron::solvers::euler2d_pcm::{Mesh, PatchUpdate};
use gridiron::{automaton, message::ordered::OrderedCommunicator};
use gridiron::{meshing::GraphTopology, rect_map::Rectangle};
use gridiron::{message::tcp::TcpHost, patch::Patch};

/// The initial model
///
struct Model {}

impl Model {
    fn primitive_at(&self, position: (f64, f64)) -> Primitive {
        let (x, y) = position;
        let r = (x * x + y * y).sqrt();

        if r < 0.24 {
            Primitive::new(1.0, 0.0, 0.0, 1.0)
        } else {
            Primitive::new(0.1, 0.0, 0.0, 0.125)
        }
    }
}

/// The simulation solution state
///
#[derive(serde::Serialize)]
struct State {
    time: f64,
    iteration: u64,
    primitive: Vec<Patch>,
}

impl State {
    fn new(mesh: &Mesh, bs: usize) -> Self {
        let bs = bs as i64;
        let ni = mesh.size.0 as i64 / bs;
        let nj = mesh.size.1 as i64 / bs;
        let model = Model {};
        let initial_data = |i| model.primitive_at(mesh.cell_center(i)).as_array();
        let primitive = range2d(0..ni, 0..nj)
            .iter()
            .map(|(i, j)| (i * bs..(i + 1) * bs, j * bs..(j + 1) * bs))
            .map(|rect| Patch::from_vector_function(0, rect, initial_data))
            .collect();

        Self {
            iteration: 0,
            time: 0.0,
            primitive,
        }
    }
}

#[derive(Debug, Clap)]
#[clap(version = "1.0", author = "J. Zrake <jzrake@clemson.edu>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    // TODO: Add flags to set which half of the grid this peer owns.
    // TODO: Add settings for host configuration and peer configuration.
    #[clap(short = 't', long, default_value = "1")]
    num_threads: usize,

    #[clap(
        short = 's',
        long,
        default_value = "serial",
        about = "serial|stupid|rayon"
    )]
    strategy: String,

    #[clap(short = 'n', long, default_value = "1000")]
    grid_resolution: usize,

    #[clap(short = 'b', long, default_value = "100")]
    block_size: usize,

    #[clap(short = 'f', long, default_value = "1")]
    fold: usize,

    #[clap(long, default_value = "0.1")]
    tfinal: f64,

    #[clap(long, required = true)]
    rank: usize,

    #[clap(long)]
    peers: Vec<String>,
}

enum Execution {
    Serial,
    Stupid(gridiron::thread_pool::ThreadPool),
    Rayon(rayon::ThreadPool),
}

fn main() {
    let opts = Opts::parse();
    init_logging();
    info!("{:?}", opts);

    // start the host receiver
    let num_blocks = opts.grid_resolution / opts.block_size;
    let peers: Vec<SocketAddr> = opts
        .peers
        .iter()
        .map(|peer| peer.parse::<SocketAddr>().unwrap())
        .take(num_blocks)
        .collect();

    let mesh = Mesh {
        area: (-1.0..1.0, -1.0..1.0),
        size: (opts.grid_resolution, opts.grid_resolution),
    };
    let State {
        mut iteration,
        mut time,
        primitive,
    } = State::new(&mesh, opts.block_size);

    let primitive_map: RectangleMap<_, _> = primitive
        .into_iter()
        .map(|p| (p.high_resolution_rect(), p))
        .collect();
    let dt = mesh.cell_spacing().0 * 0.1;
    let edge_list = primitive_map.adjacency_list(1);

    let mut router: HashMap<Rectangle<i64>, usize> = HashMap::new();

    let start = (num_blocks / peers.len() * opts.rank * opts.block_size) as i64;
    let end = (if opts.rank == peers.len() - 1 {
        num_blocks
    } else {
        num_blocks / peers.len() * (opts.rank + 1)
    } * opts.block_size) as i64;
    info!("Start: {}; End: {}", start, end);

    let primitive: Vec<_> = primitive_map
        .into_iter()
        .map(|(_, prim)| prim)
        .filter(|prim| {
            let mut rank = (prim.local_rect().0.start / opts.block_size as i64)
                / (num_blocks / peers.len()) as i64;
            if rank >= peers.len() as i64 {
                rank = peers.len() as i64 - 1;
            }
            router.insert(prim.local_rect().clone(), rank as usize);

            rank == opts.rank as i64
        })
        .collect();

    // TODO: Connect to peer which will have the second half of the grid
    let (tcp_host, recv_src, send_sink) = TcpHost::new(opts.rank, peers.clone());
    let mut client = OrderedCommunicator::new(opts.rank, peers.len(), recv_src, send_sink);

    println!("num blocks .... {}", primitive.len());
    println!("num threads ... {}", opts.num_threads);
    println!("");

    let mut task_list: Vec<_> = primitive
        .into_iter()
        .enumerate()
        .map(|(n, patch)| {
            PatchUpdate::new(
                patch,
                mesh.clone(),
                dt,
                Some(n % opts.num_threads),
                &edge_list,
            )
        })
        .collect();

    if opts.grid_resolution % opts.block_size != 0 {
        eprintln!("Error: block size must divide the grid resolution");
        return;
    }

    if opts.strategy == "serial" && opts.num_threads != 1 {
        eprintln!("Error: serial option requires --num-threads=1");
        return;
    }

    let executor = match opts.strategy.as_str() {
        "serial" => Execution::Serial,
        "stupid" => Execution::Stupid(gridiron::thread_pool::ThreadPool::new(opts.num_threads)),
        "rayon" => Execution::Rayon(
            rayon::ThreadPoolBuilder::new()
                .num_threads(opts.num_threads)
                .build()
                .unwrap(),
        ),
        _ => {
            eprintln!("Error: --strategy options are [serial|stupid|rayon]");
            return;
        }
    };

    let num_frames = (opts.tfinal / dt).ceil() as u64;
    info!("Total Frames: {}", num_frames);
    let start_time = std::time::Instant::now();

    let (stats_sink, stats_src) = crossbeam_channel::unbounded();
    for frame in 0..num_frames {
        time = dt * frame as f64;
        let start = std::time::Instant::now();

        for _ in 0..opts.fold {
            client.increment();
            task_list = match &executor {
                Execution::Serial => {
                    automaton::execute(frame as usize, task_list, &client, &router).collect()
                }
                Execution::Stupid(pool) => automaton::execute_par_stupid(
                    &pool,
                    frame as usize,
                    task_list,
                    &client,
                    &router,
                )
                .collect(),
                Execution::Rayon(pool) => pool
                    .scope_fifo(|scope| {
                        automaton::execute_par(
                            scope,
                            frame as usize,
                            task_list,
                            &client,
                            &router,
                            stats_sink.clone(),
                        )
                    })
                    .collect(),
            };
            time += dt;
            iteration += 1;
        }

        let step_seconds = start.elapsed().as_secs_f64() / opts.fold as f64;
        let mzps = mesh.total_zones() as f64 / 1e6 / step_seconds;

        println!(
            "[{}]: [{}] t={:.3} Mzps={:.2} ({:.2}-thread)",
            opts.rank,
            frame,
            time,
            mzps,
            mzps / opts.num_threads as f64
        );
    }
    let compute_duration = start_time.elapsed();
    info!("Time to Compute: {}s", compute_duration.as_secs_f32());

    drop(stats_sink);
    let data = stats::compute_worker_stats(stats_src);
    stats::write_to_file("./durations.csv", &data);

    let primitive = task_list
        .into_iter()
        .map(|block| block.primitive())
        .collect();
    let state = State {
        iteration,
        time,
        primitive,
    };

    let result_file = format!("state-rank-{}.cbor", opts.rank);
    info!("Writing results to {}", result_file);
    let file = std::fs::File::create(result_file).unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();

    info!("Messages to send: {}", client.outbound_len());
    info!("Messages to be processed: {}", client.inbound_len());
    drop(client);
    tcp_host.shutdown();
}

mod stats {
    use crossbeam_channel::Receiver;

    use super::*;
    use std::{
        io::Write,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    pub fn write_to_file(file: &str, v: &[(SystemTime, SystemTime)]) {
        let file = std::fs::File::create(file).unwrap();
        let mut buffer = std::io::BufWriter::new(file);

        writeln!(buffer, "start,stop").unwrap();
        for (start, stop) in v {
            writeln!(
                buffer,
                "{},{}",
                (start.duration_since(UNIX_EPOCH).unwrap()).as_micros(),
                (stop.duration_since(UNIX_EPOCH).unwrap()).as_micros(),
            )
            .unwrap();
        }
    }

    pub fn compute_worker_stats(
        stats_src: Receiver<(SystemTime, SystemTime)>,
    ) -> Vec<(SystemTime, SystemTime)> {
        info!("Worker Measurement Events: {}", stats_src.len());

        let stats: Vec<(SystemTime, SystemTime)> = stats_src.iter().collect();
        let mut durations: Vec<Duration> = stats
            .iter()
            .map(|(s, e)| e.duration_since(*s).unwrap())
            .collect();
        durations.sort_by(|a, b| a.cmp(b));

        let total_time: Duration = durations.iter().sum();
        info!("Total Time Computing: {}ms", total_time.as_millis());

        if durations.len() > 0 {
            let median = durations[durations.len() / 2];
            info!("Median Duration: {}μs", median.as_micros());

            let p90_idx = (durations.len() * 90) / 100;
            info!("p90 Duration: {}μs", durations[p90_idx].as_micros());

            let p95_idx = (durations.len() * 95) / 100;
            info!("p95 Duration: {}μs", durations[p95_idx].as_micros());
        }

        stats
    }
}

fn init_logging() {
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
}
