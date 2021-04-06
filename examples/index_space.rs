use gridiron::index_space::iter_slice_3d_v1;
use gridiron::index_space::iter_slice_3d_v2;

const NI: usize = 100;
const NJ: usize = 100;
const NK: usize = 100;
const NUM_FIELDS: usize = 5;
const NUM_LOOPS: usize = 500;




// ============================================================================
fn traversal_with_linear_iteration() {
    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    let mut total = [0.0; 5];
    for x in data.chunks_exact(NUM_FIELDS) {
        for s in 0..NUM_FIELDS {
            total[s] += x[s]
        }
    }
    assert_eq!(total[0], (NI * NJ * NK) as f64);
}




// ============================================================================
fn traversal_with_triple_for_loop() {
    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    let mut total = [0.0; 5];
    for i in 0..NI {
        for j in 0..NJ {
            for k in 0..NK {
                let n = ((i * NJ + j) * NK + k) * NUM_FIELDS;
                for s in 0..NUM_FIELDS {
                    total[s] += data[n + s];
                }
            }
        }
    }
    assert_eq!(total[0], (NI * NJ * NK) as f64);
}




// ============================================================================
fn traversal_with_nested_iter_v1() {
    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    let mut total = [0.0; 5];
    for x in iter_slice_3d_v1(&data, (0, 0, 0), (NI, NJ, NK), (NI, NJ, NK), NUM_FIELDS) {
        for s in 0..NUM_FIELDS {
            total[s] += x[s];
        }
    }
    assert_eq!(total[0], (NI * NJ * NK) as f64);
}




// ============================================================================
fn traversal_with_nested_iter_v2() {
    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    let mut total = [0.0; 5];
    for x in iter_slice_3d_v2(&data, (0, 0, 0), (NI, NJ, NK), (NI, NJ, NK), NUM_FIELDS) {
        for s in 0..NUM_FIELDS {
            total[s] += x[s];
        }
    }
    assert_eq!(total[0], (NI * NJ * NK) as f64);
}




// ============================================================================
fn main() {

    let mut total = 0.0;
    for _ in 0..NUM_LOOPS {
        let start = std::time::Instant::now();
        traversal_with_linear_iteration();
        total += start.elapsed().as_secs_f64();
    }
    println!("traversal_with_linear_iteration ... {:.4}s", total / NUM_LOOPS as f64);

    let mut total = 0.0;
    for _ in 0..NUM_LOOPS {
        let start = std::time::Instant::now();
        traversal_with_triple_for_loop();
        total += start.elapsed().as_secs_f64();
    }
    println!("traversal_with_triple_for_loop .... {:.4}s", total / NUM_LOOPS as f64);

    let mut total = 0.0;
    for _ in 0..NUM_LOOPS {
        let start = std::time::Instant::now();
        traversal_with_nested_iter_v1();
        total += start.elapsed().as_secs_f64();
    }
    println!("traversal_with_nested_iter_v1 ..... {:.4}s", total / NUM_LOOPS as f64);

    let mut total = 0.0;
    for _ in 0..NUM_LOOPS {
        let start = std::time::Instant::now();
        traversal_with_nested_iter_v2();
        total += start.elapsed().as_secs_f64();
    }
    println!("traversal_with_nested_iter_v2 ..... {:.4}s", total / NUM_LOOPS as f64);
}
