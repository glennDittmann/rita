use egui::Context;

use crate::types::{AppSettings, FileHandler, PlotSettings, Tab, TriangulationData};

use super::{debug, lab};

/// Show the currently selected tab, which updates the side and central panels.
pub fn show(
    ctx: &Context,
    open_tab: &mut Tab,
    app_settings: &mut AppSettings,
    file_handler: &mut FileHandler,
    plot_settings: &mut PlotSettings,
    triangulation_data: &mut TriangulationData,
) {
    match open_tab {
        Tab::Lab => {
            lab::side_panel::show(ctx, triangulation_data, app_settings, file_handler);
            lab::central_panel::show(ctx, plot_settings, triangulation_data);
        }
        Tab::Debug => {
            debug::side_panel::show(ctx, app_settings, plot_settings);
            debug::central_panel::show(ctx, plot_settings, triangulation_data);
        }
    }
}
