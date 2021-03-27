use std::cmp::Ordering::*;
use crate::index_space::IndexSpace2d;




/**
 * A patch is a mapping from a rectangular subset of a high-resolution index
 * space (HRIS), to associated field values. The mapping is backed by an array
 * of data, which is in general at a coarser level of granularity than the HRIS;
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
#[derive(Clone)]
pub struct Patch {

    /// The granularity level of this patch. Level 0 is the highest resolution
    level: u32,

    /// The region of index space covered by this patch. The indexes are with
    /// respect to the ticks at this patch's granularity level.
    space: IndexSpace2d,

    /// The array backing for the data on this patch.
    data: Vec<f64>,
}




impl Patch {




    /**
     * Generate a patch at a given level, covering the given space, with values
     * defined from a closure.
     */
    pub fn from_function<I, F>(level: u32, space: I, f: F) -> Self
    where
        I: Into<IndexSpace2d>,
        F: Copy + Fn((i64, i64)) -> f64
    {
        let space: IndexSpace2d = space.into();
        Self {
            level,
            data: space.iter().map(f).collect(),
            space,
        }
    }




    /**
     * Return the index space at the high-resolution level below this patch.
     */
    pub fn high_resolution_space(&self) -> IndexSpace2d {
        self.space.scale(1 << self.level)
    }




    /**
     * Sample the field at the given level and index. The index measures
     * ticks at the target sampling level, not the HRIS.
     */
    pub fn sample(&self, level: u32, index: (i64, i64)) -> f64 {

        match level.cmp(&self.level) {
            Equal => {
                self.validate_index(index);

                let (i0, j0) = self.space.start();
                let i = (index.0 - i0) as usize;
                let j = (index.1 - j0) as usize;

                let (_m, n) = self.space.dim();
                self.data[i * n + j]
            }
            Less => {
                self.sample(level + 1, (index.0 / 2, index.1 / 2))
            }
            Greater => {
                let y00 = self.sample(level - 1, (index.0 * 2, index.1 * 2));
                let y01 = self.sample(level - 1, (index.0 * 2, index.1 * 2 + 1));
                let y10 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2));
                let y11 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2 + 1));
                0.25 * (y00 + y01 + y10 + y11)
            }
        }
    }





    fn validate_index(&self, index: (i64, i64)) {
        if !self.space.contains(index) {
            panic!("index ({} {}) out of range on patch ({}..{} {}..{})",
                index.0,
                index.1,
                self.space.start().0,
                self.space.end().0,
                self.space.start().1,
                self.space.end().1);
        }
    }
}




use crate::rect_map::RectangleMap;
use crate::rect_map::RectangleRef;

pub fn finest_patch(map: &RectangleMap<i64, Patch>, index: (i64, i64)) -> Option<&Patch> {
    map.query_point(index)
       .map(|(_, p)| p)
       .min_by_key(|p| p.level)
}

pub fn extend_patch(map: &RectangleMap<i64, Patch>, rect: RectangleRef<i64>) -> Patch {

    let space: IndexSpace2d = rect.into();
    let local_map: RectangleMap<_, _> = map.query_rect(space.extend_all(2)).collect();
    let p = local_map.get(rect).unwrap();

    let sample = |index| {
        if p.space.contains(index) {
            p.sample(p.level, index)
        } else if let Some(n) = finest_patch(map, index) {
            n.sample(p.level, index)
        } else {
            0.0
        }
    };
    Patch::from_function(p.level, space.extend_all(2), sample)
}




// ============================================================================
#[cfg(test)]
mod test {

    use std::ops::Range;
    use crate::index_space::IndexSpace2d;
    use crate::rect_map::RectangleMap;
    use super::{Patch, extend_patch};


    fn range2d(di: Range<i64>, dj: Range<i64>) -> IndexSpace2d {
        IndexSpace2d::new(di, dj)
    }


    #[test]
    fn patch_sampling_works() {
        let patch = Patch::from_function(1, (4..10, 4..10), |(i, j)| i as f64 + j as f64);
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


    #[test]
    fn can_extend_patch() {

        let mut quilt = RectangleMap::new();

        for (i, j) in range2d(0..4, 0..4) {
            let area = (i * 10 .. (i + 1) * 10, j * 10 .. (j + 1) * 10);
            let patch = Patch::from_function(0, area, |ij| ij.0 as f64 + ij.1 as f64);
            quilt.insert(patch.high_resolution_space(), patch);
        }

        for (rect, _) in &quilt {
            let p = extend_patch(&quilt, rect);
            assert_eq!(p.space.dim(), (14, 14));
        }

        assert_eq!(quilt.query_point((40, 40)).count(), 0);
    }
}
