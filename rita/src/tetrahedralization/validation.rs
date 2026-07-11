use alloc::vec;

use anyhow::Result as HowResult;
#[cfg(feature = "logging")]
use log::error;
use rayon::prelude::*;

use super::{ExtendedTetrahedron, Tetrahedralization};
use crate::{VertexNode, predicates};

impl Tetrahedralization {
    pub(crate) fn is_v_in_powersphere(
        &self,
        v_idx: usize,
        tet_idx: usize,
        strict: bool,
    ) -> HowResult<bool> {
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

                predicates::orient_3dlifted_SOS(&a, &b, &c, &d, &p, h_a, h_b, h_c, h_d, h_p)
            }
            // if the triangle is a line segment, then the power sphere is a sphere with infinite radius and we can use a orientation test
            ExtendedTetrahedron::Triangle([a, b, c]) => -predicates::orient_3d(&a, &b, &c, &p),
        };

        if strict {
            Ok(in_sphere > 0.0)
        } else {
            Ok(in_sphere >= 0.0)
        }
    }

    pub(crate) fn is_v_in_eps_powersphere(&self, v_idx: usize, tet_idx: usize) -> HowResult<bool> {
        #[cfg(feature = "wasm")]
        let _ = (v_idx, tet_idx);

        #[cfg(feature = "wasm")]
        return Err(anyhow::Error::msg(
            "Epsilon power sphere is not supported in wasm (robust predicates are unweighted).",
        ));

        #[cfg(not(feature = "wasm"))]
        {
            let p = self.vertices[v_idx];

            let h_p = if let Some(epsilon) = self.epsilon {
                self.height(v_idx) + epsilon
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

                    let in_eps_circle = predicates::orient_3dlifted_SOS(
                        &a, &b, &c, &d, &p, h_a, h_b, h_c, h_d, h_p,
                    );

                    Ok(in_eps_circle > 0.0)
                }
                ExtendedTetrahedron::Triangle(_) => Err(anyhow::Error::msg(
                    "Epsilon power circle test not allowed for conceptual triangles yet!",
                )),
            }
        }
    }

    pub(crate) fn is_tet_flat(&self, tet_idx: usize) -> HowResult<bool> {
        let ext_tri = self.get_tet_as_extended(tet_idx)?;

        // TODO: completely cover this with match
        let is_flat = if let ExtendedTetrahedron::Tetrahedron(tri) = ext_tri {
            predicates::orient_3d(&tri[0], &tri[1], &tri[2], &tri[3]) == 0.0
        } else {
            false
        };

        Ok(is_flat)
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

                        predicates::orient_3dlifted_SOS(&a, &b, &c, &d, v, h_a, h_b, h_c, h_d, h_v)
                    }
                    // if the triangle is a line segment, then the power sphere is a sphere with infinite radius and we can use a orientation test
                    ExtendedTetrahedron::Triangle([a, b, c]) => {
                        -predicates::orient_3d(&a, &b, &c, v)
                    }
                };

                if in_sphere > 0.0 {
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
}
