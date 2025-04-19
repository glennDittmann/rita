use crate::VertexNode;
use anyhow::{Ok, Result};

use super::{
    half_tri_iterator::HalfTriIterator, hedge_iterator::HedgeIterator, tet_iterator::TetIterator,
};

// For each tri idx within a tet, associate list of vertex idx triples, i.e. the face indices
/// For each triangle index within tetrahedron, associate list of vertices within tetrahedron
pub(crate) const TRIANGLE_SUBINDICES: [[usize; 3]; 4] = [[1, 3, 2], [0, 2, 3], [0, 3, 1], [0, 1, 2]];

/// For each triangle index, for each halfedge index, associate triangle and halfedge index within
/// tetrahedron
pub(crate) const NEIGHBOR_HALFEDGE: [[(usize, usize); 3]; 4] = [
    [(2, 1), (1, 1), (3, 1)],
    [(3, 2), (0, 1), (2, 0)],
    [(1, 2), (0, 0), (3, 0)],
    [(2, 2), (0, 2), (1, 0)],
];

// Flips in 3D
// OneToFour,
// FourToOne,
// TwoToThree,
// ThreeToTwo,

/// A 3D triangulation data structure.
///
/// The edges are stored in a doubly-connected edge list (DCEL) manner.
///
/// ```ignore
/// i   --> node0 \
/// |        |      \
/// v        v       |
/// i+1 --> node1 ---|
/// |        |       |-->  tetrahedron
/// v        v       |
/// i+2 --> node2 ---|
/// |        |       |
/// v        v      /
/// i+3 --> node3 /
/// ```
//
// such that `tri0 = (i+1, i+3, i+2)`
//
// such that `tri1 = (i, i+2, i+3)`
//
// such that `tri2 = (i, i+3, i+1)`
//
// such that `tri3 = (i, i+1, i+2)`
#[derive(Debug)]
pub struct TetDataStructure {
    pub tet_nodes: Vec<VertexNode>,
    /// Opposite half triangle index of this tet
    pub(crate) half_tri_opposite: Vec<usize>,

    num_tets: usize,

    // structures to speed up tetrahedra insertion with Bowyer Watson algorithm
    pub(crate) should_del_tet: Vec<bool>,
    pub(crate) should_keep_tet: Vec<bool>,
    tets_to_del: Vec<usize>,
    tets_to_keep: Vec<usize>,
    tets_to_check: Vec<usize>,
}

impl Default for TetDataStructure {
    fn default() -> Self {
        Self::new()
    }
}

impl TetDataStructure {
    /// Simplicial structure initialisation
    pub const fn new() -> Self {
        Self {
            tet_nodes: Vec::new(),
            half_tri_opposite: Vec::new(),
            num_tets: 0,
            should_del_tet: Vec::new(),
            should_keep_tet: Vec::new(),
            tets_to_del: Vec::new(),
            tets_to_keep: Vec::new(),
            tets_to_check: Vec::new(),
        }
    }

    const fn hedge(&self, ind_halftriangle: usize, ind_halfedge: usize) -> HedgeIterator<'_> {
        // TODO: remove this, this is just HedgeIterator::new(self, ind_halftriangle, ind_halfedge)
        HedgeIterator {
            tds: self,
            half_tri_idx: ind_halftriangle,
            hedge_idx: ind_halfedge,
        }
    }

    const fn half_triangle(&self, ind_halftriangle: usize) -> HalfTriIterator {
        // TODO: remove this, this is just HalfTriIterator::new(self, ind_halftriangle, ind_halfedge)
        HalfTriIterator {
            tds: self,
            half_tri_idx: ind_halftriangle,
        }
    }

    /// Gets halfedge iterator from index
    pub fn get_half_tri(&self, half_tri_idx: usize) -> Result<HalfTriIterator> {
        if half_tri_idx < self.half_tri_opposite.len() {
            Ok(self.half_triangle(half_tri_idx))
        } else {
            Err(anyhow::Error::msg(
                "Halftriangle value not in tetrahedron data structure",
            ))
        }
    }

    /// Get the number of triangles in the triangulation, without the ones connected to the dummy point.
    pub fn num_casual_tets(&self) -> usize {
        let mut num_casual_tets = 0;
        for i in 0..self.num_tets() {
            let tri = self.get_tet(i).unwrap();
            let [n0, n1, n2, n3] = tri.nodes();

            if !tri.is_conceptual()
                && n0.idx().is_some()
                && n1.idx().is_some()
                && n2.idx().is_some()
                && n3.idx().is_some()
            {
                num_casual_tets += 1;
            }
        }
        num_casual_tets
    }

    const fn tet(&self, ind_tetrahedron: usize) -> TetIterator {
        // TODO: remove this, this is just TetIterator::new(self, ind_halftriangle, ind_halfedge)
        TetIterator {
            tds: self,
            tet_idx: ind_tetrahedron,
        }
    }

    /// Gets tetrahedron iterator from index
    pub fn get_tet(&self, ind_tetrahedron: usize) -> Result<TetIterator> {
        if ind_tetrahedron < self.num_tets {
            Ok(self.tet(ind_tetrahedron))
        } else {
            Err(anyhow::Error::msg("Tetrahedron value not in simplicial"))
        }
    }

    /// Gets number of triangles
    pub const fn num_tets(&self) -> usize {
        self.num_tets
    }

    /// Gets halfedges containing a pair of nodes
    pub fn get_hedge_containing(
        &self,
        node0: &VertexNode,
        node1: &VertexNode,
    ) -> Vec<HedgeIterator> {
        let mut hedges = Vec::new();

        for i in 0..self.num_tets() {
            let first_node = i << 2;
            let mut sub_ind_v0 = 4;
            let mut sub_ind_v1 = 4;

            for j in 0..4 {
                if self.tet_nodes[first_node + j] == *node0 {
                    sub_ind_v0 = j;
                } else if self.tet_nodes[first_node + j] == *node1 {
                    sub_ind_v1 = j;
                }
            }

            if sub_ind_v0 == 4 || sub_ind_v1 == 4 {
                continue;
            }

            for (j, tri_sub_idxs) in TRIANGLE_SUBINDICES.iter().enumerate() {
                if j == sub_ind_v0 || j == sub_ind_v1 {
                    continue;
                }

                for (k, tri_sub_idx) in tri_sub_idxs.iter().enumerate() {
                    if *tri_sub_idx == sub_ind_v0 && tri_sub_idxs[(k + 1) % 3] == sub_ind_v1 {
                        hedges.push(self.hedge(first_node + j, k));
                        break;
                    }
                }
            }
        }

        hedges
    }

    /// Gets halftriangle containing a triple of nodes
    pub fn get_half_tri_containing(
        &self,
        node1: &VertexNode,
        node2: &VertexNode,
        node3: &VertexNode,
    ) -> Option<HalfTriIterator> {
        for i in 0..self.num_tets {
            let first_node = i << 2;
            let mut sub_ind_v0 = 4;
            let mut sub_ind_v1 = 4;
            let mut sub_ind_v2 = 4;

            for j in 0..4 {
                if self.tet_nodes[first_node + j] == *node1 {
                    sub_ind_v0 = j;
                } else if self.tet_nodes[first_node + j] == *node2 {
                    sub_ind_v1 = j;
                } else if self.tet_nodes[first_node + j] == *node3 {
                    sub_ind_v2 = j;
                }
            }

            if sub_ind_v0 == 4 || sub_ind_v1 == 4 || sub_ind_v2 == 4 {
                continue;
            }

            for (j, tri_sub_idxs) in TRIANGLE_SUBINDICES.iter().enumerate() {
                if j == sub_ind_v0 || j == sub_ind_v1 || j == sub_ind_v2 {
                    continue;
                }

                for (k, tri_sub_idx) in tri_sub_idxs.iter().enumerate() {
                    if *tri_sub_idx == sub_ind_v0
                        && tri_sub_idxs[(k + 1) % 3] == sub_ind_v1
                        && tri_sub_idxs[(k + 2) % 3] == sub_ind_v2
                    {
                        return Some(self.half_triangle(first_node + j));
                    }
                }

                return Some(self.half_triangle(first_node + j).opposite());
            }
        }

        None
    }

    /// Gets tetrahedra containing a specific node
    pub fn get_tet_containing(&self, node: &VertexNode) -> Vec<TetIterator> {
        let mut tets = Vec::new();

        for i in 0..self.num_tets {
            let first_node = i << 2;

            for j in 0..4 {
                if self.tet_nodes[first_node + j] == *node {
                    tets.push(self.tet(i));
                    break;
                }
            }
        }

        tets
    }

    /// Starts BW insertion, setting a first tetrahedron to remove
    pub fn bw_start(&mut self, first_tet_idx: usize) -> Result<()> {
        if !self.tets_to_check.is_empty() || !self.tets_to_keep.is_empty() {
            return Err(anyhow::Error::msg(
                "Bowyer Watson algorithm already started",
            ));
        }

        self.bw_rem_tet(first_tet_idx);

        Ok(())
    }

    /// Gets next tetrahedron to check
    pub fn bw_tets_to_check(&mut self) -> Option<usize> {
        while let Some(tet_idx) = self.tets_to_check.pop() {
            if !self.should_del_tet[tet_idx] && !self.should_keep_tet[tet_idx] {
                return Some(tet_idx);
            }
        }
        None
    }

    /// Sets tetrahedron to remove
    pub fn bw_rem_tet(&mut self, tet_idx: usize) {
        let tri0 = tet_idx << 2;
        let tri1 = tri0 + 1;
        let tri2 = tri0 + 2;
        let tri3 = tri0 + 3;

        let opp_tri0 = self.half_tri_opposite[tri0];
        let opp_tri1 = self.half_tri_opposite[tri1];
        let opp_tri2 = self.half_tri_opposite[tri2];
        let opp_tri3 = self.half_tri_opposite[tri3];

        self.tets_to_check.push(opp_tri0 >> 2);
        self.tets_to_check.push(opp_tri1 >> 2);
        self.tets_to_check.push(opp_tri2 >> 2);
        self.tets_to_check.push(opp_tri3 >> 2);

        self.should_del_tet[tet_idx] = true;
        self.tets_to_del.push(tet_idx);
    }

    /// Sets tetrahedron to keep
    pub fn bw_keep_tetra(&mut self, tet_idx: usize) -> Result<()> {
        self.should_keep_tet[tet_idx] = true;
        self.tets_to_keep.push(tet_idx);

        Ok(())
    }

    /// BW insertion algorithm
    pub fn bw_insert_node(&mut self, nod: VertexNode) -> Result<Vec<usize>> {
        if !self.tets_to_check.is_empty() {
            return Err(anyhow::Error::msg(
                "Cannot insert node if all tetrahedra are not checked",
            ));
        }

        // 1 - find boundary triangle
        let ind_tri_first = if let Some(&ind_tetra_keep) = self.tets_to_keep.last() {
            let tetra = self.tet(ind_tetra_keep);
            let tris = tetra.half_triangles();
            if tris[0].opposite().tet().should_del() {
                tris[0].idx()
            } else if tris[1].opposite().tet().should_del() {
                tris[1].idx()
            } else if tris[2].opposite().tet().should_del() {
                tris[2].idx()
            } else if tris[3].opposite().tet().should_del() {
                tris[3].idx()
            } else {
                return Err(anyhow::Error::msg("Isolated kept tetrahedron"));
            }
        } else {
            return Err(anyhow::Error::msg("No kept tetrahedron"));
        };

        // 2 - build boundary triangles graph
        let mut vec_tri = vec![ind_tri_first];
        let mut vec_nei: Vec<[Option<usize>; 3]> = vec![[None; 3]];
        let mut ind_cur = 0;
        loop {
            let cur_tri = HalfTriIterator {
                tds: self,
                half_tri_idx: vec_tri[ind_cur],
            };

            let hedges = cur_tri.hedges();

            for (j, hedge) in hedges.iter().enumerate() {
                if vec_nei[ind_cur][j].is_none() {
                    let mut he_cur = hedge.opposite().neighbor().opposite();

                    let (ind_cur2, j2) = loop {
                        if !he_cur.tri().tet().should_del() {
                            let ind_tri2 = he_cur.tri().idx();
                            let j2 = he_cur.idx();
                            let ind_cur2 = if let Some((i2, _)) = vec_tri.iter()
                                .enumerate().find(|&(_, &ind)| ind == ind_tri2)
                            {
                                i2
                            } else {
                                vec_tri.push(ind_tri2);
                                vec_nei.push([None; 3]);
                                vec_tri.len() - 1
                            };
                            break (ind_cur2, j2);
                        } else {
                            he_cur = he_cur.neighbor().opposite();
                        }
                    };

                    vec_nei[ind_cur][j] = Some(ind_cur2);
                    vec_nei[ind_cur2][j2] = Some(ind_cur);
                }
            }
            ind_cur += 1;
            if ind_cur >= vec_tri.len() {
                break;
            }
        }

        let mut added_tets = Vec::with_capacity(vec_tri.len());
        // 3 - create tetrahedra
        for i in &vec_tri {
            let cur_tri = HalfTriIterator {
                tds: self,
                half_tri_idx: *i,
            };

            let [nod0, nod1, nod2] = cur_tri.nodes();

            if let Some(ind_add) = self.tets_to_del.pop() {
                added_tets.push(ind_add);
                self.replace_tet(ind_add, nod0, nod2, nod1, nod);
            } else {
                added_tets.push(self.num_tets());
                self.half_tri_opposite.push(0);
                self.half_tri_opposite.push(0);
                self.half_tri_opposite.push(0);
                self.half_tri_opposite.push(0);
                self.insert_tet(nod0, nod2, nod1, nod);
            };
        }

        // 4 - create links
        for i in 0..vec_tri.len() {
            let (tri0, tri1, tri2, tri3) = (
                added_tets[i] * 4,
                added_tets[i] * 4 + 1,
                added_tets[i] * 4 + 2,
                added_tets[i] * 4 + 3,
            );

            let ind_tri_nei = vec_tri[i];

            let ind_nei0 = vec_nei[i][1].unwrap();
            let ind_nei1 = vec_nei[i][0].unwrap();
            let ind_nei2 = vec_nei[i][2].unwrap();

            let ind_tet_nei0 = added_tets[ind_nei0];
            let ind_tet_nei1 = added_tets[ind_nei1];
            let ind_tet_nei2 = added_tets[ind_nei2];

            let ind_tri0_nei = if vec_nei[ind_nei0][0] == Some(i) {
                ind_tet_nei0 * 4 + 1
            } else if vec_nei[ind_nei0][1] == Some(i) {
                ind_tet_nei0 * 4
            } else {
                ind_tet_nei0 * 4 + 2
            };
            let ind_tri1_nei = if vec_nei[ind_nei1][0] == Some(i) {
                ind_tet_nei1 * 4 + 1
            } else if vec_nei[ind_nei1][1] == Some(i) {
                ind_tet_nei1 * 4
            } else {
                ind_tet_nei1 * 4 + 2
            };
            let ind_tri2_nei = if vec_nei[ind_nei2][0] == Some(i) {
                ind_tet_nei2 * 4 + 1
            } else if vec_nei[ind_nei2][1] == Some(i) {
                ind_tet_nei2 * 4
            } else {
                ind_tet_nei2 * 4 + 2
            };

            self.half_tri_opposite[tri0] = ind_tri0_nei;
            self.half_tri_opposite[tri1] = ind_tri1_nei;
            self.half_tri_opposite[tri2] = ind_tri2_nei;
            self.half_tri_opposite[tri3] = ind_tri_nei;
            self.half_tri_opposite[ind_tri_nei] = tri3;
        }

        while let Some(ind_tetra_keep) = self.tets_to_keep.pop() {
            self.should_keep_tet[ind_tetra_keep] = false;
        }

        Ok(added_tets)
    }

    /// Clean removed tetrahedra
    pub fn clean_to_del(&mut self) -> Result<()> {
        self.tets_to_del.sort_unstable();

        while let Some(tet_to_del_idx) = self.tets_to_del.pop() {
            self.should_del_tet[tet_to_del_idx] = false;
            self.mov_end_tet(tet_to_del_idx)?;
        }

        Ok(())
    }

    fn insert_tet(
        &mut self,
        nod1: VertexNode,
        nod2: VertexNode,
        nod3: VertexNode,
        nod4: VertexNode,
    ) -> (usize, usize, usize, usize) {
        let idx0 = self.tet_nodes.len();

        self.tet_nodes.push(nod1);
        self.tet_nodes.push(nod2);
        self.tet_nodes.push(nod3);
        self.tet_nodes.push(nod4);

        self.should_del_tet.push(false);
        self.should_keep_tet.push(false);

        self.num_tets += 1;

        (idx0, idx0 + 1, idx0 + 2, idx0 + 3)
    }

    fn replace_tet(
        &mut self,
        tet_idx: usize,
        nod1: VertexNode,
        nod2: VertexNode,
        nod3: VertexNode,
        nod4: VertexNode,
    ) -> (usize, usize, usize, usize) {
        let idx0 = tet_idx * 4;

        self.tet_nodes[idx0] = nod1;
        self.tet_nodes[idx0 + 1] = nod2;
        self.tet_nodes[idx0 + 2] = nod3;
        self.tet_nodes[idx0 + 3] = nod4;

        self.should_del_tet[tet_idx] = false;
        self.should_keep_tet[tet_idx] = false;

        (idx0, idx0 + 1, idx0 + 2, idx0 + 3)
    }

    fn mov_end_tet(&mut self, tet_idx: usize) -> Result<()> {
        if tet_idx != self.num_tets - 1 {
            let opp_tri_idx0 = self.half_tri_opposite[self.half_tri_opposite.len() - 4];
            let opp_tri_idx1 = self.half_tri_opposite[self.half_tri_opposite.len() - 3];
            let opp_tri_idx2 = self.half_tri_opposite[self.half_tri_opposite.len() - 2];
            let opp_tri_idx3 = self.half_tri_opposite[self.half_tri_opposite.len() - 1];

            let [node0, node1, node2, node3] = self.tet(self.num_tets - 1).nodes();

            let (tri_idx0, tri_idx1, tri_idx2, tri_idx3) =
                self.replace_tet(tet_idx, node0, node1, node2, node3);

            self.half_tri_opposite[tri_idx0] = opp_tri_idx0;
            self.half_tri_opposite[tri_idx1] = opp_tri_idx1;
            self.half_tri_opposite[tri_idx2] = opp_tri_idx2;
            self.half_tri_opposite[tri_idx3] = opp_tri_idx3;

            self.half_tri_opposite[opp_tri_idx0] = tri_idx0;
            self.half_tri_opposite[opp_tri_idx1] = tri_idx1;
            self.half_tri_opposite[opp_tri_idx2] = tri_idx2;
            self.half_tri_opposite[opp_tri_idx3] = tri_idx3;
        }

        self.tet_nodes.pop();
        self.tet_nodes.pop();
        self.tet_nodes.pop();
        self.tet_nodes.pop();

        self.half_tri_opposite.pop();
        self.half_tri_opposite.pop();
        self.half_tri_opposite.pop();
        self.half_tri_opposite.pop();

        self.should_del_tet.pop();
        self.should_keep_tet.pop();

        self.num_tets -= 1;

        Ok(())
    }

    /// Inserts a first tetrahedron in the structure
    pub fn insert_first_tet(&mut self, nodes: [usize; 4]) -> Result<[TetIterator; 4]> {
        if self.num_tets != 0 {
            return Err(anyhow::Error::msg("Already tetrahedra in simplicial"));
        }

        let node0 = VertexNode::Casual(nodes[0]);
        let node1 = VertexNode::Casual(nodes[1]);
        let node2 = VertexNode::Casual(nodes[2]);
        let node3 = VertexNode::Casual(nodes[3]);
        let node_conceptual = VertexNode::Conceptual;

        let first_tetra = self.num_tets; // aka 0, because we early out above, when it is not zero, it must be zero here !

        let (t132, t023, t031, t012) = self.insert_tet(node0, node1, node2, node3);
        let (t2i3, t13i, t1i2, t123) = self.insert_tet(node1, node2, node3, node_conceptual);
        let (t3i2, t02i, t0i3, t032) = self.insert_tet(node0, node3, node2, node_conceptual);
        let (t1i3, t03i, t0i1, t013) = self.insert_tet(node0, node1, node3, node_conceptual);
        let (t2i1, t01i, t0i2, t021) = self.insert_tet(node0, node2, node1, node_conceptual);

        self.half_tri_opposite.push(t123); // t132
        self.half_tri_opposite.push(t032); // t023
        self.half_tri_opposite.push(t013); // t031
        self.half_tri_opposite.push(t021); // t012
        self.half_tri_opposite.push(t3i2); // t2i3
        self.half_tri_opposite.push(t1i3); // t13i
        self.half_tri_opposite.push(t2i1); // t1i2
        self.half_tri_opposite.push(t132); // t123
        self.half_tri_opposite.push(t2i3); // t3i2
        self.half_tri_opposite.push(t0i2); // t02i
        self.half_tri_opposite.push(t03i); // t0i3
        self.half_tri_opposite.push(t023); // t032
        self.half_tri_opposite.push(t13i); // t1i3
        self.half_tri_opposite.push(t0i3); // t03i
        self.half_tri_opposite.push(t01i); // t0i1
        self.half_tri_opposite.push(t031); // t013
        self.half_tri_opposite.push(t1i2); // t2i1
        self.half_tri_opposite.push(t0i1); // t01i
        self.half_tri_opposite.push(t02i); // t0i2
        self.half_tri_opposite.push(t012); // t021

        Ok([
            TetIterator {
                tds: self,
                tet_idx: first_tetra,
            },
            TetIterator {
                tds: self,
                tet_idx: first_tetra + 1,
            },
            TetIterator {
                tds: self,
                tet_idx: first_tetra + 2,
            },
            TetIterator {
                tds: self,
                tet_idx: first_tetra + 3,
            },
        ])
    }

    /// Checks soundness of tetrahedral graph
    pub fn is_sound(&self) -> Result<bool> {
        let mut sound = true;

        for tet_idx in 0..self.num_tets() {
            let tet = self.get_tet(tet_idx)?;

            sound = sound && tet.is_sound();

            for tri in tet.half_triangles() {
                sound = sound && tri.is_sound();
                for he in tri.hedges() {
                    sound = sound && he.is_sound();
                }
            }
        }

        Ok(sound)
    }
}

impl std::fmt::Display for TetDataStructure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for idx in 0..self.num_tets {
            write!(f, "Tet {}: {}", idx, self.tet(idx))?;
        }

        write!(f, "TetDataStructure")
    }
}
