use egui::{ComboBox, Context, Ui};
use egui_plot::LineStyle;

use crate::types::{PlotSettings, Tab};

pub fn show(ctx: &Context, open_tab: &mut Tab, plot_settings: &mut PlotSettings) {
    #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Top panel");
        });

        menu_bar(ctx, ui, plot_settings);

        ui.separator();

        tab_selection(ui, open_tab);
    });
}

fn menu_bar(ctx: &Context, ui: &mut Ui, plot_settings: &mut PlotSettings) {
    egui::menu::bar(ui, |ui| {
        egui::widgets::global_dark_light_mode_buttons(ui);

        menu_bar_file(ctx, ui);

        menu_bar_controls(ui);

        menu_bar_plot_settings(ui, plot_settings);
    });
}

fn menu_bar_file(ctx: &Context, ui: &mut Ui) {
    ui.menu_button("File", |ui| {
        if ui.button("Quit").clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}

fn menu_bar_controls(ui: &mut Ui) {
    ui.collapsing("Controls", |ui| {
        ui.label("Pan by dragging, or scroll (+ shift = horizontal).");

        ui.label("Box zooming: Right click to zoom in and out using a selection.");

        if cfg!(target_arch = "wasm32") {
            ui.label("Zoom with ctrl / ⌘ + pointer wheel, or with pinch gesture.");
        } else if cfg!(target_os = "macos") {
            ui.label("Zoom with ctrl / ⌘ + scroll.");
        } else {
            ui.label("Zoom with ctrl + scroll.");
        }

        ui.label("Reset view with double click.");
    });
}

fn menu_bar_plot_settings(ui: &mut Ui, plot_settings: &mut PlotSettings) {
    ui.collapsing("Plot Settings", |ui| {
        ui.vertical(|ui| {
            ui.style_mut().wrap = Some(false);
            ui.checkbox(&mut plot_settings.square_view, "Square view")
                .on_hover_text("Always keep the viewport square.");
            ui.checkbox(&mut plot_settings.proportional, "Proportional data axes")
                .on_hover_text("Tick are the same size on both axes.");
            ui.checkbox(
                &mut plot_settings.show_ignored_vertices,
                "Show ignored vertices",
            );

            ComboBox::from_label("Line style")
                .selected_text(plot_settings.line_style.to_string())
                .show_ui(ui, |ui| {
                    for style in &[
                        LineStyle::Solid,
                        LineStyle::dashed_dense(),
                        LineStyle::dashed_loose(),
                        LineStyle::dotted_dense(),
                        LineStyle::dotted_loose(),
                    ] {
                        ui.selectable_value(
                            &mut plot_settings.line_style,
                            *style,
                            style.to_string(),
                        );
                    }
                });
        });
    });
}

fn tab_selection(ui: &mut Ui, open_tab: &mut Tab) {
    ui.horizontal(|ui| {
        ui.selectable_value(open_tab, Tab::Lab, "Lab");
        ui.selectable_value(open_tab, Tab::Debug, "Debug");
    });
}
