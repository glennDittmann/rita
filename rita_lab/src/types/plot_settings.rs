use egui::Color32;
use egui_plot::LineStyle;
use serde::{Deserialize, Serialize};

/// Global plots settings.
#[derive(PartialEq, Deserialize, Serialize)]
pub struct PlotSettings {
    pub line_style: LineStyle,
    pub marker_style: MarkerStyle,
    pub square_view: bool,
    pub proportional: bool,
    pub show_ignored_vertices: bool,
    pub cache_timestep_to_display: usize,
}

impl Default for PlotSettings {
    fn default() -> Self {
        Self {
            line_style: LineStyle::Solid,
            marker_style: MarkerStyle::default(),
            square_view: true,
            proportional: true,
            show_ignored_vertices: true,
            cache_timestep_to_display: 0,
        }
    }
}

#[derive(PartialEq, Deserialize, Serialize)]
pub struct MarkerStyle {
    pub fill_markers: bool,
    pub marker_radius: f32,
    pub marker_color: Color32,
    pub automatic_colors: bool,
}

impl Default for MarkerStyle {
    fn default() -> Self {
        Self {
            fill_markers: true,
            marker_radius: 3.0,
            marker_color: Color32::DARK_GRAY,
            automatic_colors: false,
        }
    }
}
