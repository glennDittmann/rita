use alloc::{vec, vec::Vec};
use core::cmp;
use core::panic;

// TODO: we could allow the epsilon filter on insertion also allow to happen, when the inserted vertex is in a casual triangle, i.e. outside the c-hull
// TODO: we could also incorporate that in the 3->1 flip, as to remove points in a later stage of the algo (not just at insertion)

use crate::{
    VertexNode,
    trids::{
        hedge_iterator::HedgeIterator, tri_data_structure::TriDataStructure,
        tri_iterator::TriIterator,
    },
    utils::{
        convexity::is_convex,
        point_order::sort_along_hilbert_curve_2d,
        types::{Edge2, Triangle2, Vertex2, VertexIdx},
    },
};
use anyhow::{Ok as HowOk, Result as HowResult};
use geogram_predicates as gp;
#[cfg(feature = "logging")]
use log::error;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

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
/// let weights = vec![0.2, 0.3, 0.55, 0.5, 0.6, 0.4, 0.65, 0.7, 0.85, 0.35];
///
/// let mut triangulation = Triangulation::new(None); // specify epsilon here
/// let result = triangulation.insert_vertices(&vertices, Some(weights), true);  // last parameter toggles spatial sorting
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

impl Triangulation {
    pub const fn new(epsilon: Option<f64>) -> Self {
        Self {
            tds: TriDataStructure::new(),
            vertices: Vec::new(),
            weights: None,
            #[cfg(feature = "timing")]
            time_flipping: 0,
            #[cfg(feature = "timing")]
            time_inserting: 0,
            #[cfg(feature = "timing")]
            time_walking: 0,
            last_inserted_triangle: None,
            epsilon,
            used_vertices: Vec::new(),
            ignored_vertices: Vec::new(),
            redundant_vertices: Vec::new(),
        }
    }

    /// Create a new `Triangulation` with a pre-allocated capacity for vertices
    pub fn new_with_vert_capacity(epsilon: Option<f64>, capacity: usize) -> Self {
        Self {
            tds: TriDataStructure::new(),
            vertices: Vec::with_capacity(capacity),
            weights: None,
            #[cfg(feature = "timing")]
            time_flipping: 0,
            #[cfg(feature = "timing")]
            time_inserting: 0,
            #[cfg(feature = "timing")]
            time_walking: 0,
            last_inserted_triangle: None,
            epsilon,
            used_vertices: Vec::new(),
            ignored_vertices: Vec::new(),
            redundant_vertices: Vec::new(),
        }
    }

    pub(crate) const fn weighted(&self) -> bool {
        self.weights.is_some()
    }

    /// Utility function for locate via vis walk.
    ///
    /// Checks all edges for a triangle to go to the next tri or return None, i.e. stop locate at current tri.
    #[must_use]
    fn choose_hedge<'a>(
        &self,
        v_hedges: &Vec<HedgeIterator<'a>>,
        v: &[f64; 2],
    ) -> Option<HedgeIterator<'a>> {
        for hedge in v_hedges {
            // TODO: note for this iter to work, HedgeIterator needs to implement Copy, you can get around this with lifetimes then the caller can't reuse the input vec..

            let idx0 = hedge.starting_node();
            let idx1 = hedge.end_node();

            // only process casual hedges
            if let (VertexNode::Casual(v0), VertexNode::Casual(v1)) = (idx0, idx1) {
                let v0 = self.vertices()[v0];
                let v1 = self.vertices()[v1];

                let orientation = gp::orient_2d(&v0, &v1, v);

                if hedge.tri().is_conceptual() {
                    if orientation <= 0 {
                        return Some(hedge.clone());
                    }
                } else if orientation < 0 {
                    return Some(hedge.clone());
                }
            }
        }
        None
    }

    /// For a tri idx get the triangle variant, i.e. a normal triangle, or a line with one of its three indices at infinity
    pub fn get_tri_type(&self, tri_idx: usize) -> HowResult<TriangleExtended> {
        let [node0, node1, node2] = self.tds.get_tri(tri_idx)?.nodes();

        let tri_extended = match (node0, node1, node2) {
            (VertexNode::Conceptual, VertexNode::Casual(idx1), VertexNode::Casual(idx2)) => {
                let v1 = self.vertices[idx1];
                let v2 = self.vertices[idx2];
                TriangleExtended::ConceptualTriangle([v1, v2])
            }
            (VertexNode::Casual(idx0), VertexNode::Conceptual, VertexNode::Casual(idx2)) => {
                let v0 = self.vertices[idx0];
                let v2 = self.vertices[idx2];
                TriangleExtended::ConceptualTriangle([v2, v0])
            }
            (VertexNode::Casual(idx0), VertexNode::Casual(idx1), VertexNode::Conceptual) => {
                let v0 = self.vertices[idx0];
                let v1 = self.vertices[idx1];
                TriangleExtended::ConceptualTriangle([v0, v1])
            }
            (VertexNode::Casual(idx0), VertexNode::Casual(idx1), VertexNode::Casual(idx2)) => {
                let v0 = self.vertices[idx0];
                let v1 = self.vertices[idx1];
                let v2 = self.vertices[idx2];
                TriangleExtended::Triangle([v0, v1, v2])
            }
            (_, _, _) => return Err(anyhow::Error::msg("An unexpected triangle case occurred")),
        };

        HowOk(tri_extended)
    }

    /// Gets the height for a vertex, this is affected by weights
    pub fn height(&self, v_idx: VertexIdx) -> f64 {
        self.vertices[v_idx][0].powi(2) + self.vertices[v_idx][1].powi(2)
            - self.weights.as_ref().map_or(0.0, |weights| weights[v_idx])
    }

    pub fn insert_init_tri(&mut self, v_idxs: &mut Vec<VertexIdx>) -> HowResult<()> {
        #[cfg(feature = "log_timing")]
        let now = std::time::Instant::now();

        if self.vertices().len() == v_idxs.len() {
            let idx0 = v_idxs.pop().unwrap();
            let idx1 = v_idxs.pop().unwrap();

            let v0 = self.vertices()[idx0];
            let v1 = self.vertices()[idx1];

            let mut aligned = Vec::new();

            // TODO: simplify the control flow here, the break and continue can be aligned more understandably
            loop {
                if let Some(idx2) = v_idxs.pop() {
                    let v2 = self.vertices()[idx2];

                    let orientation = gp::orient_2d(&v0, &v1, &v2);

                    // insert the triangle in ccw order, or if aligned, find another point to build the starting triangle
                    match orientation.cmp(&0) {
                        cmp::Ordering::Greater => {
                            self.tds_mut().add_init_tri([idx0, idx1, idx2])?
                        }
                        cmp::Ordering::Less => self.tds_mut().add_init_tri([idx0, idx2, idx1])?,
                        cmp::Ordering::Equal => {
                            aligned.push(idx2);
                            continue;
                        }
                    };

                    self.used_vertices.append(&mut vec![idx0, idx1, idx2]);
                } else {
                    return Err(anyhow::Error::msg(
                        "All points are aligned, i.e. could not find 3 non-aligned points !",
                    ));
                }
                break;
            }

            v_idxs.append(&mut aligned); // re-add the aligned points
        }

        self.last_inserted_triangle = Some(0); // here the first triangle is the last inserted, as it is the initial casual triangle

        #[cfg(feature = "log_timing")]
        log::trace!(
            "Initial triangle inserted in {:.4} µs",
            now.elapsed().as_micros()
        );
        HowOk(())
    }

    /// Insert a vertex into the triangulation.
    ///
    /// ## Errors
    /// Returns an error if `self` does not have any triangles in it.
    pub fn insert_vertex(
        &mut self,
        v: [f64; 2],
        weight: Option<f64>,
        near_to: Option<usize>,
    ) -> HowResult<()> {
        if self.tds.num_tris() == 0 {
            return Err(anyhow::Error::msg(
                "Needs at least 1 triangle in the triangulation to insert a vertex!",
            ));
        }

        let idx_to_insert = self.vertices.len();
        self.vertices.push(v);
        if let Some(weights) = &mut self.weights {
            weights.push(weight.unwrap_or(0.0));
        }

        let near_to_idx: usize;

        if near_to.is_some() {
            near_to_idx = near_to.unwrap();
        } else if self.last_inserted_triangle.is_some() {
            near_to_idx = self.last_inserted_triangle.unwrap();
        } else {
            near_to_idx = self.tds().num_tris() + self.tds().num_deleted_tris - 1;
        }

        self.insert_v_helper(idx_to_insert, near_to_idx)?;

        self.log_time();

        HowOk(())
    }

    /// Insert a set of vertices into the triangulation.
    ///
    /// For the classical Delaunay triangulation, don't set weights.
    pub fn insert_vertices(
        &mut self,
        vertices: &[Vertex2],
        weights: Option<Vec<f64>>,
        spatial_sorting: bool,
    ) -> HowResult<()> {
        let mut idxs_to_insert = Vec::new();

        for v in vertices {
            idxs_to_insert.push(self.vertices.len());
            self.vertices.push(*v);
        }

        self.weights = weights;

        if self.vertices().len() < 3 {
            return Err(anyhow::Error::msg(
                "Needs at least 3 vertices to compute a 2D Triangulation!",
            ));
        }

        if spatial_sorting {
            #[cfg(feature = "log_timing")]
            let now = std::time::Instant::now();

            idxs_to_insert = sort_along_hilbert_curve_2d(&self.vertices, &idxs_to_insert);

            #[cfg(feature = "log_timing")]
            log::trace!(
                "Spatial sorting (hilbert curve) computed in {:.4} µs",
                now.elapsed().as_micros()
            );
        }

        if self.tds.num_tris() == 0 {
            self.insert_init_tri(&mut idxs_to_insert)?;
        }

        #[cfg(feature = "logging")]
        log::debug!("Inserting {} vertices", idxs_to_insert.len());

        while let Some(v_idx) = idxs_to_insert.pop() {
            let near_to_idx = self
                .last_inserted_triangle
                .unwrap_or(self.tds().num_tris() + self.tds().num_deleted_tris - 1);

            self.insert_v_helper(v_idx, near_to_idx)?;
        }

        self.log_time();

        HowOk(())
    }

    pub fn insert_v_helper(&mut self, v_idx: usize, near_to: usize) -> HowResult<()> {
        // Perform locate and measure time
        #[cfg(feature = "timing")]
        let now = std::time::Instant::now();
        let containing_tri_idx = self.locate_vis_walk(v_idx, near_to)?; // the possibly invalid triangle

        #[cfg(feature = "timing")]
        {
            self.time_walking += now.elapsed().as_micros();
        }

        // Skip vertices that are not in power circle by epsilon (i.e. above the hyperplane)
        // but only if the containing triangle is casual (for now), i.e. the vertex is inside the current convex hull
        if self.epsilon.is_some()
            && self.tds().get_tri(containing_tri_idx)?.is_casual()
            && !self.is_v_in_eps_powercircle(v_idx, containing_tri_idx)?
        {
            self.ignored_vertices.push(v_idx);
            return HowOk(());
        }

        // Perform insert and measure time
        // Note in the weighted case we can check directly if the vertex is in the power circle of the triangle, cause it might already be redundant
        // if yes we can skip it, avoid flips and directly go to the next one
        if self.weighted() && !self.is_v_in_powercircle(v_idx, containing_tri_idx)? {
            self.redundant_vertices.push(v_idx);
            return HowOk(());
        }
        self.used_vertices.push(v_idx);

        #[cfg(feature = "timing")]
        let now = std::time::Instant::now();

        let mut hedges_to_verify = Vec::new();
        let [hedge0, hedge1, hedge2] = self.tds().get_tri(containing_tri_idx)?.hedges();
        hedges_to_verify.push(hedge0.twin().idx);
        hedges_to_verify.push(hedge1.twin().idx);
        hedges_to_verify.push(hedge2.twin().idx);

        let [t0, _, _] = self.tds.flip_1_to_3(containing_tri_idx, v_idx)?;
        self.last_inserted_triangle = Some(t0.idx);

        #[cfg(feature = "timing")]
        {
            self.time_inserting += now.elapsed().as_micros();
        };

        // Perform flips and measure time
        #[cfg(feature = "timing")]
        let now = std::time::Instant::now();
        while let Some(hedge_idx) = hedges_to_verify.pop() {
            if let Some(flip) = self.should_flip_hedge(hedge_idx)? {
                match flip {
                    Flip::TwoToTwo => {
                        let hedge = self.tds().get_hedge(hedge_idx)?;

                        // Push the hedges before perofming the flip, because the flip might shift indices
                        //
                        // We only need to push 2 new hedges on the stack, as follows
                        // Denote the inserted vertex v, the hedge to test ab and the opposing point o, that shares ab with v
                        // The flip makes vab and abo become vao and vbo respectively
                        // Now the hedges to test are the ones not connected to v in any way, i.e. ao and bo
                        hedges_to_verify.push(hedge.prev().twin().idx);
                        hedges_to_verify.push(hedge.next().twin().idx);

                        let [t0, _] = self.tds_mut().flip_2_to_2(hedge_idx)?;
                        self.last_inserted_triangle = Some(t0.idx);
                    }
                    Flip::ThreeToOne((third_tri_idx, relfex_node_idx)) => {
                        let hedge = self.tds().get_hedge(hedge_idx)?;

                        // get the two incident triangles to the hedge, the third tri idx is in the flip
                        let tri_idx_abd = hedge.tri().idx;
                        let tri_idx_bcd = hedge.twin().tri().idx;

                        let t0 = self.tds.flip_3_to_1(
                            [tri_idx_abd, tri_idx_bcd, third_tri_idx],
                            relfex_node_idx,
                            &self.vertices,
                        )?;
                        self.last_inserted_triangle = Some(t0.idx);

                        // push the new hedges on the stack, these are the three edges of the newly created triangle
                        // since in the flip 3 to 1, we overwrite the data structure, such that the new triangle now lives at tri_idx_abd

                        let [hedge0, hedge1, hedge2] = self.tds().get_tri(tri_idx_abd)?.hedges();

                        hedges_to_verify.push(hedge0.twin().idx);
                        hedges_to_verify.push(hedge1.twin().idx);
                        hedges_to_verify.push(hedge2.twin().idx);
                    }
                    _ => {
                        #[cfg(feature = "logging")]
                        log::error!("Unexpected flip type!");
                    }
                }
            }
        }
        #[cfg(feature = "timing")]
        {
            self.time_flipping += now.elapsed().as_micros();
        }
        HowOk(())
    }

    /// Check if a triangle is flat, i.e. exists of three co-linear points.
    pub fn is_tri_flat(&self, tri_idx: usize) -> HowResult<bool> {
        let tri = self.get_tri_type(tri_idx)?;

        let is_flat = match tri {
            TriangleExtended::Triangle(tri_idxs) => {
                gp::orient_2d(&tri_idxs[0], &tri_idxs[1], &tri_idxs[2]) == 0
            }
            TriangleExtended::ConceptualTriangle(_) => false, // the conceptual triangle can't be flat
        };

        HowOk(is_flat)
    }

    /// Check for a vertex, if it lies inside the power circle of a triangle.
    pub fn is_v_in_powercircle(&self, v_idx: usize, tri_idx: usize) -> HowResult<bool> {
        let p = self.vertices()[v_idx];
        let h_p = self.height(v_idx);

        let tri = self.get_tri_type(tri_idx)?;

        let in_circle = match tri {
            TriangleExtended::Triangle([a, b, c]) => {
                let [h_a, h_b, h_c] = self
                    .tds()
                    .get_tri(tri_idx)?
                    .nodes()
                    .map(|n| self.height(n.idx().unwrap()));

                gp::orient_2dlifted_SOS(&a, &b, &c, &p, h_a, h_b, h_c, h_p)
            }
            // if the triangle is a line segment, then the power circle is a circle with infinite radius and we can use an orientation test
            TriangleExtended::ConceptualTriangle(tri_idxs) => {
                gp::orient_2d(&tri_idxs[0], &tri_idxs[1], &p)
            }
        };

        HowOk(in_circle > 0)
    }

    /// Panics if `self.epsilon` is not set
    pub(crate) fn is_v_in_eps_powercircle(&self, v_idx: usize, tri_idx: usize) -> HowResult<bool> {
        let p = self.vertices()[v_idx];

        let h_p = if self.epsilon.is_some() {
            self.height(v_idx) + self.epsilon.unwrap()
        } else {
            panic!("Epsilon not set!");
        };

        let tri = self.get_tri_type(tri_idx)?;

        match tri {
            TriangleExtended::Triangle([a, b, c]) => {
                let [h_a, h_b, h_c] = self
                    .tds()
                    .get_tri(tri_idx)?
                    .nodes()
                    .map(|n| self.height(n.idx().unwrap()));

                let in_eps_circle = gp::orient_2dlifted_SOS(&a, &b, &c, &p, h_a, h_b, h_c, h_p);

                HowOk(in_eps_circle > 0)
            }
            // if the triangle is a line segment, then the power circle is a circle with infinite radius and we can use a orientation test
            TriangleExtended::ConceptualTriangle(_) => Err(anyhow::Error::msg(
                "Epsilon power circle test not allowed for conceptual triangles yet!",
            )),
        }
    }

    /// Check if the triangulation is regular w.r.t. the empty power-sphere property.
    ///
    /// Returns if the validation is valid and to what degree.
    pub fn is_regular(&self) -> HowResult<(bool, f64)> {
        let mut regular = true;
        let mut num_violated_triangles = 0;

        for tri_idx in 0..self.tds().num_tris() + self.tds().num_deleted_tris {
            // Skip triangles that have been deleted by 3->1 flips
            if self
                .tds()
                .get_tri(tri_idx)?
                .nodes()
                .contains(&VertexNode::Deleted)
            {
                continue;
            }

            if self.is_tri_flat(tri_idx)? {
                #[cfg(feature = "logging")]
                error!("Flat triangle: {}", self.tds().get_tri(tri_idx)?);
                regular = false;
                num_violated_triangles += 1;
            }

            // Check the redundant vertices, for this any computed triangulation should always be regular
            for &v_idx in &self.redundant_vertices {
                // skip vertices, that are part of the current triangle. Geogram predicates avoid return 0.0 (in favor of SOS) so a vertex exactly on the circle, might be considered inside
                if self
                    .tds()
                    .get_tri(tri_idx)?
                    .nodes()
                    .contains(&VertexNode::Casual(v_idx))
                {
                    continue;
                }

                if self.is_v_in_powercircle(v_idx, tri_idx)? {
                    // #[cfg(feature = "logging")]
                    // log::error!("Vertex in power circle: {}", self.tds().get_tri(tri_idx)?);
                    regular = false;
                    num_violated_triangles += 1; // s. the break below
                    break;
                }
            }

            // Check the used vertices, for this any computed triangulation should always be regular
            for &v_idx in &self.used_vertices {
                // skip vertices, that are part of the current triangle. Geogram predicates avoid return 0.0 (in favor of SOS) so a vertex exactly on the circle, might be considered inside
                if self
                    .tds()
                    .get_tri(tri_idx)?
                    .nodes()
                    .contains(&VertexNode::Casual(v_idx))
                {
                    continue;
                }

                if self.is_v_in_powercircle(v_idx, tri_idx)? {
                    // #[cfg(feature = "logging")]
                    // log::error!("Vertex in power circle: {}", self.tds().get_tri(tri_idx)?);
                    regular = false;
                    num_violated_triangles += 1; // s. the break below
                    break;
                }
            }
        }

        HowOk((
            regular,
            1.0 - num_violated_triangles as f64 / self.tds().num_tris() as f64,
        ))
    }

    /// Checks regularity in a parallel manner using `rayon`s `par_iter()`.
    ///
    /// This can significantly reduce the runtime of this predicate.
    #[must_use]
    pub fn par_is_regular(&self, with_ignored_vertices: bool) -> f64 {
        let num_tris = self.tds().num_tris();
        let num_deleted_tris = self.tds().num_deleted_tris;

        let num_violated_tris: f64 = (0..num_tris + num_deleted_tris)
            .into_par_iter()
            .map(|tri_idx| {
                // Skip triangles that have been deleted by 3->1 flips
                if self
                    .tds()
                    .get_tri(tri_idx)
                    .unwrap()
                    .nodes()
                    .contains(&VertexNode::Deleted)
                {
                    0.0
                } else if self.is_tri_flat(tri_idx).unwrap() {
                    1.0
                } else {
                    // Check the used vertices, for this any computed tetrahedralization should always be regular
                    let used_violation = self.used_vertices.iter().find(|&&v_idx| {
                        // Skip vertices that are part of the current tetrahedron
                        if self
                            .tds()
                            .get_tri(tri_idx)
                            .unwrap()
                            .nodes()
                            .contains(&VertexNode::Casual(v_idx))
                        {
                            return false;
                        }

                        self.is_v_in_powercircle(v_idx, tri_idx).unwrap()
                    });

                    if used_violation.is_some() {
                        return 1.0;
                    }

                    // Check the redundant vertices
                    let redundant_violation = self.redundant_vertices.iter().find(|&&v_idx| {
                        // Skip vertices that are part of the current tetrahedron
                        if self
                            .tds()
                            .get_tri(tri_idx)
                            .unwrap()
                            .nodes()
                            .contains(&VertexNode::Casual(v_idx))
                        {
                            return false;
                        }

                        self.is_v_in_powercircle(v_idx, tri_idx).unwrap()
                    });

                    if redundant_violation.is_some() {
                        return 1.0;
                    }

                    // Check the ignored vertices, here we can account for the degree of irregularity the epsilon filter introduced
                    if with_ignored_vertices {
                        let ignored_violation = self
                            .ignored_vertices
                            .iter()
                            .find(|&&v_idx| self.is_v_in_powercircle(v_idx, tri_idx).unwrap());

                        if ignored_violation.is_some() {
                            return 1.0;
                        }
                    }

                    0.0
                }
            })
            .sum();

        1.0 - num_violated_tris / self.tds().num_tris() as f64
    }

    pub fn is_regular_for_point_set(
        &self,
        vertices: &[[f64; 2]],
        weights: Option<Vec<f64>>,
    ) -> HowResult<(bool, f64)> {
        let mut regular = true;
        let mut num_violated_triangles = 0;

        let weights = if let Some(weights) = weights {
            weights
        } else {
            vec![0.0; vertices.len()]
        };

        for tri_idx in 0..self.tds().num_tris() + self.tds().num_deleted_tris {
            // Skip triangles that have been deleted by 3->1 flips
            if self
                .tds()
                .get_tri(tri_idx)?
                .nodes()
                .contains(&VertexNode::Deleted)
            {
                continue;
            }

            if self.is_tri_flat(tri_idx)? {
                #[cfg(feature = "logging")]
                error!("Flat triangle: {}", self.tds().get_tri(tri_idx)?);
                regular = false;
                num_violated_triangles += 1;
            }

            // Check the used vertices, for this any computed triangulation should always be regular
            for (idx, v) in vertices.iter().enumerate() {
                // TODO: skip vertices, that are part of the current triangle. Geogram predicates avoid return 0.0 (in favor of SOS) so a vertex exactly on the circle, might be considered inside

                let h_v = v[0].powi(2) + v[1].powi(2) - weights[idx];

                let tri = self.get_tri_type(tri_idx)?;

                let in_circle = match tri {
                    TriangleExtended::Triangle([a, b, c]) => {
                        let [h_a, h_b, h_c] = self
                            .tds()
                            .get_tri(tri_idx)?
                            .nodes()
                            .map(|n| self.height(n.idx().unwrap()));

                        gp::orient_2dlifted_SOS(&a, &b, &c, v, h_a, h_b, h_c, h_v)
                    }
                    // if the triangle is a line segment, then the power circle is a circle with infinite radius and we can use an orientation test
                    TriangleExtended::ConceptualTriangle(tri_idxs) => {
                        gp::orient_2d(&tri_idxs[0], &tri_idxs[1], v)
                    }
                };

                if in_circle > 0 {
                    regular = false;
                    num_violated_triangles += 1;
                    break; // each triangle can be violated once
                }
            }
        }

        HowOk((
            regular,
            1.0 - num_violated_triangles as f64 / self.tds().num_tris() as f64,
        ))
    }

    pub fn is_sound(&self) -> HowResult<bool> {
        if self.tds().is_sound() {
            HowOk(true)
        } else {
            #[cfg(feature = "logging")]
            error!("Triangulation is not sound!");
            HowOk(false)
        }
    }

    pub fn num_ignored_vertices(&self) -> usize {
        self.ignored_vertices.len()
    }

    /// The number of all `tris` in the triangulation, `casual` and `conceptual`.
    pub const fn num_tris(&self) -> usize {
        self.tds().num_tris()
    }

    /// The number of `casual` `tris`, i.e. without the ones that have an connection to the dummy point.
    #[must_use]
    pub fn num_casual_tris(&self) -> usize {
        self.tds().num_casual_tris()
    }

    /// The number of total tris, i.e. `casual`, `conceptual` and `deleted` tris.
    #[must_use]
    pub const fn num_all_tris(&self) -> usize {
        self.tds().num_tris() + self.tds().num_deleted_tris
    }

    pub fn num_redundant_vertices(&self) -> usize {
        self.redundant_vertices.len()
    }

    pub fn num_used_vertices(&self) -> usize {
        self.used_vertices.len()
    }

    pub(crate) fn should_flip_hedge(&mut self, hedge_idx: usize) -> HowResult<Option<Flip>> {
        let hedge = self.tds().get_hedge(hedge_idx)?;

        // Skip hedges that have been deleted by 3->1 flips
        if hedge.starting_node() == VertexNode::Deleted || hedge.end_node() == VertexNode::Deleted {
            return HowOk(None);
        }

        let tri_idx_abd = hedge.tri().idx;
        let node_a = hedge.prev().starting_node();
        let node_b = hedge.starting_node();

        let tri_idx_bcd = hedge.twin().tri().idx;
        let node_c = hedge.twin().prev().starting_node();
        let node_d = hedge.twin().starting_node();

        // Skip edges that have been deleted by 3->1 flips
        if node_a == VertexNode::Deleted
            || node_b == VertexNode::Deleted
            || node_c == VertexNode::Deleted
            || node_d == VertexNode::Deleted
        {
            return HowOk(None);
        }

        match (node_a, node_b, node_c, node_d) {
            (
                VertexNode::Casual(idx_node_a),
                VertexNode::Casual(idx_node_b), // from the hedge
                VertexNode::Casual(idx_node_c),
                VertexNode::Casual(idx_node_d), // from the hedge
            ) => {
                let mut flip = Some(Flip::TwoToTwo);

                if self.weighted() {
                    // this can make flipe a 3->1, None or stay a 2->2
                    flip = self.is_flippable(
                        [idx_node_b, idx_node_d],
                        [idx_node_a, idx_node_c],
                        hedge_idx,
                    );

                    if flip.is_none() {
                        return HowOk(None); // edge is not flippable (i.e. a 3 to 1 flip, that cant be made due to internal structure of the triangulation)
                    }
                }

                // TODO we should be able to do only one check, if we keep track of the currently inserted vertex here, then the check is clear
                if self.is_v_in_powercircle(idx_node_c, tri_idx_abd)?
                    || self.is_v_in_powercircle(idx_node_a, tri_idx_bcd)?
                {
                    HowOk(flip) // flip necessary, not regular
                } else {
                    HowOk(None) // no flip necessary, already regular
                }
            }
            (
                VertexNode::Conceptual,
                VertexNode::Casual(_),
                VertexNode::Casual(_),
                VertexNode::Casual(_),
            ) => HowOk(None),
            (
                VertexNode::Casual(idx_node_a),
                VertexNode::Conceptual,
                VertexNode::Casual(idx_node_c),
                VertexNode::Casual(idx_node_d),
            ) => {
                if is_convex(
                    self.vertices()[idx_node_c],
                    self.vertices()[idx_node_d],
                    self.vertices()[idx_node_a],
                ) {
                    HowOk(Some(Flip::TwoToTwo))
                } else {
                    HowOk(None)
                }
            }
            (
                VertexNode::Casual(idx_node_a),
                VertexNode::Casual(_),
                VertexNode::Conceptual,
                VertexNode::Casual(_),
            ) => {
                if self.is_v_in_powercircle(idx_node_a, tri_idx_bcd)?
                    || self.is_tri_flat(tri_idx_abd)?
                {
                    HowOk(Some(Flip::TwoToTwo))
                } else {
                    HowOk(None)
                }
            }
            (
                VertexNode::Casual(idx_node_a),
                VertexNode::Casual(idx_node_b),
                VertexNode::Casual(idx_node_c),
                VertexNode::Conceptual,
            ) => {
                if is_convex(
                    self.vertices()[idx_node_a],
                    self.vertices()[idx_node_b],
                    self.vertices()[idx_node_c],
                ) {
                    HowOk(Some(Flip::TwoToTwo))
                } else {
                    HowOk(None)
                }
            }
            (_, _, _, _) => Err(anyhow::Error::msg(
                "Unexpected node configuration to decide flip for!",
            )),
        }
    }

    /// Get the triangulation data structure, as reference.
    #[must_use]
    pub const fn tds(&self) -> &TriDataStructure {
        &self.tds
    }

    /// Get the triangulation data structure, as mutable reference.
    #[must_use]
    pub const fn tds_mut(&mut self) -> &mut TriDataStructure {
        &mut self.tds
    }

    /// Get the triangles of the triangulation as `Triangle2`, i.e `[[f64; 2]; 3]`.
    ///
    /// Does not include conceptual triangles, i.e. the convex hull edges
    /// connected to the point at infinity.
    pub fn tris(&self) -> Vec<Triangle2> {
        // todo: handle the results gracefully, instead of unwrapping (which is safe here though)
        (0..self.tds().num_tris() + self.tds().num_deleted_tris)
            .filter_map(|tri_idx| {
                let tri = self.tds().get_tri(tri_idx).ok()?;

                if tri.is_conceptual() || tri.is_deleted() {
                    return None;
                }

                let [node0, node1, node2] = tri.nodes();

                Some([
                    self.vertices[node0.idx().unwrap()],
                    self.vertices[node1.idx().unwrap()],
                    self.vertices[node2.idx().unwrap()],
                ])
            })
            .collect()
    }

    /// Get the used vertices.
    #[must_use]
    pub const fn used_vertices(&self) -> &Vec<usize> {
        &self.used_vertices
    }

    /// Get the vertices.
    #[must_use]
    pub const fn vertices(&self) -> &Vec<[f64; 2]> {
        &self.vertices
    }

    /// Get the weights.
    #[must_use]
    pub const fn weights(&self) -> &Option<Vec<f64>> {
        &self.weights
    }

    /// Locate the triangle that contains a point by using the visibility walk.
    pub fn locate_vis_walk(&self, v_idx: usize, tri_idx_start: usize) -> HowResult<usize> {
        let v = self.vertices()[v_idx];

        let mut tri_idx = tri_idx_start; // variable to store the current triangle index

        // start with all hedges of the starting triangle
        let mut v_hedges = self.tds().get_tri(tri_idx)?.hedges().to_vec();

        let mut side = true; // TODO or false?

        loop {
            // choose one of the two (three) hedges of the triangle
            if let Some(hedge) = self.choose_hedge(&v_hedges, &v) {
                let hedge_twin = hedge.twin();
                tri_idx = hedge_twin.tri().idx; // the triangle in question is the one incident to the twin hedge
                v_hedges.clear(); // delete the old hedges, to only look at hedges for the current tri

                assert_eq!(
                    hedge_twin.prev().starting_node(),
                    hedge_twin.next().end_node()
                );
                // if during traversal we travel outside the chull of the current trianuglation
                // we now have two conceptula triangles in question
                // they each have an casual edge on the c-hull, and are connected to the conceptual point
                // they also share a common point
                // we use the bisector to determine where the point lies in
                // TODO: refactor this special case
                if self.weighted()
                    && hedge_twin.prev().twin().tri().is_conceptual()
                    && hedge_twin.next().twin().tri().is_conceptual()
                    && !hedge_twin.prev().starting_node().is_conceptual()
                {
                    // first we check for orientation with both edges to see if we are actually already in the tri

                    let o = self.vertices[hedge_twin.prev().starting_node().idx().unwrap()];
                    let a = self.vertices[hedge_twin.prev().end_node().idx().unwrap()];
                    let a_tri_idx = hedge_twin.prev().twin().tri().idx;
                    let b = self.vertices[hedge_twin.next().starting_node().idx().unwrap()];
                    let b_tri_idx = hedge_twin.next().twin().tri().idx;

                    // take the point in the middle of hedge and check if v is on the same side than this point
                    let a_help = self.vertices[hedge.starting_node().idx().unwrap()];
                    let b_help = self.vertices[hedge.end_node().idx().unwrap()];
                    let p_help = [(a_help[0] + b_help[0]) / 2.0, (a_help[1] + b_help[1]) / 2.0];

                    let side_p_help_a = gp::orient_2d(&o, &a, &p_help);
                    let side_p_help_b = gp::orient_2d(&o, &b, &p_help);
                    let side_v_a = gp::orient_2d(&o, &a, &v);
                    let side_v_b = gp::orient_2d(&o, &b, &v);

                    if side_p_help_a == side_v_a && side_p_help_b == side_v_b {
                        return HowOk(hedge.twin().tri().idx);
                    }

                    let o_vec = nalgebra::Vector2::new(o[0], o[1]);
                    let a_vec = nalgebra::Vector2::new(a[0], a[1]);
                    let b_vec = nalgebra::Vector2::new(b[0], b[1]);

                    let ao = (a_vec - o_vec).normalize();
                    let bo = (b_vec - o_vec).normalize();
                    let oc = (ao + bo).normalize();

                    let c_vec = o_vec + oc;
                    let c = [c_vec[0], c_vec[1]];

                    if gp::orient_2d(&o, &c, &v) == gp::orient_2d(&o, &c, &a) {
                        return HowOk(a_tri_idx);
                    } else if gp::orient_2d(&o, &c, &v) == gp::orient_2d(&o, &c, &b) {
                        return HowOk(b_tri_idx);
                    } else {
                        panic!("Vertex is not on either side of the bisector");
                    }
                } else if side {
                    v_hedges.push(hedge_twin.next());
                    v_hedges.push(hedge_twin.prev());
                } else {
                    v_hedges.push(hedge_twin.prev());
                    v_hedges.push(hedge_twin.next());
                }

                side = !side;
            } else {
                return HowOk(tri_idx);
            }
        }
    }

    const fn log_time(&self) {
        #[cfg(feature = "log_timing")]
        {
            log::debug!("-------------------------------------------");
            log::debug!("Time elapsed:");
            log::debug!("Inserts computed in {} μs", self.time_inserting);
            log::debug!("Walks computed in {} μs", self.time_walking);
            log::debug!("Flips computed in {} μs", self.time_flipping);
        }
    }

    fn is_flippable(
        &self,
        vertices_from_edge: [usize; 2],
        vertices_from_incident_tris: [usize; 2],
        hedge_idx: usize,
    ) -> Option<Flip> {
        // Simplified procedure described in Incremental Topological Flipping Works for Regular Triangulations (p. 46,47)
        // Given an edge cd incident to two triangles acd and bcd
        let mut num_reflex_points = 0;
        let mut c_reflex = false;
        let mut d_reflex = false;

        // helper vertices, i.e. that form a triangle each with the vertices in question
        let a = vertices_from_incident_tris[0];
        let b = vertices_from_incident_tris[1];

        // vertices in question
        let c = vertices_from_edge[0];
        let d = vertices_from_edge[1];

        // 1) check p = {c, d} to be reflex or convex this can be done as follows, p is the point to check and q the other point of the edge
        //     - choose at random a "base point" of a, b, WLOG we choose a here (as in the paper)
        //     - draw a line through p,a
        //     - if q, b are on different side of the line, then p is reflex, else convex
        // check if side for d,b for line ca, i.e. c reflex
        let side_d = gp::orient_2d(&self.vertices[c], &self.vertices[a], &self.vertices[d]);
        let side_b = gp::orient_2d(&self.vertices[c], &self.vertices[a], &self.vertices[b]);
        if side_d != side_b {
            num_reflex_points += 1;
            c_reflex = true;
        }

        // check side for c,b for line da, i.e. d reflex
        // TODO only do this check if c is not reflex, i.e. since only one point can be reflex -> would remove 2 orientation tests in some cases
        let side_c = gp::orient_2d(&self.vertices[d], &self.vertices[a], &self.vertices[c]);
        let side_b = gp::orient_2d(&self.vertices[d], &self.vertices[a], &self.vertices[b]);
        if side_c != side_b {
            num_reflex_points += 1;
            d_reflex = true;
        }

        // Early out: iff there are no reflex points, the edge is flippable via 2->2
        if num_reflex_points == 0 {
            return Some(Flip::TwoToTwo);
        } else if num_reflex_points > 1 {
            panic!("There cannot be more than 1 reflex point.");
        }

        // 2) For the (hopefully) only marked as reflex, check their degree, if for all the points marked the degree is 3, the the edge is flippable, let again p be the point to check
        //    - for p to have degree 3, the triangle pab must be in the triangulation (we can PROBABLY check this with hede iterations)
        let hedge = self.tds().get_hedge(hedge_idx).unwrap();

        if c_reflex {
            // this triangle should contain the vertex nodes abc
            let possible_third_tri: TriIterator = if VertexNode::Casual(c) == hedge.starting_node()
            {
                hedge.prev().twin().tri()
            } else {
                // c is the end node of the hedge
                hedge.next().twin().tri()
            };

            if possible_third_tri.is_conceptual() {
                return None;
            }

            let mut idxs = [a, b, c];
            let mut tri_idxs = possible_third_tri.nodes().map(|n| n.idx().unwrap());
            idxs.sort_unstable();
            tri_idxs.sort_unstable();

            match idxs == tri_idxs {
                // if the possible third tri is the tri abc it fills the reflex wedge and we can flip
                true => Some(Flip::ThreeToOne((possible_third_tri.idx, c))),
                false => None,
            }
        } else if d_reflex {
            // this triangle should contain the vertex nodes abc
            let possible_third_tri: TriIterator = if VertexNode::Casual(d) == hedge.starting_node()
            {
                hedge.prev().twin().tri()
            } else {
                // d is the end node of the hedge
                hedge.next().twin().tri()
            };

            if possible_third_tri.is_conceptual() {
                return None;
            }

            let mut idxs = [a, b, d];
            let mut tri_idxs = possible_third_tri.nodes().map(|n| n.idx().unwrap());
            idxs.sort_unstable();
            tri_idxs.sort_unstable();

            match idxs == tri_idxs {
                // if the possible third tri is the tri abc it fills the reflex wedge and we can flip
                true => return Some(Flip::ThreeToOne((possible_third_tri.idx, d))),
                false => return None,
            }
        } else {
            panic!("No reflex point found, but we should have found one!");
        }
    }
}

// Note: this is for cg lab
impl PartialEq for Triangulation {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }
}

impl Eq for Triangulation {}

#[cfg(test)]
mod pre_test {
    #[cfg(not(feature = "logging"))]
    #[test]
    fn logging_enabled() {
        panic!(
            "\x1b[1;31;7m tests must be run with logging enabled, try `--features logging` \x1b[0m"
        )
    }
}

#[cfg(all(test, feature = "logging"))]
mod tests {
    use super::*;
    use rita_test_utils::{sample_vertices_2d, sample_weights};

    fn verify_triangulation(triangulation: &Triangulation) {
        let regularity = triangulation.par_is_regular(false);
        let sound = triangulation.is_sound().unwrap();
        assert_eq!(regularity, 1.0);
        assert!(sound);
    }

    const NUM_VERTICES_LIST: [usize; 7] = [3, 5, 10, 50, 100, 500, 1000];

    const EXAMPLE_VERTICES: [[f64; 2]; 10] = [
        [0.0, 0.0],
        [-0.5, 1.0],
        [0.0, 2.5],
        [2.0, 3.0],
        [4.0, 2.5],
        [5.0, 1.5],
        [4.5, 0.5],
        [2.5, -0.5],
        [1.5, 1.5],
        [3.0, 1.0],
    ];
    const EXAMPLE_WEIGHTS: [f64; 10] = [
        0.681, 0.579, 0.5625, 0.86225, 10.0, 0.472, 0.5865, 0.59625, 0.51225, 7.0,
    ];

    #[test]
    fn test_get_tris() {
        // Test unweighted case
        let mut triangulation = Triangulation::new(None);
        triangulation
            .insert_vertices(&EXAMPLE_VERTICES, None, true)
            .unwrap();

        let tris = triangulation.tris();
        let num_tris = tris.len();

        assert!(tris.len() == 10, "Expected 10 triangles, got {num_tris}");

        // Test weighted case
        let mut triangulation = Triangulation::new(None);
        triangulation
            .insert_vertices(&EXAMPLE_VERTICES, Some(EXAMPLE_WEIGHTS.to_vec()), true)
            .unwrap();

        let tris = triangulation.tris();
        let num_tris = tris.len();

        assert!(tris.len() == 8, "Expected 8 triangles, got {num_tris}");
    }

    #[test]
    fn test_delaunay_2d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_2d(n, None);

            let mut triangulation = Triangulation::new(None);
            let result = triangulation.insert_vertices(&vertices, None, true);

            match result {
                HowResult::Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_triangulation(&triangulation);
        }
    }

    #[test]
    fn test_weighted_delaunay_2d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_2d(n, None);
            let weights = sample_weights(n, None);

            let mut triangulation = Triangulation::new(None);
            let result = triangulation.insert_vertices(&vertices, Some(weights), true);

            match result {
                HowResult::Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_triangulation(&triangulation);

            assert!(
                triangulation.num_used_vertices()
                    + triangulation.num_redundant_vertices()
                    + triangulation.num_ignored_vertices()
                    == n
            );
        }
    }

    #[test]
    fn test_eps_delaunay_2d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_2d(n, None);

            let mut triangulation = Triangulation::new(Some(1.0 / n as f64));
            let result = triangulation.insert_vertices(&vertices, None, true);

            match result {
                HowResult::Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_triangulation(&triangulation);

            assert!(
                triangulation.num_used_vertices()
                    + triangulation.num_redundant_vertices()
                    + triangulation.num_ignored_vertices()
                    == n
            );
        }
    }

    #[test]
    fn test_eps_weighted_delaunay_2d() {
        for n in NUM_VERTICES_LIST {
            let vertices = sample_vertices_2d(n, None);
            let weights = sample_weights(n, None);

            let mut triangulation = Triangulation::new(Some(1.0 / n as f64));
            let result = triangulation.insert_vertices(&vertices, Some(weights), true);

            match result {
                HowResult::Ok(_) => (),
                Err(e) => {
                    log::error!("Error: {}", e);
                }
            }

            verify_triangulation(&triangulation);

            assert!(
                triangulation.num_used_vertices()
                    + triangulation.num_redundant_vertices()
                    + triangulation.num_ignored_vertices()
                    == n
            );
        }
    }

    #[test]
    #[ignore]
    #[cfg(feature = "timing")]
    // only run this test isolated, as test concurenncy can mess up par_iter
    fn test_parallel_regularity_2d() {
        let n_vertices = 2000;
        let vertices = sample_vertices_2d(n_vertices, None);

        let mut triangulation = Triangulation::new(None);
        let _ = triangulation.insert_vertices(&vertices, None, true);

        let now = std::time::Instant::now();
        let (_, _eps_regularity) = triangulation.is_regular().unwrap();
        let elapsed = now.elapsed().as_millis();

        let now = std::time::Instant::now();
        let _regular_p = triangulation.par_is_regular(false);
        let elapsed_p = now.elapsed().as_millis();

        assert!(elapsed_p < elapsed)
    }

    #[test]
    fn results_same_2d() {
        let vertices = &[
            [4.9, 31.9],
            [44.2, -0.05],
            [-49.31, 2.4],
            [98.5, -6.9],
            [7.7, 9.1],
            [3.5, 6.1],
            [6.0, -3.46],
            [4.7, 91.5],
            [6.7, 3.6],
            [-3.7, -40.3],
        ];

        assert_eq!(
            triangulation!(vertices).tris(),
            vec![
                [[6.0, -3.46], [3.5, 6.1], [-49.31, 2.4]],
                [[4.7, 91.5], [4.9, 31.9], [44.2, -0.05]],
                [[3.5, 6.1], [7.7, 9.1], [4.9, 31.9]],
                [[3.5, 6.1], [6.0, -3.46], [6.7, 3.6]],
                [[-3.7, -40.3], [98.5, -6.9], [44.2, -0.05]],
                [[3.5, 6.1], [6.7, 3.6], [7.7, 9.1]],
                [[44.2, -0.05], [6.0, -3.46], [-3.7, -40.3]],
                [[-49.31, 2.4], [-3.7, -40.3], [6.0, -3.46]],
                [[-49.31, 2.4], [3.5, 6.1], [4.9, 31.9]],
                [[4.9, 31.9], [7.7, 9.1], [44.2, -0.05]],
                [[4.9, 31.9], [4.7, 91.5], [-49.31, 2.4]],
                [[44.2, -0.05], [98.5, -6.9], [4.7, 91.5]],
                [[7.7, 9.1], [6.7, 3.6], [44.2, -0.05]],
                [[44.2, -0.05], [6.7, 3.6], [6.0, -3.46]]
            ]
        );

        let vertices = &[
            [-0.37122939978339264, 0.3190369464265699],
            [0.44217013845102393, -0.055915696282054284],
            [-0.4931480236200205, -0.16592024114317144],
            [0.4250889854947786, -0.11789966697253218],
            [0.24723377358550735, 0.2100464123915723],
            [0.36490258549176935, 0.1365021615193457],
            [0.3504827256051506, -0.19027659995331642],
            [-0.28683831662024745, 0.4111240123491553],
            [0.37042241707160173, 0.18423333136526698],
            [-0.3855198542371303, -0.44705493099901394],
        ];

        assert_eq!(
            triangulation!(vertices).tris(),
            vec![
                [
                    [-0.4931480236200205, -0.16592024114317144],
                    [-0.3855198542371303, -0.44705493099901394],
                    [0.3504827256051506, -0.19027659995331642]
                ],
                [
                    [-0.37122939978339264, 0.3190369464265699],
                    [-0.4931480236200205, -0.16592024114317144],
                    [0.24723377358550735, 0.2100464123915723]
                ],
                [
                    [-0.28683831662024745, 0.4111240123491553],
                    [0.24723377358550735, 0.2100464123915723],
                    [0.37042241707160173, 0.18423333136526698]
                ],
                [
                    [0.24723377358550735, 0.2100464123915723],
                    [-0.28683831662024745, 0.4111240123491553],
                    [-0.37122939978339264, 0.3190369464265699]
                ],
                [
                    [0.3504827256051506, -0.19027659995331642],
                    [0.24723377358550735, 0.2100464123915723],
                    [-0.4931480236200205, -0.16592024114317144]
                ],
                [
                    [0.24723377358550735, 0.2100464123915723],
                    [0.36490258549176935, 0.1365021615193457],
                    [0.37042241707160173, 0.18423333136526698]
                ],
                [
                    [0.37042241707160173, 0.18423333136526698],
                    [0.36490258549176935, 0.1365021615193457],
                    [0.44217013845102393, -0.055915696282054284]
                ],
                [
                    [0.36490258549176935, 0.1365021615193457],
                    [0.24723377358550735, 0.2100464123915723],
                    [0.3504827256051506, -0.19027659995331642]
                ],
                [
                    [0.44217013845102393, -0.055915696282054284],
                    [0.36490258549176935, 0.1365021615193457],
                    [0.3504827256051506, -0.19027659995331642]
                ],
                [
                    [0.3504827256051506, -0.19027659995331642],
                    [0.4250889854947786, -0.11789966697253218],
                    [0.44217013845102393, -0.055915696282054284]
                ],
            ]
        );
    }
}
