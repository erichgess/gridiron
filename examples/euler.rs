use clap::{AppSettings, Clap};
use crossbeam_channel::unbounded;
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;

use gridiron::index_space::range2d;
use gridiron::meshing::GraphTopology;
use gridiron::patch::Patch;
use gridiron::rect_map::RectangleMap;
use gridiron::solvers::euler2d_pcm::{Mesh, PatchUpdate};
use gridiron::{automaton, host::receiver};
use gridiron::{host::sender, hydro::euler2d::Primitive};

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

    #[clap(long, default_value = "0")]
    start: i64,

    #[clap(long, default_value = "1000")]
    end: i64,

    #[clap(long)]
    port: u32,

    #[clap(long)]
    peer_addr: String,
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

    let primitive: Vec<_> = primitive_map
        .into_iter()
        .map(|(_, prim)| prim)
        .filter(|prim| {
            // TODO: filter the primitives to only be half of the grid
            opts.start <= prim.local_rect().0.start && prim.local_rect().0.end <= opts.end
        })
        .collect();

    // TODO: Connect to peer which will have the second half of the grid
    let mut threads = vec![];
    let (from_peer_s, from_peer_r) = unbounded(); // i_s goes to the server and i_r goes to the worker
    let (to_peer_s, to_peer_r) = unbounded(); // o_s goes to the worker and o_r goes to the client
    let (rcv_sig_s, rcv_sig_r) = unbounded();

    // start the host receiver
    {
        let rcv_sig_r = rcv_sig_r.clone();
        let port = opts.port;
        let receiver = std::thread::spawn(move || {
            receiver::receiver(port, from_peer_s, rcv_sig_r);
            info!("Receiver thread completed");
        });
        threads.push(receiver);
    }
    {
        // start the sender thread
        let peer_addr = opts.peer_addr.clone();
        let sender = std::thread::spawn(move || {
            sender::sender(peer_addr, to_peer_r);
            info!("Sender thread completed");
        });
        threads.push(sender);
    }

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

    // TODO: change this to use an integer counter.  Needs reliable and deterministic ordering.
    // TODO: then compute the time from the frame counter
    let num_frames = (opts.tfinal / dt).ceil() as u64;
    info!("Total Frames: {}", num_frames);
    let local_range = (opts.start, opts.end);
    for frame in 0..num_frames {
        time = dt * frame as f64;
        let start = std::time::Instant::now();

        // TODO: Handle folding with distribution
        for _ in 0..opts.fold {
            let to_peer_s = to_peer_s.clone();
            let from_peer_r = from_peer_r.clone();
            task_list = match &executor {
                Execution::Serial => {
                    automaton::execute(task_list, to_peer_s, from_peer_r, local_range).collect()
                }
                Execution::Stupid(pool) => automaton::execute_par_stupid(
                    &pool,
                    task_list,
                    to_peer_s,
                    from_peer_r,
                    local_range,
                )
                .collect(),
                Execution::Rayon(pool) => pool
                    .scope_fifo(|scope| {
                        automaton::execute_par(
                            scope,
                            task_list,
                            to_peer_s,
                            from_peer_r,
                            local_range,
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
            "[{}] t={:.3} Mzps={:.2} ({:.2}-thread)",
            frame,
            time,
            mzps,
            mzps / opts.num_threads as f64
        );
    }

    let primitive = task_list
        .into_iter()
        .map(|block| block.primitive())
        .collect();
    let state = State {
        iteration,
        time,
        primitive,
    };

    let file = std::fs::File::create("state.cbor").unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();

    // Wait until all threads are complete to exit the service
    drop(to_peer_s);
    rcv_sig_s.send(gridiron::host::msg::Signal::Stop).unwrap();
    for t in threads {
        t.join().unwrap();
    }
}

fn init_logging() {
    // configure logger
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
}
