#![feature(test)]
extern crate test;

use gridiron::index_space::{
    iter_slice_3d_v1,
    iter_slice_3d_v2,
    iter_slice_3d_v3,
    range2d};

const NI: usize = 50;
const NJ: usize = 50;
const NK: usize = 50;
const NUM_FIELDS: usize = 5;




// ============================================================================
#[bench]
fn traversal_with_linear_iteration(b: &mut test::Bencher) {

    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];

    b.iter(|| {
        let mut total = [0.0; 5];
        for x in data.chunks_exact(NUM_FIELDS) {
            for s in 0..NUM_FIELDS {
                total[s] += x[s]
            }
        }
        assert_eq!(total[0], (NI * NJ * NK) as f64);
    });
}




// ============================================================================
#[bench]
fn traversal_with_linear_iteration_closure(b: &mut test::Bencher) {

    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    b.iter(|| {
        let mut total = [0.0; 5];
        let op = |x: &[f64]| {
            for s in 0..NUM_FIELDS {
                total[s] += x[s]
            }
        };
        data.chunks_exact(NUM_FIELDS).for_each(op);
        assert_eq!(total[0], (NI * NJ * NK) as f64);
    });
}




// ============================================================================
#[bench]
fn traversal_with_triple_for_loop(b: &mut test::Bencher) {

    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    b.iter(|| {
        let mut total = [0.0; 5];
        let mut op = |x: &[f64]| {
            for s in 0..NUM_FIELDS {
                total[s] += x[s]
            }
        };
        for i in 0..NI {
            for j in 0..NJ {
                for k in 0..NK {
                    let n = ((i * NJ + j) * NK + k) * NUM_FIELDS;
                    op(&data[n .. n + NUM_FIELDS]);
                }
            }
        }
        assert_eq!(total[0], (NI * NJ * NK) as f64);
    });
}




// ============================================================================
#[bench]
fn traversal_with_nested_iter_v1(b: &mut test::Bencher) {

    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    b.iter(|| {
        let mut total = [0.0; 5];
        let op = |x: &[f64]| {
            for s in 0..NUM_FIELDS {
                total[s] += x[s]
            }
        };
        iter_slice_3d_v1(&data, (0, 0, 0), (NI, NJ, NK), (NI, NJ, NK), NUM_FIELDS).for_each(op);
        assert_eq!(total[0], (NI * NJ * NK) as f64);
    });
}




// ============================================================================
#[bench]
fn traversal_with_nested_iter_v2(b: &mut test::Bencher) {

    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    b.iter(|| {
        let mut total = [0.0; 5];
        let op = |x: &[f64]| {
            for s in 0..NUM_FIELDS {
                total[s] += x[s]
            }
        };
        iter_slice_3d_v2(&data, (0, 0, 0), (NI, NJ, NK), (NI, NJ, NK), NUM_FIELDS).for_each(op);
        assert_eq!(total[0], (NI * NJ * NK) as f64);
    });
}




// ============================================================================
#[bench]
fn traversal_with_nested_iter_v3(b: &mut test::Bencher) {

    let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    
    b.iter(|| {
        let mut total = [0.0; 5];
        let op = |x: &[f64]| {
            for s in 0..NUM_FIELDS {
                total[s] += x[s]
            }
        };
        iter_slice_3d_v3(&data, (0, 0, 0), (NI, NJ, NK), (NI, NJ, NK), NUM_FIELDS).for_each(op);
        assert_eq!(total[0], (NI * NJ * NK) as f64);
    });
}




// ============================================================================
#[bench]
fn index_traversal_with_index_space(b: &mut test::Bencher) {
    b.iter(|| {
        let mut total = 0.0;
        for _ in range2d(0..200, 0..200).iter() {
            total += 1.0
        }
        assert_eq!(total, 40_000.0 as f64);
    });
}




// ============================================================================
#[bench]
fn index_traversal_with_for_loop(b: &mut test::Bencher) {
    b.iter(|| {
        let mut total = 0.0;
        for _ in 0..200 {
            for _ in 0..200 {
                total += 1.0
            }
        }
        assert_eq!(total, 40_000.0 as f64);
    });
}
