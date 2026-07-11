use alloc::{vec, vec::Vec};

use anyhow::Result as HowResult;

use super::Tetrahedralization;
use crate::{VertexNode, predicates, utils::point_order::sort_along_hilbert_curve_3d};

impl Tetrahedralization {
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
        {
            self.time_walking += now.elapsed().as_micros();
        }

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
        {
            self.time_inserting += now.elapsed().as_micros();
        }

        Ok(new_tets[0])
    }

    fn insert_first_tet(
        &mut self,
        idxs_to_insert: &mut Vec<usize>,
        spatial_sorting: bool,
    ) -> HowResult<()> {
        #[cfg(feature = "log_timing")]
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

                    let orientation = -predicates::orient_3d(&v0, &v1, &v2, &v3);

                    if orientation > 0.0 {
                        self.tds.insert_first_tet([idx0, idx1, idx2, idx3])?;
                    } else if orientation < 0.0 {
                        self.tds.insert_first_tet([idx0, idx2, idx1, idx3])?;
                    } else {
                        aligned.push(idx3);
                        continue;
                    }

                    self.used_vertices.append(&mut vec![idx0, idx1, idx2, idx3]);
                } else {
                    return Err(anyhow::Error::msg("Could not find four non aligned points"));
                }

                break;
            }
            idxs_to_insert.append(&mut aligned);
        }

        #[cfg(feature = "log_timing")]
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

        #[cfg(feature = "log_timing")]
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
        #[cfg(feature = "wasm")]
        if weights.is_some() {
            return Err(anyhow::Error::msg(
                "Weighted Delaunay is not supported in wasm (robust predicates are unweighted). Use weights: None.",
            ));
        }

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
            {
                self.time_hilbert = now.elapsed().as_micros();
            }
            #[cfg(feature = "log_timing")]
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
        #[cfg(feature = "log_timing")]
        {
            log::trace!("Walks computed in {} μs", self.time_walking);
            log::trace!("Insertions computed in {} μs", self.time_inserting);
        }

        Ok(())
    }
}
