struct Mesh {
    shape: u32,
    interval: (f64, f64),
}




// ============================================================================
impl Mesh {

    fn cell_spacing(&self) -> f64 {
        (self.interval.1 - self.interval.0) / self.shape as f64
    }

    fn cell_centers(&self, range: (u32, u32)) -> Vec<f64> {
        let dx = self.cell_spacing();
        (range.0..range.1).map(|i| (i as f64 + 0.5) * dx).collect()
    }
}




struct Patch {
    range: (u32, u32),
    primitive: Vec<f64>,
}




// ============================================================================
impl Patch {

    fn from_model<F>(f: F, range: (u32, u32), mesh: &Mesh) -> Self
    where
        F: Fn(f64) -> f64
    {
        Self {
            range: range,
            primitive: mesh.cell_centers(range).iter().cloned().map(f).collect(),
        }
    }

    fn contains_point(&self, i: u32) -> bool {
        self.range.0 <= i && i < self.range.1
    }
}




struct State {
    time: f64,
    patches: Vec<Patch>,
}




// ============================================================================
impl State {

    fn new(mesh: &Mesh, num_blocks: u32) -> Self {
        let model = |x: f64| f64::exp(-1e2 * (x - 0.5).powi(2));

        let mut patches = Vec::new();
        let n = mesh.shape / num_blocks;

        for i in 0..num_blocks {
            let range = (i * n, (i + 1) * n);
            let patch = Patch::from_model(model, range, &mesh);
            patches.push(patch);
        }

        Self {
            time: 0.0,
            patches
        }
    }

    fn patches_containing(&self, i: u32) -> impl Iterator<Item = &Patch> {
        self.patches.iter().filter(move |p| p.contains_point(i))
    }

    fn update(&mut self, mesh: &Mesh, dt: f64) {

        let mut extended_patches = Vec::new();

        for patch in &self.patches {
            let il = (patch.range.0 + mesh.shape - 1) % mesh.shape;
            let ir = (patch.range.1 + mesh.shape + 0) % mesh.shape;
            let pl = &self.patches_containing(il).next().unwrap().primitive;
            let pr = &self.patches_containing(ir).next().unwrap().primitive;

            let mut pe = patch.primitive.clone();
            pe.insert(0, *pl.last().unwrap());
            pe.push(pr[0]);

            extended_patches.push(pe);
        }

        for (pe, patch) in extended_patches.iter_mut().zip(self.patches.iter_mut()) {
            patch.primitive = pe[..].to_vec();
        }

        let dx = mesh.cell_spacing();

        for patch in &mut self.patches {
            let u = to_conserved(&patch.primitive[1..patch.primitive.len() - 1].to_vec());
            let f = intercell_flux(&patch.primitive);

            let u1: Vec<_> = f.windows(2).zip(&u).map(|(w, u)| {
                let fl = w[0];
                let fr = w[1];
                u - (fr - fl) * dt / dx
            }).collect();

            patch.primitive = to_primitive(&u1);
        }

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

    while state.time < 0.25 {
        println!("t={:.4}", state.time);
        state.update(&mesh, dt)
    }

    for (n, patch) in state.patches.iter().enumerate() {
        let file = std::fs::File::create(format!("solution-{}.dat", n)).unwrap();
        let x = mesh.cell_centers(patch.range);

        for (x, p) in x.iter().zip(&patch.primitive) {
            writeln!(&file, "{:+.8e} {:+.8e}", x, p).unwrap();
        }        
    }
}
