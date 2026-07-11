use alloc::vec;

use anyhow::{Ok as HowOk, Result as HowResult};
#[cfg(feature = "logging")]
use log::error;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::{TriangleExtended, Triangulation};
use crate::{VertexNode, predicates};

impl Triangulation {
    /// Check if a triangle is flat, i.e. exists of three co-linear points.
    pub fn is_tri_flat(&self, tri_idx: usize) -> HowResult<bool> {
        let tri = self.get_tri_type(tri_idx)?;

        let is_flat = match tri {
            TriangleExtended::Triangle(tri_idxs) => {
                predicates::orient_2d(&tri_idxs[0], &tri_idxs[1], &tri_idxs[2]) == 0.0
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

                predicates::orient_2dlifted_SOS(&a, &b, &c, &p, h_a, h_b, h_c, h_p)
            }
            // if the triangle is a line segment, then the power circle is a circle with infinite radius and we can use an orientation test
            TriangleExtended::ConceptualTriangle(tri_idxs) => {
                predicates::orient_2d(&tri_idxs[0], &tri_idxs[1], &p)
            }
        };

        HowOk(in_circle > 0.0)
    }

    /// Panics if `self.epsilon` is not set.
    /// When `wasm` feature is on, returns an error (epsilon power circle requires weighted predicates).
    pub(crate) fn is_v_in_eps_powercircle(&self, v_idx: usize, tri_idx: usize) -> HowResult<bool> {
        #[cfg(feature = "wasm")]
        let _ = (v_idx, tri_idx);
        #[cfg(feature = "wasm")]
        return Err(anyhow::Error::msg(
            "Epsilon power circle is not supported in wasm (robust predicates are unweighted).",
        ));

        #[cfg(not(feature = "wasm"))]
        {
            let p = self.vertices()[v_idx];

            let h_p = if let Some(epsilon) = self.epsilon {
                self.height(v_idx) + epsilon
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

                    let in_eps_circle =
                        predicates::orient_2dlifted_SOS(&a, &b, &c, &p, h_a, h_b, h_c, h_p);

                    HowOk(in_eps_circle > 0.0)
                }
                TriangleExtended::ConceptualTriangle(_) => Err(anyhow::Error::msg(
                    "Epsilon power circle test not allowed for conceptual triangles yet!",
                )),
            }
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

                        predicates::orient_2dlifted_SOS(&a, &b, &c, v, h_a, h_b, h_c, h_v)
                    }
                    // if the triangle is a line segment, then the power circle is a circle with infinite radius and we can use an orientation test
                    TriangleExtended::ConceptualTriangle(tri_idxs) => {
                        predicates::orient_2d(&tri_idxs[0], &tri_idxs[1], v)
                    }
                };

                if in_circle > 0.0 {
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
}
