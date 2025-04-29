use super::{hedge_iterator::HedgeIterator, tri_iterator::TriIterator};
use crate::{VertexNode, utils::types::HedgeIteratorIdx};

use alloc::vec::Vec;
use anyhow::{Ok as HowOk, Result as HowResult};
use geogram_predicates as gp;

const INACTIVE: usize = usize::MAX;

/// A 2D triangulation data structure.
///
/// The edges are stored in a doubly-connected edge list (DCEL) manner.
///
/// ```ignore
/// i   --> hedge0 \
/// |        |       \
/// v        v        |
/// i+1 --> hedge1 ---|-->  triangle
/// |        |        |
/// v        v       /
/// i+2 --> hedge2 /
/// ```
//
// where:
// `hedge2 = next(he1)`,
// `hedge3 = next(he2)`,
// `hedge1 = next(he3)`
#[derive(Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct TriDataStructure {
    /// The first node is stored, the last can be obtained via `% 3`
    pub(crate) hedge_starting_nodes: Vec<VertexNode>,
    pub(crate) hedge_twins: Vec<HedgeIteratorIdx>,
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    pub num_tris: usize,
    /// The number of deleted triangles.
    #[cfg_attr(feature = "arbitrary", arbitrary(default))]
    pub num_deleted_tris: usize, // we also need to track the number of deleted to index into the existing one correctly (otherwise we would have to shift all indices, which is tedious)
}

impl Default for TriDataStructure {
    fn default() -> Self {
        Self::new()
    }
}

impl TriDataStructure {
    pub const fn new() -> Self {
        Self {
            hedge_starting_nodes: Vec::new(),
            hedge_twins: Vec::new(),
            num_tris: 0,
            num_deleted_tris: 0,
        }
    }

    /// Add a triangle to the triangulation and retrieve the hedge indices.
    pub fn add_tri(
        &mut self,
        vertex_nodes: [VertexNode; 3],
    ) -> (HedgeIteratorIdx, HedgeIteratorIdx, HedgeIteratorIdx) {
        let hedge_idx0 = self.hedge_starting_nodes.len();

        self.hedge_starting_nodes.extend_from_slice(&vertex_nodes); // Add the three nodes to the node list

        self.num_tris += 1;

        (hedge_idx0, hedge_idx0 + 1, hedge_idx0 + 2)
    }

    /// Insert an initial triangle into the triangulation.
    pub fn add_init_tri(&mut self, v_idxs: [usize; 3]) -> HowResult<[TriIterator; 4]> {
        if self.num_tris() > 0 {
            return Err(anyhow::Error::msg(
                "Triangulation already contains triangles!",
            ));
        }
        // Create nodes for the first triangle and an infinity node; also save the first triangles index
        let a = VertexNode::Casual(v_idxs[0]);
        let b = VertexNode::Casual(v_idxs[1]);
        let c = VertexNode::Casual(v_idxs[2]);
        let n_inf = VertexNode::Conceptual;

        // Add the neighborhood relation between the current four nodes / vertices
        // i.e. add the initial triangle and also connect all pair of two vertices to the vertex at infinity
        // TODO this clones the nodes ! Is it necessary, fast enough, they are only enums with internal usize -> cloning for a usize (in general non-heap allocated data) is cheap enough
        let (hedge01, hedge12, hedge20) = self.add_tri([a, b, c]);
        let (hedgei2, hedge21, hedge1i) = self.add_tri([n_inf, c, b]);
        let (hedge2i, hedgei0, hedge02) = self.add_tri([c, n_inf, a]);
        let (hedge10, hedge0i, hedgei1) = self.add_tri([b, a, n_inf]);

        // Add all halfe edges to the opposite list
        // E.g. the opposite of hedge01 is hedge10, i.e the reverse direction of creation above
        self.hedge_twins.push(hedge10);
        self.hedge_twins.push(hedge21);
        self.hedge_twins.push(hedge02);
        self.hedge_twins.push(hedge2i);
        self.hedge_twins.push(hedge12);
        self.hedge_twins.push(hedgei1);
        self.hedge_twins.push(hedgei2);
        self.hedge_twins.push(hedge0i);
        self.hedge_twins.push(hedge20);
        self.hedge_twins.push(hedge01);
        self.hedge_twins.push(hedgei0);
        self.hedge_twins.push(hedge1i);

        // Return the four new triangle iterators
        HowOk([
            TriIterator::new(self, 0),
            TriIterator::new(self, 1),
            TriIterator::new(self, 2),
            TriIterator::new(self, 3),
        ])
    }

    /// Insert a vertex `d` into an existing triangle `abc`; called the `1 -> 3 flip`, as it deletes the triangle and creates three new ones.
    pub fn flip_1_to_3(
        &mut self,
        idx_to_remove: usize,
        v_idx: usize,
    ) -> HowResult<[TriIterator; 3]> {
        if idx_to_remove > self.num_tris() + self.num_deleted_tris {
            return Err(anyhow::Error::msg("Triangle index out of bounds!"));
        }

        let hedge_ab = idx_to_remove * 3;
        let hedge_bc = hedge_ab + 1;
        let hedge_ca = hedge_ab + 2;

        let a = self.hedge_starting_nodes[hedge_ab];
        let b = self.hedge_starting_nodes[hedge_bc];
        let c = self.hedge_starting_nodes[hedge_ca];

        let d = VertexNode::Casual(v_idx);

        let hedge_ba = self.hedge_twins[hedge_ab];
        let hedge_cb = self.hedge_twins[hedge_bc];
        let hedge_ac = self.hedge_twins[hedge_ca];

        let (hedge_ab, hedge_bd, hedge_da) = self.replace_tri(idx_to_remove, a, b, d);
        let (hedge_bc, hedge_cd, hedge_db) = self.add_tri([b, c, d]);
        let (hedge_ca, hedge_ad, hedge_dc) = self.add_tri([c, a, d]);

        self.hedge_twins[hedge_ba] = hedge_ab;
        self.hedge_twins[hedge_cb] = hedge_bc;
        self.hedge_twins[hedge_ac] = hedge_ca;
        self.hedge_twins[hedge_ab] = hedge_ba;
        self.hedge_twins[hedge_bd] = hedge_db;
        self.hedge_twins[hedge_da] = hedge_ad;
        self.hedge_twins.push(hedge_cb);
        self.hedge_twins.push(hedge_dc);
        self.hedge_twins.push(hedge_bd);
        self.hedge_twins.push(hedge_ac);
        self.hedge_twins.push(hedge_da);
        self.hedge_twins.push(hedge_cd);

        HowOk([
            TriIterator::new(self, idx_to_remove),
            TriIterator::new(self, self.num_tris() - 2),
            TriIterator::new(self, self.num_tris() - 1),
        ])
    }

    /// Flips an edge that internally connects two triangles to an edge that connects the other two triangles.
    pub fn flip_2_to_2(&mut self, idx: usize) -> HowResult<[TriIterator; 2]> {
        let hedge_twin_idx = self.hedge_twins[idx];

        let tri1_idx = idx / 3;
        let tri2_idx = hedge_twin_idx / 3;

        let hedge01 = tri1_idx * 3;
        let hedge12 = hedge01 + 1;
        let hedge20 = hedge01 + 2;

        let hedge01_twin = tri2_idx * 3;
        let hedge12_twin = hedge01_twin + 1;
        let hedge20_twin = hedge01_twin + 2;

        // get the correct flip depending on the structure of the triangles
        let (hedge_ab, hedge_bc) = if hedge01 == idx {
            (hedge12, hedge20)
        } else if hedge12 == idx {
            (hedge20, hedge01)
        } else {
            (hedge01, hedge12)
        };

        let (hedge_cd, hedge_da) = if hedge01_twin == hedge_twin_idx {
            (hedge12_twin, hedge20_twin)
        } else if hedge12_twin == hedge_twin_idx {
            (hedge20_twin, hedge01_twin)
        } else {
            (hedge01_twin, hedge12_twin)
        };

        let na = self.hedge_starting_nodes[hedge_ab];
        let nb = self.hedge_starting_nodes[hedge_bc];
        let nc = self.hedge_starting_nodes[hedge_cd];
        let nd = self.hedge_starting_nodes[hedge_da];

        let hedge_ba = self.hedge_twins[hedge_ab];
        let hedge_cb = self.hedge_twins[hedge_bc];
        let hedge_dc = self.hedge_twins[hedge_cd];
        let hedge_ad = self.hedge_twins[hedge_da];

        let (hedge_bc, hedge_cd, hedge_db) = self.replace_tri(tri1_idx, nb, nc, nd);
        let (hedge_da, hedge_ab, hedge_bd) = self.replace_tri(tri2_idx, nd, na, nb);

        self.hedge_twins[hedge_ab] = hedge_ba;
        self.hedge_twins[hedge_da] = hedge_ad;
        self.hedge_twins[hedge_bc] = hedge_cb;
        self.hedge_twins[hedge_cd] = hedge_dc;

        self.hedge_twins[hedge_bd] = hedge_db;
        self.hedge_twins[hedge_db] = hedge_bd;

        self.hedge_twins[hedge_ba] = hedge_ab;
        self.hedge_twins[hedge_ad] = hedge_da;
        self.hedge_twins[hedge_cb] = hedge_bc;
        self.hedge_twins[hedge_dc] = hedge_cd;

        HowOk([
            TriIterator::new(self, tri1_idx),
            TriIterator::new(self, tri2_idx),
        ])
    }

    /// Flips edges such that cluster of three triangles `abd`, `bcd` and `cad`, such that they form a larger triangle `abc`.
    ///
    /// `idxs_to_flip` are the indices of the triangles to flip.
    ///
    /// Assumption: `abd`, `bcd` and `cad` are arranged in a way that they form a larger triangle, i.e. the vertex `d` lies in the "middle" and is redundant.
    ///
    /// Note: only appears in weighted Delaunay triangulations, where the weights are not zero.
    pub fn flip_3_to_1(
        &mut self,
        idxs_to_flip: [usize; 3],
        reflex_node_idx: usize,
        vertices: &[[f64; 2]],
    ) -> HowResult<TriIterator> {
        // Each of the three triangles has one edge that does not contain the reflex node. i.e. is not shared with the other two triangles
        // these edges form the new triangle
        // we will find these edges (compare with the reflex node idx) and also take the edges respective twin hedge idxs
        // with that information we can update the first (WLOG) of the three triangles to become the new triangle
        let tri0 = self.get_tri(idxs_to_flip[0]).unwrap();
        let tri0_idx = tri0.idx;
        let hedges0 = tri0.hedges();

        // 0. Get the indices where the new triangle will be stored (these are the idxs of the first triangle to be removed from the trgltn)
        let h_idx0 = hedges0[0].idx;
        let h_idx1 = hedges0[1].idx;
        let h_idx2 = hedges0[2].idx;

        // 1. Get the three new edges and their twins from the three triangles to delete (i.e. find 3 in 9 edges)
        let mut starting_node0 = VertexNode::Deleted;
        let mut twin_idx0 = INACTIVE;
        // O(3) since each triangle has 3 edges, this loop will make three iterations
        for h in &hedges0 {
            if h.starting_node() != VertexNode::Casual(reflex_node_idx)
                && h.end_node() != VertexNode::Casual(reflex_node_idx)
            {
                starting_node0 = h.starting_node();
                twin_idx0 = h.twin().idx;
            }
        }

        let tri1 = self.get_tri(idxs_to_flip[1]).unwrap();
        let hedges1 = tri1.hedges();
        let mut starting_node1 = VertexNode::Deleted;
        let mut twin_idx1 = INACTIVE;
        // O(3) since each triangle has 3 edges, this loop will make three iterations
        for h in hedges1 {
            if h.starting_node() != VertexNode::Casual(reflex_node_idx)
                && h.end_node() != VertexNode::Casual(reflex_node_idx)
            {
                starting_node1 = h.starting_node();
                twin_idx1 = h.twin().idx;
            }
        }

        let tri2 = self.get_tri(idxs_to_flip[2]).unwrap();
        let hedges2 = tri2.hedges();
        let mut starting_node2 = VertexNode::Deleted;
        let mut twin_idx2 = INACTIVE;
        // O(3) since each triangle has 3 edges, this loop will make three iterations
        for h in hedges2 {
            if h.starting_node() != VertexNode::Casual(reflex_node_idx)
                && h.end_node() != VertexNode::Casual(reflex_node_idx)
            {
                starting_node2 = h.starting_node();
                twin_idx2 = h.twin().idx;
            }
        }

        // 2. Update the data structure for the new triangle, i.e adding three new edges with node and twin data
        // 2.1 check orientation of the new triangle and swap if necessary
        // TODO Note: we might be able to infer this information faster than with the predicate (e.g. by if else combinations)
        //            but the flip appears not so often, such that it is sufficient for now
        let orient = gp::orient_2d(
            &vertices[starting_node0.idx().unwrap()],
            &vertices[starting_node1.idx().unwrap()],
            &vertices[starting_node2.idx().unwrap()],
        );

        if orient == -1 {
            // swap second and third edge
            core::mem::swap(&mut starting_node1, &mut starting_node2);
            core::mem::swap(&mut twin_idx1, &mut twin_idx2);
        }

        // 2.2 First new edge
        self.hedge_starting_nodes[h_idx0] = starting_node0;
        self.hedge_twins[h_idx0] = twin_idx0;
        self.hedge_twins[twin_idx0] = h_idx0;
        // 2.3 Second new edge
        self.hedge_starting_nodes[h_idx1] = starting_node1;
        self.hedge_twins[h_idx1] = twin_idx1;
        self.hedge_twins[twin_idx1] = h_idx1;
        // 2.4 Third new edge
        self.hedge_starting_nodes[h_idx2] = starting_node2;
        self.hedge_twins[h_idx2] = twin_idx2;
        self.hedge_twins[twin_idx2] = h_idx2;

        // 3. Set the other two triangles to deleted and their twins to inactive
        self.set_tri_inactive(idxs_to_flip[1]);
        self.set_tri_inactive(idxs_to_flip[2]);

        // 4. Update number of triangles and deleted triangles
        self.num_tris -= 2;
        self.num_deleted_tris += 2;

        HowOk(TriIterator::new(self, tri0_idx))
    }

    /// Helper function for 3->1 flip. Sets a triangle to inactive.
    ///
    /// Called twice by the 3->1 flip, once for each triangle that is set to inactive.
    ///
    /// Easier for now, than to re-arrange the indices in the array.
    fn set_tri_inactive(&mut self, triangle_idx: usize) {
        let hedges = self.get_tri(triangle_idx).unwrap().hedges();
        let idx_del0 = hedges[0].idx;
        let idx_del1 = hedges[1].idx;
        let idx_del2 = hedges[2].idx;

        self.hedge_starting_nodes[idx_del0] = VertexNode::Deleted;
        self.hedge_starting_nodes[idx_del1] = VertexNode::Deleted;
        self.hedge_starting_nodes[idx_del2] = VertexNode::Deleted;

        self.hedge_twins[idx_del0] = INACTIVE;
        self.hedge_twins[idx_del1] = INACTIVE;
        self.hedge_twins[idx_del2] = INACTIVE;
    }

    /// Retrieve a half-edge iterator by index.
    pub fn get_hedge(&self, idx: usize) -> HowResult<HedgeIterator> {
        if idx >= self.hedge_starting_nodes.len() {
            return Err(anyhow::Error::msg("Hedge index out of bounds"));
        }

        HowOk(HedgeIterator::new(self, idx))
    }

    /// Retrieve a half-tri iterator by index.
    pub fn get_tri(&self, idx: usize) -> HowResult<TriIterator> {
        if idx >= self.num_tris() + self.num_deleted_tris {
            // - num_deleted_tris because we have to account for the deleted, that basically clog up array indices
            return Err(anyhow::Error::msg("Tri index out of bounds!"));
        }

        HowOk(TriIterator::new(self, idx))
    }

    /// Get the number of triangles in the triangulation.
    pub const fn num_tris(&self) -> usize {
        self.num_tris
    }

    /// Get the number of triangles in the triangulation, without the ones connected to the dummy point.
    pub fn num_casual_tris(&self) -> usize {
        let mut num_casual_tris = 0;
        for i in 0..self.num_tris() + self.num_deleted_tris {
            let tri = self.get_tri(i).unwrap();
            let [n0, n1, n2] = tri.nodes();

            if !tri.is_conceptual()
                && !tri.is_deleted()
                && n0.idx().is_some()
                && n1.idx().is_some()
                && n2.idx().is_some()
            {
                num_casual_tris += 1;
            }
        }
        num_casual_tris
    }

    /// Check if the data structure is sound, i.e. hedges point to correct next and previous nodes.
    pub fn is_sound(&self) -> bool {
        let mut sound = true;

        for hedge_idx in 0..self.hedge_starting_nodes.len() {
            if self.hedge_starting_nodes[hedge_idx] == VertexNode::Deleted {
                continue;
            }
            let hedge = self.get_hedge(hedge_idx).unwrap();
            sound = sound && hedge.is_sound();
        }

        sound
    }

    /// Replace a triangle in the triangulation and retrieve the hedge indices.
    pub fn replace_tri(
        &mut self,
        idx_to_remove: usize,
        v0: VertexNode,
        v1: VertexNode,
        v2: VertexNode,
    ) -> (usize, usize, usize) {
        let idx0 = idx_to_remove * 3;

        self.hedge_starting_nodes[idx0] = v0;
        self.hedge_starting_nodes[idx0 + 1] = v1;
        self.hedge_starting_nodes[idx0 + 2] = v2;

        (idx0, idx0 + 1, idx0 + 2)
    }
}
