use core::ops::Range;




/**
 * Identifier for a Cartesian axis
 */
pub enum Axis {
    I,
    J,
}




#[derive(Clone, Debug)]


/**
 * Represents a rectangular region in a discrete index space
 */
pub struct IndexSpace {
    di: Range<i64>,
    dj: Range<i64>,
}




/**
 * Describes a rectangular index space. The index type is signed 64-bit integer.
 */
impl IndexSpace {


    pub fn new(di: Range<i64>, dj: Range<i64>) -> Self {

        assert!(
            di.start <= di.end && dj.start < dj.end,
            "index space has negative volume");

        Self { di, dj }
    }


    /**
     * Return the number of indexes on each axis.
     */
    pub fn dim(&self) -> (usize, usize) {
        ((self.di.end - self.di.start) as usize,
         (self.dj.end - self.dj.start) as usize)
    }


    /**
     * Return the number of elements in this index space.
     */
    pub fn len(&self) -> usize {
        let (l, m) = self.dim();
        l * m
    }


    /**
     * Return the minimum index (inclusive).
     */
    pub fn start(&self) -> (i64, i64) {
        (self.di.start, self.dj.start)
    }


    /**
     * Return the maximum index (exclusive).
     */
    pub fn end(&self) -> (i64, i64) {
        (self.di.end, self.dj.end)
    }


    /**
     * Return the index space as a rectangle reference (a tuple of `Range`
     * references).
     */
    pub fn as_rect_ref(&self) -> (&Range<i64>, &Range<i64>) {
        (&self.di, &self.dj)
    }


    /**
     * Determine whether this index space contains the given index.
     */
    pub fn contains(&self, index: (i64, i64)) -> bool {
        self.di.contains(&index.0) && self.dj.contains(&index.1)
    }


    /**
     * Expand this index space by the given number of elements on each axis.
     */
    pub fn extend_all(&self, delta: i64) -> Self {
        Self::new(
            self.di.start - delta .. self.di.end + delta,
            self.dj.start - delta .. self.dj.end + delta)
    }


    /**
     * Trim this index space by the given number of elements on each axis.
     */
    pub fn trim_all(&self, delta: i64) -> Self {
        self.extend_all(-delta)
    }


    /**
     * Trim just the lower elements of this index space by a certain amount on
     * the given axis.
     */
    pub fn trim_lower(&self, delta: i64, axis: Axis) -> Self {
        match axis {
            Axis::I => Self::new(self.di.start + delta .. self.di.end, self.dj.clone()),
            Axis::J => Self::new(self.di.clone(), self.dj.start + delta .. self.dj.end),
        }
    }


    /**
     * Increase the size of this index space by the given factor.
     */
    pub fn scale(&self, factor: i64) -> Self {
        Self::new(
            self.di.start * factor .. self.di.end * factor,
            self.dj.start * factor .. self.dj.end * factor)
    }


    /**
     * Return the linear offset for the given index, in a row-major memory
     * buffer aligned with the start of this index space. 
     */
    pub fn row_major_offset(&self, index: (i64, i64)) -> usize {
        let i = (index.0 - self.di.start) as usize;
        let j = (index.1 - self.dj.start) as usize;
        let m = (self.dj.end - self.dj.start) as usize;
        i * m + j
    }


    /**
     * Return an iterator which traverses the index space in row-major order
     * (C-like; the final index increases fastest).
     */
    pub fn iter(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        self.di.clone().map(move |i| self.dj.clone().map(move |j| (i, j))).flatten()
    }
}




// ============================================================================
impl IntoIterator for IndexSpace {
    type Item = (i64, i64);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let Self { di, dj } = self;
        di.map(move |i| dj.clone().map(move |j| (i, j))).flatten()
    }
}

impl IntoIterator for &IndexSpace {
    type Item = (i64, i64);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl From<(Range<i64>, Range<i64>)> for IndexSpace {
    fn from(range: (Range<i64>, Range<i64>)) -> Self {
        Self { di: range.0, dj: range.1 }
    }
}

impl<'a> From<(&'a Range<i64>, &'a Range<i64>)> for IndexSpace {
    fn from(range: (&'a Range<i64>, &'a Range<i64>)) -> Self {
        Self { di: range.0.clone(), dj: range.1.clone() }
    }
}

impl From<IndexSpace> for (Range<i64>, Range<i64>) {
    fn from(space: IndexSpace) -> Self {
        (space.di, space.dj)
    }
}




/**
 * Less imposing factory function to construct an IndexSpace object.
 */
pub fn range2d(di: Range<i64>, dj: Range<i64>) -> IndexSpace {
    IndexSpace::new(di, dj)
}




/**
 * This is an access pattern iterator for a 3D hyperslab selection. It uses
 * nested chunks to achieve performance only slightly worse than a linear slice
 * traversal. It is ~25% faster than a triple-nested for loop.
 */
pub fn iter_slice_3d<'a>(
    slice: &'a [f64],
    start: (usize, usize, usize),
    count: (usize, usize, usize),
    shape: (usize, usize, usize),
    chunk: usize) -> impl Iterator<Item = &'a [f64]>
{
    assert!(slice.len() == shape.0 * shape.1 * shape.2 * chunk);

    let s = chunk;
    let r = shape.2 * s;
    let q = shape.1 * r;

    slice[start.0 * q .. (start.0 + count.0) * q]
    .chunks_exact(q).flat_map(move |j| j[start.1 * r .. (start.1 + count.1) * r]
    .chunks_exact(r).flat_map(move |k| k[start.2 * s .. (start.2 + count.2) * s]
    .chunks_exact(s)))
}




// ============================================================================
#[cfg(test)]
mod test {
    extern crate test;
    use test::Bencher;

    const NI: usize = 100;
    const NJ: usize = 100;
    const NK: usize = 100;
    const NUM_FIELDS: usize = 5;

    #[bench]
    fn traversal_with_linear_iteration(b: &mut Bencher) {
        let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
        b.iter(|| {
            let mut total = [0.0; 5];
            data.chunks_exact(NUM_FIELDS).for_each(|x| {
                for i in 0..NUM_FIELDS {
                    total[i] += x[i]
                }
            });
            assert_eq!(total[0], (NI * NJ * NK) as f64);
        });
    }

    #[bench]
    fn traversal_with_triple_for_loop(b: &mut Bencher) {
        let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
        b.iter(|| {
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
        });
    }

    #[bench]
    fn traversal_with_nested_iter(b: &mut Bencher) {
        let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
        b.iter(|| {
            let mut total = [0.0; 5];
            for x in super::iter_slice_3d(&data, (0, 0, 0), (NI, NJ, NK), (NI, NJ, NK), NUM_FIELDS) {
                for s in 0..NUM_FIELDS {
                    total[s] += x[s];
                }
            }
            assert_eq!(total[0], (NI * NJ * NK) as f64);
        });
    }

    // #[test]
    // fn traversal_with_nested_iter_has_correct_length() {
    //     let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
    //     assert_eq!(super::iter_slice_3d(&data, (5, 10, 15), (10, 10, 10), (NI, NJ, NK), NUM_FIELDS).count(), 1000);
    // }
}
