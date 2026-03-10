mod app;
mod codegen;
mod copilot;
mod data;
mod renderer;
mod ui;

fn main() -> eframe::Result {
    env_logger::init();

    // Collect file paths from command line arguments
    let files: Vec<std::path::PathBuf> = std::env::args()
        .skip(1)
        .map(std::path::PathBuf::from)
        .collect();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "GeoScope",
        native_options,
        Box::new(move |cc| {
            let mut app = app::GeoScopeApp::new(cc);
            for path in &files {
                if let Err(e) = app.open_file(path) {
                    log::error!("Failed to open {}: {e}", path.display());
                }
            }
            Ok(Box::new(app))
        }),
    )
}
