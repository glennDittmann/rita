use crate::VertexNode;

use super::{
    hedge_iterator::HedgeIterator,
    tet_data_structure::{TetDataStructure, TRIANGLE_SUBINDICES},
    tet_iterator::TetIterator,
};

#[derive(Copy, Clone)]
pub struct HalfTriIterator<'a> {
    pub tds: &'a TetDataStructure,
    pub half_tri_idx: usize,
}

impl<'a> HalfTriIterator<'a> {
    pub const fn hedges(&self) -> [HedgeIterator<'a>; 3] {
        [
            HedgeIterator {
                tds: self.tds,
                hedge_idx: 0,
                half_tri_idx: self.half_tri_idx,
            },
            HedgeIterator {
                tds: self.tds,
                hedge_idx: 1,
                half_tri_idx: self.half_tri_idx,
            },
            HedgeIterator {
                tds: self.tds,
                hedge_idx: 2,
                half_tri_idx: self.half_tri_idx,
            },
        ]
    }

    pub const fn idx(&self) -> usize {
        self.half_tri_idx
    }

    pub fn is_sound(&self) -> bool {
        let [n0, n1, n2] = self.nodes();

        let [n_opposite0, n_opposite1, n_opposite2] = self.opposite().nodes();

        if !((n0 == n_opposite0 && n1 == n_opposite2 && n2 == n_opposite1)
            || (n0 == n_opposite2 && n1 == n_opposite1 && n2 == n_opposite0)
            || (n0 == n_opposite1 && n1 == n_opposite0 && n2 == n_opposite2))
        {
            log::error!("{}: Wrong opposite triangle: {}", self, self.opposite());
            return false;
        }

        true
    }

    /// A triangle is considered conceptual if one of its nodes are conceptual
    pub fn is_conceptual(&self) -> bool {
        self.nodes().iter().any(VertexNode::is_conceptual)
    }

    pub fn nodes(&self) -> [VertexNode; 3] {
        let mod4 = self.half_tri_idx % 4;
        let sub_idx = TRIANGLE_SUBINDICES[mod4];

        [
            self.tds.tet_nodes[self.half_tri_idx - mod4 + sub_idx[0]],
            self.tds.tet_nodes[self.half_tri_idx - mod4 + sub_idx[1]],
            self.tds.tet_nodes[self.half_tri_idx - mod4 + sub_idx[2]],
        ]
    }

    /// Get the opposite node on the same tet, i.e the node that is not part of the triangle
    pub fn opposite_node(&self) -> VertexNode {
        self.tds.tet_nodes[self.idx()]
    }

    /// Opposite half triangle on the neighboring tet
    pub fn opposite(&self) -> HalfTriIterator<'a> {
        HalfTriIterator {
            tds: self.tds,
            half_tri_idx: self.tds.half_tri_opposite[self.idx()],
        }
    }

    pub const fn tet(&self) -> TetIterator<'a> {
        TetIterator {
            tds: self.tds,
            tet_idx: self.half_tri_idx >> 2, // this is equivalent to self.half_tri_idx / 4 (rounding down to nearest integer), but faster
        }
    }
}

impl std::fmt::Display for HalfTriIterator<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let [n0, n1, n2] = self.nodes();
        write!(
            f,
            "Triangle {}: {} -> {} -> {}",
            self.half_tri_idx, n0, n1, n2
        )
    }
}
