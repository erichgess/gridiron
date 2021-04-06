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
     * Return the linear offset for te given index, in a row-major memory buffer
     * aligned with the start of this index space. 
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
