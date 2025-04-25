use core::fmt;
#[cfg(feature = "logging")]
use log::error;
use core::cmp::Ordering;

use crate::{utils::types::HedgeIteratorIdx, VertexNode};
use super::{tri_data_structure::TriDataStructure, tri_iterator::TriIterator};

/// An iterator over the half-edges of a triangulation data structure.
#[derive(Clone)]
pub struct HedgeIterator<'a> {
    pub tds: &'a TriDataStructure,
    /// The hedge idx of this iterator
    pub idx: HedgeIteratorIdx,
}

impl<'a> HedgeIterator<'a> {
    pub const fn new(tds: &'a TriDataStructure, idx: HedgeIteratorIdx) -> Self {
        Self { tds, idx }
    }

    /// Retrieve the node this hedge originates from.
    pub fn starting_node(&self) -> VertexNode {
        self.tds.hedge_starting_nodes[self.idx]
    }

    /// Check if the hedge is conceptual, i.e. one of the nodes is the infinite node
    pub fn is_conceptual(&self) -> bool {
        self.starting_node().is_conceptual() || self.end_node().is_conceptual()
    }

    /// Check if the hedge is sound, i.e. `next`, `prev` and `twin` are pointing to correct nodes.
    pub fn is_sound(&self) -> bool {
        let mut sound = true;

        let starting_node = self.starting_node();
        let end_node = self.end_node();

        #[allow(unused)]
        let mut check = |condition: bool, error_msg: &str| {
            if !condition {
                #[cfg(feature = "logging")]
                error!("{self}: {error_msg}");
                sound = false;
            }
        };

        check(self.next().starting_node() == end_node, "Wrong next hedge");
        check(self.prev().end_node() == starting_node, "Wrong prev hedge");
        check(
            self.twin().starting_node() == end_node && self.twin().end_node() == starting_node,
            "Wrong twin hedge",
        );

        sound
    }

    /// Retrieve the node this hedge ends at.
    pub fn end_node(&self) -> VertexNode {
        match (self.idx % 3).cmp(&2) {
            Ordering::Equal => self.tds.hedge_starting_nodes[self.idx - 2],
            Ordering::Greater | Ordering::Less => self.tds.hedge_starting_nodes[self.idx + 1], // TODO: can this be greater, x % 3 is always 0, 1 or 2
        }
    }

    /// Retrieve the `next` half-edge belonging to the same triangle.
    pub fn next(&self) -> HedgeIterator<'a> {
        match (self.idx % 3).cmp(&2) {
            Ordering::Equal => Self::new(self.tds, self.idx - 2),
            Ordering::Greater | Ordering::Less => Self::new(self.tds, self.idx + 1),
        }
    }

    /// Retrieve the `twin` (aka opposite) half-edge belonging to the same triangle.
    ///
    /// This is the hedge that goes in the opposite direction,
    ///
    /// i.e. `self.starting_node() == self.twin().end_node()` and the other way around.
    pub fn twin(&self) -> HedgeIterator<'a> {
        Self::new(self.tds, self.tds.hedge_twins[self.idx])
    }

    /// Retrieve the `previous` half-edge belonging to the same triangle.
    pub fn prev(&self) -> HedgeIterator<'a> {
        match (self.idx % 3).cmp(&0) {
            Ordering::Equal => Self::new(self.tds, self.idx + 2),
            Ordering::Greater | Ordering::Less => Self::new(self.tds, self.idx - 1),
        }
    }

    /// Retrieve the triangle this half-edge belongs to.
    pub const fn tri(&self) -> TriIterator<'a> {
        TriIterator::new(self.tds, self.idx / 3)
    }
}

impl fmt::Display for HedgeIterator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Edge {}: {} -> {}",
            self.idx,
            self.starting_node(),
            self.end_node()
        )
    }
}
