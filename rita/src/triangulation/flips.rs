use core::panic;

use anyhow::{Ok as HowOk, Result as HowResult};

use super::{Flip, Triangulation};
use crate::{VertexNode, trids::tri_iterator::TriIterator, utils::convexity::is_convex};

impl Triangulation {
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
        let side_d =
            crate::predicates::orient_2d(&self.vertices[c], &self.vertices[a], &self.vertices[d]);
        let side_b =
            crate::predicates::orient_2d(&self.vertices[c], &self.vertices[a], &self.vertices[b]);
        if side_d != side_b {
            num_reflex_points += 1;
            c_reflex = true;
        }

        // check side for c,b for line da, i.e. d reflex
        // TODO only do this check if c is not reflex, i.e. since only one point can be reflex -> would remove 2 orientation tests in some cases
        let side_c =
            crate::predicates::orient_2d(&self.vertices[d], &self.vertices[a], &self.vertices[c]);
        let side_b =
            crate::predicates::orient_2d(&self.vertices[d], &self.vertices[a], &self.vertices[b]);
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
                true => Some(Flip::ThreeToOne((possible_third_tri.idx, d))),
                false => None,
            }
        } else {
            panic!("No reflex point found, but we should have found one!");
        }
    }
}
