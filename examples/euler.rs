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
    conserved: Patch,
    primitive: Patch,
    extended_primitive: Patch,
    flux_i: Patch,
    flux_j: Patch,
    mesh: Mesh,
}

impl PatchUpdate {
    fn new(primitive: Patch, mesh: Mesh, edge_list: &AdjacencyList<(Rectangle<i64>, u32)>) -> Self {
        let key = (primitive.high_resolution_rect(), primitive.level());
        let lv = primitive.level();
        let nq = primitive.num_fields();
        let space = primitive.index_space();
        Self {
            outgoing_edges: edge_list.outgoing_edges(&key).cloned().collect(),
            incoming_count: edge_list.incoming_edges(&key).count(),
            neighbor_patches: Vec::new(),
            conserved: primitive.map(Self::prim_to_cons),
            flux_i: Patch::zeros(lv, nq, space.extend_upper(1, Axis::I)),
            flux_j: Patch::zeros(lv, nq, space.extend_upper(1, Axis::J)),
            extended_primitive: Patch::zeros(lv, nq, space.extend_all(NUM_GUARD)),
            primitive,
            mesh,
        }
    }
}

impl PatchUpdate {
    fn compute_flux(pe: &Patch, axis: Axis, flux: &mut Patch) {
        match axis {
            Axis::I => flux.map_index_mut(|(i, j), f| {
                let pl = pe.get_slice((i - 1, j)).into();
                let pr = pe.get_slice((i, j)).into();
                euler2d::riemann_hlle(pl, pr, Direction::I, GAMMA_LAW_INDEX).write_to_slice(f);
            }),
            Axis::J => flux.map_index_mut(|(i, j), f| {
                let pl = pe.get_slice((i, j - 1)).into();
                let pr = pe.get_slice((i, j)).into();
                euler2d::riemann_hlle(pl, pr, Direction::J, GAMMA_LAW_INDEX).write_to_slice(f);
            }),
        }
    }

    fn cons_to_prim(u: &[f64], p: &mut [f64]) {
        Conserved::from(u)
            .to_primitive(GAMMA_LAW_INDEX)
            .unwrap()
            .write_to_slice(p)
    }

    fn prim_to_cons(p: &[f64], u: &mut [f64]) {
        Primitive::from(p)
            .to_conserved(GAMMA_LAW_INDEX)
            .write_to_slice(u)
    }

    fn boundary_value(_: (i64, i64), p: &mut [f64]) {
        p[0] = 0.1;
        p[1] = 0.0;
        p[2] = 0.0;
        p[3] = 0.125;
    }
}

impl Automaton for PatchUpdate {
    type Key = Rectangle<i64>;
    type Message = Patch;
    type Value = Self;

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
            outgoing_edges,
            incoming_count,
            mesh,
            mut neighbor_patches,
            mut primitive,
            mut extended_primitive,
            mut conserved,
            mut flux_i,
            mut flux_j,
        } = self;

        meshing::extend_patch_mut(
            &primitive,
            Self::boundary_value,
            &neighbor_patches,
            &mut extended_primitive,
        );
        neighbor_patches.clear();

        Self::compute_flux(&extended_primitive, Axis::I, &mut flux_i);
        Self::compute_flux(&extended_primitive, Axis::J, &mut flux_j);

        let (dx, dy) = mesh.cell_spacing();
        let dt = 0.0004;

        for (i, j) in conserved.index_space().iter() {
            let fim = flux_i.get_slice((i, j));
            let fjm = flux_j.get_slice((i, j));
            let fip = flux_i.get_slice((i + 1, j));
            let fjp = flux_j.get_slice((i, j + 1));
            let u = conserved.get_slice_mut((i, j));
            for (n, u) in u.iter_mut().enumerate() {
                *u -= (fip[n] - fim[n]) * dt / dx + (fjp[n] - fjm[n]) * dt / dy;
            }
        }
        conserved.map_into(&mut primitive, Self::cons_to_prim);

        Self {
            outgoing_edges,
            neighbor_patches,
            incoming_count,
            primitive,
            conserved,
            extended_primitive,
            flux_i,
            flux_j,
            mesh,
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

    let edge_list = primitive_map.adjacency_list(2);
    let primitive: Vec<_> = primitive_map.into_iter().map(|(_, prim)| prim).collect();

    println!("num total blocks: {}", primitive.len());

    let mut task_list: Vec<_> = primitive
        .into_iter()
        .map(|patch| PatchUpdate::new(patch, mesh.clone(), &edge_list))
        .collect();

    let advance4 = |task_list| -> Vec<_> {
        let task_list = automaton::execute(task_list);
        let task_list = automaton::execute(task_list);
        let task_list = automaton::execute(task_list);
        let task_list = automaton::execute(task_list);
        task_list.collect()
    };

    while time < 0.1 {
        let start = std::time::Instant::now();

        task_list = advance4(task_list);
        iteration += 4;
        time += 0.0004;

        let step_seconds = start.elapsed().as_secs_f64() / 4.0;
        let mzps = mesh.total_zones() as f64 / 1e6 / step_seconds;

        println!("[{}] t={:.3} Mzps={:.2}", iteration, time, mzps);
    }

    let primitive = task_list.into_iter().map(|block| block.primitive).collect();
    let state = State {
        iteration,
        time,
        primitive,
    };

    let file = std::fs::File::create("state.cbor").unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();
}
