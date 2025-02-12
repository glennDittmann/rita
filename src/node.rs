use crate::utils::types::VertexIdx;
use core::fmt;

/// A dcel / half-edge vertex node.
///
/// A `casual node` has an index into the input vertex list.
///
/// A `conceptual node` is at infinity. Geometric operations are handled accordingly.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum VertexNode {
    Casual(VertexIdx),
    Conceptual,
    Deleted,
}

impl VertexNode {
    /// Get the index of the node.
    pub fn idx(&self) -> Option<VertexIdx> {
        match self {
            VertexNode::Casual(idx) => Some(*idx),
            _ => None,
        }
    }

    /// Check if the node is conceptual.
    pub fn is_conceptual(&self) -> bool {
        matches!(self, VertexNode::Conceptual)
    }

    /// Check if the node is deleted.
    pub fn is_deleted(&self) -> bool {
        matches!(self, VertexNode::Deleted)
    }
}

impl fmt::Display for VertexNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VertexNode::Casual(v_idx) => write!(f, "Casual({})", v_idx),
            VertexNode::Conceptual => write!(f, "Conceptual"),
            VertexNode::Deleted => write!(f, "Deleted"),
        }
    }
}
