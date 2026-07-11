use alloc::vec::Vec;

use crate::{
    trids::tri_data_structure::TriDataStructure,
    utils::types::{Edge2, Triangle2, Vertex2},
};

mod base;
mod flips;
mod insertion;
mod locate;
mod validation;

#[cfg(all(test, feature = "logging"))]
mod pre_test;
#[cfg(all(test, any(feature = "logging", feature = "wasm")))]
mod tests;

/// Triangle, including point at infinity
pub enum TriangleExtended {
    /// Normal Triangle
    Triangle(Triangle2),
    /// Triangle with one point at infinity, i.e. a line segment
    ConceptualTriangle(Edge2),
}

#[derive(Debug)]
pub(crate) enum Flip {
    #[allow(unused)]
    OneToThree,
    TwoToTwo,
    ThreeToOne((usize, usize)), // this flip saves the index of the third triangle and the reflex vertex that is part of the reflex wedge as (third tri idx, reflex vertex idx)
}

/// A weighted 2D Delaunay Triangulation with eps-approximation.
///
/// ```
/// use rita::Triangulation;
///
/// let vertices = vec![
///     [0.0, 0.0],
///     [-0.5, 1.0],
///     [0.0, 2.5],
///     [2.0, 3.0],
///     [4.0, 2.5],
///     [5.0, 1.5],
///     [4.5, 0.5],
///     [2.5, -0.5],
///     [1.5, 1.5],
///     [3.0, 1.0],
/// ];
///
/// let mut triangulation = Triangulation::new(None); // specify epsilon here
/// let result = triangulation.insert_vertices(&vertices, None, true);  // None = unweighted; use Some(weights) with geogram for weighted
///
/// assert_eq!(triangulation.par_is_regular(false), 1.0);
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Triangulation {
    /// An artificial inverse weight to make points be considered as regular (ie. not lying in a triangles circumcircle).
    ///
    /// Even a small epsilon can make the triangulation faster.
    epsilon: Option<f64>,
    pub tds: TriDataStructure,
    pub vertices: Vec<Vertex2>,
    /// The weights of the vertices, `Some` if the vertices are weighted
    pub weights: Option<Vec<f64>>,
    last_inserted_triangle: Option<usize>,

    #[cfg(feature = "timing")]
    time_flipping: u128,
    #[cfg(feature = "timing")]
    time_inserting: u128,
    #[cfg(feature = "timing")]
    time_walking: u128,

    /// Vertices that are part of the triangulation
    /// (i.e. the input point set without redundant and ignored vertices).
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    pub used_vertices: Vec<usize>,
    /// Vertices that are not part of the triangulation, due to their weight.
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    redundant_vertices: Vec<usize>,
    /// Vertices that are not part of the triangulation, due to epsilon.
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    ignored_vertices: Vec<usize>,
}

impl Default for Triangulation {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Create a new [`Triangulation`] from vertices with optional weights and epsilon.
///
/// ## Example
/// ```
/// # use rita::triangulation;
/// triangulation!(&[[0.0, 9.9], [6.9, 12.3], [5.2, 3.33]]);
/// // with epsilon
/// triangulation!(&[[0.0, 9.9], [6.9, 12.3], [5.2, 3.33]], epsilon = 1e-9);
/// // with weights
/// triangulation!(&[[0.0, 9.9], [6.9, 12.3], [5.2, 3.33]], vec![0.2, 1.3]);
/// // with weights and epsilon
/// triangulation!(&[[0.0, 9.9], [6.9, 12.3], [5.2, 3.33]], vec![0.2, 1.3], epsilon = 1e-9);
/// ```
#[macro_export]
macro_rules! triangulation {
    ($vertices:expr) => {{
        let mut triangulation =
            $crate::Triangulation::new_with_vert_capacity(None, $vertices.len());
        let _ = triangulation.insert_vertices($vertices, None, true);
        triangulation
    }};
    ($vertices:expr, epsilon = $epsilon:expr) => {{
        let mut triangulation =
            $crate::Triangulation::new_with_vert_capacity(Some($epsilon), $vertices.len());
        let _ = triangulation.insert_vertices($vertices, None, true);
        triangulation
    }};
    // with weights
    ($vertices:expr, $weights:expr) => {{
        let mut triangulation =
            $crate::Triangulation::new_with_vert_capacity(None, $vertices.len());
        let _ = triangulation.insert_vertices($vertices, Some($weights), true);
        triangulation
    }};
    ($vertices:expr, $weights:expr, epsilon = $epsilon:expr) => {{
        let mut triangulation =
            $crate::Triangulation::new_with_vert_capacity(Some($epsilon), $vertices.len());
        let _ = triangulation.insert_vertices($vertices, Some($weights), true);
        triangulation
    }};
}

// Note: this is for cg lab
impl PartialEq for Triangulation {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }
}

impl Eq for Triangulation {}
