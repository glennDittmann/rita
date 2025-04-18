use egui::Ui;

#[derive(Debug, PartialEq)]
pub struct Metrics {
    pub runtime: f64,
    pub regular: bool,
    pub sound: bool,
}

impl Metrics {
    pub fn to_label(&self, ui: &mut Ui) {
        if self.runtime > 0.0 {
            ui.label(format!("Runtime (μs): {}", self.runtime));
            ui.label(format!("Regular: {}", self.regular));
            ui.label(format!("Sound: {}", self.sound));
        } else {
            ui.label("Runtime (μs): -".to_string());
            ui.label("Regular: -".to_string());
            ui.label("Sound: -".to_string());
        }
    }

    pub fn reset(&mut self) {
        self.runtime = 0.0;
        self.regular = false;
        self.sound = false;
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            runtime: 0.0,
            regular: false,
            sound: false,
        }
    }
}
