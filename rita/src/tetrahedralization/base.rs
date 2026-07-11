use alloc::vec::Vec;

use anyhow::Result as HowResult;

use super::{ExtendedTetrahedron, Tetrahedralization};
use crate::{
    VertexNode,
    tetds::tet_data_structure::TetDataStructure,
    utils::types::{Tetrahedron3, Triangle3, Vertex3},
};

impl Tetrahedralization {
    pub const fn new(epsilon: Option<f64>) -> Self {
        Self {
            epsilon,
            tds: TetDataStructure::new(),
            vertices: Vec::new(),
            weights: None,
            #[cfg(feature = "timing")]
            time_hilbert: 0,
            #[cfg(feature = "timing")]
            time_walking: 0,
            #[cfg(feature = "timing")]
            time_inserting: 0,
            used_vertices: Vec::new(),
            ignored_vertices: Vec::new(),
        }
    }

    /// Create a new `Tetrahedralization` with a pre-allocated capacity for vertices
    pub fn new_with_vert_capacity(epsilon: Option<f64>, capacity: usize) -> Self {
        let mut tetrahedralization = Self::new(epsilon);
        tetrahedralization.vertices = Vec::with_capacity(capacity);
        tetrahedralization
    }

    pub(crate) const fn weighted(&self) -> bool {
        self.weights.is_some()
    }

    /// Gets the height for a vertex
    pub fn height(&self, v_idx: usize) -> f64 {
        self.vertices[v_idx][0].powi(2)
            + self.vertices[v_idx][1].powi(2)
            + self.vertices[v_idx][2].powi(2)
            - self.weights.as_ref().map_or(0.0, |weights| weights[v_idx])
    }

    /// The number of triangles, without the ones that have an connection to the dummy point.
    pub fn num_casual_tets(&self) -> usize {
        self.tds().num_casual_tets()
    }

    pub fn num_ignored_vertices(&self) -> usize {
        self.ignored_vertices.len()
    }

    pub const fn num_tets(&self) -> usize {
        self.tds.num_tets()
    }

    pub fn num_used_vertices(&self) -> usize {
        self.used_vertices.len()
    }

    pub const fn tds(&self) -> &TetDataStructure {
        &self.tds
    }

    fn vertex_from_node(&self, node: VertexNode) -> HowResult<Vertex3> {
        match node {
            VertexNode::Casual(idx) => Ok(self.vertices[idx]),
            _ => Err(anyhow::Error::msg("Expected a casual vertex node")),
        }
    }

    fn triangle_from_nodes(&self, nodes: [VertexNode; 3]) -> HowResult<Triangle3> {
        let [node0, node1, node2] = nodes;
        Ok([
            self.vertex_from_node(node0)?,
            self.vertex_from_node(node1)?,
            self.vertex_from_node(node2)?,
        ])
    }

    fn tetrahedron_from_nodes(&self, nodes: [VertexNode; 4]) -> HowResult<Tetrahedron3> {
        let [node0, node1, node2, node3] = nodes;
        Ok([
            self.vertex_from_node(node0)?,
            self.vertex_from_node(node1)?,
            self.vertex_from_node(node2)?,
            self.vertex_from_node(node3)?,
        ])
    }

    /// Get the tetrahedra of the tetrahedralization as `Tetrahedron3`, i.e `[[f64; 3]; 4]`.
    ///
    /// Does not include conceptual tetrahedra, i.e. the convex hull faces
    /// connected to the point at infinity.
    pub fn tets(&self) -> Vec<Tetrahedron3> {
        // todo: handle the results gracefully, instead of unwrapping or .ok() (which is safe here though)
        (0..self.tds().num_tets())
            .filter_map(|tet_idx| {
                let tet = self.tds().get_tet(tet_idx).ok()?;

                if tet.is_conceptual() {
                    return None;
                }

                self.tetrahedron_from_nodes(tet.nodes()).ok()
            })
            .collect()
    }

    pub const fn vertices(&self) -> &Vec<Vertex3> {
        &self.vertices
    }

    /// Gets extended tetrahedron from index
    pub fn get_tet_as_extended(&self, tet_idx: usize) -> HowResult<ExtendedTetrahedron> {
        let [node0, node1, node2, node3] = self.tds().get_tet(tet_idx)?.nodes();

        let ext_tri = match (node0, node1, node2, node3) {
            (
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => ExtendedTetrahedron::Triangle(self.triangle_from_nodes([
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx3),
                VertexNode::Casual(v_idx2),
            ])?),
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ) => ExtendedTetrahedron::Triangle(self.triangle_from_nodes([
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx2),
                VertexNode::Casual(v_idx3),
            ])?),
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Conceptual,
                VertexNode::Casual(v_idx3),
            ) => ExtendedTetrahedron::Triangle(self.triangle_from_nodes([
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx3),
                VertexNode::Casual(v_idx1),
            ])?),
            (
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
                VertexNode::Conceptual,
            ) => ExtendedTetrahedron::Triangle(self.triangle_from_nodes([
                VertexNode::Casual(v_idx0),
                VertexNode::Casual(v_idx1),
                VertexNode::Casual(v_idx2),
            ])?),
            (
                VertexNode::Casual(..),
                VertexNode::Casual(..),
                VertexNode::Casual(..),
                VertexNode::Casual(..),
            ) => ExtendedTetrahedron::Tetrahedron(
                self.tetrahedron_from_nodes([node0, node1, node2, node3])?,
            ),
            (_, _, _, _) => {
                return Err(anyhow::Error::msg("Case should not happen"));
            }
        };

        Ok(ext_tri)
    }

    pub const fn used_vertices(&self) -> &Vec<usize> {
        &self.used_vertices
    }
}
