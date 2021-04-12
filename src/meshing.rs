use crate::adjacency_list::AdjacencyList;
use crate::index_space::IndexSpace;
use crate::patch::Patch;
use crate::rect_map::{Rectangle, RectangleMap};

/// A trait for a container that can respond to queries for a patch overlying
/// a point.
pub trait PatchQuery {
    /// Return a patch containing the given point, if one exists.
    fn patch_containing_point(&self, point: (i64, i64)) -> Option<&Patch>;
}

impl PatchQuery for Vec<Patch> {
    fn patch_containing_point(&self, point: (i64, i64)) -> Option<&Patch> {
        self.iter()
            .filter(|p| p.high_resolution_space().contains(point))
            .next()
    }
}

impl PatchQuery for RectangleMap<i64, Patch> {
    fn patch_containing_point(&self, point: (i64, i64)) -> Option<&Patch> {
        self.query_point(point).next().map(|(_, p)| p)
    }
}

/// Extend a patch given a container of neighbor patches which implements
/// `PatchQuery`. The callback function `extend` transforms the argument
/// patch's index space to an extended space. The other callback function
/// `boundary_value` is invoked when no patch overlies the sampling point.
/// __WARNING__: this function is currently implemented only for patches at
/// uniform refinement level.
pub fn extend_patch<P, F, G>(patch: &Patch, extend: F, boundary_value: G, neighbors: &P) -> Patch
where
    P: PatchQuery,
    F: Fn(&IndexSpace) -> IndexSpace,
    G: Fn((i64, i64), &mut [f64]),
{
    let space = patch.index_space();
    let extended = extend(&space);

    let sample = |index, slice: &mut [f64]| {
        if patch.index_space().contains(index) {
            slice.clone_from_slice(patch.get_slice(index))
        } else if let Some(neigh) = neighbors.patch_containing_point(index) {
            slice.clone_from_slice(neigh.get_slice(index))
        } else {
            boundary_value(index, slice)
        }
    };
    Patch::from_slice_function(patch.level(), extended, patch.num_fields(), sample)
}

/// A trait for a container that can yield an adjacency list. It means the
/// container items are (or can be) related to one another in one more ways to
/// yield a topology. The obvious use case is for a `RectangleMap` of patches,
/// where adjacency can mean that an edge should point from patch `A` to patch
/// `B` when `f(A)` intersects `B`, where `f` is a function to map rectangles
/// in some way.
pub trait GraphTopology {
    /// The type of key used to identify vertices
    type Key;

    type Parameter;

    /// Return an adjacency list derived from this container.
    fn adjacency_list(&self, parameter: Self::Parameter) -> AdjacencyList<Self::Key>;
}

impl GraphTopology for RectangleMap<i64, Patch> {
    type Key = (Rectangle<i64>, u32);

    type Parameter = i64;

    fn adjacency_list(&self, num_guard: Self::Parameter) -> AdjacencyList<Self::Key> {
        let mut edges = AdjacencyList::new();

        for (b, q) in self.iter() {
            for (a, p) in self.query_rect(q.index_space().extend_all(num_guard)) {
                if a != b {
                    let a = (IndexSpace::from(a).into(), p.level());
                    let b = (IndexSpace::from(b).into(), q.level());
                    edges.insert(a, b)
                }
            }
        }
        edges
    }
}
