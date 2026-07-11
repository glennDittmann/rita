use alloc::vec::Vec;

use crate::{
    tetds::tet_data_structure::TetDataStructure,
    utils::types::{Tetrahedron3, Triangle3, Vertex3},
};

mod base;
mod insertion;
mod locate;
mod validation;

#[cfg(all(test, feature = "logging"))]
mod pre_test;
#[cfg(all(test, feature = "logging"))]
mod tests;

/// Extended tetrahedron, including point at infinity
pub enum ExtendedTetrahedron {
    /// Regular tetrahedron
    Tetrahedron(Tetrahedron3),
    /// Tetrahedron with a point at infinity
    Triangle(Triangle3),
}

/// A weighted 3D Delaunay Tetrahedralization with eps-approximation.
///
/// ```
/// use rita::Tetrahedralization;
///
/// let vertices = vec![
///     [0.0, 0.0, -2.0],
///     [-0.5, 1.0, 0.5],
///     [0.0, 2.5, 2.5],
///     [2.0, 3.0, 5.0],
///     [4.0, 2.5, 9.5],
///     [1.0, 1.5, 6.5],
///     [4.5, 0.5, 5.0],
///     [2.5, -0.5, 2.0],
///     [1.5, 1.5, 3.0],
///     [3.0, 1.0, 4.0],
/// ];
///
/// let mut tetrahedralization = Tetrahedralization::new(None); // specify epsilon here
/// let result = tetrahedralization.insert_vertices(&vertices, None, true);  // None = unweighted; use Some(weights) with geogram for weighted
/// assert_eq!(tetrahedralization.par_is_regular(false), 1.0);
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Tetrahedralization {
    /// An artificial weight to make points be considered as regular (ie. not lying in a tetrahedrons circumsphere).
    ///
    /// Even a small epsilon can make the tetrahedralization faster.
    epsilon: Option<f64>,
    tds: TetDataStructure,
    vertices: Vec<Vertex3>,
    /// The weights of the vertices, `Some` if the vertices are weighted
    weights: Option<Vec<f64>>,

    #[cfg(feature = "timing")]
    pub(crate) time_hilbert: u128,
    #[cfg(feature = "timing")]
    time_walking: u128,
    #[cfg(feature = "timing")]
    time_inserting: u128,

    /// Indices of vertices that are inserted, i.e. not skipped due to epsilon
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    used_vertices: Vec<usize>,
    /// Indices of vertices that are ignored, i.e. skipped due to epsilon
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    ignored_vertices: Vec<usize>,
}

impl Default for Tetrahedralization {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Create a new [`Tetrahedralization`] from vertices with optional weights and epsilon.
///
/// ## Example
/// ```
/// # use rita::tetrahedralization;
/// tetrahedralization!(&[[0.0, 9.9, 4.2], [6.9, 12.3, 3.8], [5.2, 3.33, 1.92]]);
/// // with epsilon
/// tetrahedralization!(&[[0.0, 9.9, 4.2], [6.9, 12.3, 3.8], [5.2, 3.33, 1.92]], epsilon = 1e-9);
/// // with weights
/// tetrahedralization!(&[[0.0, 9.9, 4.2], [6.9, 12.3, 3.8], [5.2, 3.33, 1.92]], vec![0.2, 1.3]);
/// // with weights and epsilon
/// tetrahedralization!(&[[0.0, 9.9, 4.2], [6.9, 12.3, 3.8], [5.2, 3.33, 1.92]], vec![0.2, 1.3], epsilon = 1e-9);
/// ```
#[macro_export]
macro_rules! tetrahedralization {
    ($vertices:expr) => {{
        let mut tetrahedralization =
            $crate::Tetrahedralization::new_with_vert_capacity(None, $vertices.len());
        let _ = tetrahedralization.insert_vertices($vertices, None, true);
        tetrahedralization
    }};
    ($vertices:expr, epsilon = $epsilon:expr) => {{
        let mut tetrahedralization =
            $crate::Tetrahedralization::new_with_vert_capacity(Some($epsilon), $vertices.len());
        let _ = tetrahedralization.insert_vertices($vertices, None, true);
        tetrahedralization
    }};
    // with weights
    ($vertices:expr, $weights:expr) => {{
        let mut tetrahedralization =
            $crate::Tetrahedralization::new_with_vert_capacity(None, $vertices.len());
        let _ = tetrahedralization.insert_vertices($vertices, Some($weights), true);
        tetrahedralization
    }};
    ($vertices:expr, $weights:expr, epsilon = $epsilon:expr) => {{
        let mut tetrahedralization =
            $crate::Tetrahedralization::new_with_vert_capacity(Some($epsilon), $vertices.len());
        let _ = tetrahedralization.insert_vertices($vertices, Some($weights), true);
        tetrahedralization
    }};
}

impl core::fmt::Display for Tetrahedralization {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Tetrahedralization with {} vertices and {} tets",
            self.vertices.len(),
            self.tds.num_tets()
        )
    }
}
