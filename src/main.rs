mod app;
mod data;
mod renderer;
mod ui;

fn main() -> eframe::Result {
    env_logger::init();

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
        Box::new(|cc| Ok(Box::new(app::GeoScopeApp::new(cc)))),
    )
}
