use egui::{Context, Ui};

use crate::{
    types::{AppSettings, PlotSettings},
    utils,
};

pub fn show(ctx: &Context, app_settings: &mut AppSettings, plot_settings: &mut PlotSettings) {
    egui::SidePanel::left("side_panel_debug").show(ctx, |ui| {
        ui.add_enabled_ui(app_settings.sidebar_enabled, |ui| {
            ui.heading("Triangulation Debug");

            triangulation_cache(ui, plot_settings);
        });

        utils::egui_credits(ui);
    });
}

fn triangulation_cache(ui: &mut Ui, plot_settings: &mut PlotSettings) {
    ui.group(|ui| {
        ui.collapsing(
            format!(
                "Triangle Cache (Showing timestep: {})",
                plot_settings.cache_timestep_to_display
            ),
            |_ui| {},
        )
    });
}
