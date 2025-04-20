use egui::{Color32, Context, Stroke};
use egui_plot::{Legend, Plot, PlotPoint, PlotPoints, PlotResponse, PlotUi, Points, Polygon};
use vertex_clustering::VertexClusterer2;

use crate::types::{PlotSettings, TriangulationData, Vertex2, ORANGE, TRI_GREEN};

pub fn show(
    ctx: &Context,
    plot_settings: &mut PlotSettings,
    triangulation_data: &mut TriangulationData,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let mut plot = Plot::new("Triangulations").legend(Legend::default());
        if plot_settings.square_view {
            plot = plot.view_aspect(1.0);
        }
        if plot_settings.proportional {
            plot = plot.data_aspect(1.0);
        }

        let PlotResponse {
            response,
            inner: pointer_coordinate,
            ..
        } = plot.show(ui, |plot_ui| {
            if triangulation_data.triangulation.tds().num_tris() > 0 {
                draw_triangles(triangulation_data, plot_ui);
            }

            if triangulation_data.grid_sampler.is_some() {
                draw_grid(
                    triangulation_data.grid_sampler.as_ref().unwrap(),
                    plot_ui,
                    TRI_GREEN,
                );
            }

            if triangulation_data.scaled_grid_sampler.is_some() {
                draw_grid(
                    triangulation_data.scaled_grid_sampler.as_ref().unwrap(),
                    plot_ui,
                    ORANGE,
                );
            }

            plot_ui.points(vertex_markers(plot_settings, &triangulation_data.vertices));

            plot_ui.points(scaled_vertex_markers(
                plot_settings,
                &triangulation_data.scaled_vertices,
            ));

            plot_ui.pointer_coordinate()
        });

        if response.clicked() {
            if let Some(coordinate) = pointer_coordinate {
                triangulation_data
                    .vertices
                    .push([coordinate.x, coordinate.y]);
            }
        }
    });
}

fn draw_triangles(triangulation_data: &mut TriangulationData, plot_ui: &mut PlotUi) {
    for [a, b, c] in triangulation_data.triangulation.tris() {
        plot_ui.polygon(
            Polygon::new(vec![a, b, c])
                .stroke(Stroke::new(1.0, TRI_GREEN))
                .width(1.0),
        );
    }
}

/// Create the plot markers for the input vertices of the triangulation
fn vertex_markers<'p>(plot_settings: &mut PlotSettings, vertices: &'p [Vertex2]) -> Points<'p> {
    let plot_points: Vec<[f64; 2]> = vertices.iter().map(|&v| [v[0], v[1]]).collect();

    let mut points = Points::new("Vertices", plot_points)
        .filled(plot_settings.marker_style.fill_markers)
        .radius(plot_settings.marker_style.marker_radius);

    if !plot_settings.marker_style.automatic_colors {
        points = points.color(plot_settings.marker_style.marker_color);
    }
    points
}

/// Create the plot markers for the input vertices of the triangulation
fn scaled_vertex_markers<'p>(plot_settings: &mut PlotSettings, vertices: &'p [Vertex2]) -> Points<'p> {
    let plot_points: Vec<[f64; 2]> = vertices.iter().map(|&v| [v[0], v[1]]).collect();

    Points::new("Scaled Vertices", plot_points)
        .filled(plot_settings.marker_style.fill_markers)
        .radius(plot_settings.marker_style.marker_radius)
        .color(ORANGE)
}

fn draw_grid(grid_sampler: &VertexClusterer2, plot_ui: &mut PlotUi, color: Color32) {
    // Get the bottom left corner of the grid
    let mut min = [f64::INFINITY, f64::INFINITY];
    let vertices = &grid_sampler.vertices();

    for (v, _) in vertices {
        min[0] = min[0].min(v[0]);
        min[1] = min[1].min(v[1]);
    }
    let min_x = min[0];
    let min_y = min[1];

    // Draw grid bins step by step
    for x_idx in 0..grid_sampler.num_bins_x() {
        for y_idx in 0..grid_sampler.num_bins_y() {
            let bottom_left = [
                min_x + x_idx as f64 * grid_sampler.bin_size(),
                min_y + y_idx as f64 * grid_sampler.bin_size(),
            ];

            let bottom_right = [
                min_x + x_idx as f64 * grid_sampler.bin_size() + grid_sampler.bin_size(),
                min_y + y_idx as f64 * grid_sampler.bin_size(),
            ];

            let top_right = [
                min_x + x_idx as f64 * grid_sampler.bin_size() + grid_sampler.bin_size(),
                min_y + y_idx as f64 * grid_sampler.bin_size() + grid_sampler.bin_size(),
            ];

            let top_left = [
                min_x + x_idx as f64 * grid_sampler.bin_size(),
                min_y + y_idx as f64 * grid_sampler.bin_size() + grid_sampler.bin_size(),
            ];

            plot_ui.polygon(
                Polygon::new("", vec![bottom_left, bottom_right, top_right, top_left])
                    .stroke(Stroke::new(1.0, color))
                    .width(1.0),
            );
        }
    }
}
