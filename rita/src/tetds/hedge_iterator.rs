use super::{
    half_tri_iterator::HalfTriIterator,
    tet_data_structure::{TetDataStructure, NEIGHBOR_HALFEDGE, TRIANGLE_SUBINDICES},
};
use crate::VertexNode;

pub struct HedgeIterator<'a> {
    pub tds: &'a TetDataStructure,
    pub half_tri_idx: usize,
    pub hedge_idx: usize,
}

impl<'a> HedgeIterator<'a> {
    pub fn first_node(&self) -> VertexNode {
        // TODO: refactor the first two lines
        let mod4 = self.half_tri_idx % 4;

        let sub_idx = TRIANGLE_SUBINDICES[mod4];

        self.tds.tet_nodes[self.half_tri_idx - mod4 + sub_idx[self.hedge_idx]]
    }

    pub const fn idx(&self) -> usize {
        self.hedge_idx
    }

    pub fn is_sound(&self) -> bool {
        let first_node = self.first_node();
        let last_node = self.last_node();

        let hedge_next = self.next();
        let hedge_prev = self.prev();
        let hedge_opposite = self.opposite();
        let hedge_neighbor = self.neighbor();

        let mut valid = true;

        if !(hedge_next.first_node() == last_node) {
            #[cfg(feature = "logging")]
            log::error!("{self}: Wrong next hedge");
            valid = false;
        }
        if !(hedge_prev.last_node() == first_node) {
            #[cfg(feature = "logging")]
            log::error!("{self}: Wrong prev hedge");
            valid = false;
        }
        if !(hedge_opposite.first_node() == last_node)
            || !(hedge_opposite.last_node() == first_node)
        {
            #[cfg(feature = "logging")]
            log::error!("{self}: Wrong opposite hedge");
            valid = false;
        }
        if !(hedge_neighbor.first_node() == last_node)
            || !(hedge_neighbor.last_node() == first_node)
        {
            #[cfg(feature = "logging")]
            log::error!("{self}: Wrong neighboring hedge");
            valid = false;
        }

        valid
    }

    pub fn last_node(&self) -> VertexNode {
        // TODO: refactor the first two lines
        let mod4 = self.half_tri_idx % 4;

        let sub_idx = TRIANGLE_SUBINDICES[mod4];

        self.tds.tet_nodes[self.half_tri_idx - mod4 + sub_idx[(self.hedge_idx + 1) % 3]]
    }

    pub const fn neighbor(&self) -> HedgeIterator<'a> {
        let mod4 = self.half_tri_idx % 4;

        let (neighbor_half_tri_idx, neighbor_hedge_idx) = NEIGHBOR_HALFEDGE[mod4][self.hedge_idx];

        HedgeIterator {
            tds: self.tds,
            hedge_idx: neighbor_hedge_idx,
            half_tri_idx: self.half_tri_idx - mod4 + neighbor_half_tri_idx,
        }
    }

    pub const fn next(&self) -> HedgeIterator<'a> {
        HedgeIterator {
            tds: self.tds,
            hedge_idx: (self.hedge_idx + 1) % 3,
            half_tri_idx: self.half_tri_idx,
        }
    }

    pub fn opposite(&self) -> HedgeIterator<'a> {
        let [hedge0, hedge1, hedge2] = self.tri().opposite().hedges(); // TODO impl HalfTriIterator

        let last_node = self.last_node();

        if hedge0.first_node() == last_node {
            hedge0
        } else if hedge1.first_node() == last_node {
            hedge1
        } else {
            hedge2
        }
    }

    pub const fn prev(&self) -> HedgeIterator<'a> {
        HedgeIterator {
            tds: self.tds,
            hedge_idx: (self.hedge_idx + 2) % 3,
            half_tri_idx: self.half_tri_idx,
        }
    }

    pub const fn tri(&self) -> HalfTriIterator<'a> {
        HalfTriIterator {
            tds: self.tds,
            half_tri_idx: self.half_tri_idx,
        }
    }
}

impl core::fmt::Display for HedgeIterator<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Edge: {} -> {}", self.first_node(), self.last_node())
    }
}
