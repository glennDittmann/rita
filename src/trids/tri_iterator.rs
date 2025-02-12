use core::fmt;

use crate::{utils::types::TriIteratorIdx, VertexNode};

use super::{hedge_iterator::HedgeIterator, tri_data_structure::TriDataStructure};

pub struct TriIterator<'a> {
    pub tds: &'a TriDataStructure,
    pub idx: usize,
}

impl<'a> TriIterator<'a> {
    pub fn new(tds: &'a TriDataStructure, idx: TriIteratorIdx) -> Self {
        Self { tds, idx }
    }

    /// Returns the index of this.
    pub fn idx(&self) -> TriIteratorIdx {
        self.idx
    }

    /// Get the hedges of this triangle.
    // s. self.nodes() for a small explanation of the index calculation
    pub fn hedges(&self) -> [HedgeIterator<'a>; 3] {
        [
            HedgeIterator::new(self.tds, self.idx * 3),
            HedgeIterator::new(self.tds, self.idx * 3 + 1),
            HedgeIterator::new(self.tds, self.idx * 3 + 2),
        ]
    }

    /// Check if the triangle is casual, i.e. all nodes are casual
    pub fn is_casual(&self) -> bool {
        !self.is_conceptual()
    }

    /// Check if the triangle is conceptual, i.e. one of the nodes is the infinite node
    pub fn is_conceptual(&self) -> bool {
        self.hedges()
            .iter()
            .any(|hedge| hedge.starting_node().is_conceptual())
    }

    /// Check if the triangle is deleted, i.e. one of the nodes is a deleted node, which means all nodes are deleted
    pub fn is_deleted(&self) -> bool {
        self.hedges()
            .iter()
            .any(|hedge| hedge.starting_node().is_deleted())
    }

    /// Get the nodes of this triangle.
    // Since the hedges and nodes are stored index-wise like this in the tds:
    //
    //  tri0                  tri1
    //    |                     |
    //    v                     v
    // [node0, node1, node2, node3, node4, node5, ... ]
    // [hedge0, hedge1, hedge2, hedge3, hedge4, hedge5, ... ]
    //
    // the indices of the nodes can be retrieved by multiplying the triangle index by 3
    pub fn nodes(&self) -> [VertexNode; 3] {
        [
            self.tds.hedge_starting_nodes[self.idx() * 3],
            self.tds.hedge_starting_nodes[self.idx() * 3 + 1],
            self.tds.hedge_starting_nodes[self.idx() * 3 + 2],
        ]
    }
}

impl fmt::Display for TriIterator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Triangle {}: {} -> {} -> {}",
            self.idx(),
            self.nodes()[0],
            self.nodes()[1],
            self.nodes()[2]
        )
    }
}
