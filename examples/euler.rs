#![allow(unused)]

use std::sync::Arc;
use gridiron::automaton::{self, Automaton};
use gridiron::compute::Compute;
use gridiron::hydro::{self, euler};
use gridiron::index_space::{IndexSpace, Axis, range2d};
use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleRef, RectangleMap};




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
fn advance(state: State) -> State {

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
    outgoing_edges: Vec<Rectangle<i64>>,
    received_edges: Vec<Patch>,
    number_expected: usize,
    base_primitive: Patch,
}

impl PatchUpdate {
    fn new(base_primitive: Patch) -> Self {
        Self {
            outgoing_edges: Vec::new(),
            received_edges: Vec::new(),
            number_expected: 0,
            base_primitive
        }
    }
}

impl Automaton for PatchUpdate {

    type Key = Rectangle<i64>;
    type Message = Patch;
    type Value = Patch;

    /**
     * The task is keyed on its high resolution index space.
     */
    fn key(&self) -> Self::Key {
        self.base_primitive.high_resolution_space().into()
    }

    /**
     * When computing messages, rect is in the high-resolution index space, so
     * it needs to be converted to this patch's level for the purpose of
     * extracting this patch's data. 
     */
    fn messages(&self) -> Vec<(Self::Key, Self::Message)> {
        self.outgoing_edges
            .iter()
            .map(|r| (r.clone(), self.base_primitive.extract_overlap_with_high(r.clone())))
            .collect()
    }

    fn receive(&mut self, patch: Self::Message) -> automaton::Status {
        self.received_edges.push(patch);
        automaton::Status::eligible_if(self.received_edges.len() == self.number_expected)
    }

    fn value(self) -> Self::Value { todo!() }
}




// ============================================================================
fn main() {
    let state = State::new();

   /*
    * 1. Create a Vec of Patches
    *
    * 2. Generate a RectangleMap keyed on the HRIS (value = the block's local
    * index space)
    *
    * 3. Query each rectangle in the RectangleMap for the patches it will
    * overlap when it's expanded, to build an adjacency list
    *
    * 4. Map the Vec of Patches into a Vec of Tasks, querying the adjacency list
    * to provide incoming and outgoing edges to each task
    */

    // let State { iteration: _, time: _, primitive_patches } = state;

}
