use egui_dock::TabViewer;

use crate::data::DataStore;
use crate::renderer::GlobeRenderer;

/// View mode for the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Globe,
    Map,
}

/// Colormap selection.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Colormap {
    #[default]
    Viridis,
    RdBuR,
}

impl Colormap {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Viridis => "viridis",
            Self::RdBuR => "RdBu_r",
        }
    }

    const ALL: [Colormap; 2] = [Colormap::Viridis, Colormap::RdBuR];
}

/// Persistent UI state (stored in GeoScopeApp).
#[derive(Debug)]
pub struct UiState {
    pub view_mode: ViewMode,
    pub colormap: Colormap,
    pub time_index: usize,
    pub status_text: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Globe,
            colormap: Colormap::default(),
            time_index: 0,
            status_text: "Ready".to_string(),
        }
    }
}

/// Tab types for the dock layout.
#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    DataBrowser,
    Viewport,
    Inspector,
}

/// Tab viewer that renders each panel.
pub struct GeoScopeTabViewer<'a> {
    pub data_store: &'a mut DataStore,
    pub globe_renderer: &'a mut GlobeRenderer,
    pub ui_state: &'a mut UiState,
    /// Incremented when field data changes, triggers GPU upload.
    pub data_generation: &'a mut u64,
}

impl TabViewer for GeoScopeTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::DataBrowser => "Data Browser".into(),
            Tab::Viewport => "Globe".into(),
            Tab::Inspector => "Inspector".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::DataBrowser => self.data_browser_ui(ui),
            Tab::Viewport => self.viewport_ui(ui),
            Tab::Inspector => self.inspector_ui(ui),
        }
    }
}

impl GeoScopeTabViewer<'_> {
    fn data_browser_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Variables");
        ui.separator();

        if self.data_store.files.is_empty() {
            ui.label("Drop a NetCDF file here to get started.");
            return;
        }

        // Collect click events to avoid borrow conflict
        let mut load_request: Option<(usize, usize)> = None;

        for (file_idx, file) in self.data_store.files.iter().enumerate() {
            let file_name = std::path::Path::new(&file.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.clone());

            egui::CollapsingHeader::new(&file_name)
                .default_open(true)
                .show(ui, |ui| {
                    for (var_idx, var) in file.variables.iter().enumerate() {
                        let is_coord = var.dimensions.len() <= 1
                            && var.dimensions.first().is_some_and(|(d, _)| d == &var.name);
                        if is_coord {
                            continue;
                        }

                        let label = if let Some(ref long_name) = var.long_name {
                            format!("{} ({})", var.name, long_name)
                        } else {
                            var.name.clone()
                        };

                        let is_selected = file.selected_variable == Some(var_idx);
                        let response = ui.selectable_label(is_selected, &label)
                            .on_hover_ui(|ui| {
                                ui.label(egui::RichText::new(&var.name).strong());
                                if let Some(ref long_name) = var.long_name {
                                    ui.label(long_name.as_str());
                                }
                                if let Some(ref units) = var.units {
                                    ui.label(format!("Units: {units}"));
                                }
                                ui.separator();
                                ui.label("Dimensions:");
                                for (dim_name, dim_size) in &var.dimensions {
                                    ui.label(format!("  {dim_name}: {dim_size}"));
                                }
                            });

                        if response.double_clicked() {
                            load_request = Some((file_idx, var_idx));
                        }
                    }
                });
        }

        if let Some((file_idx, var_idx)) = load_request {
            if self.data_store.load_field(file_idx, var_idx).is_ok() {
                *self.data_generation += 1;
            }
        }
    }

    fn viewport_ui(&mut self, ui: &mut egui::Ui) {
        // View mode tabs at the top
        ui.horizontal(|ui| {
            if ui.selectable_label(self.ui_state.view_mode == ViewMode::Globe, "Globe").clicked() {
                self.ui_state.view_mode = ViewMode::Globe;
            }
            if ui.selectable_label(self.ui_state.view_mode == ViewMode::Map, "Map").clicked() {
                self.ui_state.view_mode = ViewMode::Map;
            }
        });
        ui.separator();

        // Time slider (if active variable has a time dimension)
        if let Some(time_len) = self.active_time_dim_len() {
            if time_len > 1 {
                ui.horizontal(|ui| {
                    ui.label("Time:");
                    let mut t = self.ui_state.time_index;
                    if t >= time_len {
                        t = 0;
                        self.ui_state.time_index = 0;
                    }
                    let max = (time_len - 1) as u32;
                    let slider = egui::Slider::new(&mut t, 0..=max as usize)
                        .step_by(1.0)
                        .text(format!("/ {}", time_len - 1));
                    if ui.add(slider).changed() {
                        self.ui_state.time_index = t;
                        // Reload field data at new time index
                        if let Some(fi) = self.data_store.active_file {
                            if let Some(file) = self.data_store.files.get(fi) {
                                if let Some(vi) = file.selected_variable {
                                    if self.data_store.load_field_at(fi, vi, t, 0).is_ok() {
                                        *self.data_generation += 1;
                                    }
                                }
                            }
                        }
                    }
                });
                ui.separator();
            }
        }

        // Globe / Map viewport
        self.globe_renderer.paint(ui);
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        ui.separator();

        if let Some(file_idx) = self.data_store.active_file {
            if let Some(file) = self.data_store.files.get(file_idx) {
                if let Some(var_idx) = file.selected_variable {
                    let var = &file.variables[var_idx];
                    ui.label(format!("Variable: {}", var.name));
                    if let Some(ref units) = var.units {
                        ui.label(format!("Units: {units}"));
                    }
                    let dims: Vec<String> = var.dimensions.iter()
                        .map(|(n, s)| format!("{n}={s}"))
                        .collect();
                    ui.label(format!("Dims: {}", dims.join(", ")));

                    if let Some(ref field) = file.field_data {
                        ui.separator();
                        ui.label(format!("Min: {:.6}", field.min));
                        ui.label(format!("Max: {:.6}", field.max));
                    }

                    // Colormap selector
                    ui.separator();
                    ui.label("Colormap:");
                    egui::ComboBox::from_id_salt("colormap_combo")
                        .selected_text(self.ui_state.colormap.label())
                        .show_ui(ui, |ui| {
                            for cm in Colormap::ALL {
                                ui.selectable_value(
                                    &mut self.ui_state.colormap,
                                    cm,
                                    cm.label(),
                                );
                            }
                        });

                    // Point info placeholder
                    ui.separator();
                    ui.heading("Point Info");
                    ui.label("Hover over the viewport to see coordinates and values.");
                } else {
                    ui.label("Select a variable to inspect.");
                }
            }
        } else {
            ui.label("No file loaded.");
        }
    }

    /// Returns the length of the time dimension for the active variable, if any.
    fn active_time_dim_len(&self) -> Option<usize> {
        let file = self.data_store.files.get(self.data_store.active_file?)?;
        let var_idx = file.selected_variable?;
        let var = &file.variables[var_idx];
        // Convention: dimension named "time" or "t"
        var.dimensions.iter()
            .find(|(name, _)| name == "time" || name == "t")
            .map(|(_, size)| *size)
    }
}
