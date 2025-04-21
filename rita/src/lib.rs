//! # rita
//!
//! An implementation of 2D and 3D weighted delaunay triangulation via incremental algorithms.
#![cfg_attr(not(feature = "std"), no_std)]

#![forbid(unsafe_code)]
#![deny(unused, clippy::incompatible_msrv)]
#![warn(clippy::all, clippy::missing_const_for_fn)]

extern crate alloc;

pub use node::VertexNode;
pub use tetrahedralization::Tetrahedralization;
pub use triangulation::Triangulation;

pub mod node;
mod tetds;
pub mod tetrahedralization;
pub mod triangulation;
mod trids;
mod utils;
