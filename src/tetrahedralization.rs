use std::cmp;

use crate::{
    tetds::{half_tri_iterator::HalfTriIterator, tet_data_structure::TetDataStructure},
    utils::{
        point_order::sort_along_hilbert_curve_3d,
        types::{Tetrahedron3, Triangle3, Vertex3, VertexIdx},
    },
    VertexNode,
};
use anyhow::Result;
use geogram_predicates as gp;
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
/// assert_eq!(tetrahedralization.is_regular_p(false), 1.0);
/// ```
pub struct Tetrahedralization {
    epsilon: Option<f64>,
    tds: TetDataStructure,
    vertices: Vec<Vertex3>,
    /// The weights of the vertices
    weights: Vec<f64>,
    pub time_hilbert: u128,
    time_walking: u128,
    time_inserting: u128,
    /// Indices of vertices that are inserted, i.e. not skipped due to epsilon
    used_vertices: Vec<VertexIdx>,
    /// Indices of vertices that are ignored, i.e. skipped due to epsilon
    ignored_vertices: Vec<VertexIdx>,
    /// If the vertices are weighted
    weighted: bool,
}

impl Default for Tetrahedralization {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Tetrahedralization {
    pub const fn new(epsilon: Option<f64>) -> Self {
        Self {
            epsilon,
            tds: TetDataStructure::new(),
            vertices: Vec::new(),
            weights: Vec::new(),
            time_hilbert: 0,
            time_walking: 0,
            time_inserting: 0,
            used_vertices: Vec::new(),
            ignored_vertices: Vec::new(),
            weighted: false,
        }
    }

    /// Gets the height for a vertex
    pub fn height(&self, v_idx: usize) -> f64 {
        self.vertices[v_idx][0].powi(2)
            + self.vertices[v_idx][1].powi(2)
            + self.vertices[v_idx][2].powi(2)
            - self.weights[v_idx]
    }

    /// The number of triangles, without the ones that have an connection to the dummy point.
    pub fn num_casual_tets(&self) -> usize {
        self.tds().num_casual_tets()
    }

    pub const fn num_ignored_vertices(&self) -> usize {
        self.ignored_vertices.len()
    }

    pub const fn num_tets(&self) -> usize {
        self.tds.num_tets()
    }

    pub const fn num_used_vertices(&self) -> usize {
        self.used_vertices.len()
    }

    pub const fn tds(&self) -> &TetDataStructure {
        &self.tds
    }

    pub const fn vertices(&self) -> &Vec<Vertex3> {
        &self.vertices
    }

    /// Gets extended tetrahedron from index
    pub fn get_tet_as_extended(&self, tet_idx: usize) -> Result<ExtendedTetrahedron> {
        let [node0, node1, node2, node3] = self.tds().get_tet(tet_idx)?.nodes();

        let ext_tri = match (node0, node1, node2, node3) {
            (
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => {
                let v1 = self.vertices()[v_idx1];
                let v2 = self.vertices()[v_idx2];
                let v3 = self.vertices()[v_idx3];
                ExtendedTetrahedron::Triangle([v1, v3, v2])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => {
                let v0 = self.vertices()[v_idx0];
                let v2 = self.vertices()[v_idx2];
                let v3 = self.vertices()[v_idx3];
                ExtendedTetrahedron::Triangle([v0, v2, v3])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx3),
            ) => {
                let v0 = self.vertices()[v_idx0];
                let v1 = self.vertices()[v_idx1];
                let v3 = self.vertices()[v_idx3];
                ExtendedTetrahedron::Triangle([v0, v3, v1])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Conceptual,
            ) => {
                let v0 = self.vertices()[v_idx0];
                let v1 = self.vertices()[v_idx1];
                let v2 = self.vertices()[v_idx2];
                ExtendedTetrahedron::Triangle([v0, v1, v2])
            }
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => {
                let v0 = self.vertices()[v_idx0];
                let v1 = self.vertices()[v_idx1];
                let v2 = self.vertices()[v_idx2];
                let v3 = self.vertices()[v_idx3];
                ExtendedTetrahedron::Tetrahedron([v0, v1, v2, v3])
            }
            (_, _, _, _) => {
                return Err(anyhow::Error::msg("Case should not happen"));
            }
        };

        Ok(ext_tri)
    }

    pub fn is_v_in_sphere(&self, v_idx: usize, tet_idx: usize, strict: bool) -> Result<bool> {
        let p = self.vertices()[v_idx];

        let ext_tet = self.get_tet_as_extended(tet_idx)?;

        let in_sphere = match ext_tet {
            // TODO: why do we need to invert gp's in sphere, compared to robust's, they should have the same signes for the same cases
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

    fn is_v_in_powersphere(&self, v_idx: usize, tet_idx: usize, strict: bool) -> Result<bool> {
        let p = self.vertices()[v_idx];
        let h_p = self.height(v_idx);

        let ext_tet = self.get_tet_as_extended(tet_idx)?;

        let in_sphere = match ext_tet {
            // TODO: why do we need to invert gp's in sphere, compared to robust's, they should have the same signes for the same cases
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

    fn is_v_in_eps_powersphere(&self, v_idx: usize, tet_idx: usize) -> Result<bool> {
        let p = self.vertices()[v_idx];

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

    fn is_tet_flat(&self, tet_idx: usize) -> Result<bool> {
        let ext_tri = self.get_tet_as_extended(tet_idx)?;

        // TODO: completely cover this with match
        let is_flat = if let ExtendedTetrahedron::Tetrahedron(tri) = ext_tri {
            gp::orient_3d(&tri[0], &tri[1], &tri[2], &tri[3]) == 0
        } else {
            false
        };

        Ok(is_flat)
    }

    fn choose_tri<'a>(
        &self,
        tris: &Vec<HalfTriIterator<'a>>,
        v: &[f64; 3],
    ) -> Option<HalfTriIterator<'a>> {
        for &tri in tris {
            let [node0, node1, node2] = tri.nodes();

            if let (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
            ) = (node0, node1, node2)
            {
                let v0 = self.vertices()[v_idx0];
                let v1 = self.vertices()[v_idx1];
                let v2 = self.vertices()[v_idx2];

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

    fn walk_check_all(&self, v_idx: usize) -> Result<usize> {
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

    fn locate_vis_walk(&self, v_idx: usize, starting_tet_idx: usize) -> Result<usize> {
        let v = self.vertices()[v_idx];

        let mut curr_tet_idx = starting_tet_idx;
        let starting_tet = self.tds().get_tet(curr_tet_idx)?;
        let mut tris: Vec<HalfTriIterator> = starting_tet.half_triangles().to_vec();

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
    fn insert_bw(&mut self, v_idx: usize, first_tet_idx: usize) -> Result<Vec<usize>> {
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

    fn insert_vertex_helper(&mut self, v_idx: usize, near_to_idx: usize) -> Result<usize> {
        // Locating vertex via vis walk
        let now = std::time::Instant::now();

        let containing_tet_idx = if let Ok(idx) = self.locate_vis_walk(v_idx, near_to_idx) {
            idx
        } else {
            self.tds.clean_to_del()?;
            self.walk_check_all(v_idx)?
        };

        self.time_walking += now.elapsed().as_micros();

        if self.epsilon.is_some()
            && self.tds().get_tet(containing_tet_idx)?.is_casual()
            && !self.is_v_in_eps_powersphere(v_idx, containing_tet_idx)?
        {
            // Skip vertices that are not in power sphere by epsilon (i.e. above the hyperplane)
            // but only if the containing tet is casual (for now), i.e. the vertex is inside the current convex hull
            self.ignored_vertices.push(v_idx);
            return Ok(0); // TODO return correct last added idx
        } else if self.weighted
            && self.tds().get_tet(containing_tet_idx)?.is_casual()
            && !self.is_v_in_powersphere(v_idx, containing_tet_idx, false)?
        {
            // Skip redundant vertices
            self.ignored_vertices.push(v_idx);
            return Ok(0); // TODO return correct last added idx
        }

        // Inserting vertex
        self.used_vertices.push(v_idx);

        let now = std::time::Instant::now();

        let new_tets = self.insert_bw(v_idx, containing_tet_idx)?;

        self.time_inserting += now.elapsed().as_micros();

        Ok(new_tets[0])
    }

    fn insert_first_tet(&mut self, idxs_to_insert: &mut Vec<usize>) -> Result<()> {
        let now = std::time::Instant::now();

        // first tetrahedron insertion
        if self.vertices().len() == idxs_to_insert.len() {
            let idx0 = idxs_to_insert.pop().unwrap();
            let idx1 = idxs_to_insert.pop().unwrap();

            let v0 = self.vertices()[idx0];
            let v1 = self.vertices()[idx1];

            let mut aligned = Vec::new();
            let v01 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];

            let i2 = idxs_to_insert
                .iter()
                .rev()
                .enumerate()
                .map(|(e, &idx)| (e, self.vertices()[idx]))
                .map(|(e, v)| (e, [v[0] - v0[0], v[1] - v0[1], v[2] - v0[2]]))
                .map(|(e, vec)| (e, vec[0] * v01[0] + vec[1] * v01[1] + vec[2] * v01[2]))
                .map(|(e, scal)| if scal < 0.0 { (e, -scal) } else { (e, scal) })
                .max_by(|(_, val1), (_, val2)| val1.partial_cmp(val2).unwrap())
                .map(|(e, _)| e)
                .unwrap();

            let idx2 = idxs_to_insert.remove(i2);
            let v2 = self.vertices()[idx2];

            loop {
                if let Some(idx3) = idxs_to_insert.pop() {
                    let v3 = self.vertices()[idx3];

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

        log::info!(
            "First tetrahedron computed in {}μs",
            now.elapsed().as_micros()
        );

        Ok(())
    }

    /// insert a single vertex in the structure
    pub fn insert_vertex(&mut self, v: [f64; 3], near_to_idx: Option<usize>) -> Result<()> {
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

        log::info!("Walks computed in {} μs", self.time_walking);
        log::info!("Insertions computed in {} μs", self.time_inserting);

        Ok(())
    }

    /// Updates delaunay graph, including newly inserted vertices
    pub fn insert_vertices(
        &mut self,
        vertices: &[[f64; 3]],
        weights: Option<Vec<f64>>,
        spatial_sorting: bool,
    ) -> Result<()> {
        let mut idxs_to_insert = Vec::new();

        if weights.is_some() {
            self.weighted = true;
        }

        for &v in vertices.iter() {
            idxs_to_insert.push(self.vertices.len());
            self.vertices.push(v);
        }

        if let Some(weights) = weights {
            self.weights = weights.to_vec();
        } else {
            self.weights = vec![0.0; vertices.len()];
        }

        if self.vertices().len() < 4 {
            return Err(anyhow::Error::msg(
                "Needs at least 4 vertices to compute Delaunay",
            ));
        }

        if spatial_sorting {
            let now = std::time::Instant::now();
            idxs_to_insert = sort_along_hilbert_curve_3d(self.vertices(), &idxs_to_insert);
            self.time_hilbert = now.elapsed().as_micros();
            log::info!("Hilbert curve computed in {} μs", now.elapsed().as_micros());
        }

        if self.tds.num_tets() == 0 {
            self.insert_first_tet(&mut idxs_to_insert)?;
        }

        let mut last_added_idx = self.tds.num_tets() - 1;
        while let Some(v_idx) = idxs_to_insert.pop() {
            last_added_idx = self.insert_vertex_helper(v_idx, last_added_idx)?;
        }

        self.tds.clean_to_del()?;

        log::info!("Walks computed in {} μs", self.time_walking);
        log::info!("Insertions computed in {} μs", self.time_inserting);

        Ok(())
    }

    /// Check if the tetrahedralization is valid, i.e. no vertices are inside the circumsphere of any tetrahedron
    pub fn is_regular(&self) -> Result<(bool, f64)> {
        let mut regular = true;
        let mut num_violated_tets = 0;

        for tet_idx in 0..self.tds().num_tets() {
            if self.is_tet_flat(tet_idx)? {
                error!("Flat tetrahedron: {}", self.tds().get_tet(tet_idx)?);
                regular = false;
                num_violated_tets += 1;
                continue;
            }

            // Check the used vertices, for this any computed tetrahedralization should always be regular
            for &v_idx in self.used_vertices.iter() {
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

    /// Checks regularity in a parallel manner using `rayon`s `par_iter()`.
    ///
    /// This can significantly reduce the runtime of this predicate.
    pub fn is_regular_p(&self, with_ignored_vertices: bool) -> f64 {
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
    ) -> Result<(bool, f64)> {
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

    pub fn is_sound(&self) -> Result<bool> {
        match self.tds().is_sound() {
            Ok(true) => Ok(true),
            Ok(false) => {
                error!("Triangulation is not sound!");
                Ok(false)
            }
            Err(e) => {
                error!("Triangulation is not sound: {}", e);
                Ok(false)
            }
        }
    }

    pub const fn used_vertices(&self) -> &Vec<usize> {
        &self.used_vertices
    }
}

impl std::fmt::Display for Tetrahedralization {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Tetrahedralization with {} vertices and {} tets",
            self.vertices.len(),
            self.tds.num_tets()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{sample_vertices_3d, sample_weights};

    fn verify_tetrahedralization(tetrahedralization: &Tetrahedralization) {
        let (_, regularity) = tetrahedralization.is_regular().unwrap(); // a triangulation will always be regular for the used vertices, i.e. without ignored
        let sound = tetrahedralization.is_sound().unwrap();
        assert_eq!(regularity, 1.0);
        assert!(sound);
    }

    const NUM_VERTICES_LIST: [usize; 7] = [4, 5, 10, 50, 100, 500, 1000];

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
        let _regular_p = tetrahedralization.is_regular_p(false);
        let elapsed_p = now.elapsed().as_millis();

        assert!(elapsed_p < elapsed)
    }
}
