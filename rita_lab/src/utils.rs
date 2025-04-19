use egui::Ui;
use std::{future::Future, time::Instant};

use crate::types::Vertex2;
pub use rita_test_utils::{sample_vertices_2d, sample_weights};

/// Part of the side panel that shows the egui credits.
pub fn egui_credits(ui: &mut Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("powered by ");
            ui.hyperlink_to("egui", "https://github.com/emilk/egui");
            ui.label(" and ");
            ui.hyperlink_to(
                "eframe",
                "https://github.com/emilk/egui/tree/master/crates/eframe",
            );
            ui.label(".");
        });
    });
}

/// Measures the time it takes to execute a function.
///
/// Returns the result of the function and the time it took to execute it in `μs`.
pub fn measure_time<F: FnOnce() -> T, T>(f: F) -> (T, u128) {
    let start = Instant::now();
    let result = f();
    let end = Instant::now();
    let duration = end.duration_since(start).as_micros();
    (result, duration)
}

/// Get a predefined set of vertices.
pub fn get_example_vertices() -> Vec<Vertex2> {
    // points for the Running Example
    vec![
        [0.0, 0.0],
        [-0.5, 1.0],
        [0.0, 2.5],
        [2.0, 3.0],
        [4.0, 2.5],
        [5.0, 1.5],
        [4.5, 0.5],
        [2.5, -0.5],
        [1.5, 1.5],
        [3.0, 1.0],
    ]
}

/// Get a predefined set of weights.
pub fn get_example_weights() -> Vec<f64> {
    vec![
        0.681, 0.579, 0.5625, 0.86225, 10.0, 0.472, 0.5865, 0.59625, 0.51225, 7.0,
    ]
}

/// Read vertices from a string in CSV format
///
/// The string should be in the following format:
///
/// ```csv
/// x,y
/// 1.0,2.0
/// 3.0,4.0
/// ```
pub fn read_vertices_from_string(input: &str) -> Vec<Vertex2> {
    let mut reader = csv::Reader::from_reader(input.as_bytes());
    let vertices = reader.deserialize().map(|record| record.unwrap()).collect();

    vertices
}

pub fn bbox_2d(vertices: &[Vertex2]) -> (Vertex2, Vertex2) {
    let mut min = [f64::INFINITY, f64::INFINITY];
    let mut max = [f64::NEG_INFINITY, f64::NEG_INFINITY];

    for v in vertices.iter() {
        min[0] = min[0].min(v[0]);
        min[1] = min[1].min(v[1]);
        max[0] = max[0].max(v[0]);
        max[1] = max[1].max(v[1]);
    }

    (min, max)
}

/// Scales vertices into a `side_length * side_length` bounding square centered around the origin.
///
/// Returns the scaled vertices and the scaling factor.
///
/// E.g., if `side_length = 1.0`, the bounding square will be `[-0.5, 0.5] x [-0.5, 0.5]`, i.e. the unit square.
pub fn scale_vertices_2d(vertices: &[Vertex2], side_length: f64) -> (Vec<Vertex2>, f64) {
    if vertices.is_empty() {
        return (vec![], 1.0);
    }

    let ([min_x, min_y], [max_x, max_y]) = bbox_2d(vertices);

    // 2) Compute scaling factor and center
    let scale = side_length / (max_x - min_x).max(max_y - min_y);
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    // 3) Scale the vertices
    let scaled_vertices: Vec<Vertex2> = vertices
        .iter()
        .map(|v| [(v[0] - center_x) * scale, (v[1] - center_y) * scale])
        .collect();

    (scaled_vertices, scale)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
