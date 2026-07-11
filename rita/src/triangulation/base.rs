use alloc::vec::Vec;

use anyhow::{Ok as HowOk, Result as HowResult};

use super::{TriangleExtended, Triangulation};
use crate::{
    VertexNode,
    trids::tri_data_structure::TriDataStructure,
    utils::types::{Triangle2, Vertex2, VertexIdx},
};

impl Triangulation {
    pub const fn new(epsilon: Option<f64>) -> Self {
        Self {
            tds: TriDataStructure::new(),
            vertices: Vec::new(),
            weights: None,
            #[cfg(feature = "timing")]
            time_flipping: 0,
            #[cfg(feature = "timing")]
            time_inserting: 0,
            #[cfg(feature = "timing")]
            time_walking: 0,
            last_inserted_triangle: None,
            epsilon,
            used_vertices: Vec::new(),
            ignored_vertices: Vec::new(),
            redundant_vertices: Vec::new(),
        }
    }

    /// Create a new `Triangulation` with a pre-allocated capacity for vertices
    pub fn new_with_vert_capacity(epsilon: Option<f64>, capacity: usize) -> Self {
        let mut triangulation = Self::new(epsilon);
        triangulation.vertices = Vec::with_capacity(capacity);
        triangulation
    }

    pub(crate) const fn weighted(&self) -> bool {
        self.weights.is_some()
    }

    fn vertex_from_node(&self, node: VertexNode) -> HowResult<Vertex2> {
        match node {
            VertexNode::Casual(idx) => HowOk(self.vertices[idx]),
            _ => Err(anyhow::Error::msg("Expected a casual vertex node")),
        }
    }

    fn triangle_from_nodes(&self, nodes: [VertexNode; 3]) -> HowResult<Triangle2> {
        let [node0, node1, node2] = nodes;
        HowOk([
            self.vertex_from_node(node0)?,
            self.vertex_from_node(node1)?,
            self.vertex_from_node(node2)?,
        ])
    }

    /// For a tri idx get the triangle variant, i.e. a normal triangle, or a line with one of its three indices at infinity
    pub fn get_tri_type(&self, tri_idx: usize) -> HowResult<TriangleExtended> {
        let [node0, node1, node2] = self.tds.get_tri(tri_idx)?.nodes();

        let tri_extended = match (node0, node1, node2) {
            (VertexNode::Conceptual, VertexNode::Casual(idx1), VertexNode::Casual(idx2)) => {
                TriangleExtended::ConceptualTriangle([
                    self.vertex_from_node(VertexNode::Casual(idx1))?,
                    self.vertex_from_node(VertexNode::Casual(idx2))?,
                ])
            }
            (VertexNode::Casual(idx0), VertexNode::Conceptual, VertexNode::Casual(idx2)) => {
                TriangleExtended::ConceptualTriangle([
                    self.vertex_from_node(VertexNode::Casual(idx2))?,
                    self.vertex_from_node(VertexNode::Casual(idx0))?,
                ])
            }
            (VertexNode::Casual(idx0), VertexNode::Casual(idx1), VertexNode::Conceptual) => {
                TriangleExtended::ConceptualTriangle([
                    self.vertex_from_node(VertexNode::Casual(idx0))?,
                    self.vertex_from_node(VertexNode::Casual(idx1))?,
                ])
            }
            (VertexNode::Casual(..), VertexNode::Casual(..), VertexNode::Casual(..)) => {
                TriangleExtended::Triangle(self.triangle_from_nodes([node0, node1, node2])?)
            }
            (_, _, _) => return Err(anyhow::Error::msg("An unexpected triangle case occurred")),
        };

        HowOk(tri_extended)
    }

    /// Gets the height for a vertex, this is affected by weights
    pub fn height(&self, v_idx: VertexIdx) -> f64 {
        self.vertices[v_idx][0].powi(2) + self.vertices[v_idx][1].powi(2)
            - self.weights.as_ref().map_or(0.0, |weights| weights[v_idx])
    }

    pub fn num_ignored_vertices(&self) -> usize {
        self.ignored_vertices.len()
    }

    /// The number of all `tris` in the triangulation, `casual` and `conceptual`.
    pub const fn num_tris(&self) -> usize {
        self.tds().num_tris()
    }

    /// The number of `casual` `tris`, i.e. without the ones that have an connection to the dummy point.
    #[must_use]
    pub fn num_casual_tris(&self) -> usize {
        self.tds().num_casual_tris()
    }

    /// The number of total tris, i.e. `casual`, `conceptual` and `deleted` tris.
    #[must_use]
    pub const fn num_all_tris(&self) -> usize {
        self.tds().num_tris() + self.tds().num_deleted_tris
    }

    pub fn num_redundant_vertices(&self) -> usize {
        self.redundant_vertices.len()
    }

    pub fn num_used_vertices(&self) -> usize {
        self.used_vertices.len()
    }

    /// Get the triangulation data structure, as reference.
    #[must_use]
    pub const fn tds(&self) -> &TriDataStructure {
        &self.tds
    }

    /// Get the triangulation data structure, as mutable reference.
    #[must_use]
    pub const fn tds_mut(&mut self) -> &mut TriDataStructure {
        &mut self.tds
    }

    /// Get the triangles of the triangulation as `Triangle2`, i.e `[[f64; 2]; 3]`.
    ///
    /// Does not include conceptual triangles, i.e. the convex hull edges
    /// connected to the point at infinity.
    pub fn tris(&self) -> Vec<Triangle2> {
        // todo: handle the results gracefully, instead of unwrapping (which is safe here though)
        (0..self.tds().num_tris() + self.tds().num_deleted_tris)
            .filter_map(|tri_idx| {
                let tri = self.tds().get_tri(tri_idx).ok()?;

                if tri.is_conceptual() || tri.is_deleted() {
                    return None;
                }

                self.triangle_from_nodes(tri.nodes()).ok()
            })
            .collect()
    }

    /// Get the used vertices.
    #[must_use]
    pub const fn used_vertices(&self) -> &Vec<usize> {
        &self.used_vertices
    }

    /// Get the vertices.
    #[must_use]
    pub const fn vertices(&self) -> &Vec<[f64; 2]> {
        &self.vertices
    }

    /// Get the weights.
    #[must_use]
    pub const fn weights(&self) -> &Option<Vec<f64>> {
        &self.weights
    }
}
