use crate::{utils::types::TetIteratorIdx, VertexNode};

use super::{half_tri_iterator::HalfTriIterator, tet_data_structure::TetDataStructure};

pub struct TetIterator<'a> {
    pub tds: &'a TetDataStructure,
    pub tet_idx: usize,
}

impl<'a> TetIterator<'a> {
    pub const fn half_triangles(&self) -> [HalfTriIterator<'a>; 4] {
        let idx_first = self.idx() << 2;

        [
            HalfTriIterator {
                tds: self.tds,
                half_tri_idx: idx_first,
            },
            HalfTriIterator {
                tds: self.tds,
                half_tri_idx: idx_first + 1,
            },
            HalfTriIterator {
                tds: self.tds,
                half_tri_idx: idx_first + 2,
            },
            HalfTriIterator {
                tds: self.tds,
                half_tri_idx: idx_first + 3,
            },
        ]
    }

    pub const fn idx(&self) -> TetIteratorIdx {
        self.tet_idx
    }

    /// Check if the tetrahedron is casual, i.e. all nodes are casual
    pub fn is_casual(&self) -> bool {
        !self.is_conceptual()
    }

    /// Check if the tetrahedron is conceptual, i.e. one of the nodes is the infinite node
    pub fn is_conceptual(&self) -> bool {
        self.nodes().iter().any(VertexNode::is_conceptual)
    }

    pub fn is_sound(&self) -> bool {
        if self.should_del() || self.should_keep() {
            log::error!("{self}: tetrahedron remaining after triangulation.");
            return false;
        }

        let [n0, n1, n2, n3] = self.nodes();

        let mut sound = true;

        if n0 == n1 || n0 == n2 || n0 == n3 || n1 == n2 || n1 == n3 || n2 == n3 {
            log::error!("{self}: tetrahedron with duplicate nodes.");
            sound = false;
        }

        sound
    }

    pub fn nodes(&self) -> [VertexNode; 4] {
        let idx_first = self.idx() << 2; // this is equivalent to self.tet_idx * 4 (rounding down to nearest integer), but faster
        [
            self.tds.tet_nodes[idx_first],
            self.tds.tet_nodes[idx_first + 1],
            self.tds.tet_nodes[idx_first + 2],
            self.tds.tet_nodes[idx_first + 3],
        ]
    }

    pub fn should_del(&self) -> bool {
        self.tds.should_del_tet[self.idx()]
    }

    pub fn should_keep(&self) -> bool {
        self.tds.should_keep_tet[self.idx()]
    }
}

impl std::fmt::Display for TetIterator<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let [n0, n1, n2, n3] = self.nodes();
        write!(
            f,
            "Tetrahedron {}: {} -> {} -> {} -> {}",
            self.idx(),
            n0,
            n1,
            n2,
            n3
        )
    }
}
