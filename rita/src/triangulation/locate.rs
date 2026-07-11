use anyhow::{Ok as HowOk, Result as HowResult};

use super::Triangulation;
use crate::{VertexNode, predicates, trids::hedge_iterator::HedgeIterator};

impl Triangulation {
    /// Utility function for locate via vis walk.
    ///
    /// Checks all edges for a triangle to go to the next tri or return None, i.e. stop locate at current tri.
    #[must_use]
    fn choose_hedge<'a>(
        &self,
        v_hedges: &[HedgeIterator<'a>],
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

                let orientation = predicates::orient_2d(&v0, &v1, v);

                if hedge.tri().is_conceptual() {
                    if orientation <= 0.0 {
                        return Some(hedge.clone());
                    }
                } else if orientation < 0.0 {
                    return Some(hedge.clone());
                }
            }
        }
        None
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

                    let side_p_help_a = predicates::orient_2d(&o, &a, &p_help);
                    let side_p_help_b = predicates::orient_2d(&o, &b, &p_help);
                    let side_v_a = predicates::orient_2d(&o, &a, &v);
                    let side_v_b = predicates::orient_2d(&o, &b, &v);

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

                    if predicates::orient_2d(&o, &c, &v) == predicates::orient_2d(&o, &c, &a) {
                        return HowOk(a_tri_idx);
                    } else if predicates::orient_2d(&o, &c, &v) == predicates::orient_2d(&o, &c, &b)
                    {
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
}
