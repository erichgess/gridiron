use gridiron::adjacency_list::AdjacencyList;
use gridiron::automaton::{self, Automaton, Status};
use gridiron::hydro::{euler2d, euler2d::Conserved, euler2d::Primitive, geometry::Direction};
use gridiron::index_space::{range2d, Axis, IndexSpace};
use gridiron::meshing::{self, GraphTopology};
use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleMap};

const NUM_GUARD: i64 = 1;
const GAMMA_LAW_INDEX: f64 = 5.0 / 3.0;

/**
 * The mesh
 */
#[derive(Clone)]
struct Mesh {
    area: Rectangle<f64>,
    size: (i64, i64),
}

impl Mesh {
    fn cell_spacing(&self) -> (f64, f64) {
        let d0 = (self.area.0.end - self.area.0.start) / self.size.0 as f64;
        let d1 = (self.area.1.end - self.area.1.start) / self.size.1 as f64;
        (d0, d1)
    }

    fn cell_center(&self, index: (i64, i64)) -> (f64, f64) {
        let (d0, d1) = self.cell_spacing();
        let x0 = self.area.0.start + d0 * (index.0 as f64 + 0.5);
        let x1 = self.area.1.start + d1 * (index.1 as f64 + 0.5);
        (x0, x1)
    }

    fn total_zones(&self) -> i64 {
        self.size.0 * self.size.1
    }
}

/**
 * The initial model
 */
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

/**
 * The simulation solution state
 */
#[derive(serde::Serialize)]
struct State {
    time: f64,
    iteration: u64,
    primitive: Vec<Patch>,
}

impl State {
    fn new(mesh: &Mesh) -> Self {
        let bs = 100;
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
struct PatchUpdate {
    outgoing_edges: Vec<(Rectangle<i64>, u32)>,
    neighbor_patches: Vec<Patch>,
    incoming_count: usize,
    primitive: Patch,
    mesh: Mesh,
}

impl PatchUpdate {
    fn new(primitive: Patch, mesh: Mesh, edge_list: &AdjacencyList<(Rectangle<i64>, u32)>) -> Self {
        let key = (primitive.high_resolution_rect(), primitive.level());
        Self {
            outgoing_edges: edge_list.outgoing_edges(&key).cloned().collect(),
            incoming_count: edge_list.incoming_edges(&key).count(),
            neighbor_patches: Vec::new(),
            primitive,
            mesh,
        }
    }
}

impl PatchUpdate {
    fn compute_flux(pe: &Patch, axis: Axis) -> Patch {
        match axis {
            Axis::I => Patch::from_slice_function(
                pe.level(),
                pe.index_space().trim_lower(1, Axis::I),
                pe.num_fields(),
                |(i, j), f| {
                    let pl = pe.get_slice((i - 1, j)).into();
                    let pr = pe.get_slice((i, j)).into();
                    euler2d::riemann_hlle(pl, pr, Direction::I, GAMMA_LAW_INDEX).write_to_slice(f);
                },
            ),
            Axis::J => Patch::from_slice_function(
                pe.level(),
                pe.index_space().trim_lower(1, Axis::J),
                pe.num_fields(),
                |(i, j), f| {
                    let pl = pe.get_slice((i, j - 1)).into();
                    let pr = pe.get_slice((i, j)).into();
                    euler2d::riemann_hlle(pl, pr, Direction::J, GAMMA_LAW_INDEX).write_to_slice(f);
                },
            ),
        }
    }
}

impl Automaton for PatchUpdate {
    type Key = Rectangle<i64>;
    type Message = Patch;
    type Value = Patch;

    fn key(&self) -> Self::Key {
        self.primitive.high_resolution_space().into()
    }

    fn messages(&self) -> Vec<(Self::Key, Self::Message)> {
        self.outgoing_edges
            .iter()
            .cloned()
            .map(|(rect, level)| {
                let overlap = IndexSpace::from(rect.clone())
                    .extend_all(NUM_GUARD * (1 << level))
                    .coarsen_by(1 << self.primitive.level())
                    .intersect(self.primitive.index_space());
                (rect, self.primitive.extract(overlap))
            })
            .collect()
    }

    fn receive(&mut self, patch: Self::Message) -> Status {
        self.neighbor_patches.push(patch);
        Status::eligible_if(self.neighbor_patches.len() == self.incoming_count)
    }

    fn value(self) -> Self::Value {
        let Self {
            outgoing_edges: _,
            neighbor_patches,
            incoming_count: _,
            primitive,
            mesh
        } = self;
        let space = primitive.index_space();
        let pe = meshing::extend_patch(
            &primitive,
            |s| s.extend_all(NUM_GUARD),
            |_index, slice| slice.clone_from_slice(&[0.1, 0.0, 0.0, 0.125]),
            &neighbor_patches,
        );
        let flux_i = Self::compute_flux(&pe, Axis::I);
        let flux_j = Self::compute_flux(&pe, Axis::J);
        let next_u = Patch::from_slice_function(0, space, 4, |(i, j), u|  {
            let fim = flux_i.get_slice((i, j));
            let fjm = flux_j.get_slice((i, j));
            let fip = flux_i.get_slice((i + 1, j));
            let fjp = flux_j.get_slice((i, j + 1));

            let uc = Primitive::from(pe.get_slice((i, j))).to_conserved(GAMMA_LAW_INDEX).as_array();
            let (dx, dy) = mesh.cell_spacing();
            let dt = 0.0004;

            for i in 0..4 {
                u[i] = uc[i] - (fip[i] - fim[i]) * dt / dx - (fjp[i] - fjm[i]) * dt / dy;
            }
        });
        next_u.map(|(u, slice)| Conserved::from(u).to_primitive(GAMMA_LAW_INDEX).unwrap().write_to_slice(slice))
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

    let edge_list = primitive_map.adjacency_list(2);
    let mut primitive: Vec<_> = primitive_map.into_iter().map(|(_, prim)| prim).collect();

    println!("num total blocks: {}", primitive.len());

    while time < 0.1 {
        let start = std::time::Instant::now();
        let task_list = primitive
            .into_iter()
            .map(|patch| PatchUpdate::new(patch, mesh.clone(), &edge_list));
        primitive = automaton::execute(task_list).collect();
        iteration += 1;
        time += 0.0004;

        let step_seconds = start.elapsed().as_secs_f64();
        let mzps = mesh.total_zones() as f64 / 1e6 / step_seconds;

        println!("[{}] t={:.3} Mzps={:.2}", iteration, time, mzps);
    }

    let state = State {
        iteration,
        time,
        primitive,
    };

    let file = std::fs::File::create("state.cbor").unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();
}
