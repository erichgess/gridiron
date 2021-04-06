use gridiron::patch::Patch;
use gridiron::rect_map::{Rectangle, RectangleRef, RectangleMap};
use gridiron::index_space::{IndexSpace, Axis, range2d};
use gridiron::hydro::{self, euler};




#[derive(serde::Serialize)]


/**
 * The mesh
 */
struct Mesh {
    area: Rectangle<f64>,
    size: (usize, usize),
}




// ============================================================================
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




struct Model {
}




// ============================================================================
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
    patches: Vec<Patch>
}




// ============================================================================
impl State {

    fn new() -> Self {
        let block_size = 10;
        let mesh = Mesh { area: (0.0 .. 1.0, 0.0 .. 1.0), size: (40, 40) };
        let model = Model{};
        let primitive = |index| model.primitive_at(mesh.cell_center(index)).as_array();
        let patches = range2d(0..4, 0..4)
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
            patches,
        }
    }
}




// ============================================================================
fn finest_patch<'a>(map: &'a RectangleMap<i64, &'a Patch>, index: (i64, i64)) -> Option<&'a Patch> {
    map.query_point(index)
       .map(|(_, &p)| p)
       .min_by_key(|p| p.level())
}

fn extend_patch(map: &RectangleMap<i64, Patch>, rect: RectangleRef<i64>) -> (Rectangle<i64>, Patch) {

    let space: IndexSpace = rect.into();
    let extended = space.extend_all(1);
    let local_map: RectangleMap<_, _> = map.query_rect(extended.clone()).collect();
    let p = local_map.get(rect).unwrap();

    let sample = |index, slice: &mut [f64]| {
        if p.index_space().contains(index) {
            p.sample_slice(p.level(), index, slice)
        } else if let Some(n) = finest_patch(&local_map, index) {
            n.sample_slice(p.level(), index, slice)
        }
    };
    let extended_patch = Patch::from_slice_function(p.level(), extended.clone(), 5, sample);
    (extended.into(), extended_patch)
}




#[allow(unused)]
struct SchemeScratch {
    extended_primitive: Patch,
    flux_i: Patch,
    flux_j: Patch,
}




fn compute_flux(pe: &Patch, axis: Axis) -> Patch {
    match axis {
        Axis::I => Patch::from_slice_function(
            pe.level(),
            pe.index_space().trim_lower(1, Axis::I),
            pe.num_fields(), |(i, j), f| {
                let pl: euler::Primitive = pe.get_slice((i - 1, j)).into();
                let pr: euler::Primitive = pe.get_slice((i, j)).into();
                euler::riemann_hlle(pl, pr, hydro::geometry::Direction::X, 5.0 / 3.0).write_to_slice(f);
            }
        ),
        Axis::J => Patch::from_slice_function(
            pe.level(),
            pe.index_space().trim_lower(1, Axis::J),
            pe.num_fields(), |(i, j), f| {
                let pl: euler::Primitive = pe.get_slice((i, j - 1)).into();
                let pr: euler::Primitive = pe.get_slice((i, j)).into();
                euler::riemann_hlle(pl, pr, hydro::geometry::Direction::Y, 5.0 / 3.0).write_to_slice(f);
            }
        ),
    }
}




// ============================================================================
fn advance(state: State) -> State {

    let State { mut iteration, mut time, patches } = state;

    let mesh: RectangleMap<_, _> = patches.into_iter().map(|p| (p.rect(), p)).collect();

    let extended_mesh: RectangleMap<i64, Patch> = mesh
        .keys()
        .map(|rect| extend_patch(&mesh, rect))
        .collect();

    let mut scheme_scratch: Vec<_> = extended_mesh
    .clone()
    .into_iter()
    .map(|(_, p)|
        SchemeScratch {
            extended_primitive: p,
            flux_i: Patch::default(),
            flux_j: Patch::default(),
        })
    .collect();

    for scratch in &mut scheme_scratch {
        scratch.flux_i = compute_flux(&scratch.extended_primitive, Axis::I);
        scratch.flux_j = compute_flux(&scratch.extended_primitive, Axis::J);
    }

    let mut patches: Vec<_> = extended_mesh.into_iter().map(|(_, p)| p).collect();

    for patch in &mut patches {
        patch.extract_mut(patch.index_space().trim_all(1));
    }

    iteration += 1;
    time += 0.01;

    State {
        time,
        iteration,
        patches,
    }
}




// ============================================================================
fn main() {
    let mut state = State::new();

    while state.time < 1.0 {
        state = advance(state);
        println!("[{}] t={:.4}", state.iteration, state.time);
    }

    let file = std::fs::File::create("state.cbor").unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();
}
