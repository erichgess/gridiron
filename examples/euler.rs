use gridiron::adjacency_list::AdjacencyList;
use gridiron::automaton::{self, Automaton, Status};
use gridiron::hydro::{euler2d, euler2d::Conserved, euler2d::Primitive, geometry::Direction};
use gridiron::index_space::{range2d, Axis, IndexSpace};
use gridiron::meshing::{self, GraphTopology};
use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleMap};

const NUM_GUARD: i64 = 1;
const GAMMA_LAW_INDEX: f64 = 5.0 / 3.0;

/// The mesh
///
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
struct PatchUpdate {
    conserved: Patch,
    extended_primitive: Patch,
    flux_i: Patch,
    flux_j: Patch,
    incoming_count: usize,
    index_space: IndexSpace,
    level: u32,
    mesh: Mesh,
    neighbor_patches: Vec<Patch>,
    outgoing_edges: Vec<(Rectangle<i64>, u32)>,
}

impl PatchUpdate {
    fn new(primitive: Patch, mesh: Mesh, edge_list: &AdjacencyList<(Rectangle<i64>, u32)>) -> Self {
        let key = (primitive.high_resolution_rect(), primitive.level());
        let lv = primitive.level();
        let nq = primitive.num_fields();
        let index_space = primitive.index_space();
        Self {
            conserved: primitive.map(Self::prim_to_cons),
            extended_primitive: Patch::extract_from(&primitive, index_space.extend_all(NUM_GUARD)),
            flux_i: Patch::zeros(lv, nq, index_space.extend_upper(1, Axis::I)),
            flux_j: Patch::zeros(lv, nq, index_space.extend_upper(1, Axis::J)),
            incoming_count: edge_list.incoming_edges(&key).count(),
            index_space,
            level: primitive.level(),
            mesh,
            neighbor_patches: Vec::new(),
            outgoing_edges: edge_list.outgoing_edges(&key).cloned().collect(),
        }
    }
}

impl PatchUpdate {
    fn compute_flux(pe: &Patch, axis: Axis, flux: &mut Patch) {
        let pl = pe.select(flux.index_space().translate(-1, axis));
        let pr = pe.select(flux.index_space());

        let dir = match axis {
            Axis::I => Direction::I,
            Axis::J => Direction::J,
        };

        for (f, (pl, pr)) in flux.iter_data_mut().zip(pl.zip(pr)) {
            euler2d::riemann_hlle(pl.into(), pr.into(), dir, GAMMA_LAW_INDEX).write_to_slice(f)
        }
    }

    fn primitive(&self) -> Patch {
        self.extended_primitive.extract(self.index_space.clone())
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
        self.index_space.refine_by(1 << self.level).into_rect()
    }

    fn messages(&self) -> Vec<(Self::Key, Self::Message)> {
        self.outgoing_edges
            .iter()
            .cloned()
            .map(|(rect, level)| {
                let overlap = IndexSpace::from(rect.clone())
                    .extend_all(NUM_GUARD * (1 << level))
                    .coarsen_by(1 << self.level)
                    .intersect(self.index_space.clone());
                (rect, self.extended_primitive.extract(overlap))
            })
            .collect()
    }

    fn receive(&mut self, patch: Self::Message) -> Status {
        self.neighbor_patches.push(patch);
        Status::eligible_if(self.neighbor_patches.len() == self.incoming_count)
    }

    fn value(self) -> Self::Value {
        let Self {
            mut conserved,
            mut extended_primitive,
            mut flux_i,
            mut flux_j,
            incoming_count,
            index_space,
            level,
            mesh,
            mut neighbor_patches,
            outgoing_edges,
        } = self;

        meshing::extend_patch_mut(
            &mut extended_primitive,
            &index_space,
            Self::boundary_value,
            &neighbor_patches,
        );
        neighbor_patches.clear();

        Self::compute_flux(&extended_primitive, Axis::I, &mut flux_i);
        Self::compute_flux(&extended_primitive, Axis::J, &mut flux_j);

        let (dx, dy) = mesh.cell_spacing();
        let dt = 0.0004;

        let fim = flux_i.select(index_space.clone());
        let fip = flux_i.select(index_space.translate(1, Axis::I));
        let fjm = flux_j.select(index_space.clone());
        let fjp = flux_j.select(index_space.translate(1, Axis::J));
        let u = conserved.iter_data_mut();

        for (fip, (fim, (fjp, (fjm, u)))) in fip.zip(fim.zip(fjp.zip(fjm.zip(u)))) {
            for (n, u) in u.iter_mut().enumerate() {
                *u -= (fip[n] - fim[n]) * dt / dx + (fjp[n] - fjm[n]) * dt / dy;
            }
        }
        conserved.map_into(&mut extended_primitive, Self::cons_to_prim);

        Self {
            conserved,
            extended_primitive,
            flux_i,
            flux_j,
            incoming_count,
            index_space,
            level,
            mesh,
            neighbor_patches,
            outgoing_edges,
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

    while time < 0.1 {
        let start = std::time::Instant::now();

        task_list = automaton::execute(task_list).collect();
        iteration += 1;
        time += 0.0004;

        let step_seconds = start.elapsed().as_secs_f64();
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
