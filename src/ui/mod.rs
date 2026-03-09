use egui_dock::TabViewer;

use crate::data::DataStore;
use crate::renderer::GlobeRenderer;
use crate::renderer::MapRenderer;
use crate::renderer::cross_section::CrossSectionRenderer;
use crate::renderer::hovmoller::HovmollerRenderer;
use crate::renderer::map::MapProjection;
use crate::renderer::spectrum::SpectrumRenderer;
use crate::renderer::contour::ContourOverlay;
use crate::renderer::profile::ProfileRenderer;
use crate::renderer::streamline::StreamlineOverlay;
use crate::renderer::trajectory::TrajectoryOverlay;
use crate::renderer::vector_overlay::VectorOverlay;

/// View mode for the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Globe,
    Map,
    Hovmoller,
    Spectrum,
    CrossSection,
    Profile,
}

/// Hover information for Point Info display.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub lon_deg: f32,
    pub lat_deg: f32,
    pub value: f32,
}

/// Profile view mode.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ProfileMode {
    #[default]
    Vertical,
    TimeSeries,
}

/// Colormap selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Colormap {
    // Sequential
    #[default]
    Viridis,
    Plasma,
    Inferno,
    Magma,
    Cividis,
    Turbo,
    // Diverging
    RdBuR,
    Coolwarm,
    Spectral,
    BrBG,
}

impl Colormap {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Viridis => "Viridis",
            Self::Plasma => "Plasma",
            Self::Inferno => "Inferno",
            Self::Magma => "Magma",
            Self::Cividis => "Cividis",
            Self::Turbo => "Turbo",
            Self::RdBuR => "RdBu_r",
            Self::Coolwarm => "Coolwarm",
            Self::Spectral => "Spectral",
            Self::BrBG => "BrBG",
        }
    }

    pub const SEQUENTIAL: [Colormap; 6] = [
        Colormap::Viridis, Colormap::Plasma, Colormap::Inferno,
        Colormap::Magma, Colormap::Cividis, Colormap::Turbo,
    ];

    pub const DIVERGING: [Colormap; 4] = [
        Colormap::RdBuR, Colormap::Coolwarm, Colormap::Spectral, Colormap::BrBG,
    ];

    pub fn description(&self) -> &'static str {
        match self {
            Self::Viridis => "sequential, perceptual",
            Self::Plasma => "sequential, perceptual",
            Self::Inferno => "sequential, perceptual",
            Self::Magma => "sequential, perceptual",
            Self::Cividis => "sequential, colorblind-safe",
            Self::Turbo => "sequential, rainbow",
            Self::RdBuR => "diverging, 0-centered",
            Self::Coolwarm => "diverging, 0-centered",
            Self::Spectral => "diverging, multicolor",
            Self::BrBG => "diverging, 0-centered",
        }
    }
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
    // Map projection
    pub map_projection: MapProjection,
    // Cross-section settings
    pub cross_section_axis: crate::data::CrossSectionAxis,
    pub cross_section_idx: usize,
    // Level selection
    pub level_index: usize,
    // Vector overlay settings
    pub vector_overlay_enabled: bool,
    pub vector_u_var: Option<usize>,
    pub vector_v_var: Option<usize>,
    pub vector_density: usize,
    pub vector_scale: f32,
    // Colormap range mode
    pub range_mode: RangeMode,
    pub manual_min: f32,
    pub manual_max: f32,
    /// Cached global range (computed in app.rs, displayed in Inspector)
    pub global_range: Option<(f32, f32)>,
    // Export dialog
    pub export_dialog_open: bool,
    pub export_settings: crate::renderer::export::ExportSettings,
    // Point Info (cursor hover)
    pub hover_info: Option<HoverInfo>,
    // Profile view
    pub profile_point: Option<(usize, usize)>, // (lon_idx, lat_idx)
    pub profile_mode: ProfileMode,
    // Zonal Mean
    pub zonal_mean_enabled: bool,
    // Contour overlay
    pub contour_enabled: bool,
    pub contour_levels: usize,
    // Streamline overlay
    pub streamline_enabled: bool,
    // Trajectory overlay
    pub trajectory_enabled: bool,
    pub trajectory_lon_var: Option<usize>,
    pub trajectory_lat_var: Option<usize>,
    pub trajectory_trail_length: usize,
    // Visualization suggestion
    pub suggestion: Option<crate::data::inference::VisualizationSuggestion>,
    pub suggestion_dismissed: bool,
}

/// Range mode for colormap scaling.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RangeMode {
    /// Auto-scale from the currently displayed slice.
    #[default]
    Slice,
    /// Auto-scale from the global min/max across all time steps and levels.
    Global,
    /// User-specified fixed min/max.
    Manual,
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
            map_projection: MapProjection::default(),
            cross_section_axis: crate::data::CrossSectionAxis::default(),
            cross_section_idx: 0,
            level_index: 0,
            vector_overlay_enabled: false,
            vector_u_var: None,
            vector_v_var: None,
            vector_density: 8,
            vector_scale: 1.0,
            range_mode: RangeMode::Slice,
            manual_min: 0.0,
            manual_max: 1.0,
            global_range: None,
            export_dialog_open: false,
            export_settings: crate::renderer::export::ExportSettings::default(),
            hover_info: None,
            profile_point: None,
            profile_mode: ProfileMode::default(),
            zonal_mean_enabled: false,
            contour_enabled: false,
            contour_levels: 10,
            streamline_enabled: false,
            trajectory_enabled: false,
            trajectory_lon_var: None,
            trajectory_lat_var: None,
            trajectory_trail_length: 500,
            suggestion: None,
            suggestion_dismissed: false,
        }
    }
}

/// Tab types for the dock layout.
#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    DataBrowser,
    Viewport,
    Inspector,
    CodePanel,
}

/// Tab viewer that renders each panel.
pub struct GeoScopeTabViewer<'a> {
    pub data_store: &'a mut DataStore,
    pub globe_renderer: &'a mut GlobeRenderer,
    pub map_renderer: &'a mut MapRenderer,
    pub hovmoller_renderer: &'a mut HovmollerRenderer,
    pub spectrum_renderer: &'a mut SpectrumRenderer,
    pub cross_section_renderer: &'a mut CrossSectionRenderer,
    pub vector_overlay: &'a mut VectorOverlay,
    pub profile_renderer: &'a mut ProfileRenderer,
    pub contour_overlay: &'a mut ContourOverlay,
    pub streamline_overlay: &'a mut StreamlineOverlay,
    pub trajectory_overlay: &'a mut TrajectoryOverlay,
    pub ui_state: &'a mut UiState,
    /// Incremented when field data changes, triggers GPU upload.
    pub data_generation: &'a mut u64,
    /// Paths requested to open via the UI.
    pub open_file_request: &'a mut Vec<std::path::PathBuf>,
    /// Pre-computed colormap LUTs.
    pub lut_cache: &'a std::collections::HashMap<Colormap, Vec<u8>>,
}

impl TabViewer for GeoScopeTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::DataBrowser => "Data".into(),
            Tab::Viewport => "Globe".into(),
            Tab::Inspector => "Inspector".into(),
            Tab::CodePanel => "Code".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::DataBrowser => self.data_browser_ui(ui),
            Tab::Viewport => self.viewport_ui(ui),
            Tab::Inspector => self.inspector_ui(ui),
            Tab::CodePanel => self.code_panel_ui(ui),
        }
    }
}

impl GeoScopeTabViewer<'_> {
    fn data_browser_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Data").strong().size(14.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("+").size(14.0)).clicked() {
                    let paths = rfd::FileDialog::new()
                        .add_filter("NetCDF", &["nc", "nc4", "netcdf"])
                        .pick_files()
                        .unwrap_or_default();
                    self.open_file_request.extend(paths);
                }
            });
        });
        ui.add_space(4.0);

        if self.data_store.files.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Drop a .nc file here\nor click Open")
                        .color(crate::app::TEXT_CAPTION),
                );
            });
            return;
        }

        // Collect click events to avoid borrow conflict
        let mut load_request: Option<(usize, usize)> = None;
        let mut close_request: Option<usize> = None;

        for (file_idx, file) in self.data_store.files.iter().enumerate() {
            let file_name = std::path::Path::new(&file.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.clone());

            let is_active_file = self.data_store.active_file == Some(file_idx);
            let header_text = if is_active_file {
                egui::RichText::new(format!("📁 {file_name}")).size(12.0).color(crate::app::PRIMARY)
            } else {
                egui::RichText::new(format!("📁 {file_name}")).size(12.0)
            };

            let header_resp = egui::CollapsingHeader::new(header_text)
                .default_open(true)
                .show(ui, |ui| {
                for (var_idx, var) in file.variables.iter().enumerate() {
                    let is_coord = var.dimensions.len() <= 1
                        && var.dimensions.first().is_some_and(|(d, _)| d == &var.name);
                    if is_coord {
                        continue;
                    }

                    let is_selected = is_active_file && file.selected_variable == Some(var_idx);

                    // Color indicator based on variable category (inference)
                    let inference = crate::data::inference::infer_variable(var, None);
                    let indicator_color = if is_selected {
                        crate::app::PRIMARY
                    } else {
                        inference.category.dot_color()
                    };

                    let dim_names: Vec<&str> = var
                        .dimensions
                        .iter()
                        .map(|(name, _)| name.as_str())
                        .collect();
                    let dim_text = format!("({})", dim_names.join(", "));

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
                                .color(crate::app::TEXT_CAPTION),
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
            // Close button painted on header row (using interact to avoid layout disruption)
            let header_rect = header_resp.header_response.rect;
            let btn_size = 14.0;
            let btn_pos = egui::pos2(header_rect.right() - btn_size - 2.0, header_rect.center().y - btn_size * 0.5);
            let btn_rect = egui::Rect::from_min_size(btn_pos, egui::vec2(btn_size, btn_size));
            let btn_id = ui.id().with("close_file").with(file_idx);
            let btn_resp = ui.interact(btn_rect, btn_id, egui::Sense::click());
            let btn_color = if btn_resp.hovered() {
                crate::app::TEXT_HEADING
            } else {
                crate::app::TEXT_CAPTION
            };
            ui.painter().text(
                btn_rect.center(), egui::Align2::CENTER_CENTER,
                "×", egui::FontId::proportional(12.0), btn_color,
            );
            if btn_resp.clicked() {
                close_request = Some(file_idx);
            }
        }

        if let Some((file_idx, var_idx)) = load_request {
            self.data_store.active_file = Some(file_idx);
            if self.data_store.load_field(file_idx, var_idx).is_ok() {
                *self.data_generation += 1;
                self.ui_state.suggestion_dismissed = false;
            }
        }

        if let Some(file_idx) = close_request {
            self.data_store.files.remove(file_idx);
            // Fix active_file index
            if self.data_store.files.is_empty() {
                self.data_store.active_file = None;
            } else if let Some(active) = self.data_store.active_file {
                if active == file_idx {
                    self.data_store.active_file = Some(active.min(self.data_store.files.len() - 1));
                    *self.data_generation += 1;
                } else if active > file_idx {
                    self.data_store.active_file = Some(active - 1);
                }
            }
        }
    }

    fn viewport_ui(&mut self, ui: &mut egui::Ui) {
        // --- Keyboard shortcuts ---
        let ctx = ui.ctx().clone();
        ctx.input(|i| {
            // Space: toggle play/pause
            if i.key_pressed(egui::Key::Space) {
                self.ui_state.playing = !self.ui_state.playing;
                self.ui_state.play_accumulator = 0.0;
            }
            // Left/Right arrows: step time
            if let Some(time_len) = self.active_time_dim_len() {
                if time_len > 1 {
                    if i.key_pressed(egui::Key::ArrowRight) {
                        let new_t = (self.ui_state.time_index + 1) % time_len;
                        self.ui_state.time_index = new_t;
                        self.ui_state.playing = false;
                        if self.ui_state.view_mode != ViewMode::Profile {
                            if let Some(fi) = self.data_store.active_file {
                                if let Some(file) = self.data_store.files.get(fi) {
                                    if let Some(vi) = file.selected_variable {
                                        if self.data_store.load_field_at(fi, vi, new_t, self.ui_state.level_index).is_ok() {
                                            *self.data_generation += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if i.key_pressed(egui::Key::ArrowLeft) {
                        let new_t = if self.ui_state.time_index == 0 { time_len - 1 } else { self.ui_state.time_index - 1 };
                        self.ui_state.time_index = new_t;
                        self.ui_state.playing = false;
                        if self.ui_state.view_mode != ViewMode::Profile {
                            if let Some(fi) = self.data_store.active_file {
                                if let Some(file) = self.data_store.files.get(fi) {
                                    if let Some(vi) = file.selected_variable {
                                        if self.data_store.load_field_at(fi, vi, new_t, self.ui_state.level_index).is_ok() {
                                            *self.data_generation += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Up/Down arrows: step level
            if let Some((_, level_size)) = self.active_level_dim() {
                if level_size > 1 {
                    if i.key_pressed(egui::Key::ArrowUp) {
                        let new_lev = if self.ui_state.level_index == 0 { 0 } else { self.ui_state.level_index - 1 };
                        if new_lev != self.ui_state.level_index {
                            self.ui_state.level_index = new_lev;
                            if let Some(fi) = self.data_store.active_file {
                                if let Some(file) = self.data_store.files.get(fi) {
                                    if let Some(vi) = file.selected_variable {
                                        if self.data_store.load_field_at(fi, vi, self.ui_state.time_index, new_lev).is_ok() {
                                            *self.data_generation += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if i.key_pressed(egui::Key::ArrowDown) {
                        let new_lev = (self.ui_state.level_index + 1).min(level_size - 1);
                        if new_lev != self.ui_state.level_index {
                            self.ui_state.level_index = new_lev;
                            if let Some(fi) = self.data_store.active_file {
                                if let Some(file) = self.data_store.files.get(fi) {
                                    if let Some(vi) = file.selected_variable {
                                        if self.data_store.load_field_at(fi, vi, self.ui_state.time_index, new_lev).is_ok() {
                                            *self.data_generation += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // 1-6: switch view mode
            if i.key_pressed(egui::Key::Num1) { self.ui_state.view_mode = ViewMode::Globe; }
            if i.key_pressed(egui::Key::Num2) { self.ui_state.view_mode = ViewMode::Map; }
            if i.key_pressed(egui::Key::Num3) { self.ui_state.view_mode = ViewMode::Hovmoller; }
            if i.key_pressed(egui::Key::Num4) { self.ui_state.view_mode = ViewMode::Spectrum; }
            if i.key_pressed(egui::Key::Num5) { self.ui_state.view_mode = ViewMode::Profile; }
            if i.key_pressed(egui::Key::Num6) { self.ui_state.view_mode = ViewMode::CrossSection; }
            // G: toggle grid/smooth
            if i.key_pressed(egui::Key::G) {
                self.ui_state.interpolated = !self.ui_state.interpolated;
                *self.data_generation += 1;
            }
            // C: toggle contour
            if i.key_pressed(egui::Key::C) {
                self.ui_state.contour_enabled = !self.ui_state.contour_enabled;
            }
            // V: toggle vector/streamline
            if i.key_pressed(egui::Key::V) {
                self.ui_state.streamline_enabled = !self.ui_state.streamline_enabled;
            }
            // T: toggle trajectory
            if i.key_pressed(egui::Key::T) {
                self.ui_state.trajectory_enabled = !self.ui_state.trajectory_enabled;
            }
        });

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
                        // Profile view only needs the playhead to move — skip expensive field reload
                        if self.ui_state.view_mode != ViewMode::Profile {
                            if let Some(fi) = self.data_store.active_file {
                                if let Some(file) = self.data_store.files.get(fi) {
                                    if let Some(vi) = file.selected_variable {
                                        if self.data_store.load_field_at(fi, vi, new_t, self.ui_state.level_index).is_ok() {
                                            *self.data_generation += 1;
                                        }
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
            .frame(egui::Frame::NONE.fill(crate::app::BG_DARK).inner_margin(egui::Margin::symmetric(8, 4)))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    for mode in [ViewMode::Globe, ViewMode::Map, ViewMode::Hovmoller, ViewMode::Spectrum, ViewMode::CrossSection, ViewMode::Profile] {
                        let label = match mode {
                            ViewMode::Globe => "🌐 Globe",
                            ViewMode::Map => "Map",
                            ViewMode::Hovmoller => "Hovmoller",
                            ViewMode::Spectrum => "Spectrum",
                            ViewMode::CrossSection => "Section",
                            ViewMode::Profile => "Profile",
                        };
                        let is_active = self.ui_state.view_mode == mode;
                        let text = egui::RichText::new(label).size(12.0).color(
                            if is_active { crate::app::TEXT_HEADING } else { crate::app::TEXT_SECONDARY }
                        );
                        let btn = egui::Button::new(text)
                            .fill(if is_active { crate::app::BG_WIDGET } else { egui::Color32::TRANSPARENT })
                            .corner_radius(4.0);
                        if ui.add(btn).clicked() {
                            self.ui_state.view_mode = mode;
                        }
                    }
                });
            });

        // Level slider (vertical, left side of viewport, vertically centered)
        if let Some((level_name, level_size)) = self.active_level_dim() {
            if level_size > 1 {
                egui::SidePanel::left("viewport_level")
                    .exact_width(40.0)
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(4, 8)))
                    .show_inside(ui, |ui| {
                        let max_lev = level_size - 1;
                        let mut lev = self.ui_state.level_index.min(max_lev);

                        // Coordinate value string
                        let coord_str = self.data_store.active_file
                            .and_then(|fi| self.data_store.files.get(fi))
                            .and_then(|f| f.grid.lev.as_ref())
                            .and_then(|lev_vals| lev_vals.get(lev))
                            .map(|&v| {
                                if v.abs() >= 100.0 || (v.abs() < 0.01 && v != 0.0) {
                                    format!("{v:.1e}")
                                } else {
                                    format!("{v:.2}")
                                }
                            })
                            .unwrap_or_else(|| format!("{lev}"));

                        // Center everything vertically
                        let available_h = ui.available_height();
                        let slider_h = (available_h - 60.0).max(40.0);
                        let top_pad = (available_h - slider_h - 40.0).max(0.0) / 2.0;
                        ui.add_space(top_pad);

                        // Label: dimension name
                        ui.label(
                            egui::RichText::new(&level_name)
                                .monospace()
                                .size(9.0)
                                .color(crate::app::TEXT_CAPTION),
                        );

                        // Vertical slider
                        let slider = egui::Slider::new(&mut lev, 0..=max_lev)
                            .vertical()
                            .show_value(false);
                        if ui.add_sized([28.0, slider_h], slider).changed() {
                            self.ui_state.level_index = lev;
                            if let Some(fi) = self.data_store.active_file {
                                if let Some(file) = self.data_store.files.get(fi) {
                                    if let Some(vi) = file.selected_variable {
                                        let t = self.ui_state.time_index;
                                        if self.data_store.load_field_at(fi, vi, t, lev).is_ok() {
                                            *self.data_generation += 1;
                                        }
                                    }
                                }
                            }
                        }

                        // Current value below slider
                        ui.label(
                            egui::RichText::new(coord_str)
                                .monospace()
                                .size(10.0)
                                .color(crate::app::TEXT_SECONDARY),
                        );
                    });
            }
        }

        // Time controls (video player style: full-width seekbar + control row)
        if let Some(time_len) = self.active_time_dim_len() {
            if time_len > 1 {
                egui::TopBottomPanel::bottom("viewport_time")
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 0)))
                    .show_inside(ui, |ui| {
                        let mut t = self.ui_state.time_index;
                        if t >= time_len {
                            t = 0;
                            self.ui_state.time_index = 0;
                        }
                        let max = time_len - 1;

                        // Row 1: Full-width seekbar (no side margins)
                        ui.spacing_mut().slider_width = ui.available_width() - 16.0;
                        let slider = egui::Slider::new(&mut t, 0..=max)
                            .show_value(false);
                        if ui.add_sized([ui.available_width(), 14.0], slider).changed() {
                            self.ui_state.time_index = t;
                            self.ui_state.playing = false;
                            if self.ui_state.view_mode != ViewMode::Profile {
                                if let Some(fi) = self.data_store.active_file {
                                    if let Some(file) = self.data_store.files.get(fi) {
                                        if let Some(vi) = file.selected_variable {
                                            if self.data_store.load_field_at(fi, vi, t, self.ui_state.level_index).is_ok() {
                                                *self.data_generation += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Row 2: ⏮ ▶ ⏭  3/1000                      10fps
                        let mut step_delta: isize = 0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.add_space(4.0);

                            // Step backward
                            if ui.button(egui::RichText::new("⏮").size(12.0)).clicked() {
                                step_delta = -1;
                            }

                            // Play/Pause
                            let icon = if self.ui_state.playing { "⏸" } else { "▶" };
                            if ui.button(egui::RichText::new(icon).size(13.0)).clicked() {
                                self.ui_state.playing = !self.ui_state.playing;
                                self.ui_state.play_accumulator = 0.0;
                            }

                            // Step forward
                            if ui.button(egui::RichText::new("⏭").size(12.0)).clicked() {
                                step_delta = 1;
                            }

                            // Step counter
                            ui.label(
                                egui::RichText::new(format!("{}/{}", t, max))
                                    .monospace()
                                    .size(11.0)
                                    .color(crate::app::TEXT_SECONDARY),
                            );

                            // Right-aligned FPS
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.add_space(4.0);
                                let fps_text = format!("{:.0}fps", self.ui_state.play_speed);
                                let fps_btn = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(fps_text)
                                            .monospace()
                                            .size(10.0)
                                            .color(crate::app::TEXT_CAPTION),
                                    )
                                    .frame(false),
                                );
                                if fps_btn.clicked() {
                                    let fps_steps = [1.0, 2.0, 5.0, 10.0, 30.0, 60.0];
                                    let current = self.ui_state.play_speed;
                                    let next = fps_steps.iter()
                                        .find(|&&s| s > current + 0.5)
                                        .copied()
                                        .unwrap_or(fps_steps[0]);
                                    self.ui_state.play_speed = next;
                                }
                                if fps_btn.hovered() {
                                    egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("fps_tooltip"), |ui| {
                                        ui.label("Click to cycle FPS");
                                    });
                                }
                            });
                        });

                        // Apply step button
                        if step_delta != 0 {
                            let new_t = if step_delta > 0 {
                                (t + 1) % time_len
                            } else {
                                if t == 0 { max } else { t - 1 }
                            };
                            self.ui_state.time_index = new_t;
                            self.ui_state.playing = false;
                            if self.ui_state.view_mode != ViewMode::Profile {
                                if let Some(fi) = self.data_store.active_file {
                                    if let Some(file) = self.data_store.files.get(fi) {
                                        if let Some(vi) = file.selected_variable {
                                            if self.data_store.load_field_at(fi, vi, new_t, self.ui_state.level_index).is_ok() {
                                                *self.data_generation += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
            }
        }

        // Central area: the actual view (gets all remaining space)
        let central = ui.available_rect_before_wrap();
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(central));
        match self.ui_state.view_mode {
            ViewMode::Globe => {
                self.globe_renderer.paint(&mut child_ui);
                // Overlays on Globe
                let avail = central.size();
                let pad_x = (avail.x * 0.05).max(8.0);
                let pad_y = (avail.y * 0.05).max(8.0);
                let padded = egui::vec2(avail.x - pad_x * 2.0, avail.y - pad_y * 2.0);
                let globe_rect = egui::Rect::from_center_size(central.center(), padded);

                // Shared view/view_proj for all Globe overlays
                let (view_for_overlays, view_proj_for_overlays) = crate::renderer::common::build_view_proj(
                    self.globe_renderer.cam_lon,
                    self.globe_renderer.cam_lat,
                    self.globe_renderer.zoom,
                    globe_rect,
                );
                if self.ui_state.vector_overlay_enabled {
                    self.vector_overlay.paint_on_globe(
                        child_ui.painter(),
                        globe_rect,
                        &view_for_overlays,
                        &view_proj_for_overlays,
                    );
                }
                if self.ui_state.contour_enabled {
                    self.contour_overlay.paint_on_globe(
                        child_ui.painter(),
                        globe_rect,
                        &view_for_overlays,
                        &view_proj_for_overlays,
                    );
                }
                if self.ui_state.trajectory_enabled {
                    self.trajectory_overlay.paint_on_globe(
                        child_ui.painter(),
                        globe_rect,
                        &view_for_overlays,
                        &view_proj_for_overlays,
                    );
                }
                self.ui_state.hover_info = None;
            }
            ViewMode::Map => {
                self.map_renderer.paint(&mut child_ui);
                if self.ui_state.vector_overlay_enabled {
                    self.vector_overlay.paint_on_map(
                        child_ui.painter(),
                        central,
                        self.map_renderer.pan_x,
                        self.map_renderer.pan_y,
                        self.map_renderer.zoom,
                    );
                }
                if self.ui_state.contour_enabled {
                    self.contour_overlay.paint_on_map(
                        child_ui.painter(),
                        central,
                        self.map_renderer.pan_x,
                        self.map_renderer.pan_y,
                        self.map_renderer.zoom,
                    );
                }
                if self.ui_state.streamline_enabled {
                    self.streamline_overlay.paint_on_map(
                        child_ui.painter(),
                        central,
                        self.map_renderer.pan_x,
                        self.map_renderer.pan_y,
                        self.map_renderer.zoom,
                    );
                }
                if self.ui_state.trajectory_enabled {
                    self.trajectory_overlay.paint_on_map(
                        child_ui.painter(),
                        central,
                        self.map_renderer.pan_x,
                        self.map_renderer.pan_y,
                        self.map_renderer.zoom,
                    );
                }
                // Map hover → Point Info (Equirectangular only)
                if self.map_renderer.projection == MapProjection::Equirectangular {
                    let hover_pos = child_ui.input(|i| i.pointer.hover_pos());
                    if let Some(pos) = hover_pos {
                        if central.contains(pos) {
                            // Screen → NDC (account for pan/zoom via inverse ortho)
                            let aspect = central.width() / central.height().max(1.0);
                            let (sx, sy) = if aspect > 1.0 {
                                (self.map_renderer.zoom / aspect, self.map_renderer.zoom)
                            } else {
                                (self.map_renderer.zoom, self.map_renderer.zoom * aspect)
                            };
                            // Screen pos → normalized [-1,1] in rect
                            let nx = (pos.x - central.center().x) / (central.width() * 0.5);
                            let ny = -(pos.y - central.center().y) / (central.height() * 0.5);
                            // Inverse ortho transform → world coords
                            let wx = (nx + self.map_renderer.pan_x * sx) / sx;
                            let wy = (ny + self.map_renderer.pan_y * sy) / sy;
                            // World [-1,1] → UV [0,1]
                            let u = (wx + 1.0) * 0.5;
                            let v = (1.0 - wy) * 0.5;
                            if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                                let lon_deg = u * 360.0 - 180.0;
                                let lat_deg = 90.0 - v * 180.0;
                                // Look up data value at nearest grid point
                                let value = self.data_store.active_field().map(|field| {
                                    let gx = ((u * field.width as f32) as usize).min(field.width - 1);
                                    let gy = ((v * field.height as f32) as usize).min(field.height - 1);
                                    field.values[gy * field.width + gx]
                                });
                                if let Some(val) = value {
                                    self.ui_state.hover_info = Some(HoverInfo {
                                        lon_deg,
                                        lat_deg,
                                        value: val,
                                    });
                                }
                            } else {
                                self.ui_state.hover_info = None;
                            }
                        } else {
                            self.ui_state.hover_info = None;
                        }
                    } else {
                        self.ui_state.hover_info = None;
                    }
                } else {
                    self.ui_state.hover_info = None;
                }
            }
            ViewMode::Hovmoller => self.hovmoller_renderer.paint(&mut child_ui),
            ViewMode::Spectrum => self.spectrum_renderer.paint(&mut child_ui),
            ViewMode::CrossSection => self.cross_section_renderer.paint(&mut child_ui),
            ViewMode::Profile => {
                self.profile_renderer.paint(&mut child_ui);
            }
        }
        // Hover info overlay (bottom-left of viewport)
        if let Some(ref info) = self.ui_state.hover_info {
            let lon_label = if info.lon_deg >= 0.0 {
                format!("{:.1}\u{00b0}E", info.lon_deg)
            } else {
                format!("{:.1}\u{00b0}W", -info.lon_deg)
            };
            let lat_label = if info.lat_deg >= 0.0 {
                format!("{:.1}\u{00b0}N", info.lat_deg)
            } else {
                format!("{:.1}\u{00b0}S", -info.lat_deg)
            };
            let text = format!("{}, {}  Value: {:.4e}", lon_label, lat_label, info.value);
            let font = egui::FontId::monospace(11.0);
            let painter = ui.painter();
            let galley = painter.layout_no_wrap(text, font, crate::app::TEXT_BODY);
            let text_size = galley.size();
            let margin = 6.0;
            let pill_rect = egui::Rect::from_min_size(
                egui::pos2(central.left() + 8.0, central.bottom() - text_size.y - margin * 2.0 - 8.0),
                egui::vec2(text_size.x + margin * 2.0, text_size.y + margin * 2.0),
            );
            painter.rect_filled(
                pill_rect,
                4.0,
                egui::Color32::from_rgba_unmultiplied(15, 15, 23, 200),
            );
            painter.galley(
                egui::pos2(pill_rect.left() + margin, pill_rect.top() + margin),
                galley,
                crate::app::TEXT_BODY,
            );
        }

        // Floating zoom controls (bottom-right corner, Globe/Map only)
        if matches!(self.ui_state.view_mode, ViewMode::Globe | ViewMode::Map) {
            let btn_size = egui::vec2(26.0, 26.0);
            let margin = 10.0;
            let gap = 2.0;
            // Stack vertically: [+] above [−]
            let minus_pos = egui::pos2(
                central.right() - btn_size.x - margin,
                central.bottom() - btn_size.y - margin,
            );
            let plus_pos = egui::pos2(minus_pos.x, minus_pos.y - btn_size.y - gap);

            let plus_rect = egui::Rect::from_min_size(plus_pos, btn_size);
            let minus_rect = egui::Rect::from_min_size(minus_pos, btn_size);

            let plus_resp = ui.allocate_rect(plus_rect, egui::Sense::click());
            let minus_resp = ui.allocate_rect(minus_rect, egui::Sense::click());

            let bg = egui::Color32::from_rgba_unmultiplied(15, 15, 23, 180);
            let combined_rect = egui::Rect::from_min_max(plus_pos, minus_pos + btn_size);
            let painter = ui.painter();
            painter.rect_filled(combined_rect, 4.0, bg);

            let text_color = if plus_resp.hovered() { crate::app::TEXT_HEADING } else { crate::app::TEXT_SECONDARY };
            painter.text(plus_rect.center(), egui::Align2::CENTER_CENTER,
                "+", egui::FontId::monospace(14.0), text_color);

            let text_color = if minus_resp.hovered() { crate::app::TEXT_HEADING } else { crate::app::TEXT_SECONDARY };
            painter.text(minus_rect.center(), egui::Align2::CENTER_CENTER,
                "−", egui::FontId::monospace(14.0), text_color);

            // Divider line between + and −
            let div_y = plus_rect.bottom();
            painter.line_segment(
                [egui::pos2(plus_rect.left() + 4.0, div_y), egui::pos2(plus_rect.right() - 4.0, div_y)],
                egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
            );

            let zoom_factor = 1.15;
            match self.ui_state.view_mode {
                ViewMode::Globe => {
                    if plus_resp.clicked() {
                        self.globe_renderer.zoom = (self.globe_renderer.zoom * zoom_factor).min(5.0);
                    }
                    if minus_resp.clicked() {
                        self.globe_renderer.zoom = (self.globe_renderer.zoom / zoom_factor).max(0.3);
                    }
                }
                ViewMode::Map => {
                    if plus_resp.clicked() {
                        self.map_renderer.zoom = (self.map_renderer.zoom * zoom_factor).min(10.0);
                    }
                    if minus_resp.clicked() {
                        self.map_renderer.zoom = (self.map_renderer.zoom / zoom_factor).max(0.3);
                    }
                }
                _ => {}
            }
        }

        ui.allocate_rect(central, egui::Sense::hover());
    }

    /// Helper: draw a section header label.
    fn section_header(ui: &mut egui::Ui, text: &str) {
        ui.label(egui::RichText::new(text).size(11.0).strong().color(crate::app::TEXT_SECONDARY));
    }

    /// Helper: draw a dim label.
    fn dim_label(ui: &mut egui::Ui, text: &str) {
        ui.label(egui::RichText::new(text).size(10.0).color(crate::app::TEXT_CAPTION));
    }

    /// Helper: draw colorbar gradient mesh into an allocated rect, returns the rect.
    fn draw_colorbar(ui: &mut egui::Ui, lut: &[u8], width: f32, height: f32) -> egui::Rect {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let painter = ui.painter();
        let mut mesh = egui::Mesh::default();
        let n = 64usize;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let idx = (t * 255.0) as usize;
            let base = idx * 4;
            let color = egui::Color32::from_rgb(lut[base], lut[base + 1], lut[base + 2]);
            let x = rect.left() + t * rect.width();
            mesh.colored_vertex(egui::pos2(x, rect.top()), color);
            mesh.colored_vertex(egui::pos2(x, rect.bottom()), color);
        }
        for i in 0..n {
            let tl = (i * 2) as u32;
            mesh.add_triangle(tl, tl + 1, tl + 2);
            mesh.add_triangle(tl + 1, tl + 3, tl + 2);
        }
        painter.add(egui::Shape::mesh(mesh));
        rect
    }

    fn inspector_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Inspector").strong().size(13.0));
        ui.add_space(6.0);

        let mut inspector_load_request: Option<(usize, usize)> = None;

        if let Some(file_idx) = self.data_store.active_file {
            if let Some(file) = self.data_store.files.get(file_idx) {
                if let Some(var_idx) = file.selected_variable {
                    let var = &file.variables[var_idx];

                    // --- Variable ---
                    Self::section_header(ui, "Variable");
                    ui.add_space(2.0);
                    {
                        let var_names: Vec<String> = file.variables.iter().enumerate()
                            .filter(|(_, v)| !(v.dimensions.len() <= 1 && v.dimensions.first().is_some_and(|(d, _)| d == &v.name)))
                            .map(|(_, v)| v.name.clone())
                            .collect();
                        let var_indices: Vec<usize> = file.variables.iter().enumerate()
                            .filter(|(_, v)| !(v.dimensions.len() <= 1 && v.dimensions.first().is_some_and(|(d, _)| d == &v.name)))
                            .map(|(i, _)| i)
                            .collect();
                        let current_name = var.name.clone();
                        let mut selected_pos = var_indices.iter().position(|&i| i == var_idx).unwrap_or(0);
                        egui::ComboBox::from_id_salt("inspector_var_combo")
                            .selected_text(&current_name)
                            .width(ui.available_width() - 8.0)
                            .show_ui(ui, |ui| {
                                for (pos, name) in var_names.iter().enumerate() {
                                    ui.selectable_value(&mut selected_pos, pos, name);
                                }
                            });
                        if let Some(&new_var_idx) = var_indices.get(selected_pos) {
                            if new_var_idx != var_idx {
                                inspector_load_request = Some((file_idx, new_var_idx));
                            }
                        }
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // --- Projection ---
                    Self::section_header(ui, "Projection");
                    ui.add_space(3.0);
                    if self.ui_state.view_mode == ViewMode::Map {
                        egui::ComboBox::from_id_salt("projection_combo_main")
                            .selected_text(match self.ui_state.map_projection {
                                MapProjection::Equirectangular => "Equirectangular",
                                MapProjection::Mollweide => "Mollweide",
                                MapProjection::PolarNorth => "Polar (North)",
                                MapProjection::PolarSouth => "Polar (South)",
                            })
                            .width(ui.available_width() - 8.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.ui_state.map_projection, MapProjection::Equirectangular, "Equirectangular");
                                ui.selectable_value(&mut self.ui_state.map_projection, MapProjection::Mollweide, "Mollweide");
                                ui.selectable_value(&mut self.ui_state.map_projection, MapProjection::PolarNorth, "Polar (North)");
                                ui.selectable_value(&mut self.ui_state.map_projection, MapProjection::PolarSouth, "Polar (South)");
                            });
                    } else {
                        let proj_label = match self.ui_state.view_mode {
                            ViewMode::Globe => "Orthographic",
                            ViewMode::Hovmoller => "Time-Longitude",
                            ViewMode::Spectrum => "Log-Log",
                            ViewMode::CrossSection => "Level-Space",
                            ViewMode::Profile => "Line Graph",
                            ViewMode::Map => unreachable!(),
                        };
                        egui::ComboBox::from_id_salt("projection_combo_main")
                            .selected_text(proj_label)
                            .width(ui.available_width() - 8.0)
                            .show_ui(ui, |_ui| {
                                // Read-only for non-Map views
                            });
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // --- Colormap ---
                    Self::section_header(ui, "Colormap");
                    ui.add_space(3.0);
                    egui::ComboBox::from_id_salt("colormap_combo")
                        .selected_text(self.ui_state.colormap.label())
                        .width(ui.available_width() - 8.0)
                        .show_ui(ui, |ui| {
                            ui.label(egui::RichText::new("Sequential").size(10.0).color(crate::app::TEXT_CAPTION));
                            for cm in Colormap::SEQUENTIAL {
                                ui.selectable_value(&mut self.ui_state.colormap, cm, cm.label());
                            }
                            ui.separator();
                            ui.label(egui::RichText::new("Diverging").size(10.0).color(crate::app::TEXT_CAPTION));
                            for cm in Colormap::DIVERGING {
                                ui.selectable_value(&mut self.ui_state.colormap, cm, cm.label());
                            }
                        });

                    // Gradient preview
                    ui.add_space(4.0);
                    let bar_w = ui.available_width() - 8.0;
                    let lut = &self.lut_cache[&self.ui_state.colormap];
                    let bar_rect = Self::draw_colorbar(ui, lut, bar_w, 20.0);
                    // Description label below colorbar
                    ui.label(
                        egui::RichText::new(format!("{} ({})", self.ui_state.colormap.label(), self.ui_state.colormap.description()))
                            .size(10.0)
                            .color(crate::app::TEXT_CAPTION),
                    );

                    // Min/max labels below description
                    if let Some(ref field) = file.field_data {
                        let (dmin, dmax) = match self.ui_state.range_mode {
                            RangeMode::Slice => (field.min, field.max),
                            RangeMode::Global => self.ui_state.global_range.unwrap_or((field.min, field.max)),
                            RangeMode::Manual => (self.ui_state.manual_min, self.ui_state.manual_max),
                        };
                        let painter = ui.painter();
                        let label_color = crate::app::TEXT_SECONDARY;
                        let font = egui::FontId::monospace(10.0);
                        painter.text(
                            egui::pos2(bar_rect.left(), bar_rect.bottom() + 1.0),
                            egui::Align2::LEFT_TOP, format!("{:.3e}", dmin), font.clone(), label_color,
                        );
                        painter.text(
                            egui::pos2(bar_rect.right(), bar_rect.bottom() + 1.0),
                            egui::Align2::RIGHT_TOP, format!("{:.3e}", dmax), font, label_color,
                        );
                        ui.add_space(12.0);
                    }

                    // Display mode toggle
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        Self::dim_label(ui, "Display");
                        for (label, is_smooth) in [("Grid", false), ("Smooth", true)] {
                            let active = self.ui_state.interpolated == is_smooth;
                            let text = egui::RichText::new(label).size(10.0).color(
                                if active { crate::app::PRIMARY } else { crate::app::TEXT_SECONDARY }
                            );
                            let btn = egui::Button::new(text)
                                .fill(if active { crate::app::BG_WIDGET } else { egui::Color32::TRANSPARENT })
                                .corner_radius(3.0);
                            if ui.add(btn).clicked() {
                                self.ui_state.interpolated = is_smooth;
                                *self.data_generation += 1;
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // --- Range ---
                    if let Some(ref field) = file.field_data {
                        Self::section_header(ui, "Range");
                        ui.add_space(3.0);

                        // Min/Max DragValues with "to" separator (always visible)
                        let (display_min, display_max) = match self.ui_state.range_mode {
                            RangeMode::Slice => (field.min, field.max),
                            RangeMode::Global => self.ui_state.global_range.unwrap_or((field.min, field.max)),
                            RangeMode::Manual => (self.ui_state.manual_min, self.ui_state.manual_max),
                        };
                        ui.horizontal(|ui| {
                            let w = (ui.available_width() - 30.0) / 2.0;
                            if self.ui_state.range_mode == RangeMode::Manual {
                                if ui.add_sized([w, 20.0], egui::DragValue::new(&mut self.ui_state.manual_min).speed(0.001).max_decimals(4)).changed() {
                                    *self.data_generation += 1;
                                }
                                ui.label(egui::RichText::new("to").size(10.0).color(crate::app::TEXT_CAPTION));
                                if ui.add_sized([w, 20.0], egui::DragValue::new(&mut self.ui_state.manual_max).speed(0.001).max_decimals(4)).changed() {
                                    *self.data_generation += 1;
                                }
                            } else {
                                ui.add_sized([w, 20.0], egui::Label::new(
                                    egui::RichText::new(format!("{:.3e}", display_min)).monospace().size(11.0)
                                ));
                                ui.label(egui::RichText::new("to").size(10.0).color(crate::app::TEXT_CAPTION));
                                ui.add_sized([w, 20.0], egui::Label::new(
                                    egui::RichText::new(format!("{:.3e}", display_max)).monospace().size(11.0)
                                ));
                            }
                        });

                        // Scale mode buttons
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            for (mode, label) in [(RangeMode::Slice, "Slice"), (RangeMode::Global, "Global"), (RangeMode::Manual, "Manual")] {
                                let active = self.ui_state.range_mode == mode;
                                let text = egui::RichText::new(label).size(10.0).color(
                                    if active { crate::app::PRIMARY } else { crate::app::TEXT_SECONDARY }
                                );
                                let btn = egui::Button::new(text)
                                    .fill(if active { crate::app::BG_WIDGET } else { egui::Color32::TRANSPARENT })
                                    .corner_radius(3.0);
                                if ui.add(btn).clicked() {
                                    self.ui_state.range_mode = mode;
                                    if mode == RangeMode::Manual {
                                        self.ui_state.manual_min = display_min;
                                        self.ui_state.manual_max = display_max;
                                    }
                                    *self.data_generation += 1;
                                }
                            }
                        });

                        // Symmetric (0-centered) checkbox
                        if Colormap::DIVERGING.contains(&self.ui_state.colormap) {
                            ui.add_space(2.0);
                            let mut symmetric = self.ui_state.range_mode == RangeMode::Manual
                                && (self.ui_state.manual_min + self.ui_state.manual_max).abs() < 1e-10;
                            if ui.checkbox(&mut symmetric, egui::RichText::new("Symmetric (0-centered)").size(10.0)).changed() {
                                if symmetric {
                                    let abs_max = display_min.abs().max(display_max.abs());
                                    self.ui_state.range_mode = RangeMode::Manual;
                                    self.ui_state.manual_min = -abs_max;
                                    self.ui_state.manual_max = abs_max;
                                    *self.data_generation += 1;
                                }
                            }
                        }
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // --- View-specific settings ---

                    // Cross-section (CrossSection view only)
                    if self.ui_state.view_mode == ViewMode::CrossSection {
                        Self::section_header(ui, "Cross Section");
                        ui.add_space(3.0);
                        ui.horizontal(|ui| {
                            Self::dim_label(ui, "Axis");
                            if ui.selectable_label(
                                self.ui_state.cross_section_axis == crate::data::CrossSectionAxis::Latitude,
                                egui::RichText::new("Fix Lat").size(10.0),
                            ).clicked() {
                                self.ui_state.cross_section_axis = crate::data::CrossSectionAxis::Latitude;
                                *self.data_generation += 1;
                            }
                            if ui.selectable_label(
                                self.ui_state.cross_section_axis == crate::data::CrossSectionAxis::Longitude,
                                egui::RichText::new("Fix Lon").size(10.0),
                            ).clicked() {
                                self.ui_state.cross_section_axis = crate::data::CrossSectionAxis::Longitude;
                                *self.data_generation += 1;
                            }
                        });

                        let max_idx = if let Some(ref field) = file.field_data {
                            match self.ui_state.cross_section_axis {
                                crate::data::CrossSectionAxis::Latitude => field.height.saturating_sub(1),
                                crate::data::CrossSectionAxis::Longitude => field.width.saturating_sub(1),
                            }
                        } else { 0 };

                        if max_idx > 0 {
                            let mut idx = self.ui_state.cross_section_idx.min(max_idx);
                            // Show coordinate value next to slider
                            let coord_label = match self.ui_state.cross_section_axis {
                                crate::data::CrossSectionAxis::Latitude => {
                                    // Fix Lat: show latitude value
                                    file.grid.lat.as_ref()
                                        .and_then(|lat| lat.get(idx))
                                        .map(|&v| format!("{v:.1}°"))
                                        .unwrap_or_default()
                                }
                                crate::data::CrossSectionAxis::Longitude => {
                                    // Fix Lon: show longitude value
                                    file.grid.lon.as_ref()
                                        .and_then(|lon| lon.get(idx))
                                        .map(|&v| format!("{v:.1}°"))
                                        .unwrap_or_default()
                                }
                            };
                            let slider_text = if coord_label.is_empty() {
                                "Index".to_string()
                            } else {
                                coord_label
                            };
                            if ui.add(egui::Slider::new(&mut idx, 0..=max_idx).text(slider_text)).changed() {
                                self.ui_state.cross_section_idx = idx;
                                *self.data_generation += 1;
                            }
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                    }

                    // Vector overlay (Globe/Map views)
                    if self.ui_state.view_mode == ViewMode::Globe || self.ui_state.view_mode == ViewMode::Map {
                        Self::section_header(ui, "Vector Overlay");
                        ui.add_space(3.0);
                        ui.checkbox(&mut self.ui_state.vector_overlay_enabled, egui::RichText::new("Enabled").size(11.0));

                        if self.ui_state.vector_overlay_enabled {
                            if self.ui_state.vector_u_var.is_none() {
                                if let Some((u_idx, v_idx)) = crate::data::inference::detect_wind_pair(&file.variables) {
                                    self.ui_state.vector_u_var = Some(u_idx);
                                    self.ui_state.vector_v_var = Some(v_idx);
                                }
                            }

                            let var_names: Vec<String> = file.variables.iter().map(|v| v.name.clone()).collect();
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("u:").size(10.0));
                                let combo_w = (ui.available_width() - crate::app::SP_SM).max(60.0);
                                let mut u_idx = self.ui_state.vector_u_var.unwrap_or(0);
                                egui::ComboBox::from_id_salt("vector_u_combo")
                                    .selected_text(var_names.get(u_idx).map(|s| s.as_str()).unwrap_or("?"))
                                    .width(combo_w)
                                    .show_ui(ui, |ui| {
                                        for (i, name) in var_names.iter().enumerate() {
                                            ui.selectable_value(&mut u_idx, i, name);
                                        }
                                    });
                                self.ui_state.vector_u_var = Some(u_idx);
                            });
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("v:").size(10.0));
                                let combo_w = (ui.available_width() - crate::app::SP_SM).max(60.0);
                                let mut v_idx = self.ui_state.vector_v_var.unwrap_or(0);
                                egui::ComboBox::from_id_salt("vector_v_combo")
                                    .selected_text(var_names.get(v_idx).map(|s| s.as_str()).unwrap_or("?"))
                                    .width(combo_w)
                                    .show_ui(ui, |ui| {
                                        for (i, name) in var_names.iter().enumerate() {
                                            ui.selectable_value(&mut v_idx, i, name);
                                        }
                                    });
                                self.ui_state.vector_v_var = Some(v_idx);
                            });

                            let mut density = self.ui_state.vector_density;
                            if ui.add(egui::Slider::new(&mut density, 2..=20).text("Density")).changed() {
                                self.ui_state.vector_density = density;
                            }
                            let mut scale = self.ui_state.vector_scale;
                            if ui.add(egui::Slider::new(&mut scale, 0.1..=5.0).text("Scale")).changed() {
                                self.ui_state.vector_scale = scale;
                            }
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                    }

                    // Contour overlay (Globe/Map views)
                    if self.ui_state.view_mode == ViewMode::Globe || self.ui_state.view_mode == ViewMode::Map {
                        Self::section_header(ui, "Contour Lines");
                        ui.add_space(3.0);
                        ui.checkbox(&mut self.ui_state.contour_enabled, egui::RichText::new("Enabled").size(11.0));
                        if self.ui_state.contour_enabled {
                            let mut levels = self.ui_state.contour_levels;
                            if ui.add(egui::Slider::new(&mut levels, 3..=30).text("Levels")).changed() {
                                self.ui_state.contour_levels = levels;
                            }
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                    }

                    // Streamline overlay (Map view only for now)
                    if self.ui_state.view_mode == ViewMode::Map {
                        Self::section_header(ui, "Streamlines");
                        ui.add_space(3.0);
                        ui.checkbox(&mut self.ui_state.streamline_enabled, egui::RichText::new("Enabled").size(11.0));
                        if self.ui_state.streamline_enabled {
                            if self.ui_state.vector_u_var.is_none() {
                                if let Some((u_idx, v_idx)) = crate::data::inference::detect_wind_pair(&file.variables) {
                                    self.ui_state.vector_u_var = Some(u_idx);
                                    self.ui_state.vector_v_var = Some(v_idx);
                                }
                            }
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                    }

                    // Trajectory overlay (Globe/Map views)
                    if self.ui_state.view_mode == ViewMode::Globe || self.ui_state.view_mode == ViewMode::Map {
                        Self::section_header(ui, "Trajectory");
                        ui.add_space(3.0);
                        ui.checkbox(&mut self.ui_state.trajectory_enabled, egui::RichText::new("Enabled").size(11.0));

                        if self.ui_state.trajectory_enabled {
                            // Auto-detect trajectory pair on first enable
                            if self.ui_state.trajectory_lon_var.is_none() {
                                if let Some((lon_idx, lat_idx)) = crate::data::inference::detect_trajectory_pair(&file.variables) {
                                    self.ui_state.trajectory_lon_var = Some(lon_idx);
                                    self.ui_state.trajectory_lat_var = Some(lat_idx);
                                    *self.data_generation += 1;
                                }
                            }

                            if self.ui_state.trajectory_lon_var.is_some() {
                                let var_names: Vec<String> = file.variables.iter().map(|v| v.name.clone()).collect();
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("lon:").size(10.0));
                                    let combo_w = (ui.available_width() - crate::app::SP_SM).max(60.0);
                                    let mut lon_idx = self.ui_state.trajectory_lon_var.unwrap_or(0);
                                    egui::ComboBox::from_id_salt("traj_lon_combo")
                                        .selected_text(var_names.get(lon_idx).map(|s| s.as_str()).unwrap_or("?"))
                                        .width(combo_w)
                                        .show_ui(ui, |ui| {
                                            for (i, name) in var_names.iter().enumerate() {
                                                ui.selectable_value(&mut lon_idx, i, name);
                                            }
                                        });
                                    self.ui_state.trajectory_lon_var = Some(lon_idx);
                                });
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("lat:").size(10.0));
                                    let combo_w = (ui.available_width() - crate::app::SP_SM).max(60.0);
                                    let mut lat_idx = self.ui_state.trajectory_lat_var.unwrap_or(0);
                                    egui::ComboBox::from_id_salt("traj_lat_combo")
                                        .selected_text(var_names.get(lat_idx).map(|s| s.as_str()).unwrap_or("?"))
                                        .width(combo_w)
                                        .show_ui(ui, |ui| {
                                            for (i, name) in var_names.iter().enumerate() {
                                                ui.selectable_value(&mut lat_idx, i, name);
                                            }
                                        });
                                    self.ui_state.trajectory_lat_var = Some(lat_idx);
                                });

                                let mut trail = self.ui_state.trajectory_trail_length;
                                if ui.add(egui::Slider::new(&mut trail, 10..=2000).logarithmic(true).text("Trail")).changed() {
                                    self.ui_state.trajectory_trail_length = trail;
                                }
                            } else {
                                ui.label(egui::RichText::new("No trajectory pair detected").size(10.0).color(crate::app::TEXT_CAPTION));
                            }
                        }
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                    }

                    // --- Suggestion ---
                    {
                        let inference = crate::data::inference::infer_variable(var, file.field_data.as_ref());
                        let suggestion = crate::data::inference::suggest_visualization(var, &inference, &file.variables);
                        if !self.ui_state.suggestion_dismissed {
                            Self::section_header(ui, "Suggested");
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(&suggestion.description).size(10.0));

                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(
                                    egui::RichText::new("Apply").size(10.0).color(egui::Color32::WHITE)
                                ).fill(crate::app::PRIMARY).corner_radius(3.0)).clicked() {
                                    // Apply suggestion
                                    match suggestion.view_mode.as_str() {
                                        "Globe" => self.ui_state.view_mode = ViewMode::Globe,
                                        "Map" => self.ui_state.view_mode = ViewMode::Map,
                                        "Profile" => self.ui_state.view_mode = ViewMode::Profile,
                                        _ => {}
                                    }
                                    match suggestion.colormap.as_str() {
                                        "RdBu_r" => self.ui_state.colormap = Colormap::RdBuR,
                                        "Viridis" => self.ui_state.colormap = Colormap::Viridis,
                                        _ => {}
                                    }
                                    if suggestion.symmetric {
                                        if let Some(ref field) = file.field_data {
                                            let abs_max = field.min.abs().max(field.max.abs());
                                            self.ui_state.range_mode = RangeMode::Manual;
                                            self.ui_state.manual_min = -abs_max;
                                            self.ui_state.manual_max = abs_max;
                                        }
                                    }
                                    self.ui_state.contour_enabled = suggestion.overlays.contains(&"contours".to_string());
                                    self.ui_state.streamline_enabled = suggestion.overlays.contains(&"streamlines".to_string());
                                    if suggestion.overlays.contains(&"trajectory".to_string()) {
                                        self.ui_state.trajectory_enabled = true;
                                    }
                                    *self.data_generation += 1;
                                }
                                if ui.button(egui::RichText::new("×").size(11.0)).clicked() {
                                    self.ui_state.suggestion_dismissed = true;
                                }
                            });

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);
                        }
                    }

                    // --- Inference ---
                    let inference = crate::data::inference::infer_variable(var, file.field_data.as_ref());
                    Self::section_header(ui, "Inference");
                    ui.add_space(2.0);
                    ui.label(egui::RichText::new(&inference.description).size(10.0));
                    let confidence_label = match inference.confidence {
                        crate::data::inference::InferenceLevel::L1StandardName => "L1: standard_name",
                        crate::data::inference::InferenceLevel::L2NamePattern => "L2: name pattern",
                        crate::data::inference::InferenceLevel::L3Statistics => "L3: statistics",
                    };
                    Self::dim_label(ui, confidence_label);

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // --- Export ---
                    if file.field_data.is_some() {
                        if ui.button(egui::RichText::new("Export PNG...").size(11.0)).clicked() {
                            self.ui_state.export_settings.title = var.name.clone();
                            self.ui_state.export_dialog_open = true;
                        }
                    }
                } else {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("Select a variable")
                            .color(crate::app::TEXT_CAPTION),
                    );
                }
            }
        } else {
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new("No file loaded")
                    .color(crate::app::TEXT_CAPTION),
            );
        }

        // Deferred variable load from Inspector ComboBox (avoids borrow conflict)
        if let Some((fi, vi)) = inspector_load_request {
            if self.data_store.load_field(fi, vi).is_ok() {
                *self.data_generation += 1;
                self.ui_state.suggestion_dismissed = false;
            }
        }
    }

    fn code_panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Code").strong().size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("Copy").size(11.0)).clicked() {
                    let code = crate::codegen::python::generate_python(self.ui_state, self.data_store);
                    ui.ctx().copy_text(code);
                    self.ui_state.status_text = "Code copied to clipboard".to_string();
                }
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let code = crate::codegen::python::generate_python(self.ui_state, self.data_store);

        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut code_display = code;
            ui.add(
                egui::TextEdit::multiline(&mut code_display)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .interactive(false),
            );
        });
    }

    /// Returns the length of the time dimension for the active variable, if any.
    pub fn active_time_dim_len(&self) -> Option<usize> {
        let file = self.data_store.files.get(self.data_store.active_file?)?;
        let var_idx = file.selected_variable?;
        let var = &file.variables[var_idx];
        var.dimensions
            .iter()
            .find(|(name, _)| name == "time" || name == "t")
            .map(|(_, size)| *size)
    }

    /// Returns (level_dim_name, level_dim_size) for the active variable, if any.
    /// Matches common vertical dimension names in GFD/climate model output.
    fn active_level_dim(&self) -> Option<(String, usize)> {
        let file = self.data_store.files.get(self.data_store.active_file?)?;
        let var_idx = file.selected_variable?;
        let var = &file.variables[var_idx];
        // Exclude time/lat/lon — anything else with size > 1 that looks vertical
        let exact = ["level", "lev", "z", "sigma", "sig", "depth", "height",
                     "plev", "pressure", "p", "k", "eta", "hybrid"];
        let contains = ["lev", "sig", "depth", "height", "press"];
        var.dimensions
            .iter()
            .filter(|(name, _)| {
                let lower = name.to_ascii_lowercase();
                // Skip known horizontal/time dimensions
                !["time", "t", "lon", "longitude", "lat", "latitude", "x", "y"].contains(&lower.as_str())
            })
            .find(|(name, _)| {
                let lower = name.to_ascii_lowercase();
                exact.iter().any(|&c| c == lower) || contains.iter().any(|&c| lower.contains(c))
            })
            .map(|(name, size)| (name.clone(), *size))
    }
}
