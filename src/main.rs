#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod fft;
mod ui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 560.0])
            .with_min_inner_size([600.0, 400.0])
            .with_decorations(true)
            .with_transparent(true)
            .with_icon(load_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Audio Visualizer",
        native_options,
        Box::new(|cc| Ok(Box::new(ui::VisualizerApp::new(cc)))),
    )
}

fn load_app_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/logo.ico"))
        .expect("assets/logo.ico must be a valid icon")
        .to_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
