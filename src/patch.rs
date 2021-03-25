use std::ops::Range;




/**
 * A patch is a mapping from a rectangular subset of a high-resolution index
 * space (HRIS), to associated field values. The mapping is backed by an array of
 * data, which is in general at a coarser level of granularity than the HRIS;
 * each zone in the backing array stands for (2^level)^rank zones in the HRIS.
 * The HRIS is at level 0.
 *
 * The patch can be sampled at different granularity levels. If the sampling
 * level is finer than the patch granularity, then sub-cell sampling is
 * employed, either with piecewise constant or multi-linear interpolation. If
 * the sampling level is coarser than the patch granularity, then the result is
 * obtained by averaging over the region within the patch covered by the coarse
 * cell.  
 */
pub struct Patch {
    level: u32,
    area: (Range<i64>, Range<i64>),
    data: Vec<f64>,
}




impl Patch {




    /**
     * Generate a patch at a given level, covering the given area, with values
     * defined from a closure.
     */
    pub fn from_function<F>(level: u32, area: (Range<i64>, Range<i64>), f: F) -> Self
    where
        F: Copy + Fn(i64, i64) -> f64
    {
        let (di, dj) = area.clone();
        Self {
            level,
            area,
            data: di.map(|i| dj.clone().map(move |j| f(i, j))).flatten().collect()
        }
    }


    pub fn from_function_n<F, const NUM_FIELDS: usize>(level: u32, area: (Range<i64>, Range<i64>), f: F) -> Self
    where
        F: Copy + Fn(i64, i64) -> [f64; NUM_FIELDS]
    {
        let (di, dj) = area.clone();
        Self {
            level,
            area,
            data: di.map(|i| dj.clone().map(move |j| f(i, j)[0])).flatten().collect()
        }
    }




    /**
     * Return the number of HRIS ticks covered by this
     */
    pub fn high_resolution_area(&self) -> (Range<i64>, Range<i64>) {
        let i0 = self.area.0.start * (1 << self.level);
        let j0 = self.area.1.start * (1 << self.level);
        let i1 = self.area.0.end * (1 << self.level);
        let j1 = self.area.1.end * (1 << self.level);
        (i0..i1, j0..j1)
    }




    /**
     * Return the logical dimensions (the memory extent) of the backing array.
     */
    pub fn dim(&self) -> (usize, usize) {
        ((self.area.0.end - self.area.0.start) as usize,
         (self.area.1.end - self.area.1.start) as usize)
    }




    /**
     * Sample the field at the given level and index. The index measures
     * ticks at the target sampling level, not the HRIS.
     */
    pub fn sample(&self, level: u32, index: (i64, i64)) -> f64 {

        if level == self.level {
            self.validate_index(index);

            let i = (index.0 - self.area.0.start) as usize;
            let j = (index.1 - self.area.1.start) as usize;
            let (_m, n) = self.dim();
            self.data[i * n + j]

        } else if level < self.level {
            self.sample(level + 1, (index.0 / 2, index.1 / 2))

        } else {
            let y00 = self.sample(level - 1, (index.0 * 2 + 0, index.1 * 2 + 0));
            let y01 = self.sample(level - 1, (index.0 * 2 + 0, index.1 * 2 + 1));
            let y10 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2 + 0));
            let y11 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2 + 1));
            0.25 * (y00 + y01 + y10 + y11)
        }
    }

    fn validate_index(&self, index: (i64, i64)) {
        if !(self.area.0.contains(&index.0) && self.area.1.contains(&index.1)) {
            panic!("index ({} {}) out of range on patch ({}..{} {}..{})",
                index.0,
                index.1,
                self.area.0.start,
                self.area.0.end,
                self.area.1.start,
                self.area.1.start);
        }
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use super::Patch;

    #[test]
    fn patch_sampling_works() {
        let patch = Patch::from_function(1, (4..10, 4..10), |i, j| i as f64 + j as f64);
        assert_eq!(patch.sample(1, (5, 5)), 10.0);
        assert_eq!(patch.sample(1, (6, 8)), 14.0);
        assert_eq!(patch.sample(2, (2, 2)), 0.25 * (8.0 + 9.0 + 9.0 + 10.0));

        // Piecewise constant sampling
        assert_eq!(patch.sample(0, (8, 8)), 8.0);
        assert_eq!(patch.sample(0, (9, 8)), 8.0);
        assert_eq!(patch.sample(0, (8, 9)), 8.0);
        assert_eq!(patch.sample(0, (9, 9)), 8.0);
        assert_eq!(patch.sample(0, (10, 10)), 10.0);
    }
}
