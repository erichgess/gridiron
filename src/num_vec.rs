use core::ops::{Add, Sub, Mul, Div, Index};




/**
 * A statically-sized numeric vector over a generic scalar data type T, which
 * supports arithmetic operations also supported by T.
 */
#[derive(Clone, Copy)]
pub struct Vector<T, const DIM: usize> {
    data: [T; DIM]
}




// ============================================================================
impl<T, U, V, const DIM: usize> Add<Vector<U, DIM>> for Vector<T, DIM>
where
    T: Copy + Add<U, Output = V>,
    U: Copy,
    V: Copy + Default
{
    type Output = Vector<V, DIM>;

    fn add(self, other: Vector<U, DIM>) -> Self::Output {
        let mut data = [V::default(); DIM];

        for (i, x) in data.iter_mut().enumerate() {
            *x = self[i].add(other[i])
        }
        Self::Output { data }
    }
}

impl<T, U, V, const DIM: usize> Sub<Vector<U, DIM>> for Vector<T, DIM>
where
    T: Copy + Sub<U, Output = V>,
    U: Copy,
    V: Copy + Default
{
    type Output = Vector<V, DIM>;

    fn sub(self, other: Vector<U, DIM>) -> Self::Output {
        let mut data = [V::default(); DIM];

        for (i, x) in data.iter_mut().enumerate() {
            *x = self[i].sub(other[i])
        }
        Self::Output { data }
    }
}

impl<T, U, V, const DIM: usize> Mul<U> for Vector<T, DIM>
where
    T: Copy + Mul<U, Output = V>,
    U: Copy,
    V: Copy + Default
{
    type Output = Vector<V, DIM>;

    fn mul(self, other: U) -> Self::Output {
        let mut data = [V::default(); DIM];

        for (i, x) in data.iter_mut().enumerate() {
            *x = self[i].mul(other)
        }
        Self::Output { data }
    }
}

impl<T, U, V, const DIM: usize> Div<U> for Vector<T, DIM>
where
    T: Copy + Div<U, Output = V>,
    U: Copy,
    V: Copy + Default
{
    type Output = Vector<V, DIM>;

    fn div(self, other: U) -> Self::Output {
        let mut data = [V::default(); DIM];

        for (i, x) in data.iter_mut().enumerate() {
            *x = self[i].div(other)
        }
        Self::Output { data }
    }
}




// ============================================================================
impl<T, const DIM: usize> Index<usize> for Vector<T, DIM> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}




// ============================================================================
#[cfg(test)]
mod test {
    extern crate test;
    use test::Bencher;
    use super::Vector;

    const COUNT: usize = 160000;

    #[bench]
    fn bench_add_raw_floats_in_vec(b: &mut Bencher) {
        b.iter(|| {
            let x: Vec<_> = (0..COUNT).map(|_| 1.0).collect();
            let y: Vec<_> = (0..COUNT).map(|_| 1.0).collect();
            let _: Vec<_> = x.into_iter().zip(y).map(|(x, y)| x + y).collect();
        })
    }

    #[bench]
    fn bench_add_numeric_vectors4_floats_in_vec(b: &mut Bencher) {
        b.iter(|| {
            let x: Vec<_> = (0..COUNT/4).map(|_| Vector { data: [0.0, 1.0, 2.0, 3.0] }).collect();
            let y: Vec<_> = (0..COUNT/4).map(|_| Vector { data: [0.0, 1.0, 2.0, 3.0] }).collect();
            let _: Vec<_> = x.into_iter().zip(y).map(|(x, y)| x + y).collect();
        })
    }

    #[bench]
    fn bench_add_numeric_vectors8_floats_in_vec(b: &mut Bencher) {
        b.iter(|| {
            let x: Vec<_> = (0..COUNT/8).map(|_| Vector { data: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0] }).collect();
            let y: Vec<_> = (0..COUNT/8).map(|_| Vector { data: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0] }).collect();
            let _: Vec<_> = x.into_iter().zip(y).map(|(x, y)| x + y).collect();
        })
    }
}
