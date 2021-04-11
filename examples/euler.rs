use gridiron::adjacency_list::AdjacencyList;
use gridiron::automaton::{self, Automaton, Status};
use gridiron::hydro::euler;
use gridiron::index_space::{range2d, IndexSpace};
use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleMap};
use gridiron::meshing::{self, GraphTopology};

const NUM_GUARD: i64 = 2;
const _NUM_FIELDS: usize = 5;

/**
 * The mesh
 */
#[derive(serde::Serialize)]
struct Mesh {
    area: Rectangle<f64>,
    size: (usize, usize),
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
}

/**
 * The initial model
 */
struct Model {}

impl Model {
    fn primitive_at(&self, position: (f64, f64)) -> euler::Primitive {
        euler::Primitive::new(1.0 + position.0 + position.1, 0.0, 0.0, 0.0, 1.0)
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
    fn new() -> Self {
        let block_size = 10;
        let mesh = Mesh {
            area: (0.0..1.0, 0.0..1.0),
            size: (40, 40),
        };
        let model = Model {};
        let primitive = |index| model.primitive_at(mesh.cell_center(index)).as_array();
        let primitive = range2d(0..4, 0..4)
            .iter()
            .map(|(i, j)| {
                (
                    i * block_size..(i + 1) * block_size,
                    j * block_size..(j + 1) * block_size,
                )
            })
            .map(|rect| Patch::from_vector_function(0, rect, primitive))
            .collect();

        Self {
            iteration: 0,
            time: 0.0,
            primitive,
        }
    }
}

// ============================================================================
fn _advance(state: State) -> State {
    let State {
        mut iteration,
        mut time,
        primitive,
    } = state;

    iteration += 1;
    time += 0.01;

    State {
        time,
        iteration,
        primitive,
    }
}

struct PatchUpdate {
    outgoing_edges: Vec<(Rectangle<i64>, u32)>,
    neighbor_patches: Vec<Patch>,
    incoming_count: usize,
    primitive: Patch,
}

impl PatchUpdate {
    fn new(primitive: Patch, edge_list: &AdjacencyList<(Rectangle<i64>, u32)>) -> Self {
        let key = (primitive.high_resolution_rect(), primitive.level());
        Self {
            outgoing_edges: edge_list.outgoing_edges(&key).cloned().collect(),
            incoming_count: edge_list.incoming_edges(&key).count(),
            neighbor_patches: Vec::new(),
            primitive,
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
        } = self;

        meshing::extend_patch(primitive, |s| s.extend_all(NUM_GUARD), |_, _| {}, &neighbor_patches)
    }
}

// ============================================================================
fn main() {
    let state = State::new();

    let State {
        iteration,
        time,
        primitive,
    } = state;

    let primitive_map: RectangleMap<_, _> = primitive
        .into_iter()
        .map(|p| (p.high_resolution_rect(), p))
        .collect();

    let edge_list = primitive_map.adjacency_list(2);
    let task_list = primitive_map.into_iter()
        .map(|(_, patch)| PatchUpdate::new(patch, &edge_list));

    let primitive = automaton::execute(task_list).collect();

    let state = State {
        iteration,
        time,
        primitive,
    };

    let file = std::fs::File::create("state.cbor").unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();
}
