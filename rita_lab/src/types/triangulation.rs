use rita::Triangulation;
use vertex_clustering::VertexClusterer2;

use crate::panels::tabs::lab::side_panel::VertexGenerator;
use super::{Metrics, Vertex2};

/// Global triangulation settings. Note: atm still a bit convoluted.
#[derive(PartialEq)]
pub struct TriangulationData {
    pub baseline_area3d: Option<f64>,
    pub equal_tris: u32,
    pub metrics: Metrics,
    pub number_vertices: usize,
    pub epsilon: f64,
    pub save_to_file: bool,
    pub triangulation: Triangulation,
    pub vertex_generator: VertexGenerator,
    pub vertices: Vec<Vertex2>,
    pub scaled_vertices: Vec<Vertex2>,
    pub weights: Option<Vec<f64>>,
    pub grid_sampler: Option<VertexClusterer2>,
    pub grid_size: f64,
    pub scaled_grid_sampler: Option<VertexClusterer2>,
    pub scale_factor: f64,
}

impl Default for TriangulationData {
    fn default() -> Self {
        Self {
            baseline_area3d: None,
            equal_tris: 0,
            metrics: Metrics::default(),
            number_vertices: 10,
            epsilon: 0.0,
            save_to_file: false,
            triangulation: Triangulation::default(),
            vertex_generator: VertexGenerator::Random,
            vertices: vec![],
            scaled_vertices: vec![],
            weights: None,
            grid_sampler: None,
            grid_size: 0.5,
            scaled_grid_sampler: None,
            scale_factor: 1.0,
        }
    }
}
