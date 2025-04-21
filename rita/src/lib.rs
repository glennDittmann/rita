//! # rita
//!
//! An implementation of 2D and 3D weighted delaunay triangulation via incremental algorithms.
#![forbid(unsafe_code)]
#![deny(unused, clippy::incompatible_msrv)]
#![warn(clippy::all, clippy::missing_const_for_fn)]

pub use node::VertexNode;
pub use tetrahedralization::Tetrahedralization;
pub use triangulation::Triangulation;

pub mod node;
mod tetds;
pub mod tetrahedralization;
pub mod triangulation;
mod trids;
mod utils;
