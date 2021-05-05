use core::ops::Range;

/// Identifier for a Cartesian axis
///
#[derive(Clone, Copy)]
pub enum Axis {
    I,
    J,
}

impl Axis {
    pub fn dual(&self) -> Self {
        match self {
            Self::I => Self::J,
            Self::J => Self::I,
        }
    }
}

/// Describes a rectangular index space. The index type is signed 64-bit integer.
///
#[derive(Clone, Debug)]
pub struct IndexSpace {
    di: Range<i64>,
    dj: Range<i64>,
}

impl IndexSpace {
    /// Construct a new index space from the given ranges. The ranges are
    /// allowed to be empty but this function panics if either has negative
    /// length.
    ///
    pub fn new(di: Range<i64>, dj: Range<i64>) -> Self {
        assert! {
            di.start <= di.end && dj.start < dj.end,
            "index space has negative volume"
        };
        Self { di, dj }
    }

    /// Determine whether this index space is empty.
    ///
    pub fn is_empty(&self) -> bool {
        self.di.is_empty() || self.dj.is_empty()
    }

    /// Return the number of indexes on each axis.
    ///
    pub fn dim(&self) -> (usize, usize) {
        (
            (self.di.end - self.di.start) as usize,
            (self.dj.end - self.dj.start) as usize,
        )
    }

    /// Return the number of elements in this index space.
    ///
    pub fn len(&self) -> usize {
        let (l, m) = self.dim();
        l * m
    }

    /// Return the minimum index (inclusive).
    ///
    pub fn start(&self) -> (i64, i64) {
        (self.di.start, self.dj.start)
    }

    /// Return the maximum index (exclusive).
    ///
    pub fn end(&self) -> (i64, i64) {
        (self.di.end, self.dj.end)
    }

    /// Return the index space as a rectangle reference (a tuple of `Range`
    /// references).
    ///
    pub fn to_rect_ref(&self) -> (&Range<i64>, &Range<i64>) {
        (&self.di, &self.dj)
    }

    /// Convert this index space as a rectangle (a tuple of `Range` objects).
    ///
    pub fn into_rect(self) -> (Range<i64>, Range<i64>) {
        (self.di, self.dj)
    }

    /// Determine whether this index space contains the given index.
    ///
    pub fn contains(&self, index: (i64, i64)) -> bool {
        self.di.contains(&index.0) && self.dj.contains(&index.1)
    }

    /// Determine whether another index space is a subset of this one.
    ///
    pub fn contains_space(&self, other: &Self) -> bool {
        other.di.start >= self.di.start
            && other.di.end <= self.di.end
            && other.dj.start >= self.dj.start
            && other.dj.end <= self.dj.end
    }

    /// Return the overlapping region between two index spaces.
    ///
    pub fn intersect<I: Into<Self>>(&self, other: I) -> Self {
        let other = other.into();
        let i0 = self.di.start.max(other.di.start);
        let j0 = self.dj.start.max(other.dj.start);
        let i1 = self.di.end.min(other.di.end);
        let j1 = self.dj.end.min(other.dj.end);
        Self::new(i0..i1, j0..j1)
    }

    /// Extend this index space by the given number of elements on both sides
    /// of each axis.
    ///
    pub fn extend_all(&self, delta: i64) -> Self {
        Self::new(
            self.di.start - delta..self.di.end + delta,
            self.dj.start - delta..self.dj.end + delta,
        )
    }

    /// Extend the elements at both ends of the given axis by a certain
    /// amount.
    ///
    pub fn extend(&self, delta: i64, axis: Axis) -> Self {
        match axis {
            Axis::I => Self::new(self.di.start - delta..self.di.end + delta, self.dj.clone()),
            Axis::J => Self::new(self.di.clone(), self.dj.start - delta..self.dj.end + delta),
        }
    }

    /// Extend just the lower elements of this index space by a certain amount
    /// on the given axis.
    ///
    pub fn extend_lower(&self, delta: i64, axis: Axis) -> Self {
        match axis {
            Axis::I => Self::new(self.di.start - delta..self.di.end, self.dj.clone()),
            Axis::J => Self::new(self.di.clone(), self.dj.start - delta..self.dj.end),
        }
    }

    /// Extend just the upper elements of this index space by a certain amount on
    /// the given axis.
    ///
    pub fn extend_upper(&self, delta: i64, axis: Axis) -> Self {
        match axis {
            Axis::I => Self::new(self.di.start..self.di.end + delta, self.dj.clone()),
            Axis::J => Self::new(self.di.clone(), self.dj.start..self.dj.end + delta),
        }
    }

    /// Trim this index space by the given number of elements on both sides of
    /// each axis.
    ///
    pub fn trim_all(&self, delta: i64) -> Self {
        self.extend_all(-delta)
    }

    /// Trim the elements at both ends of the given axis by a certain amount.
    ///
    pub fn trim(&self, delta: i64, axis: Axis) -> Self {
        self.extend(-delta, axis)
    }

    /// Trim just the lower elements of this index space by a certain amount on
    /// the given axis.
    ///
    pub fn trim_lower(&self, delta: i64, axis: Axis) -> Self {
        self.extend_lower(-delta, axis)
    }

    /// Trim just the upper elements of this index space by a certain amount on
    /// the given axis.
    ///
    pub fn trim_upper(&self, delta: i64, axis: Axis) -> Self {
        self.extend_upper(-delta, axis)
    }

    /// Shift this index space by some amount on the given axis. The shape is
    /// unchanged.
    ///
    pub fn translate(&self, delta: i64, axis: Axis) -> Self {
        match axis {
            Axis::I => Self::new(self.di.start + delta..self.di.end + delta, self.dj.clone()),
            Axis::J => Self::new(self.di.clone(), self.dj.start + delta..self.dj.end + delta),
        }
    }

    /// Increase the size of this index space by the given factor.
    ///
    pub fn refine_by(&self, factor: u32) -> Self {
        let factor = factor as i64;
        Self::new(
            self.di.start * factor..self.di.end * factor,
            self.dj.start * factor..self.dj.end * factor,
        )
    }

    /// Increase the size of this index space by the given factor.
    ///
    pub fn coarsen_by(&self, factor: u32) -> Self {
        let factor = factor as i64;

        assert! {
            self.di.start % factor == 0 &&
            self.dj.start % factor == 0 &&
            self.di.end % factor == 0 &&
            self.dj.end % factor == 0,
            "index space must divide the coarsening factor"
        };

        Self::new(
            self.di.start / factor..self.di.end / factor,
            self.dj.start / factor..self.dj.end / factor,
        )
    }

    /// Return the linear offset for the given index, in a row-major memory
    /// buffer aligned with the start of this index space.
    ///
    pub fn row_major_offset(&self, index: (i64, i64)) -> usize {
        let i = (index.0 - self.di.start) as usize;
        let j = (index.1 - self.dj.start) as usize;
        let m = (self.dj.end - self.dj.start) as usize;
        i * m + j
    }

    /// Return a memory region object for a buffer mapped to this index space.
    ///
    pub fn memory_region(&self) -> MemoryRegion {
        let start = (0, 0);
        let count = self.dim();
        let shape = self.dim();
        MemoryRegion {
            start,
            count,
            shape,
        }
    }

    /// Return a memory region object corresponding to the selection of this
    /// index space in the buffer allocated for another one.
    ///
    pub fn memory_region_in(&self, parent: Self) -> MemoryRegion {
        let start = (
            (self.di.start - parent.di.start) as usize,
            (self.dj.start - parent.dj.start) as usize,
        );
        let count = self.dim();
        let shape = parent.dim();
        MemoryRegion {
            start,
            count,
            shape,
        }
    }

    /// Return an iterator which traverses the index space in row-major order
    /// (C-like; the final index increases fastest).
    ///
    pub fn iter(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        self.di
            .clone()
            .map(move |i| self.dj.clone().map(move |j| (i, j)))
            .flatten()
    }
}

// The impl's below enable syntactic sugar for iteration, but since the
// iterators use combinators and closures, the iterator type cannt be written
// explicitly for the `IntoIter` associated type. The
// `min_type_alias_impl_trait` feature on nightly allows the syntax below.

// ============================================================================
// impl IntoIterator for IndexSpace {
//     type Item = (i64, i64);
//     type IntoIter = impl Iterator<Item = Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         let Self { di, dj } = self;
//         di.map(move |i| dj.clone().map(move |j| (i, j))).flatten()
//     }
// }

// impl IntoIterator for &IndexSpace {
//     type Item = (i64, i64);
//     type IntoIter = impl Iterator<Item = Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.iter()
//     }
// }

impl From<(Range<i64>, Range<i64>)> for IndexSpace {
    fn from(range: (Range<i64>, Range<i64>)) -> Self {
        Self {
            di: range.0,
            dj: range.1,
        }
    }
}

impl<'a> From<(&'a Range<i64>, &'a Range<i64>)> for IndexSpace {
    fn from(range: (&'a Range<i64>, &'a Range<i64>)) -> Self {
        Self {
            di: range.0.clone(),
            dj: range.1.clone(),
        }
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
 * A 2D memory region within a contiguous buffer.
 */
#[derive(Debug)]
pub struct MemoryRegion {
    start: (usize, usize),
    count: (usize, usize),
    shape: (usize, usize),
}

impl MemoryRegion {
    pub fn iter_slice(self, slice: &[f64], chunk: usize) -> impl Iterator<Item = &'_ [f64]> {
        let Self {
            start,
            shape,
            count,
        } = self;
        let r = chunk;
        let q = shape.1 * r;

        assert!(slice.len() == shape.0 * shape.1 * chunk);

        slice[start.0 * q..(start.0 + count.0) * q]
            .chunks_exact(q)
            .flat_map(move |j| j[start.1 * r..(start.1 + count.1) * r].chunks_exact(r))
    }

    pub fn iter_slice_mut(
        self,
        slice: &mut [f64],
        chunk: usize,
    ) -> impl Iterator<Item = &'_ mut [f64]> {
        let Self {
            start,
            shape,
            count,
        } = self;
        let r = chunk;
        let q = shape.1 * r;

        assert!(slice.len() == shape.0 * shape.1 * chunk);

        slice[start.0 * q..(start.0 + count.0) * q]
            .chunks_exact_mut(q)
            .flat_map(move |j| j[start.1 * r..(start.1 + count.1) * r].chunks_exact_mut(r))
    }
}

/// This is an access pattern iterator for a 3D hyperslab selection. *Experimental*.
///
pub fn iter_slice_3d_v1(
    slice: &[f64],
    start: (usize, usize, usize),
    count: (usize, usize, usize),
    shape: (usize, usize, usize),
    chunk: usize,
) -> impl Iterator<Item = &[f64]> {
    assert!(slice.len() == shape.0 * shape.1 * shape.2 * chunk);

    slice
        .chunks_exact(shape.1 * shape.2 * chunk)
        .skip(start.0)
        .take(count.0)
        .flat_map(move |j| {
            j.chunks_exact(shape.1 * chunk)
                .skip(start.1)
                .take(count.1)
                .flat_map(move |k| k.chunks_exact(chunk).skip(start.2).take(count.2))
        })
}

/// This is an access pattern iterator for a 3D hyperslab selection,
/// equivalent to the one above but faster. Most benchmarks suggest neither is
/// faster than a triple for-loop. *Experimental*.
///
pub fn iter_slice_3d_v2(
    slice: &[f64],
    start: (usize, usize, usize),
    count: (usize, usize, usize),
    shape: (usize, usize, usize),
    chunk: usize,
) -> impl Iterator<Item = &[f64]> {
    assert!(slice.len() == shape.0 * shape.1 * shape.2 * chunk);

    let s = chunk;
    let r = shape.2 * s;
    let q = shape.1 * r;

    slice[start.0 * q..(start.0 + count.0) * q]
        .chunks_exact(q)
        .flat_map(move |j| {
            j[start.1 * r..(start.1 + count.1) * r]
                .chunks_exact(r)
                .flat_map(move |k| k[start.2 * s..(start.2 + count.2) * s].chunks_exact(s))
        })
}

/**
 * This is yet another version of the hyperslab traversal. Benchmarks suggest
 * it's the slowest. *Experimental*.
 */
pub fn iter_slice_3d_v3(
    slice: &[f64],
    start: (usize, usize, usize),
    count: (usize, usize, usize),
    shape: (usize, usize, usize),
    chunk: usize,
) -> impl Iterator<Item = &[f64]> {
    let s = chunk;
    let r = shape.2 * s;
    let q = shape.1 * r;

    (start.0..start.0 + count.0).flat_map(move |i| {
        (start.1..start.1 + count.1).flat_map(move |j| {
            (start.2..start.2 + count.2).map(move |k| {
                let n = i * q + j * r + k * s;
                &slice[n..n + chunk]
            })
        })
    })
}

// ============================================================================
#[cfg(test)]
mod test {

    const NI: usize = 100;
    const NJ: usize = 100;
    const NK: usize = 100;
    const NUM_FIELDS: usize = 5;

    #[test]
    fn traversal_with_nested_iter_has_correct_length_v1() {
        let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
        assert_eq!(
            super::iter_slice_3d_v1(&data, (5, 10, 15), (10, 10, 10), (NI, NJ, NK), NUM_FIELDS)
                .count(),
            1000
        );
    }

    #[test]
    fn traversal_with_nested_iter_has_correct_length_v2() {
        let data = vec![1.0; NI * NJ * NK * NUM_FIELDS];
        assert_eq!(
            super::iter_slice_3d_v2(&data, (5, 10, 15), (10, 10, 10), (NI, NJ, NK), NUM_FIELDS)
                .count(),
            1000
        );
    }
}
