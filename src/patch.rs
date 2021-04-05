use std::cmp::Ordering::*;
use crate::rect_map::Rectangle;
use crate::index_space::IndexSpace;




/**
 * Identifies the part of the mesh where patch data resides. An `n`-dimensional
 * cartesian array has `n` of these parameters, one per axis. `Cell` regions are
 * the spaces between `Node` points. For example in a 3D array, the tuple
 * `(Cell, Cell, Cell)` refers to 3D cell volumes, `(Node, Cell, Cell)` refers
 * to the 2D, `i`-directed cell faces, and `(Node, Node, Cell)` refers to the 1D
 * `k`-directed cell edges. (`Node`, `Node`, `Node`) are the point-like vertices
 * of the dual mesh.
 *
 * Different mesh locations have different sampling policies. For example,
 * sampling data in 3D finite volumes at a coarser granularity level involves
 * averaging over 8 smaller volumes, whereas down-sampling the data residing on
 * 2D faces involves averaging over 4 smaller faces and down-sampling edge-like
 * data averages over 2 smaller edges. Data can only be sampled up and down on
 * its cell-like axes. For example, up-sampling a 3D array of faces only splits
 * each "window" into four smaller ones; it does not add new "panes" between the
 * existing ones.
 *
 * A patch's index space is the same regardless of the mesh location: it always
 * refers to the patch's index extent on the primary grid. However the array
 * size is one larger on the node-like axes.
 *
 * The flux correction on a patch P at level n procedes by identifying all
 * patches which overlap P at a higher granularity, and sampling those patches
 * at level n wherever they intersect P.
 */
pub enum MeshLocation {
    Cell,
    Node,
}




/**
 * A patch is a mapping from a rectangular subset of a high-resolution index
 * space (HRIS), to associated field values. The mapping is backed by an array
 * of data, which is in general at a coarser level of granularity than the HRIS;
 * each cell in the backing array stands for (2^level)^rank cells in the HRIS.
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
#[derive(serde::Serialize)]
pub struct Patch {

    /// The granularity level of this patch. Level 0 is the highest
    /// resolution.
    level: u32,

    /// The region of index space covered by this patch. The indexes are with
    /// respect to the ticks at this patch's granularity level.
    area: Rectangle<i64>,

    /// The number of fields stored at each zone.
    num_fields: usize,

    /// The backing array of data on this patch.
    data: Vec<f64>,
}




// ============================================================================
impl Patch {




    /**
     * Generate a patch at a given level, covering the given space, with values
     * defined from a closure.
     */
    pub fn from_scalar_function<I, F>(level: u32, space: I, f: F) -> Self
    where
        I: Into<IndexSpace>,
        F: Copy + Fn((i64, i64)) -> f64
    {
        Self::from_vector_function(level, space, |index| [f(index)])
    }




    /**
     * Generate a patch at a given level, covering the given space, with values
     * defined from a closure which returns a fixed-length array. The number of
     * fields in the patch is inferred from the size of the fixed length array
     * returned by the closure.
     */
    pub fn from_vector_function<I, F, const NUM_FIELDS: usize>(level: u32, space: I, f: F) -> Self
    where
        I: Into<IndexSpace>,
        F: Copy + Fn((i64, i64)) -> [f64; NUM_FIELDS]
    {
        let space: IndexSpace = space.into();
        let mut data = Vec::with_capacity(space.len() * NUM_FIELDS);

        for index in space.iter() {
            data.extend(f(index).iter());
        }
        Self {
            level,
            data,
            area: space.into(),
            num_fields: NUM_FIELDS,
        }
    }




    /**
     * Return the index space of this patch.
     */
    pub fn index_space(&self) -> IndexSpace {
        self.area.clone().into()
    }




    /**
     * Return the index space at the high-resolution level below this patch.
     */
    pub fn high_resolution_space(&self) -> IndexSpace {
        self.index_space().scale(1 << self.level)
    }




    /**
     * Sample the field at the given level and index. The index measures
     * ticks at the target sampling level, not the HRIS.
     */
    pub fn sample(&self, level: u32, index: (i64, i64), field: usize) -> f64 {
        match level.cmp(&self.level) {
            Equal => {
                self.validate_index(index, field);

                let (i0, j0) = self.index_space().start();
                let i = (index.0 - i0) as usize;
                let j = (index.1 - j0) as usize;

                let (_m, n) = self.index_space().dim();
                self.data[(i * n + j) * self.num_fields + field]
            }
            Less => {
                self.sample(level + 1, (index.0 / 2, index.1 / 2), field)
            }
            Greater => {
                let y00 = self.sample(level - 1, (index.0 * 2, index.1 * 2), field);
                let y01 = self.sample(level - 1, (index.0 * 2, index.1 * 2 + 1), field);
                let y10 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2), field);
                let y11 = self.sample(level - 1, (index.0 * 2 + 1, index.1 * 2 + 1), field);
                0.25 * (y00 + y01 + y10 + y11)
            }
        }
    }




    pub fn sample_vector<const NUM_FIELDS: usize>(&self, level: u32, index: (i64, i64)) -> [f64; NUM_FIELDS] {
        let mut result = [0.0; NUM_FIELDS];
        self.sample_slice(level, index, &mut result);
        result
    }




    pub fn sample_slice(&self, level: u32, index: (i64, i64), result: &mut [f64]) {
        for field in 0..self.num_fields {
            result[field] = self.sample(level, index, field)
        }
    }




    fn validate_index(&self, index: (i64, i64), field: usize) {
        let space = self.index_space();

        assert!(
            field < self.num_fields,
            "field index {} out of range on patch with {} fields",
            field,
            self.num_fields);

        assert!(
            space.contains(index),
            "index ({} {}) out of range on patch ({}..{} {}..{})",
            index.0,
            index.1,
            space.start().0,
            space.end().0,
            space.start().1,
            space.end().1);
    }
}




// ============================================================================
#[cfg(test)]
mod test {

    use crate::index_space::{
        IndexSpace,
        range2d};
    use crate::rect_map::{
        Rectangle,
        RectangleRef,
        RectangleMap};
    use super::Patch;


    // ============================================================================
    fn finest_patch<'a>(map: &'a RectangleMap<i64, &'a Patch>, index: (i64, i64)) -> Option<&'a Patch> {
        map.query_point(index)
           .map(|(_, &p)| p)
           .min_by_key(|p| p.level)
    }


    fn extend_patch(map: &RectangleMap<i64, Patch>, rect: RectangleRef<i64>) -> (Rectangle<i64>, Patch) {

        let space: IndexSpace = rect.into();
        let extended = space.extend_all(2);
        let local_map: RectangleMap<_, _> = map.query_rect(extended.clone()).collect();
        let p = local_map.get(rect).unwrap();

        let sample = |index| {
            if p.index_space().contains(index) {
                p.sample(p.level, index, 0)
            } else if let Some(n) = finest_patch(&local_map, index) {
                n.sample(p.level, index, 0)
            } else {
                0.0
            }
        };
        (extended.clone().into(), Patch::from_scalar_function(p.level, extended, sample))
    }


    #[test]
    fn patch_sampling_works() {
        let patch = Patch::from_scalar_function(1, (4..10, 4..10), |(i, j)| i as f64 + j as f64);
        assert_eq!(patch.sample(1, (5, 5), 0), 10.0);
        assert_eq!(patch.sample(1, (6, 8), 0), 14.0);
        assert_eq!(patch.sample(2, (2, 2), 0), 0.25 * (8.0 + 9.0 + 9.0 + 10.0));

        // Piecewise constant sampling
        assert_eq!(patch.sample(0, (8, 8), 0), 8.0);
        assert_eq!(patch.sample(0, (9, 8), 0), 8.0);
        assert_eq!(patch.sample(0, (8, 9), 0), 8.0);
        assert_eq!(patch.sample(0, (9, 9), 0), 8.0);
        assert_eq!(patch.sample(0, (10, 10), 0), 10.0);
    }


    #[test]
    fn can_extend_patch() {

        let mut quilt = RectangleMap::new();

        for (i, j) in range2d(0..4, 0..4) {
            let area = (i * 10 .. (i + 1) * 10, j * 10 .. (j + 1) * 10);
            let patch = Patch::from_scalar_function(0, area, |ij| ij.0 as f64 + ij.1 as f64);
            quilt.insert(patch.high_resolution_space(), patch);
        }

        let extended_quilt: RectangleMap<i64, Patch> = quilt
            .keys()
            .map(|rect| extend_patch(&quilt, rect))
            .collect();

        let p12 = extended_quilt.get((&(10 - 2..20 + 2), &(20 - 2..30 + 2))).unwrap();
        let p21 = extended_quilt.get((&(20 - 2..30 + 2), &(10 - 2..20 + 2))).unwrap();

        assert_eq!(p12.sample(0, (20, 20), 0), p21.sample(0, (20, 20), 0));
    }
}
