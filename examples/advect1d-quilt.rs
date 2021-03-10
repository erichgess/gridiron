use gridflow::quilt::{Quilt, Area};




struct Mesh {
    shape: usize,
    interval: (f64, f64),
}




// ============================================================================
impl Mesh {

    fn cell_spacing(&self) -> f64 {
        (self.interval.1 - self.interval.0) / self.shape as f64
    }

    fn cell_center(&self, i: i64) -> f64 {
        let dx = self.cell_spacing();
        (i as f64 + 0.5) * dx
    }
}




struct State {
    time: f64,
    quilt: Quilt,
}




impl State {

    fn new(mesh: &Mesh, num_blocks: usize) -> Self {
        let model = |i| f64::exp(-1e2 * (mesh.cell_center(i) - 0.5).powi(2));

        let mut quilt = Quilt::new(Area::with_shape(mesh.shape));
        let n = mesh.shape / num_blocks;

        for i in 0..num_blocks {
            let area = Area::covering(i * n, (i + 1) * n);
            quilt.insert_with_function(area, model);
        }

        Self {
            time: 0.0,
            quilt,
        }
    }

    fn update(&mut self, mesh: &Mesh, dt: f64) {

        let mut extended_quilt = Quilt::new(self.quilt.area);

        for patch in self.quilt.iter() {
            extended_quilt.insert(self.quilt.fabricate_patch(patch.area.expand(1)));
        }

        let dx = mesh.cell_spacing();

        for patch in extended_quilt.iter_mut() {
            let u = to_conserved(&patch.data[1..patch.data.len() - 1].to_vec());
            let f = intercell_flux(&patch.data);

            let u1: Vec<_> = f.windows(2).zip(&u).map(|(w, u)| {
                let fl = w[0];
                let fr = w[1];
                u - (fr - fl) * dt / dx
            }).collect();

            patch.area = patch.area.expand(-1);
            patch.data = to_primitive(&u1);
        }

        self.quilt = extended_quilt;
        self.time += dt;
    }
}




// ============================================================================
fn to_conserved(primitive: &Vec<f64>) -> Vec<f64> {
    primitive.clone()
}

fn to_primitive(conserved: &Vec<f64>) -> Vec<f64> {
    conserved.clone()
}

fn flux(p: f64) -> f64 {
    p
}

fn intercell_flux(primitive: &Vec<f64>) -> Vec<f64> {
    let mut result = vec![0.0; primitive.len() - 1];

    for i in 0..result.len() {
        let fl = flux(primitive[i]);
        let fr = flux(primitive[i + 1]);

        result[i] =

        if fl > 0.0 && fr > 0.0 {
            fl
        } else if fl < 0.0 && fr < 0.0 {
            fr
        } else {
            0.0
        }
    }
    result
}




// ============================================================================
fn main() {

    use std::io::Write;

    let mesh = Mesh {
        shape: 200,
        interval: (0.0, 1.0)
    };

    let mut state = State::new(&mesh, 8);
    let dt = mesh.cell_spacing() * 0.5;


    while state.time < 0.1 {
        println!("t={:.4}", state.time);
        state.update(&mesh, dt)
    }

    let file = std::fs::File::create(format!("solution-{}.dat", 0)).unwrap();

    for n in state.quilt.area.iter() {
        let x = mesh.cell_center(n);
        let y = state.quilt.sample(n);
        writeln!(&file, "{:+.8e} {:+.8e}", x, y).unwrap();
    }
}
