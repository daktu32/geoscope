use egui_dock::TabViewer;

use crate::data::DataStore;
use crate::renderer::GlobeRenderer;
use crate::renderer::MapRenderer;
use crate::renderer::hovmoller::HovmollerRenderer;
use crate::renderer::spectrum::SpectrumRenderer;

/// View mode for the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Globe,
    Map,
    Hovmoller,
    Spectrum,
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

    pub const ALL: [Colormap; 2] = [Colormap::Viridis, Colormap::RdBuR];

}

/// Persistent UI state (stored in GeoScopeApp).
#[derive(Debug)]
pub struct UiState {
    pub view_mode: ViewMode,
    pub colormap: Colormap,
    pub time_index: usize,
    pub status_text: String,
    pub playing: bool,
    pub play_speed: f32,
    play_accumulator: f64,
    /// When true, use bilinear interpolation for field data; otherwise nearest-neighbor (grid-point).
    pub interpolated: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Globe,
            colormap: Colormap::default(),
            time_index: 0,
            status_text: "Ready".to_string(),
            playing: false,
            play_speed: 10.0,
            play_accumulator: 0.0,
            interpolated: true,
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
    pub map_renderer: &'a mut MapRenderer,
    pub hovmoller_renderer: &'a mut HovmollerRenderer,
    pub spectrum_renderer: &'a mut SpectrumRenderer,
    pub ui_state: &'a mut UiState,
    /// Incremented when field data changes, triggers GPU upload.
    pub data_generation: &'a mut u64,
}

impl TabViewer for GeoScopeTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::DataBrowser => "Data".into(),
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
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Data").strong().size(14.0));
        ui.add_space(4.0);

        if self.data_store.files.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Drop a .nc file here")
                        .color(egui::Color32::from_gray(128)),
                );
            });
            return;
        }

        // Collect click events to avoid borrow conflict
        let mut load_request: Option<(usize, usize)> = None;

        for (file_idx, file) in self.data_store.files.iter().enumerate() {
            let file_name = std::path::Path::new(&file.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.clone());

            egui::CollapsingHeader::new(
                egui::RichText::new(format!("📁 {file_name}")).size(12.0),
            )
            .default_open(true)
            .show(ui, |ui| {
                for (var_idx, var) in file.variables.iter().enumerate() {
                    let is_coord = var.dimensions.len() <= 1
                        && var.dimensions.first().is_some_and(|(d, _)| d == &var.name);
                    if is_coord {
                        continue;
                    }

                    let is_selected = file.selected_variable == Some(var_idx);

                    // Color indicator based on variable type
                    let indicator_color = if is_selected {
                        egui::Color32::from_rgb(0, 164, 154) // teal / primary
                    } else {
                        egui::Color32::from_gray(100)
                    };

                    let dims: Vec<String> = var
                        .dimensions
                        .iter()
                        .map(|(_, s)| s.to_string())
                        .collect();
                    let dim_text = dims.join("×");

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("●")
                                .color(indicator_color)
                                .size(10.0),
                        );
                        let response = ui.selectable_label(
                            is_selected,
                            egui::RichText::new(&var.name).size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(&dim_text)
                                .size(10.0)
                                .color(egui::Color32::from_gray(128)),
                        );

                        if response.clicked() {
                            load_request = Some((file_idx, var_idx));
                        }

                        response.on_hover_ui(|ui| {
                            ui.label(egui::RichText::new(&var.name).strong());
                            if let Some(ref long_name) = var.long_name {
                                ui.label(long_name.as_str());
                            }
                            if let Some(ref units) = var.units {
                                ui.label(format!("Units: {units}"));
                            }
                            ui.separator();
                            for (dim_name, dim_size) in &var.dimensions {
                                ui.label(format!("  {dim_name}: {dim_size}"));
                            }
                        });
                    });
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
        // Auto-play logic (runs before layout)
        if let Some(time_len) = self.active_time_dim_len() {
            if time_len > 1 && self.ui_state.playing {
                let dt = ui.input(|i| i.stable_dt) as f64;
                self.ui_state.play_accumulator += dt * self.ui_state.play_speed as f64;
                let steps = self.ui_state.play_accumulator as usize;
                if steps > 0 {
                    self.ui_state.play_accumulator -= steps as f64;
                    let new_t = (self.ui_state.time_index + steps) % time_len;
                    if new_t != self.ui_state.time_index {
                        self.ui_state.time_index = new_t;
                        if let Some(fi) = self.data_store.active_file {
                            if let Some(file) = self.data_store.files.get(fi) {
                                if let Some(vi) = file.selected_variable {
                                    if self.data_store.load_field_at(fi, vi, new_t, 0).is_ok() {
                                        *self.data_generation += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                ui.ctx().request_repaint();
            }
        }

        // Bottom controls first (so the view gets remaining space)
        // View mode tab bar
        egui::TopBottomPanel::bottom("viewport_tabs")
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    let primary = egui::Color32::from_rgb(0, 164, 154);
                    for mode in [ViewMode::Globe, ViewMode::Map, ViewMode::Hovmoller, ViewMode::Spectrum] {
                        let label = match mode {
                            ViewMode::Globe => "Globe",
                            ViewMode::Map => "Map",
                            ViewMode::Hovmoller => "Hovmoller",
                            ViewMode::Spectrum => "E(n)",
                        };
                        let is_active = self.ui_state.view_mode == mode;
                        let text = if is_active {
                            egui::RichText::new(label).color(primary).strong().size(12.0)
                        } else {
                            egui::RichText::new(label)
                                .color(egui::Color32::from_gray(160))
                                .size(12.0)
                        };
                        if ui.selectable_label(is_active, text).clicked() {
                            self.ui_state.view_mode = mode;
                        }
                    }
                });
            });

        // Time slider (above tab bar)
        if let Some(time_len) = self.active_time_dim_len() {
            if time_len > 1 {
                egui::TopBottomPanel::bottom("viewport_time")
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            let icon = if self.ui_state.playing { "⏸" } else { "▶" };
                            if ui.button(egui::RichText::new(icon).size(12.0)).clicked() {
                                self.ui_state.playing = !self.ui_state.playing;
                                self.ui_state.play_accumulator = 0.0;
                            }

                            ui.label(
                                egui::RichText::new(format!("t={}", self.ui_state.time_index))
                                    .monospace()
                                    .size(11.0),
                            );

                            let mut t = self.ui_state.time_index;
                            if t >= time_len {
                                t = 0;
                                self.ui_state.time_index = 0;
                            }
                            let max = time_len - 1;
                            let slider = egui::Slider::new(&mut t, 0..=max)
                                .show_value(false);
                            if ui.add(slider).changed() {
                                self.ui_state.time_index = t;
                                self.ui_state.playing = false;
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

                            ui.separator();
                            ui.label(egui::RichText::new("×").size(11.0).color(egui::Color32::from_gray(128)));
                            let mut speed = self.ui_state.play_speed;
                            let speed_slider = egui::Slider::new(&mut speed, 1.0..=60.0)
                                .logarithmic(true)
                                .show_value(true)
                                .suffix(" fps")
                                .custom_formatter(|v, _| format!("{:.0}", v));
                            if ui.add_sized([120.0, 18.0], speed_slider).changed() {
                                self.ui_state.play_speed = speed;
                            }
                        });
                    });
            }
        }

        // Central area: the actual view (gets all remaining space)
        let central = ui.available_rect_before_wrap();
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(central));
        match self.ui_state.view_mode {
            ViewMode::Globe => self.globe_renderer.paint(&mut child_ui),
            ViewMode::Map => self.map_renderer.paint(&mut child_ui),
            ViewMode::Hovmoller => self.hovmoller_renderer.paint(&mut child_ui),
            ViewMode::Spectrum => self.spectrum_renderer.paint(&mut child_ui),
        }
        ui.allocate_rect(central, egui::Sense::hover());
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Inspector").strong().size(14.0));
        ui.add_space(4.0);

        if let Some(file_idx) = self.data_store.active_file {
            if let Some(file) = self.data_store.files.get(file_idx) {
                if let Some(var_idx) = file.selected_variable {
                    let var = &file.variables[var_idx];

                    // Variable name section
                    ui.label(egui::RichText::new("Variable").size(11.0).color(egui::Color32::from_gray(160)));
                    ui.label(egui::RichText::new(&var.name).strong().size(14.0));
                    if let Some(ref units) = var.units {
                        ui.label(
                            egui::RichText::new(units.as_str())
                                .size(11.0)
                                .color(egui::Color32::from_gray(160)),
                        );
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Colormap section
                    ui.label(egui::RichText::new("Colormap").size(11.0).color(egui::Color32::from_gray(160)));
                    ui.add_space(2.0);
                    egui::ComboBox::from_id_salt("colormap_combo")
                        .selected_text(self.ui_state.colormap.label())
                        .width(ui.available_width() - 8.0)
                        .show_ui(ui, |ui| {
                            for cm in Colormap::ALL {
                                ui.selectable_value(
                                    &mut self.ui_state.colormap,
                                    cm,
                                    cm.label(),
                                );
                            }
                        });

                    // Colormap gradient preview (smooth, LUT-based)
                    ui.add_space(4.0);
                    let available_width = ui.available_width() - 8.0;
                    let bar_height = 14.0;
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(available_width, bar_height), egui::Sense::hover());
                    let lut = crate::renderer::common::colormap_lut(self.ui_state.colormap);
                    let painter = ui.painter();
                    // Use an egui Mesh for smooth per-vertex color interpolation
                    let mut mesh = egui::Mesh::default();
                    let n_stops = 64;
                    for i in 0..=n_stops {
                        let t = i as f32 / n_stops as f32;
                        let idx = (t * 255.0) as usize;
                        let base = idx * 4;
                        let color = egui::Color32::from_rgb(lut[base], lut[base + 1], lut[base + 2]);
                        let x = rect.left() + t * rect.width();
                        mesh.colored_vertex(egui::pos2(x, rect.top()), color);
                        mesh.colored_vertex(egui::pos2(x, rect.bottom()), color);
                    }
                    for i in 0..n_stops {
                        let tl = (i * 2) as u32;
                        let bl = tl + 1;
                        let tr = tl + 2;
                        let br = tl + 3;
                        mesh.add_triangle(tl, bl, tr);
                        mesh.add_triangle(bl, br, tr);
                    }
                    painter.add(egui::Shape::mesh(mesh));

                    // Interpolation mode toggle
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Display:").size(11.0).color(egui::Color32::from_gray(160)));
                        if ui.selectable_label(!self.ui_state.interpolated, egui::RichText::new("Grid").size(11.0)).clicked() {
                            self.ui_state.interpolated = false;
                            *self.data_generation += 1;
                        }
                        if ui.selectable_label(self.ui_state.interpolated, egui::RichText::new("Smooth").size(11.0)).clicked() {
                            self.ui_state.interpolated = true;
                            *self.data_generation += 1;
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Range section
                    if let Some(ref field) = file.field_data {
                        ui.label(egui::RichText::new("Range").size(11.0).color(egui::Color32::from_gray(160)));
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("{:.4e}", field.min)).monospace().size(11.0));
                            ui.label(egui::RichText::new("→").size(11.0));
                            ui.label(egui::RichText::new(format!("{:.4e}", field.max)).monospace().size(11.0));
                        });

                        let dims: Vec<String> = var
                            .dimensions
                            .iter()
                            .map(|(n, s)| format!("{n}={s}"))
                            .collect();
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!("{}×{}", field.width, field.height))
                                .monospace()
                                .size(11.0)
                                .color(egui::Color32::from_gray(160)),
                        );
                        ui.label(
                            egui::RichText::new(dims.join(", "))
                                .size(10.0)
                                .color(egui::Color32::from_gray(128)),
                        );
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Inference result
                    let inference = crate::data::inference::infer_variable(var, file.field_data.as_ref());
                    ui.label(egui::RichText::new("Inference").size(11.0).color(egui::Color32::from_gray(160)));
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(&inference.description)
                            .size(11.0),
                    );
                    let confidence_label = match inference.confidence {
                        crate::data::inference::InferenceLevel::L1StandardName => "L1: standard_name",
                        crate::data::inference::InferenceLevel::L2NamePattern => "L2: name pattern",
                        crate::data::inference::InferenceLevel::L3Statistics => "L3: statistics",
                    };
                    ui.label(
                        egui::RichText::new(confidence_label)
                            .size(10.0)
                            .color(egui::Color32::from_gray(128)),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Export PNG button
                    if file.field_data.is_some() {
                        ui.label(egui::RichText::new("Export").size(11.0).color(egui::Color32::from_gray(160)));
                        ui.add_space(2.0);
                        if ui.button("Save PNG").clicked() {
                            if let Some(ref field) = file.field_data {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("PNG", &["png"])
                                    .set_file_name(&format!("{}.png", var.name))
                                    .save_file()
                                {
                                    match crate::renderer::export::export_png(field, self.ui_state.colormap, &path) {
                                        Ok(()) => {
                                            self.ui_state.status_text = format!("Exported: {}", path.display());
                                        }
                                        Err(e) => {
                                            self.ui_state.status_text = format!("Export error: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("Select a variable")
                            .color(egui::Color32::from_gray(128)),
                    );
                }
            }
        } else {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("No file loaded")
                    .color(egui::Color32::from_gray(128)),
            );
        }
    }

    /// Returns the length of the time dimension for the active variable, if any.
    fn active_time_dim_len(&self) -> Option<usize> {
        let file = self.data_store.files.get(self.data_store.active_file?)?;
        let var_idx = file.selected_variable?;
        let var = &file.variables[var_idx];
        var.dimensions
            .iter()
            .find(|(name, _)| name == "time" || name == "t")
            .map(|(_, size)| *size)
    }
}
