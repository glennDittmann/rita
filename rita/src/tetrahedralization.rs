use core::cmp;
use alloc::{vec::Vec, vec};

use crate::{
    tetds::{half_tri_iterator::HalfTriIterator, tet_data_structure::TetDataStructure},
    utils::{
        point_order::sort_along_hilbert_curve_3d,
        types::{Tetrahedron3, Triangle3, Vertex3, VertexIdx},
    },
    VertexNode,
};
use anyhow::Result as HowResult;
use geogram_predicates as gp;
#[cfg(feature = "logging")]
use log::error;
use rayon::prelude::*;

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
/// let weights = vec![0.2, 0.3, 0.55, 0.5, 0.6, 0.4, 0.65, 0.7, 0.85, 0.35];
///
/// let mut tetrahedralization = Tetrahedralization::new(None); // specify epsilon here
/// let result = tetrahedralization.insert_vertices(&vertices, Some(weights), true);  // last parameter toggles spatial sorting
/// println!("{:?}", result);
/// assert_eq!(tetrahedralization.par_is_regular(false), 1.0);
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Tetrahedralization {
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
    used_vertices: Vec<VertexIdx>,
    /// Indices of vertices that are ignored, i.e. skipped due to epsilon
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    ignored_vertices: Vec<VertexIdx>,
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
        let mut tetrahedralization = $crate::Tetrahedralization::new_with_vert_capacity(Some($epsilon), $vertices.len());
        let _ = tetrahedralization.insert_vertices($vertices, None, true);
        tetrahedralization
    }};
    // with weights
    ($vertices:expr, $weights:expr) => {{
        let mut tetrahedralization = $crate::Tetrahedralization::new_with_vert_capacity(None, $vertices.len());
        let _ = tetrahedralization.insert_vertices($vertices, Some($weights), true);
        tetrahedralization
    }};
    ($vertices:expr, $weights:expr, epsilon = $epsilon:expr) => {{
        let mut tetrahedralization = $crate::Tetrahedralization::new_with_vert_capacity(Some($epsilon), $vertices.len());
        let _ = tetrahedralization.insert_vertices($vertices, Some($weights), true);
        tetrahedralization
    }};
}

impl Tetrahedralization {
    pub const fn new(epsilon: Option<f64>) -> Self {
        Self {
            epsilon,
            tds: TetDataStructure::new(),
            vertices: Vec::new(),
            weights: None,
            #[cfg(feature = "timing")]
            time_hilbert: 0,
            #[cfg(feature = "timing")]
            time_walking: 0,
            #[cfg(feature = "timing")]
            time_inserting: 0,
            used_vertices: Vec::new(),
            ignored_vertices: Vec::new(),
        }
    }

    /// Create a new `Tetrahedralization` with a pre-allocated capacity for vertices
    pub fn new_with_vert_capacity(epsilon: Option<f64>, capacity: usize) -> Self {
        Self {
            epsilon,
            tds: TetDataStructure::new(),
            vertices: Vec::with_capacity(capacity),
            weights: None,
            #[cfg(feature = "timing")]
            time_hilbert: 0,
            #[cfg(feature = "timing")]
            time_walking: 0,
            #[cfg(feature = "timing")]
            time_inserting: 0,
            used_vertices: Vec::new(),
            ignored_vertices: Vec::new(),
        }
    }

    pub(crate) const fn weighted(&self) -> bool {
        self.weights.is_some()
    }

    /// Gets the height for a vertex
    pub fn height(&self, v_idx: usize) -> f64 {
        self.vertices[v_idx][0].powi(2)
            + self.vertices[v_idx][1].powi(2)
            + self.vertices[v_idx][2].powi(2)
            - self.weights.as_ref().map_or(0.0, |weights| weights[v_idx])
    }

    /// The number of triangles, without the ones that have an connection to the dummy point.
    pub fn num_casual_tets(&self) -> usize {
        self.tds().num_casual_tets()
    }

    pub fn num_ignored_vertices(&self) -> usize {
        self.ignored_vertices.len()
    }

    pub const fn num_tets(&self) -> usize {
        self.tds.num_tets()
    }

    pub fn num_used_vertices(&self) -> usize {
        self.used_vertices.len()
    }

    pub const fn tds(&self) -> &TetDataStructure {
        &self.tds
    }

    /// Get the tetrahedra of the tetrahedralization as `Tetrahedron3`, i.e `[[f64; 3]; 4]`.
    ///
    /// Does not include conceptual tetrahedra, i.e. the convex hull faces
    /// connected to the point at infinity.
    pub fn tets(&self) -> Vec<Tetrahedron3> {
        // todo: handle the results gracefully, instead of unwrapping or .ok() (which is safe here though)
        (0..self.tds().num_tets())
            .filter_map(|tet_idx| {
                let tet = self.tds().get_tet(tet_idx).ok()?;

                if tet.is_conceptual() {
                    return None;
                }

                let [node0, node1, node2, node3] = tet.nodes();
                Some([
                    self.vertices[node0.idx().unwrap()],
                    self.vertices[node1.idx().unwrap()],
                    self.vertices[node2.idx().unwrap()],
                    self.vertices[node3.idx().unwrap()],
                ])
            })
            .collect()
    }

    pub const fn vertices(&self) -> &Vec<Vertex3> {
        &self.vertices
    }

    /// Gets extended tetrahedron from index
    pub fn get_tet_as_extended(&self, tet_idx: usize) -> HowResult<ExtendedTetrahedron> {
        let [node0, node1, node2, node3] = self.tds().get_tet(tet_idx)?.nodes();

        let ext_tri = match (node0, node1, node2, node3) {
            (
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => {
                let v1 = self.vertices[v_idx1];
                let v2 = self.vertices[v_idx2];
                let v3 = self.vertices[v_idx3];
                ExtendedTetrahedron::Triangle([v1, v3, v2])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => {
                let v0 = self.vertices[v_idx0];
                let v2 = self.vertices[v_idx2];
                let v3 = self.vertices[v_idx3];
                ExtendedTetrahedron::Triangle([v0, v2, v3])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx3),
            ) => {
                let v0 = self.vertices[v_idx0];
                let v1 = self.vertices[v_idx1];
                let v3 = self.vertices[v_idx3];
                ExtendedTetrahedron::Triangle([v0, v3, v1])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Conceptual,
            ) => {
                let v0 = self.vertices[v_idx0];
                let v1 = self.vertices[v_idx1];
                let v2 = self.vertices[v_idx2];
                ExtendedTetrahedron::Triangle([v0, v1, v2])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => {
                let v0 = self.vertices[v_idx0];
                let v1 = self.vertices[v_idx1];
                let v2 = self.vertices[v_idx2];
                let v3 = self.vertices[v_idx3];
                ExtendedTetrahedron::Tetrahedron([v0, v1, v2, v3])
            }
            (_, _, _, _) => {
                return Err(anyhow::Error::msg("Case should not happen"));
            }
        };

        Ok(ext_tri)
    }

    pub fn is_v_in_sphere(&self, v_idx: usize, tet_idx: usize, strict: bool) -> HowResult<bool> {
        let p = self.vertices[v_idx];

        let ext_tet = self.get_tet_as_extended(tet_idx)?;

        let in_sphere = match ext_tet {
            // TODO: why do we need to invert gp's in sphere, compared to robust's, they should have the same signs for the same cases
            ExtendedTetrahedron::Tetrahedron([a, b, c, d]) => {
                -gp::in_sphere_3d_SOS(&a, &b, &c, &d, &p)
            }
            ExtendedTetrahedron::Triangle([a, b, c]) => -gp::orient_3d(&a, &b, &c, &p),
        };

        if strict {
            Ok(in_sphere > 0)
        } else {
            Ok(in_sphere >= 0)
        }
    }

    fn is_v_in_powersphere(&self, v_idx: usize, tet_idx: usize, strict: bool) -> HowResult<bool> {
        let p = self.vertices[v_idx];
        let h_p = self.height(v_idx);

        let ext_tet = self.get_tet_as_extended(tet_idx)?;

        let in_sphere = match ext_tet {
            // TODO: why do we need to invert gp's in sphere, compared to robust's, they should have the same signs for the same cases
            ExtendedTetrahedron::Tetrahedron([a, b, c, d]) => {
                let [h_a, h_b, h_c, h_d] = self
                    .tds()
                    .get_tet(tet_idx)?
                    .nodes()
                    .map(|n| self.height(n.idx().unwrap()));

                gp::orient_3dlifted_SOS(&a, &b, &c, &d, &p, h_a, h_b, h_c, h_d, h_p)
            }
            // if the triangle is a line segment, then the power sphere is a sphere with infinite radius and we can use a orientation test
            ExtendedTetrahedron::Triangle([a, b, c]) => -gp::orient_3d(&a, &b, &c, &p),
        };

        if strict {
            Ok(in_sphere > 0)
        } else {
            Ok(in_sphere >= 0)
        }
    }

    fn is_v_in_eps_powersphere(&self, v_idx: usize, tet_idx: usize) -> HowResult<bool> {
        let p = self.vertices[v_idx];

        let h_p = if self.epsilon.is_some() {
            self.height(v_idx) + self.epsilon.unwrap()
        } else {
            panic!("Epsilon not set!");
        };

        let ext_tet = self.get_tet_as_extended(tet_idx)?;

        match ext_tet {
            ExtendedTetrahedron::Tetrahedron([a, b, c, d]) => {
                let [h_a, h_b, h_c, h_d] = self
                    .tds()
                    .get_tet(tet_idx)?
                    .nodes()
                    .map(|n| self.height(n.idx().unwrap()));

                let in_eps_circle =
                    gp::orient_3dlifted_SOS(&a, &b, &c, &d, &p, h_a, h_b, h_c, h_d, h_p);

                Ok(in_eps_circle > 0)
            }
            ExtendedTetrahedron::Triangle(_) => Err(anyhow::Error::msg(
                "Epsilon power circle test not allowed for conceptual triangles yet!",
            )),
        }
    }

    fn is_tet_flat(&self, tet_idx: usize) -> HowResult<bool> {
        let ext_tri = self.get_tet_as_extended(tet_idx)?;

        // TODO: completely cover this with match
        let is_flat = if let ExtendedTetrahedron::Tetrahedron(tri) = ext_tri {
            gp::orient_3d(&tri[0], &tri[1], &tri[2], &tri[3]) == 0
        } else {
            false
        };

        Ok(is_flat)
    }

    fn choose_tri<'a, 'hi>(
        &self,
        tris: &'hi [HalfTriIterator<'a>],
        v: &[f64; 3],
    ) -> Option<&'hi HalfTriIterator<'a>> {
        for tri in tris {
            let [node0, node1, node2] = tri.nodes();

            if let (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
            ) = (node0, node1, node2)
            {
                let v0 = self.vertices[v_idx0];
                let v1 = self.vertices[v_idx1];
                let v2 = self.vertices[v_idx2];

                let orientation = -gp::orient_3d(&v0, &v1, &v2, v);

                if tri.tet().is_conceptual() {
                    if orientation <= 0 {
                        return Some(tri);
                    }
                } else if orientation < 0 {
                    return Some(tri);
                }
            }
        }

        None
    }

    fn walk_check_all(&self, v_idx: usize) -> HowResult<usize> {
        for curr_tet_idx in 0..self.tds().num_tets() {
            if self.is_tet_flat(curr_tet_idx)? {
                continue;
            }

            if self.is_v_in_powersphere(v_idx, curr_tet_idx, false)? {
                return Ok(curr_tet_idx);
            }
        }

        Err(anyhow::Error::msg("Could not find sphere containing point"))
    }

    fn locate_vis_walk(&self, v_idx: usize, starting_tet_idx: usize) -> HowResult<usize> {
        let v = self.vertices[v_idx];

        let mut curr_tet_idx = starting_tet_idx;
        let starting_tet = self.tds().get_tet(curr_tet_idx)?;
        let mut tris = starting_tet.half_triangles().to_vec();

        let mut side = 0;
        let mut num_visited = 0;
        let tets_visitable = self.tds().num_tets() >> 2;

        loop {
            if num_visited > tets_visitable {
                break Err(anyhow::Error::msg("Could not find sphere containing point"));
            }

            if let Some(tri) = self.choose_tri(&tris, &v) {
                num_visited += 1;

                let opp_tri = tri.opposite();
                curr_tet_idx = opp_tri.tet().idx();

                tris.clear();

                let hedges = opp_tri.hedges();
                tris.push(hedges[side % 3].neighbor().tri());
                tris.push(hedges[(1 + side) % 3].neighbor().tri());
                tris.push(hedges[(2 + side) % 3].neighbor().tri());

                side = (side + 1) % 3;
            } else if self.is_v_in_sphere(v_idx, curr_tet_idx, false)? {
                break Ok(curr_tet_idx);
            } else {
                break Err(anyhow::Error::msg("Could not find sphere containing point"));
            }
        }
    }

    /// Inserts point using Bowyer Watson method
    fn insert_bw(&mut self, v_idx: usize, first_tet_idx: usize) -> HowResult<Vec<usize>> {
        self.tds.bw_start(first_tet_idx)?;

        while let Some(tet_idx) = self.tds.bw_tets_to_check() {
            if self.is_v_in_powersphere(v_idx, tet_idx, false)? {
                self.tds.bw_rem_tet(tet_idx);
            } else {
                self.tds.bw_keep_tetra(tet_idx)?;
            }
        }

        let node = VertexNode::Casual(v_idx);
        self.tds.bw_insert_node(node)
    }

    fn insert_vertex_helper(&mut self, v_idx: usize, near_to_idx: usize) -> HowResult<usize> {
        // Locating vertex via vis walk
        #[cfg(feature = "timing")]
        let now = std::time::Instant::now();

        let containing_tet_idx = if let Ok(idx) = self.locate_vis_walk(v_idx, near_to_idx) {
            idx
        } else {
            self.tds.clean_to_del()?;
            self.walk_check_all(v_idx)?
        };

        #[cfg(feature = "timing")]
        { self.time_walking += now.elapsed().as_micros(); }

        if self.epsilon.is_some()
            && self.tds().get_tet(containing_tet_idx)?.is_casual()
            && !self.is_v_in_eps_powersphere(v_idx, containing_tet_idx)?
        {
            // Skip vertices that are not in power sphere by epsilon (i.e. above the hyperplane)
            // but only if the containing tet is casual (for now), i.e. the vertex is inside the current convex hull
            self.ignored_vertices.push(v_idx);
            return Ok(0); // TODO return correct last added idx
        } else if self.weighted()
            && self.tds().get_tet(containing_tet_idx)?.is_casual()
            && !self.is_v_in_powersphere(v_idx, containing_tet_idx, false)?
        {
            // Skip redundant vertices
            self.ignored_vertices.push(v_idx);
            return Ok(0); // TODO return correct last added idx
        }

        // Inserting vertex
        self.used_vertices.push(v_idx);

        #[cfg(feature = "timing")]
        let now = std::time::Instant::now();

        let new_tets = self.insert_bw(v_idx, containing_tet_idx)?;

        #[cfg(feature = "timing")]
        { self.time_inserting += now.elapsed().as_micros(); }

        Ok(new_tets[0])
    }

    fn insert_first_tet(
        &mut self,
        idxs_to_insert: &mut Vec<usize>,
        spatial_sorting: bool,
    ) -> HowResult<()> {
        #[cfg(feature = "logging")]
        let now = std::time::Instant::now();

        // first tetrahedron insertion
        if self.vertices.len() == idxs_to_insert.len() {
            let idx0 = idxs_to_insert.pop().unwrap();
            let idx1 = idxs_to_insert.pop().unwrap();

            let v0 = self.vertices[idx0];
            let v1 = self.vertices[idx1];

            let mut aligned = Vec::new();
            let v01 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];

            let i2 = idxs_to_insert
                .iter()
                .rev()
                .enumerate()
                .map(|(e, &idx)| (e, self.vertices[idx]))
                .map(|(e, v)| (e, [v[0] - v0[0], v[1] - v0[1], v[2] - v0[2]]))
                .map(|(e, vec)| (e, vec[0] * v01[0] + vec[1] * v01[1] + vec[2] * v01[2]))
                .map(|(e, scal)| if scal < 0.0 { (e, -scal) } else { (e, scal) })
                .max_by(|(_, val1), (_, val2)| val1.partial_cmp(val2).unwrap())
                .map(|(e, _)| e)
                .unwrap();

            // todo this needs a double check
            let idx2 = if spatial_sorting {
                idxs_to_insert.remove(i2)
            } else {
                idxs_to_insert.swap_remove(i2)
            };
            let v2 = self.vertices[idx2];

            loop {
                if let Some(idx3) = idxs_to_insert.pop() {
                    let v3 = self.vertices[idx3];

                    let orientation = -gp::orient_3d(&v0, &v1, &v2, &v3);

                    match orientation.cmp(&0) {
                        cmp::Ordering::Greater => {
                            self.tds.insert_first_tet([idx0, idx1, idx2, idx3])?
                        }
                        cmp::Ordering::Less => {
                            self.tds.insert_first_tet([idx0, idx2, idx1, idx3])?
                        }
                        cmp::Ordering::Equal => {
                            aligned.push(idx3);
                            continue;
                        }
                    };

                    self.used_vertices.append(&mut vec![idx0, idx1, idx2, idx3]);
                } else {
                    return Err(anyhow::Error::msg("Could not find four non aligned points"));
                }

                break;
            }
            idxs_to_insert.append(&mut aligned);
        }

        #[cfg(feature = "logging")]
        log::trace!(
            "First tetrahedron computed in {}μs",
            now.elapsed().as_micros()
        );

        Ok(())
    }

    /// Insert a single vertex in the structure
    ///
    /// ## Errors
    /// Returns an error if `self` does not have any triangles in it.
    pub fn insert_vertex(&mut self, v: [f64; 3], near_to_idx: Option<usize>) -> HowResult<()> {
        if self.tds.num_tets() == 0 {
            return Err(anyhow::Error::msg(
                "Needs at least 1 tetrahedron to insert a single point",
            ));
        }

        let idxs_to_insert = self.vertices.len();
        self.vertices.push(v);

        self.insert_vertex_helper(
            idxs_to_insert,
            near_to_idx.unwrap_or(self.tds.num_tets() - 1),
        )?;

        self.tds.clean_to_del()?;

        #[cfg(feature = "logging")]
        {
            log::trace!("Walks computed in {} μs", self.time_walking);
            log::trace!("Insertions computed in {} μs", self.time_inserting);
        }

        Ok(())
    }

    /// Updates delaunay graph, including newly inserted vertices
    pub fn insert_vertices(
        &mut self,
        vertices: &[[f64; 3]],
        weights: Option<Vec<f64>>,
        spatial_sorting: bool,
    ) -> HowResult<()> {
        let mut idxs_to_insert = Vec::with_capacity(vertices.len());

        for &v in vertices {
            idxs_to_insert.push(self.vertices.len());
            self.vertices.push(v);
        }

        self.weights = weights;

        if self.vertices.len() < 4 {
            return Err(anyhow::Error::msg(
                "Needs at least 4 vertices to compute Delaunay",
            ));
        }

        if spatial_sorting {
            #[cfg(feature = "timing")]
            let now = std::time::Instant::now();

            idxs_to_insert = sort_along_hilbert_curve_3d(&self.vertices, idxs_to_insert);

            #[cfg(feature = "timing")]
            { self.time_hilbert = now.elapsed().as_micros(); }
            #[cfg(feature = "logging")]
            log::trace!("Hilbert curve computed in {} μs", now.elapsed().as_micros());
        }

        if self.tds.num_tets() == 0 {
            self.insert_first_tet(&mut idxs_to_insert, spatial_sorting)?;
        }

        let mut last_added_idx = self.tds.num_tets() - 1;
        while let Some(v_idx) = idxs_to_insert.pop() {
            last_added_idx = self.insert_vertex_helper(v_idx, last_added_idx)?;
        }

        self.tds.clean_to_del()?;
        #[cfg(feature = "logging")]
        {
            log::trace!("Walks computed in {} μs", self.time_walking);
            log::trace!("Insertions computed in {} μs", self.time_inserting);
        }

        Ok(())
    }

    /// Check if the tetrahedralization is valid, i.e. no vertices are inside the circumsphere of any tetrahedron
    pub fn is_regular(&self) -> HowResult<(bool, f64)> {
        let mut regular = true;
        let mut num_violated_tets = 0;

        for tet_idx in 0..self.tds().num_tets() {
            if self.is_tet_flat(tet_idx)? {
                #[cfg(feature = "logging")]
                error!("Flat tetrahedron: {}", self.tds().get_tet(tet_idx)?);
                regular = false;
                num_violated_tets += 1;
                continue;
            }

            // Check the used vertices, for this any computed tetrahedralization should always be regular
            for &v_idx in &self.used_vertices {
                // NOTE: skip vertices, that are part of the current triangle. Geogram predicates avoid return 0.0 (in favor of SOS) so a vertex exactly on the circle, might be considered inside
                if self
                    .tds()
                    .get_tet(tet_idx)?
                    .nodes()
                    .contains(&VertexNode::Casual(v_idx))
                {
                    continue;
                }

                if self.is_v_in_powersphere(v_idx, tet_idx, false)? {
                    #[cfg(feature = "logging")]
                    // FIXME: should this not be an error?
                    log::error!("Non Delaunay tetrahedron: {}", self.tds().get_tet(tet_idx)?);
                    regular = false;
                    num_violated_tets += 1;
                }
            }
        }

        Ok((
            regular,
            1.0 - num_violated_tets as f64 / self.tds().num_tets() as f64,
        ))
    }

    /// Checks regularity in parallel using [`rayon`]s.
    ///
    /// This can significantly reduce the runtime of this predicate.
    #[must_use]
    pub fn par_is_regular(&self, with_ignored_vertices: bool) -> f64 {
        let num_tets = self.tds().num_tets();

        let num_violated_tets: f64 = (0..num_tets)
            .into_par_iter()
            .map(|tet_idx| {
                if self.is_tet_flat(tet_idx).unwrap() {
                    1.0
                } else {
                    // Check the used vertices, for this any computed tetrahedralization should always be regular
                    let used_violation = self.used_vertices.iter().find(|&&v_idx| {
                        // Skip vertices that are part of the current tetrahedron
                        if self
                            .tds()
                            .get_tet(tet_idx)
                            .unwrap()
                            .nodes()
                            .contains(&VertexNode::Casual(v_idx))
                        {
                            return false;
                        }

                        self.is_v_in_powersphere(v_idx, tet_idx, false).unwrap()
                    });

                    if used_violation.is_some() {
                        return 1.0;
                    }

                    // Check the ignored vertices, here we can account for the degree of irregularity the epsilon filter introduced
                    if with_ignored_vertices {
                        let ignored_violation = self.ignored_vertices.iter().find(|&&v_idx| {
                            self.is_v_in_powersphere(v_idx, tet_idx, false).unwrap()
                        });

                        if ignored_violation.is_some() {
                            return 1.0;
                        }
                    }

                    0.0
                }
            })
            .sum();

        1.0 - num_violated_tets / self.tds().num_tets() as f64
    }

    pub fn is_regular_for_point_set(
        &self,
        vertices: &[[f64; 3]],
        weights: Option<Vec<f64>>,
    ) -> HowResult<(bool, f64)> {
        let mut regular = true;
        let mut num_violated_tets = 0;

        let weights = if let Some(weights) = weights {
            weights
        } else {
            vec![0.0; vertices.len()]
        };

        for tet_idx in 0..self.tds().num_tets() {
            // Skip triangles that have been deleted by 3->1 flips
            if self
                .tds()
                .get_tet(tet_idx)?
                .nodes()
                .contains(&VertexNode::Deleted)
            {
                continue;
            }

            if self.is_tet_flat(tet_idx)? {
                #[cfg(feature = "logging")]
                error!("Flat tetrahedron: {}", self.tds().get_tet(tet_idx)?);
                regular = false;
                num_violated_tets += 1;
                continue;
            }

            // Check the used vertices, for this any computed triangulation should always be regular
            for (idx, v) in vertices.iter().enumerate() {
                // TODO: skip vertices, that are part of the current triangle. Geogram predicates avoid return 0.0 (in favor of SOS) so a vertex exactly on the circle, might be considered inside

                let h_v = v[0].powi(2) + v[1].powi(2) + v[2].powi(2) - weights[idx];

                let ext_tet = self.get_tet_as_extended(tet_idx)?;

                let in_sphere = match ext_tet {
                    ExtendedTetrahedron::Tetrahedron([a, b, c, d]) => {
                        let [h_a, h_b, h_c, h_d] = self
                            .tds()
                            .get_tet(tet_idx)?
                            .nodes()
                            .map(|n| self.height(n.idx().unwrap()));

                        gp::orient_3dlifted_SOS(&a, &b, &c, &d, v, h_a, h_b, h_c, h_d, h_v)
                    }
                    // if the triangle is a line segment, then the power sphere is a sphere with infinite radius and we can use a orientation test
                    ExtendedTetrahedron::Triangle([a, b, c]) => -gp::orient_3d(&a, &b, &c, v),
                };

                if in_sphere > 0 {
                    regular = false;
                    num_violated_tets += 1;
                    break; // each triangle can be violated once
                }
            }
        }

        Ok((
            regular,
            1.0 - num_violated_tets as f64 / self.tds().num_tets() as f64,
        ))
    }

    pub fn is_sound(&self) -> HowResult<bool> {
        match self.tds().is_sound() {
            Ok(true) => Ok(true),
            Ok(false) => {
                #[cfg(feature = "logging")]
                error!("Triangulation is not sound!");
                Ok(false)
            }
            #[allow(unused)]
            Err(e) => {
                #[cfg(feature = "logging")]
                error!("Triangulation is not sound: {e}");
                Ok(false)
            }
        }
    }

    pub const fn used_vertices(&self) -> &Vec<usize> {
        &self.used_vertices
    }
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

#[cfg(test)]
mod pre_test {
    #[cfg(not(feature = "logging"))]
    #[test]
    fn logging_enabled() {
        panic!("\x1b[1;31;7m tests must be run with logging enabled, try `--features logging` \x1b[0m")
    }
}

#[cfg(all(test, feature = "logging"))]
mod tests {
    use super::*;
    use rita_test_utils::{sample_vertices_3d, sample_weights};

    fn verify_tetrahedralization(tetrahedralization: &Tetrahedralization) {
        let (_, regularity) = tetrahedralization.is_regular().unwrap(); // a triangulation will always be regular for the used vertices, i.e. without ignored
        let sound = tetrahedralization.is_sound().unwrap();
        assert_eq!(regularity, 1.0);
        assert!(sound);
    }

    const NUM_VERTICES_LIST: [usize; 7] = [4, 5, 10, 50, 100, 500, 1000];

    #[test]
    fn test_get_tets() {
        let vertices = sample_vertices_3d(5, None);
        let mut tetrahedralization = Tetrahedralization::new(None);

        tetrahedralization
            .insert_vertices(&vertices, None, true)
            .unwrap();

        let tets = tetrahedralization.tets();
        let num_tets = tets.len();

        assert!(
            tets.len() == 2 || tets.len() == 3 || tets.len() == 4,
            "Expected 2, 3 or 4 tetrahedra, got {num_tets}"
        );
    }

    #[test]
    fn test_delaunay_3d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_3d(n, None);

            let mut tetrahedralization = Tetrahedralization::new(None);
            let result = tetrahedralization.insert_vertices(&vertices, None, true);

            match result {
                Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_tetrahedralization(&tetrahedralization);
        }
    }

    #[test]
    fn test_weighted_delaunay_3d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_3d(n, None);
            let weights = sample_weights(n, None);

            let mut tetrahedralization = Tetrahedralization::new(None);
            let result = tetrahedralization.insert_vertices(&vertices, Some(weights), true);

            match result {
                Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_tetrahedralization(&tetrahedralization);
        }
    }

    #[test]
    fn test_eps_delaunay_3d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_3d(n, None);

            let mut tetrahedralization = Tetrahedralization::new(Some(0.0012));
            let result = tetrahedralization.insert_vertices(&vertices, None, true);

            match result {
                Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_tetrahedralization(&tetrahedralization);

            assert!(
                tetrahedralization.num_used_vertices() + tetrahedralization.num_ignored_vertices()
                    == n
            );
        }
    }

    #[test]
    fn test_eps_weighted_delaunay_3d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_3d(n, None);
            let weights = sample_weights(n, None);

            let mut tetrahedralization = Tetrahedralization::new(Some(0.0012));
            let result = tetrahedralization.insert_vertices(&vertices, Some(weights), true);

            match result {
                Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_tetrahedralization(&tetrahedralization);

            assert!(
                tetrahedralization.num_used_vertices() + tetrahedralization.num_ignored_vertices()
                    == n
            );
        }
    }

    #[test]
    #[ignore]
    // only run this test isolated, as test concurenncy can mess up par_iter
    fn test_parallel_regularity_3d() {
        let n_vertices = 2000;
        let vertices = sample_vertices_3d(n_vertices, None);

        let mut tetrahedralization = Tetrahedralization::new(None);
        let _ = tetrahedralization.insert_vertices(&vertices, None, true);

        let now = std::time::Instant::now();
        let (_, _eps_regularity) = tetrahedralization.is_regular().unwrap();
        let elapsed = now.elapsed().as_millis();

        let now = std::time::Instant::now();
        let _regular_p = tetrahedralization.par_is_regular(false);
        let elapsed_p = now.elapsed().as_millis();

        assert!(elapsed_p < elapsed)
    }

    #[test]
    fn results_same_3d() {
        let vertices = &[
            [-2.91, 4.7, 60.85],
            [6.49, -5.9, 96.9],
            [-7.1, -91.7, 8.5],
            [8.7, -4.5, -6.4],
            [9.8, 49.0, 42.9],
            [-41.65, 6.3, 2.69],
            [4.105, -1.8, -9.71],
            [5.3, -3.2, 2.68],
            [7.62, 5.3, -1.57],
            [7.28, 4.9, -1.81],
        ];

        assert_eq!(
            tetrahedralization!(vertices).tets(),
            vec![
                [[-41.65, 6.3, 2.69], [-7.1, -91.7, 8.5], [6.49, -5.9, 96.9], [-2.91, 4.7, 60.85]],
                [[-41.65, 6.3, 2.69], [-2.91, 4.7, 60.85], [7.62, 5.3, -1.57], [5.3, -3.2, 2.68]],
                [[-7.1, -91.7, 8.5], [6.49, -5.9, 96.9], [-2.91, 4.7, 60.85], [5.3, -3.2, 2.68]],
                [[-41.65, 6.3, 2.69], [-2.91, 4.7, 60.85], [6.49, -5.9, 96.9], [9.8, 49.0, 42.9]],
                [[-41.65, 6.3, 2.69], [-2.91, 4.7, 60.85], [9.8, 49.0, 42.9], [7.62, 5.3, -1.57]],
                [[-41.65, 6.3, 2.69], [4.105, -1.8, -9.71], [-7.1, -91.7, 8.5], [5.3, -3.2, 2.68]],
                [[6.49, -5.9, 96.9], [7.62, 5.3, -1.57], [5.3, -3.2, 2.68], [8.7, -4.5, -6.4]],
                [[-41.65, 6.3, 2.69], [-7.1, -91.7, 8.5], [-2.91, 4.7, 60.85], [5.3, -3.2, 2.68]],
                [[-41.65, 6.3, 2.69], [7.28, 4.9, -1.81], [4.105, -1.8, -9.71], [5.3, -3.2, 2.68]],
                [[6.49, -5.9, 96.9], [9.8, 49.0, 42.9], [7.62, 5.3, -1.57], [8.7, -4.5, -6.4]],
                [[-41.65, 6.3, 2.69], [7.28, 4.9, -1.81], [7.62, 5.3, -1.57], [4.105, -1.8, -9.71]],
                [[-41.65, 6.3, 2.69], [7.62, 5.3, -1.57], [9.8, 49.0, 42.9], [4.105, -1.8, -9.71]],
                [[-41.65, 6.3, 2.69], [7.62, 5.3, -1.57], [7.28, 4.9, -1.81], [5.3, -3.2, 2.68]],
                [[-2.91, 4.7, 60.85], [9.8, 49.0, 42.9], [7.62, 5.3, -1.57], [5.3, -3.2, 2.68]],
                [[-2.91, 4.7, 60.85], [6.49, -5.9, 96.9], [9.8, 49.0, 42.9], [5.3, -3.2, 2.68]],
                [[6.49, -5.9, 96.9], [7.62, 5.3, -1.57], [9.8, 49.0, 42.9], [5.3, -3.2, 2.68]],
                [[-7.1, -91.7, 8.5], [6.49, -5.9, 96.9], [5.3, -3.2, 2.68], [8.7, -4.5, -6.4]],
                [[7.62, 5.3, -1.57], [7.28, 4.9, -1.81], [5.3, -3.2, 2.68], [8.7, -4.5, -6.4]],
                [[7.28, 4.9, -1.81], [7.62, 5.3, -1.57], [4.105, -1.8, -9.71], [8.7, -4.5, -6.4]],
                [[4.105, -1.8, -9.71], [-7.1, -91.7, 8.5], [5.3, -3.2, 2.68], [8.7, -4.5, -6.4]],
                [[7.28, 4.9, -1.81], [4.105, -1.8, -9.71], [5.3, -3.2, 2.68], [8.7, -4.5, -6.4]],
            ]
        );

        let vertices = &[
            [-0.04725968862914487, 0.3516462125678388, -0.12313760895205272],
            [0.22292364004203769, -0.09745743275599683, 0.05550159697839596],
            [-0.12150571763445661, -0.03990107532727405, -0.08537975686394306],
            [-0.3192238770476341, -0.0067495248588208545, -0.45779316426328687],
            [-0.07998418694311427, 0.19729937490029037, 0.06739429707395683],
            [-0.07082940540173965, -0.21955363061383965, 0.412806916526937],
            [0.038053334853741405, -0.45937873618870206, -0.09889301224830771],
            [0.26555392349136553, -0.32992168321175064, 0.22636353961636158],
            [0.2730786166118322, 0.06453656113465944, -0.01530615283103176],
            [0.04798679923829818, 0.4761807498607096, -0.010111564381819371]
        ];

        assert_eq!(
            tetrahedralization!(vertices).tets(),
            vec![
                [[-0.07082940540173965, -0.21955363061383965, 0.412806916526937], [0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [0.26555392349136553, -0.32992168321175064, 0.22636353961636158], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.07998418694311427, 0.19729937490029037, 0.06739429707395683], [0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [-0.07082940540173965, -0.21955363061383965, 0.412806916526937], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176], [0.038053334853741405, -0.45937873618870206, -0.09889301224830771]],
                [[-0.07082940540173965, -0.21955363061383965, 0.412806916526937], [0.26555392349136553, -0.32992168321175064, 0.22636353961636158], [0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [0.038053334853741405, -0.45937873618870206, -0.09889301224830771]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [-0.3192238770476341, -0.0067495248588208545, -0.45779316426328687], [-0.04725968862914487, 0.3516462125678388, -0.12313760895205272], [-0.07998418694311427, 0.19729937490029037, 0.06739429707395683]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [-0.07998418694311427, 0.19729937490029037, 0.06739429707395683], [0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [-0.07082940540173965, -0.21955363061383965, 0.412806916526937]],
                [[-0.07082940540173965, -0.21955363061383965, 0.412806916526937], [0.26555392349136553, -0.32992168321175064, 0.22636353961636158], [0.04798679923829818, 0.4761807498607096, -0.010111564381819371], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [-0.3192238770476341, -0.0067495248588208545, -0.45779316426328687], [-0.07998418694311427, 0.19729937490029037, 0.06739429707395683], [-0.07082940540173965, -0.21955363061383965, 0.412806916526937]],
                [[-0.3192238770476341, -0.0067495248588208545, -0.45779316426328687], [-0.04725968862914487, 0.3516462125678388, -0.12313760895205272], [0.04798679923829818, 0.4761807498607096, -0.010111564381819371], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [0.26555392349136553, -0.32992168321175064, 0.22636353961636158], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176], [0.038053334853741405, -0.45937873618870206, -0.09889301224830771]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [-0.07998418694311427, 0.19729937490029037, 0.06739429707395683], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [-0.04725968862914487, 0.3516462125678388, -0.12313760895205272], [-0.3192238770476341, -0.0067495248588208545, -0.45779316426328687], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.07998418694311427, 0.19729937490029037, 0.06739429707395683], [-0.07082940540173965, -0.21955363061383965, 0.412806916526937], [0.04798679923829818, 0.4761807498607096, -0.010111564381819371], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.04725968862914487, 0.3516462125678388, -0.12313760895205272], [-0.07998418694311427, 0.19729937490029037, 0.06739429707395683], [0.04798679923829818, 0.4761807498607096, -0.010111564381819371], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [-0.07998418694311427, 0.19729937490029037, 0.06739429707395683], [-0.04725968862914487, 0.3516462125678388, -0.12313760895205272], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [0.2730786166118322, 0.06453656113465944, -0.01530615283103176], [-0.3192238770476341, -0.0067495248588208545, -0.45779316426328687], [0.038053334853741405, -0.45937873618870206, -0.09889301224830771]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [-0.3192238770476341, -0.0067495248588208545, -0.45779316426328687], [-0.07082940540173965, -0.21955363061383965, 0.412806916526937], [0.038053334853741405, -0.45937873618870206, -0.09889301224830771]],
                [[-0.12150571763445661, -0.03990107532727405, -0.08537975686394306], [-0.07082940540173965, -0.21955363061383965, 0.412806916526937], [0.22292364004203769, -0.09745743275599683, 0.05550159697839596], [0.038053334853741405, -0.45937873618870206, -0.09889301224830771]],
            ]
        );
    }
}
