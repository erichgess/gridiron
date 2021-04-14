use gridiron::automaton;
use gridiron::hydro::euler2d::Primitive;
use gridiron::index_space::range2d;
use gridiron::meshing::GraphTopology;
use gridiron::patch::Patch;
use gridiron::rect_map::RectangleMap;
use gridiron::solvers::euler2d_pcm::{Mesh, PatchUpdate};

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
    fn new(mesh: &Mesh) -> Self {
        let bs = 200;
        let ni = mesh.size.0 / bs;
        let nj = mesh.size.1 / bs;
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

// ============================================================================
fn main() {
    let mesh = Mesh {
        area: (-1.0..1.0, -1.0..1.0),
        size: (1000, 1000),
    };
    let State {
        mut iteration,
        mut time,
        primitive,
    } = State::new(&mesh);

    let primitive_map: RectangleMap<_, _> = primitive
        .into_iter()
        .map(|p| (p.high_resolution_rect(), p))
        .collect();
    let dt = mesh.cell_spacing().0 * 0.1;
    let edge_list = primitive_map.adjacency_list(2);
    let primitive: Vec<_> = primitive_map.into_iter().map(|(_, prim)| prim).collect();

    println!("num total blocks: {}", primitive.len());

    let mut task_list: Vec<_> = primitive
        .into_iter()
        .map(|patch| PatchUpdate::new(patch, mesh.clone(), dt, &edge_list))
        .collect();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();

    while time < 0.1 {
        let start = std::time::Instant::now();

        task_list = pool.scope_fifo(|scope| {
            let task_list_iter = automaton::execute_par(scope, task_list);
            let task_list_iter = automaton::execute_par(scope, task_list_iter);
            task_list_iter.collect()
        });

        iteration += 2;
        time += dt * 2.0;

        let step_seconds = start.elapsed().as_secs_f64() / 2.0;
        let mzps = mesh.total_zones() as f64 / 1e6 / step_seconds;

        println!("[{}] t={:.3} Mzps={:.2}", iteration, time, mzps);
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
}
