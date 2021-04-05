use gridiron::patch::Patch;
use gridiron::rect_map::Rectangle;
use gridiron::index_space::range2d;
use gridiron::hydro::euler;




#[derive(serde::Serialize)]
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
struct State {
    patches: Vec<Patch>
}




// ============================================================================
impl State {
    fn new() -> Self {

        let block_size = 25;
        let mesh = Mesh { area: (0.0 .. 1.0, 0.0 .. 1.0), size: (100, 100) };
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
            patches
        }
    }
}




fn main() {
    let state = State::new();
    let file = std::fs::File::create("state.cbor").unwrap();
    let mut buffer = std::io::BufWriter::new(file);
    ciborium::ser::into_writer(&state, &mut buffer).unwrap();
}
