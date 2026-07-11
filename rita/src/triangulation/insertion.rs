use alloc::{vec, vec::Vec};

use anyhow::{Ok as HowOk, Result as HowResult};

use super::{Flip, Triangulation};
use crate::{
    predicates,
    utils::{
        point_order::sort_along_hilbert_curve_2d,
        types::{Vertex2, VertexIdx},
    },
};

impl Triangulation {
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

                    let orientation = predicates::orient_2d(&v0, &v1, &v2);

                    // insert the triangle in ccw order, or if aligned, find another point to build the starting triangle
                    if orientation > 0.0 {
                        self.tds_mut().add_init_tri([idx0, idx1, idx2])?;
                    } else if orientation < 0.0 {
                        self.tds_mut().add_init_tri([idx0, idx2, idx1])?;
                    } else {
                        aligned.push(idx2);
                        continue;
                    }

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

        if let Some(near_to) = near_to {
            near_to_idx = near_to;
        } else if let Some(last_inserted_triangle) = self.last_inserted_triangle {
            near_to_idx = last_inserted_triangle;
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
        #[cfg(feature = "wasm")]
        if weights.is_some() {
            return Err(anyhow::Error::msg(
                "Weighted Delaunay is not supported in wasm (robust predicates are unweighted). Use weights: None.",
            ));
        }

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
}
