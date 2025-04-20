use crate::{
    panels::{tabs::tab_handler, top_panel},
    types::{AppSettings, FileHandler, PlotSettings, Tab, TriangulationData},
};

const SHOW_WINDOW: bool = false;

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
        // Load previous app state (if any).
        if let Some(storage) = cc.storage {
            if let Some((app_settings, open_tab)) = eframe::get_value(storage, eframe::APP_KEY) {
                return Self {
                    app_settings,
                    open_tab,
                    ..Default::default()
                };
            }
        };

        Self::default()
    }
}

impl eframe::App for TriangulationApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &(&self.app_settings, &self.open_tab));
    }

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
