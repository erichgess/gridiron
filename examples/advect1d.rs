struct State {
    time: f64,
    primitive: Vec<f64>
}

fn initial_primitive(x: &Vec<f64>) -> Vec<f64> {
    x.iter().map(|x| f64::exp(-1e2 * (x - 0.5).powi(2))).collect()
}

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
    let mut result = vec![0.0; primitive.len() + 1];

    for i in 0..result.len() {

        let il = (i + primitive.len() - 1) % primitive.len();
        let ir = (i + primitive.len() + 0) % primitive.len();
        let fl = flux(primitive[il]);
        let fr = flux(primitive[ir]);

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

fn update(state: State, dx: f64, dt: f64) -> State {
    let u = to_conserved(&state.primitive);
    let f = intercell_flux(&state.primitive);

    let u1: Vec<_> = f.windows(2).zip(u).map(|(w, u)| {
        let fl = w[0];
        let fr = w[1];
        u - (fr - fl) * dt / dx
    }).collect();
    let p1 = to_primitive(&u1);

    State {
        time: state.time + dt,
        primitive: p1,
    }
}

fn main() {
    let num_cells = 1000;
    let dx = 1.0 / num_cells as f64;
    let x: Vec<f64> = (0..num_cells).map(|i| i as f64 * dx).collect();

    let mut state = State {
        time: 0.0,
        primitive: initial_primitive(&x),
    };

    let dt = dx * 0.5;

    while state.time < 1.25 {
        state = update(state, dx, dt);
        println!("t = {:.4}", state.time);
    }

    use std::io::Write;
    let file = std::fs::File::create("solution.dat").unwrap();

    for (x, p) in x.iter().zip(state.primitive) {
        writeln!(&file, "{:+.8e} {:+.8e}", x, p).unwrap();
    }
}
