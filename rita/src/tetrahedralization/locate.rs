use anyhow::Result as HowResult;

use super::Tetrahedralization;
use crate::{VertexNode, predicates, tetds::half_tri_iterator::HalfTriIterator};

impl Tetrahedralization {
    pub fn is_v_in_sphere(&self, v_idx: usize, tet_idx: usize, strict: bool) -> HowResult<bool> {
        let p = self.vertices[v_idx];

        let ext_tet = self.get_tet_as_extended(tet_idx)?;

        let in_sphere = match ext_tet {
            // TODO: why do we need to invert gp's in sphere, compared to robust's, they should have the same signs for the same cases
            super::ExtendedTetrahedron::Tetrahedron([a, b, c, d]) => {
                -predicates::in_sphere_3d_SOS(&a, &b, &c, &d, &p)
            }
            super::ExtendedTetrahedron::Triangle([a, b, c]) => {
                -predicates::orient_3d(&a, &b, &c, &p)
            }
        };

        if strict {
            Ok(in_sphere > 0.0)
        } else {
            Ok(in_sphere >= 0.0)
        }
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

                let orientation = -predicates::orient_3d(&v0, &v1, &v2, v);

                if tri.tet().is_conceptual() {
                    if orientation <= 0.0 {
                        return Some(tri);
                    }
                } else if orientation < 0.0 {
                    return Some(tri);
                }
            }
        }

        None
    }

    pub(crate) fn walk_check_all(&self, v_idx: usize) -> HowResult<usize> {
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

    pub(crate) fn locate_vis_walk(
        &self,
        v_idx: usize,
        starting_tet_idx: usize,
    ) -> HowResult<usize> {
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
}
