use std::cmp::Ordering;

use egui::{Color32, Context};
use egui_plot::{Legend, Plot, PlotUi, Points, Polygon};

use crate::types::{PlotSettings, TriangulationData};

pub fn show(
    ctx: &Context,
    plot_settings: &mut PlotSettings,
    triangulation_data: &mut TriangulationData,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let mut plot = Plot::new("Triangulations Debug").legend(Legend::default());
        if plot_settings.square_view {
            plot = plot.view_aspect(1.0);
        }
        if plot_settings.proportional {
            plot = plot.data_aspect(1.0);
        }

        plot.show(ui, |plot_ui| {
            draw_triangles(plot_ui, triangulation_data);

            draw_points(plot_ui, plot_settings, triangulation_data);
        })
        .response
    });
}

fn draw_triangles(plot_ui: &mut PlotUi, triangulation_data: &mut TriangulationData) {
    for i in 0..triangulation_data.triangulation.tds().num_tris() {
        let tri = triangulation_data.triangulation.tds().get_tri(i).unwrap();
        let [n0, n1, n2] = tri.nodes();

        if !tri.is_conceptual() {
            let v0 = triangulation_data.vertices[n0.idx().unwrap()];
            let v1 = triangulation_data.vertices[n1.idx().unwrap()];
            let v2 = triangulation_data.vertices[n2.idx().unwrap()];

            plot_ui.polygon(
                Polygon::new(vec![v0, v1, v2])
                    .name(format!("Triangle {}", i))
                    .fill_color(Color32::from_rgba_premultiplied(46, 128, 115, 2))
                    .width(1.0),
            );
        }
    }
}

fn draw_points(
    plot_ui: &mut PlotUi,
    plot_settings: &mut PlotSettings,
    triangulation_data: &mut TriangulationData,
) {
    let (points_to_add, highlighted_point, points_added) =
        debug_vertex_markers(plot_settings, triangulation_data);

    plot_ui.points(points_to_add);
    plot_ui.points(highlighted_point);
    plot_ui.points(points_added);
}

/// Create the plot markers for the input vertices for debugging, i.e. the currently inserted vertex is highlighted.
fn debug_vertex_markers(
    plot_settings: &mut PlotSettings,
    triangulation_data: &mut TriangulationData,
) -> (Points, Points, Points) {
    let mut points_to_add: Vec<[f64; 2]> = vec![];
    let mut point_highlighted: Vec<[f64; 2]> = vec![];
    let mut points_added: Vec<[f64; 2]> = vec![];

    for i in 0..triangulation_data.vertices.len() {
        match plot_settings.cache_timestep_to_display.cmp(&(i + 1)) {
            Ordering::Equal => {
                point_highlighted.push([
                    triangulation_data.vertices[i][0],
                    triangulation_data.vertices[i][1],
                ]);
            }
            Ordering::Less => {
                points_added.push([
                    triangulation_data.vertices[i][0],
                    triangulation_data.vertices[i][1],
                ]);
            }
            _ => {
                points_to_add.push([
                    triangulation_data.vertices[i][0],
                    triangulation_data.vertices[i][1],
                ]);
            }
        }
    }

    let points_to_add = Points::new(points_to_add)
        .name("Vertices to add")
        .filled(plot_settings.marker_style.fill_markers)
        .radius(plot_settings.marker_style.marker_radius)
        .color(Color32::GRAY);

    let point_highlighted = Points::new(point_highlighted)
        .name("Current Vertex")
        .filled(plot_settings.marker_style.fill_markers)
        .radius(plot_settings.marker_style.marker_radius)
        .color(Color32::RED);

    let points_added = Points::new(points_added)
        .name("Added Vertices")
        .filled(plot_settings.marker_style.fill_markers)
        .radius(plot_settings.marker_style.marker_radius)
        .color(Color32::BLACK);

    (points_to_add, point_highlighted, points_added)
}
