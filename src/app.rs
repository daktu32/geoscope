use eframe::CreationContext;
use egui_dock::{DockArea, DockState, NodeIndex};

use crate::data::DataStore;
use crate::renderer::GlobeRenderer;
use crate::ui::{GeoScopeTabViewer, Tab};

/// GeoScope application state.
pub struct GeoScopeApp {
    dock_state: DockState<Tab>,
    data_store: DataStore,
    globe_renderer: GlobeRenderer,
    ui_state: crate::ui::UiState,
    /// Generation counter: incremented when field data changes.
    data_generation: u64,
    /// Last generation uploaded to GPU.
    gpu_generation: u64,
    /// Track colormap changes.
    last_colormap: crate::ui::Colormap,
}

impl GeoScopeApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let globe_renderer = GlobeRenderer::new(cc);

        // 3-column layout: Data Browser | Globe Viewport | Inspector
        let mut dock_state = DockState::new(vec![Tab::Viewport]);
        let surface = dock_state.main_surface_mut();
        surface.split_left(NodeIndex::root(), 0.18, vec![Tab::DataBrowser]);
        surface.split_right(NodeIndex::root(), 0.22, vec![Tab::Inspector]);

        Self {
            dock_state,
            data_store: DataStore::new(),
            globe_renderer,
            ui_state: crate::ui::UiState::default(),
            data_generation: 0,
            gpu_generation: 0,
            last_colormap: crate::ui::Colormap::default(),
        }
    }
}

impl GeoScopeApp {
    /// Open a NetCDF file and auto-select the first non-coordinate variable.
    pub fn open_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.data_store.open_file(path)?;
        let file_idx = self.data_store.files.len() - 1;
        let file = &self.data_store.files[file_idx];

        // Find the first non-coordinate variable with 2+ dimensions
        let var_idx = file.variables.iter().position(|v| {
            v.dimensions.len() >= 2
                && !(v.dimensions.len() == 1
                    && v.dimensions[0].0 == v.name)
        });

        if let Some(vi) = var_idx {
            self.data_store.load_field(file_idx, vi).ok();
            self.data_generation += 1;
        }

        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.ui_state.status_text = format!("Opened: {name}");
        Ok(())
    }
}

impl eframe::App for GeoScopeApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle drag & drop
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    match self.data_store.open_file(path) {
                        Ok(()) => {
                            let name = path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            self.ui_state.status_text = format!("Opened: {name}");
                        }
                        Err(e) => {
                            log::error!("Failed to open file: {e}");
                            self.ui_state.status_text = format!("Error: {e}");
                        }
                    }
                }
            }
        });

        let mut tab_viewer = GeoScopeTabViewer {
            data_store: &mut self.data_store,
            globe_renderer: &mut self.globe_renderer,
            ui_state: &mut self.ui_state,
            data_generation: &mut self.data_generation,
        };

        // Status bar at the bottom
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&tab_viewer.ui_state.status_text);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let view_label = match tab_viewer.ui_state.view_mode {
                        crate::ui::ViewMode::Globe => "Globe",
                        crate::ui::ViewMode::Map => "Map",
                    };
                    ui.label(view_label);
                });
            });
        });

        DockArea::new(&mut self.dock_state)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);

        // Detect colormap change
        if self.ui_state.colormap != self.last_colormap {
            self.last_colormap = self.ui_state.colormap;
            self.data_generation += 1;
        }

        // Upload field data to GPU when it changes
        if self.data_generation != self.gpu_generation {
            if let Some(field) = self.data_store.active_field() {
                if let Some(render_state) = frame.wgpu_render_state() {
                    self.globe_renderer.upload_field_data(
                        render_state,
                        &field.values,
                        field.width,
                        field.height,
                        self.ui_state.colormap,
                    );
                    self.gpu_generation = self.data_generation;

                    // Update status with variable info
                    if let Some(file_idx) = self.data_store.active_file {
                        if let Some(file) = self.data_store.files.get(file_idx) {
                            if let Some(var_idx) = file.selected_variable {
                                let var = &file.variables[var_idx];
                                self.ui_state.status_text = format!(
                                    "{}: {}x{}, range [{:.4e}, {:.4e}]",
                                    var.name, field.width, field.height, field.min, field.max,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
