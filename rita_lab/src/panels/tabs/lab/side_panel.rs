use std::ops::RangeInclusive;

use egui::{Context, Ui};
use log::info;
use rita::Triangulation;
use vertex_clustering::VertexClusterer2;

use crate::{
    types::{AppSettings, FileHandler, TriangulationData},
    utils::{self, execute, measure_time, sample_vertices_2d, sample_weights, scale_vertices_2d},
};

#[derive(Debug, PartialEq)]
pub enum VertexGenerator {
    RunningExample,
    FromFile,
    Random,
    RandomWeighted,
}

pub fn show(
    ctx: &Context,
    triangulation_data: &mut TriangulationData,
    app_settings: &mut AppSettings,
    file_handler: &mut FileHandler,
) {
    egui::SidePanel::left("side_panel").show(ctx, |ui| {
        ui.heading("Triangulation Settings");

        vertex_generator(ui, triangulation_data, file_handler);

        triangulation_computer(ui, triangulation_data, app_settings);

        metric_list(ui, triangulation_data);

        vertex_list(ui, triangulation_data);

        triangle_list(ui, triangulation_data);

        utils::egui_credits(ui);
    });
}

/// Part of the side panel that allows for setting the vertex generation methods.
fn vertex_generator(
    ui: &mut Ui,
    triangulation_data: &mut TriangulationData,
    file_handler: &mut FileHandler,
) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            // Select vertex generation method
            egui::ComboBox::from_label("Select vertex generation")
                .selected_text(format!("{:?}", triangulation_data.vertex_generator))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut triangulation_data.vertex_generator,
                        VertexGenerator::Random,
                        "Random",
                    );
                    ui.selectable_value(
                        &mut triangulation_data.vertex_generator,
                        VertexGenerator::RandomWeighted,
                        "RandomWeighted",
                    );
                    ui.selectable_value(
                        &mut triangulation_data.vertex_generator,
                        VertexGenerator::RunningExample,
                        "RunningExample",
                    );
                    ui.selectable_value(
                        &mut triangulation_data.vertex_generator,
                        VertexGenerator::FromFile,
                        "FromFile",
                    );
                });

            // Set the number of vertices and an equal weight for all randomly generated vertices
            if triangulation_data.vertex_generator == VertexGenerator::Random
                || triangulation_data.vertex_generator == VertexGenerator::RandomWeighted
            {
                ui.label("Number of Vertices:");

                ui.add(egui::Slider::new(
                    &mut triangulation_data.number_vertices,
                    3..=1000,
                ));
            }

            // Select csv file, when reading vertices from file
            if triangulation_data.vertex_generator == VertexGenerator::FromFile
                && ui
                    .button("📂 Open csv file")
                    .on_hover_text("Read a csv file with the format:\nx,y\n1.0,5.0\n...")
                    .clicked()
            {
                {
                    let sender = file_handler.get_sender_cloned();

                    let task = rfd::AsyncFileDialog::new().pick_file();

                    let ctx = ui.ctx().clone(); // context is cheap to clone: https://docs.rs/egui/0.24.1/egui/struct.Context.html

                    execute(async move {
                        let file = task.await;
                        if let Some(file) = file {
                            let text = file.read().await;

                            let _ = sender.send(String::from_utf8_lossy(&text).to_string());

                            ctx.request_repaint();
                        }
                    });
                }
            }

            // Generate and delete buttons
            ui.horizontal(|ui| {
                if ui.button("Generate vertices").clicked() {
                    triangulation_data.metrics.reset();
                    triangulation_data.grid_sampler = None;
                    match triangulation_data.vertex_generator {
                        VertexGenerator::Random => {
                            triangulation_data.vertices = sample_vertices_2d(
                                triangulation_data.number_vertices,
                                Some(RangeInclusive::new(1.0, 2.0)),
                            );
                            triangulation_data.weights = None;
                        }
                        VertexGenerator::RandomWeighted => {
                            triangulation_data.vertices =
                                sample_vertices_2d(triangulation_data.number_vertices, None);
                            triangulation_data.weights =
                                Some(sample_weights(triangulation_data.number_vertices, None));
                        }
                        VertexGenerator::RunningExample => {
                            triangulation_data.vertices = utils::get_example_vertices();
                            triangulation_data.weights = None;
                        }
                        VertexGenerator::FromFile => {
                            triangulation_data.vertices =
                                utils::read_vertices_from_string(&file_handler.text.clone());
                            triangulation_data.weights = None;
                        }
                    }

                    // Reset triangulation data, when generating new vertices
                    triangulation_data.triangulation = Triangulation::new(None);
                }
                if ui.button("Delete Vertices").clicked() {
                    triangulation_data.vertices.clear();
                    triangulation_data.triangulation = Triangulation::new(None);
                    triangulation_data.metrics.reset();
                    triangulation_data.grid_sampler = None;
                }

                if ui.button("Comp. Grids").clicked() {
                    triangulation_data.grid_sampler = Some(VertexClusterer2::new(
                        triangulation_data.vertices.clone(),
                        triangulation_data.weights.clone(),
                        triangulation_data.grid_size,
                    ));
                }
                if ui.button("Simplify").clicked() && triangulation_data.grid_sampler.is_some() {
                    let (simplified_vs, _) =
                        triangulation_data.grid_sampler.as_ref().unwrap().simplify();
                    triangulation_data.vertices = simplified_vs;
                }
                ui.label("Grid Size:");
                ui.add(
                    egui::Slider::new(&mut triangulation_data.grid_size, 0.1..=20.0).step_by(0.1),
                );
            });

            // Scale vertices
            ui.horizontal(|ui| {
                if ui.button("Scale Vertices").clicked() {
                    let (scaled_vs, scale_factor) =
                        scale_vertices_2d(&triangulation_data.vertices, 1.0);
                    triangulation_data.scaled_vertices = scaled_vs;
                    triangulation_data.scale_factor = scale_factor;
                }
                if ui.button("Delete Scaled Vertices").clicked() {
                    triangulation_data.scaled_vertices.clear();
                    triangulation_data.scaled_grid_sampler = None;
                }
                if ui.button("Comp. Scaled Grids").clicked() {
                    triangulation_data.scaled_grid_sampler = Some(VertexClusterer2::new(
                        triangulation_data.scaled_vertices.clone(),
                        None,
                        triangulation_data.grid_size * triangulation_data.scale_factor,
                    ));
                }
                if ui.button("Simplify").clicked()
                    && triangulation_data.scaled_grid_sampler.is_some()
                {
                    let (simplified_vs, _) = triangulation_data
                        .scaled_grid_sampler
                        .as_ref()
                        .unwrap()
                        .simplify();
                    triangulation_data.scaled_vertices = simplified_vs;
                }
            })
        });
    });
}

/// Part of the side panel that handlles the triangulation computation.
fn triangulation_computer(
    ui: &mut Ui,
    triangulation_data: &mut TriangulationData,
    app_settings: &mut AppSettings,
) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            // Set the epsilon parameter
            ui.horizontal(|ui| {
                let (param_name, param_range, drag_speed) = ("Epsilon", 0.0..=100.0, 0.1);
                ui.add(
                    egui::Slider::new(&mut triangulation_data.epsilon, param_range)
                        .prefix(format!("{}: ", param_name))
                        .drag_value_speed(drag_speed),
                );
                if ui
                    .button("↺")
                    .on_hover_text(format!("Reset {} to 0.0", param_name))
                    .clicked()
                {
                    triangulation_data.epsilon = 0.0;
                }
            });

            // Handle triangulation button click
            ui.add_enabled_ui(!triangulation_data.vertices.is_empty(), |ui| {
                if ui.button("Triangulate").clicked() {
                    app_settings.sidebar_enabled = false;

                    let eps = if triangulation_data.epsilon > 0.0 {
                        Some(triangulation_data.epsilon)
                    } else {
                        None
                    };
                    info!("Triangulating with epsilon: {:?}", eps);

                    triangulation_data.triangulation = Triangulation::new(eps);

                    let (_, runtime_micros) = measure_time(|| {
                        triangulation_data.triangulation.insert_vertices(
                            &triangulation_data.vertices,
                            triangulation_data.weights.clone(),
                            true,
                        )
                    });

                    log::info!("Triangulation took {} μs", runtime_micros);
                    triangulation_data.metrics.runtime = runtime_micros as f64;

                    let (regular, _) = triangulation_data.triangulation.is_regular().unwrap();
                    triangulation_data.metrics.regular = regular;

                    triangulation_data.metrics.sound =
                        triangulation_data.triangulation.is_sound().unwrap();

                    app_settings.sidebar_enabled = true;
                }
            });
        })
    });
}

/// Part of the side panel that lists the metrics.
fn metric_list(ui: &mut Ui, triangulation_data: &TriangulationData) {
    ui.group(|ui| {
        triangulation_data.metrics.to_label(ui);
    });
}

/// Part of the side panel that lists the vertices.
fn vertex_list(ui: &mut Ui, triangulation_data: &TriangulationData) {
    ui.group(|ui| {
        ui.collapsing("Vertices", |ui| {
            if triangulation_data.vertices.is_empty() {
                ui.label("No vertices generated yet.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for (i, vertex) in triangulation_data.vertices.iter().enumerate() {
                            let weight = match &triangulation_data.weights {
                                Some(weights) => format!("{:.2}", weights[i]),
                                None => "-".to_string(),
                            };

                            let vertex_text = format!(
                                "v{} ({:.2}, {:.2})  w: {}",
                                i, vertex[0], vertex[1], weight
                            );
                            ui.label(vertex_text);
                        }
                    });
            }
        })
    });
}

/// Part of the side panel that lists the triangles.
fn triangle_list(ui: &mut Ui, triangulation_data: &TriangulationData) {
    ui.group(|ui| {
        ui.collapsing("Triangles", |ui| {
            if triangulation_data.triangulation.tds().num_tris() == 0 {
                ui.label("No triangles computed.");
            } else {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for i in 0..triangulation_data.triangulation.tds().num_tris() {
                            ui.collapsing(format!("Triangle {}", i), |ui| {
                                ui.vertical(|ui| {
                                    let tri =
                                        triangulation_data.triangulation.tds().get_tri(i).unwrap();
                                    let [n0, n1, n2] = tri.nodes();

                                    if !tri.is_conceptual()
                                        && n0.idx().is_some()
                                        && n1.idx().is_some()
                                        && n2.idx().is_some()
                                    {
                                        let v0 = triangulation_data.vertices[n0.idx().unwrap()];
                                        let v1 = triangulation_data.vertices[n1.idx().unwrap()];
                                        let v2 = triangulation_data.vertices[n2.idx().unwrap()];

                                        ui.label(format!("\tA ({:.2}, {:.2})", v0[0], v0[1],));
                                        ui.label(format!("\tB ({:.2}, {:.2})", v1[0], v1[1],));
                                        ui.label(format!("\tC ({:.2}, {:.2})", v2[0], v2[1],));
                                    } else if tri.is_conceptual() {
                                        if n0.is_conceptual() {
                                            ui.label("\t∞");
                                        } else if n0.idx().is_some() {
                                            let v = triangulation_data.vertices[n0.idx().unwrap()];
                                            ui.label(format!("\tA ({:.2}, {:.2})", v[0], v[1],));
                                        }

                                        if n1.is_conceptual() {
                                            ui.label("\t∞");
                                        } else if n1.idx().is_some() {
                                            let v = triangulation_data.vertices[n1.idx().unwrap()];
                                            ui.label(format!("\tB ({:.2}, {:.2})", v[0], v[1],));
                                        }

                                        if n2.is_conceptual() {
                                            ui.label("\t∞");
                                        } else if n2.idx().is_some() {
                                            let v = triangulation_data.vertices[n2.idx().unwrap()];
                                            ui.label(format!("\tC ({:.2}, {:.2})", v[0], v[1],));
                                        }
                                    }
                                });
                            });
                        }
                    });
            }
        })
    });
}
