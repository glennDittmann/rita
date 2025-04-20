#![warn(clippy::all, rust_2018_idioms)]
#![allow(elided_lifetimes_in_paths)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod utils;
pub use app::TriangulationApp;

mod panels {
    pub mod top_panel;
    pub mod tabs {
        pub mod tab_handler;
        pub mod debug {
            pub mod central_panel;
            pub mod side_panel;
        }
        pub mod lab {
            pub mod central_panel;
            pub mod side_panel;
        }
    }
}

mod types;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        // "📐 Computer Graphics Lab", // actual footage of +1 mark
        "Triangulations GUI",
        native_options,
        Box::new(|cc| Ok(Box::new(app::TriangulationApp::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Ok(Box::new(triangulations_gui::TemplateApp::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
