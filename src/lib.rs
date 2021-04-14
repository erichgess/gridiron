//! Gridiron is an adaptive mesh refinement (AMR) library for solving
//! time-dependent systems of conservation laws, like the Euler equations of
//! gas dynamics. It uses structured, rectilinear grid patches in the style of
//! Berger-Oliger AMR, where grid patches can be placed at different
//! refinement levels in flexible configurations: patches may overlap one
//! another and fine patches may cross coarse patch boundaries. This is in
//! contrast to more constrained quad-tree / oct-tree mesh topologies used
//! e.g. in the Flash code.

pub mod adjacency_list;
pub mod aug_node;
pub mod automaton;
pub mod hydro;
pub mod index_space;
pub mod interval_map;
pub mod interval_set;
pub mod meshing;
pub mod num_vec;
pub mod overlap;
pub mod patch;
pub mod rect_map;
pub mod solvers;
