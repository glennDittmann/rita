use egui::Visuals;

use crate::{
    panels::{tabs::tab_handler, top_panel},
    types::{AppSettings, FileHandler, PlotSettings, Tab, TriangulationData},
};

const SHOW_WINDOW: bool = false;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, PartialEq)]
pub struct TriangulationApp {
    pub app_settings: AppSettings,
    pub file_handler: FileHandler,
    pub open_tab: Tab,
    pub plot_settings: PlotSettings,
    pub triangulation_data: TriangulationData,
}

impl TriangulationApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        cc.egui_ctx.set_visuals(Visuals::light());

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }
        Default::default()
    }
}

impl eframe::App for TriangulationApp {
    // /// Called by the frame work to save state before shutdown.
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.file_handler.update(); // update text from file once it comes in from opening a file via the file dialog

        let Self {
            app_settings: _,
            file_handler: _,
            open_tab: _,
            plot_settings: _,
            triangulation_data: _,
        } = self;

        top_panel::show(ctx, &mut self.open_tab, &mut self.plot_settings);

        tab_handler::show(
            ctx,
            &mut self.open_tab,
            &mut self.app_settings,
            &mut self.file_handler,
            &mut self.plot_settings,
            &mut self.triangulation_data,
        );

        if SHOW_WINDOW {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally choose either panels OR windows.");
            });
        }
    }
}
