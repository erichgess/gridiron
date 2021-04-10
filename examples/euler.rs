use gridiron::adjacency_list::AdjacencyList;
use gridiron::automaton::{self, Automaton, Status};
use gridiron::hydro::euler;
use gridiron::index_space::{IndexSpace, range2d};
use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleRef, RectangleMap};


// ============================================================================
const NUM_GUARD: i64 = 2;
const _NUM_FIELDS: usize = 5;




#[derive(serde::Serialize)]


/**
 * The mesh
 */
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
struct Model {
}

impl Model {
    fn primitive_at(&self, position: (f64, f64)) -> euler::Primitive {
        euler::Primitive::new(1.0 + position.0 + position.1, 0.0, 0.0, 0.0, 1.0)
    }
}




#[derive(serde::Serialize)]


/**
 * The simulation solution state
 */
struct State {
    iteration: u64,
    time: f64,
    primitive_patches: Vec<Patch>
}

impl State {

    fn new() -> Self {
        let block_size = 10;
        let mesh = Mesh { area: (0.0 .. 1.0, 0.0 .. 1.0), size: (40, 40) };
        let model = Model{};
        let primitive = |index| model.primitive_at(mesh.cell_center(index)).as_array();
        let primitive_patches = range2d(0..4, 0..4)
            .iter()
            .map(|(i, j)| (
                i * block_size .. (i + 1) * block_size, 
                j * block_size .. (j + 1) * block_size)
            )
            .map(|rect| Patch::from_vector_function(0, rect, primitive))
            .collect();

        Self {
            iteration: 0,
            time: 0.0,
            primitive_patches,
        }
    }
}




// ============================================================================
fn _advance(state: State) -> State {

    let State { mut iteration, mut time, primitive_patches } = state;

    iteration += 1;
    time += 0.01;

    State {
        time,
        iteration,
        primitive_patches,
    }
}




struct PatchUpdate {
    outgoing_edges: Vec<(Rectangle<i64>, (IndexSpace, u32))>,
    received_edges: Vec<Patch>,
    number_expected: usize,
    base_primitive: Patch,
}

impl PatchUpdate {
    fn new(
        base_primitive: Patch,
        edges: &AdjacencyList<RectangleRef<i64>>,
        index_space: &RectangleMap<i64, (IndexSpace, u32)>
    ) -> Self {

        let hris = base_primitive.high_resolution_space();
        let outgoing = edges
            .outgoing_edges(&hris.to_rect_ref())
            .cloned()
            .map(|rect_ref| (IndexSpace::from(rect_ref).into_rect(), index_space.get(rect_ref).cloned().unwrap()))
            .collect();

        Self {
            outgoing_edges: outgoing,
            received_edges: Vec::new(),
            number_expected: edges.incoming_edges(&hris.to_rect_ref()).count(),
            base_primitive
        }
    }
}

impl Automaton for PatchUpdate {

    type Key = Rectangle<i64>;
    type Message = Patch;
    type Value = Patch;




    fn key(&self) -> Self::Key {
        self.base_primitive.high_resolution_space().into()
    }




    fn messages(&self) -> Vec<(Self::Key, Self::Message)> {
        self.outgoing_edges
            .iter()
            .cloned()
            .map(|(key, (space, level))| {

                // key .......... key for the outgoing edge
                // space ........ index space of the outgoing edge
                // level ........ level of the outgoing edge

                // The message for the block pointed to by this outgoing edge
                // is the data at the overlap between that block's local
                // index space, extended by NUM_GUARD, scaled to the high
                // resolution space, then coarsened to the level of our patch,
                // and finally intersected with our local index space.

                let overlap = space
                    .extend_all(NUM_GUARD)
                    .refine_by(1 << level)
                    .coarsen_by(1 << self.base_primitive.level())
                    .intersect(self.base_primitive.index_space());

                (key, self.base_primitive.extract(overlap))
            })
            .collect()
    }




    fn receive(&mut self, patch: Self::Message) -> Status {
        self.received_edges.push(patch);
        Status::eligible_if(self.received_edges.len() == self.number_expected)
    }




    fn value(self) -> Self::Value {

        println!("{:?}", self.received_edges.len());

        self.base_primitive
    }
}




// ============================================================================
fn main() {
    let state = State::new();


    // 1. Create a Vec of Patches.
    // ------------------------------------------------------------------------
    let State { iteration: _, time: _, primitive_patches } = state;


    // 2. Generate a RectangleMap keyed on the HRIS (value = the block's local
    // index space and level). 
    // ------------------------------------------------------------------------
    let rectangle_map: RectangleMap<_, _> = primitive_patches
        .iter()
        .map(|p| (p.high_resolution_space().into_rect(), (p.index_space(), p.level())))
        .collect();


    // 3. Query each rectangle in the RectangleMap for the patches it will
    // overlap when it's expanded, to build an adjacency list.
    // ------------------------------------------------------------------------
    let mut edges = AdjacencyList::new();

    for (rect_b, (space_b, _level_b)) in rectangle_map.iter() {
        for (rect_a, (_space_a, _level_a)) in rectangle_map.query_rect(space_b.extend_all(NUM_GUARD)) {
            if rect_a != rect_b {
                edges.insert(rect_a, rect_b)
            }
        }
    }


    // 4. Map the Vec of Patches into a Vec of Tasks, querying the adjacency
    // list to provide incoming and outgoing edges to each task
    // ------------------------------------------------------------------------
    let task_list: Vec<_> = primitive_patches
        .into_iter()
        .map(|patch| PatchUpdate::new(patch, &edges, &rectangle_map))
        .collect();


    for _ in automaton::execute(task_list) {

    }
}
