use eframe::CreationContext;
use egui_dock::{DockArea, DockState};

use std::collections::HashMap;

use crate::data::DataStore;
use crate::renderer::GlobeRenderer;
use crate::renderer::MapRenderer;
use crate::renderer::cross_section::CrossSectionRenderer;
use crate::renderer::hovmoller::HovmollerRenderer;
use crate::renderer::spectrum::SpectrumRenderer;
use crate::renderer::contour::ContourOverlay;
use crate::renderer::profile::ProfileRenderer;
use crate::renderer::streamline::StreamlineOverlay;
use crate::renderer::trajectory::TrajectoryOverlay;
use crate::renderer::vector_overlay::VectorOverlay;
use crate::ui::{Colormap, GeoScopeTabViewer, Tab};

const APP_KEY: &str = "geoscope-session";

/// Serializable session state for persistence across restarts.
#[derive(serde::Serialize, serde::Deserialize)]
struct SessionState {
    // Opened files & selected variable
    file_paths: Vec<String>,
    active_file: Option<usize>,
    selected_variables: Vec<Option<usize>>,
    // View settings
    view_mode: crate::ui::ViewMode,
    colormap: crate::ui::Colormap,
    map_projection: crate::renderer::map::MapProjection,
    interpolated: bool,
    // Indices
    time_index: usize,
    level_index: usize,
    // Range
    range_mode: crate::ui::RangeMode,
    manual_min: f32,
    manual_max: f32,
    // Camera — Globe
    globe_cam_lon: f32,
    globe_cam_lat: f32,
    globe_zoom: f32,
    // Camera — Map
    map_pan_x: f32,
    map_pan_y: f32,
    map_zoom: f32,
    // Overlays
    vector_overlay_enabled: bool,
    vector_u_var: Option<usize>,
    vector_v_var: Option<usize>,
    vector_density: usize,
    vector_scale: f32,
    contour_enabled: bool,
    contour_levels: usize,
    streamline_enabled: bool,
    // Profile
    profile_mode: crate::ui::ProfileMode,
    profile_split: bool,
    profile_point: Option<(usize, usize)>,
    // Trajectory
    trajectory_enabled: bool,
    trajectory_trail_length: usize,
}

// ---------------------------------------------------------------------------
// Design tokens — unified color system
// ---------------------------------------------------------------------------

// Accent
pub(crate) const PRIMARY: egui::Color32 = egui::Color32::from_rgb(0, 164, 154);

// Backgrounds (darkest → lightest)
pub(crate) const BG_DARK: egui::Color32 = egui::Color32::from_rgb(15, 15, 23);
pub(crate) const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(26, 26, 36);
pub(crate) const BG_WIDGET: egui::Color32 = egui::Color32::from_rgb(37, 37, 48);
pub(crate) const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(54, 54, 76);

// Text hierarchy
pub(crate) const TEXT_HEADING: egui::Color32 = egui::Color32::from_rgb(245, 245, 250);
pub(crate) const TEXT_BODY: egui::Color32 = egui::Color32::from_rgb(224, 224, 232);
pub(crate) const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(156, 156, 176);
pub(crate) const TEXT_CAPTION: egui::Color32 = egui::Color32::from_rgb(110, 110, 132);
pub(crate) const TEXT_DISABLED: egui::Color32 = egui::Color32::from_rgb(74, 74, 94);

// Semantic
pub(crate) const ACCENT_ERROR: egui::Color32 = egui::Color32::from_rgb(255, 107, 107);
pub(crate) const ACCENT_SUCCESS: egui::Color32 = egui::Color32::from_rgb(81, 207, 102);
pub(crate) const ACCENT_MONO: egui::Color32 = egui::Color32::from_rgb(212, 165, 116);

// Divider
pub(crate) const DIVIDER: egui::Color32 = egui::Color32::from_rgb(45, 45, 58);

// Spacing scale (4px grid)
pub(crate) const SP_XS: f32 = 4.0;
pub(crate) const SP_SM: f32 = 8.0;
pub(crate) const SP_MD: f32 = 12.0;
pub(crate) const SP_LG: f32 = 16.0;

/// GeoScope application state.
pub struct GeoScopeApp {
    dock_state: DockState<Tab>,
    data_store: DataStore,
    globe_renderer: GlobeRenderer,
    map_renderer: MapRenderer,
    hovmoller_renderer: HovmollerRenderer,
    spectrum_renderer: SpectrumRenderer,
    cross_section_renderer: CrossSectionRenderer,
    vector_overlay: VectorOverlay,
    profile_renderer: ProfileRenderer,
    contour_overlay: ContourOverlay,
    streamline_overlay: StreamlineOverlay,
    trajectory_overlay: TrajectoryOverlay,
    ui_state: crate::ui::UiState,
    data_generation: u64,
    gpu_generation: u64,
    last_colormap: crate::ui::Colormap,
    hovmoller_generation: u64,
    cross_section_generation: u64,
    vector_generation: u64,
    profile_generation: u64,
    profile_is_time_series: bool,
    /// Track profile_point changes separately so clicks reload even during animation
    last_profile_point: Option<(usize, usize)>,
    last_profile_mode: crate::ui::ProfileMode,
    last_profile_var: Option<(usize, usize)>, // (file_idx, var_idx)
    contour_generation: u64,
    streamline_generation: u64,
    trajectory_generation: u64,
    last_map_projection: crate::renderer::map::MapProjection,
    /// Pending file open requests from UI.
    open_file_request: Vec<std::path::PathBuf>,
    /// Cached global (min, max) for the current variable. Reset on variable change.
    global_range_cache: Option<(f32, f32)>,
    /// Variable index used to compute the cached global range.
    global_range_var: Option<(usize, usize)>,
    theme_applied: bool,
    /// Pre-computed colormap LUTs (256×RGBA per colormap).
    lut_cache: HashMap<Colormap, Vec<u8>>,
}

impl GeoScopeApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);

        let mut globe_renderer = GlobeRenderer::new(cc);

        // Viewport-only dock; sidebars are native egui::SidePanel (collapsible)
        let dock_state = DockState::new(vec![Tab::Viewport]);

        let mut ui_state = crate::ui::UiState::default();
        let mut map_renderer = MapRenderer::new();
        let mut data_store = DataStore::new();
        let mut restored_files = false;

        // Restore session state from storage
        if let Some(storage) = cc.storage {
            if let Some(state) = eframe::get_value::<SessionState>(storage, APP_KEY) {
                // Re-open files
                for (i, path_str) in state.file_paths.iter().enumerate() {
                    let path = std::path::Path::new(path_str);
                    if path.exists() {
                        if let Ok(()) = data_store.open_file(path) {
                            let fi = data_store.files.len() - 1;
                            // Restore selected variable
                            if let Some(&Some(vi)) = state.selected_variables.get(i) {
                                if vi < data_store.files[fi].variables.len() {
                                    data_store.files[fi].selected_variable = Some(vi);
                                }
                            }
                        }
                    }
                }
                if let Some(af) = state.active_file {
                    if af < data_store.files.len() {
                        data_store.active_file = Some(af);
                    }
                }
                restored_files = !data_store.files.is_empty();

                // Restore UI state
                ui_state.view_mode = state.view_mode;
                ui_state.colormap = state.colormap;
                ui_state.map_projection = state.map_projection;
                ui_state.interpolated = state.interpolated;
                ui_state.time_index = state.time_index;
                ui_state.level_index = state.level_index;
                ui_state.range_mode = state.range_mode;
                ui_state.manual_min = state.manual_min;
                ui_state.manual_max = state.manual_max;
                ui_state.vector_overlay_enabled = state.vector_overlay_enabled;
                ui_state.vector_u_var = state.vector_u_var;
                ui_state.vector_v_var = state.vector_v_var;
                ui_state.vector_density = state.vector_density;
                ui_state.vector_scale = state.vector_scale;
                ui_state.contour_enabled = state.contour_enabled;
                ui_state.contour_levels = state.contour_levels;
                ui_state.streamline_enabled = state.streamline_enabled;
                ui_state.profile_mode = state.profile_mode;
                ui_state.profile_split = state.profile_split;
                ui_state.profile_point = state.profile_point;
                ui_state.trajectory_enabled = state.trajectory_enabled;
                ui_state.trajectory_trail_length = state.trajectory_trail_length;

                // Restore camera
                globe_renderer.cam_lon = state.globe_cam_lon;
                globe_renderer.cam_lat = state.globe_cam_lat;
                globe_renderer.zoom = state.globe_zoom;
                map_renderer.pan_x = state.map_pan_x;
                map_renderer.pan_y = state.map_pan_y;
                map_renderer.zoom = state.map_zoom;
            }
        }

        // Load field data for the active file's selected variable at restored indices
        let data_generation = if restored_files {
            if let Some(fi) = data_store.active_file {
                if let Some(vi) = data_store.files[fi].selected_variable {
                    let _ = data_store.load_field_at(
                        fi, vi, ui_state.time_index, ui_state.level_index,
                    );
                }
            }
            1_u64
        } else {
            0_u64
        };

        Self {
            dock_state,
            data_store,
            globe_renderer,
            map_renderer,
            hovmoller_renderer: HovmollerRenderer::new(),
            spectrum_renderer: SpectrumRenderer::new(),
            cross_section_renderer: CrossSectionRenderer::new(),
            vector_overlay: VectorOverlay::new(),
            profile_renderer: ProfileRenderer::new(),
            contour_overlay: ContourOverlay::new(),
            streamline_overlay: StreamlineOverlay::new(),
            trajectory_overlay: TrajectoryOverlay::new(),
            ui_state,
            data_generation,
            gpu_generation: 0,
            last_colormap: crate::ui::Colormap::default(),
            hovmoller_generation: 0,
            cross_section_generation: 0,
            vector_generation: 0,
            profile_generation: 0,
            last_profile_point: None,
            last_profile_mode: crate::ui::ProfileMode::Vertical,
            last_profile_var: None,
            profile_is_time_series: false,
            contour_generation: 0,
            streamline_generation: 0,
            trajectory_generation: 0,
            last_map_projection: crate::renderer::map::MapProjection::default(),
            open_file_request: Vec::new(),
            global_range_cache: None,
            global_range_var: None,
            theme_applied: false,
            lut_cache: {
                let mut m = HashMap::new();
                for cm in Colormap::SEQUENTIAL.iter().chain(Colormap::DIVERGING.iter()) {
                    m.insert(*cm, crate::renderer::common::colormap_lut(*cm));
                }
                m
            },
        }
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_PANEL;
    visuals.extreme_bg_color = BG_DARK;
    visuals.faint_bg_color = BG_WIDGET;

    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, DIVIDER);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = BG_WIDGET;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_BODY);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, DIVIDER);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT_HEADING);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, PRIMARY);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.active.bg_fill = PRIMARY;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.open.bg_fill = BG_WIDGET;
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, TEXT_BODY);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(4);

    visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(0, 164, 154, 50);
    visuals.selection.stroke = egui::Stroke::new(1.0, PRIMARY);

    visuals.window_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: egui::Color32::from_black_alpha(60),
    };
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: egui::Color32::from_black_alpha(80),
    };
    visuals.window_corner_radius = egui::CornerRadius::same(8);

    visuals.override_text_color = Some(TEXT_BODY);
    visuals.striped = true;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(SP_SM, SP_XS);
    style.spacing.button_padding = egui::vec2(SP_MD, 6.0);
    style.spacing.window_margin = egui::Margin::same(SP_MD as i8);
    style.spacing.combo_width = 0.0;

    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(12.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::new(12.0, egui::FontFamily::Monospace),
    );

    ctx.set_style(style);
}

fn dock_style(ctx: &egui::Context) -> egui_dock::Style {
    let mut style = egui_dock::Style::from_egui(ctx.style().as_ref());

    style.tab_bar.bg_fill = BG_DARK;
    style.tab_bar.height = 0.0; // Hide tab bar — single viewport only

    style.tab.tab_body.bg_fill = BG_PANEL;

    style.tab.active.bg_fill = BG_PANEL;
    style.tab.active.text_color = PRIMARY;
    style.tab.active.outline_color = egui::Color32::TRANSPARENT;

    style.tab.inactive.bg_fill = BG_DARK;
    style.tab.inactive.text_color = TEXT_CAPTION;
    style.tab.inactive.outline_color = egui::Color32::TRANSPARENT;

    style.tab.focused.bg_fill = BG_PANEL;
    style.tab.focused.text_color = PRIMARY;
    style.tab.focused.outline_color = egui::Color32::TRANSPARENT;

    style.tab.hovered.bg_fill = BG_WIDGET;
    style.tab.hovered.text_color = egui::Color32::WHITE;
    style.tab.hovered.outline_color = egui::Color32::TRANSPARENT;

    style.separator.width = 1.0;
    style.separator.color_idle = egui::Color32::from_rgb(40, 40, 52);
    style.separator.color_hovered = PRIMARY;
    style.separator.color_dragged = PRIMARY;

    style
}

impl GeoScopeApp {
    pub fn open_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.data_store.open_file(path)?;
        let file_idx = self.data_store.files.len() - 1;
        let file = &self.data_store.files[file_idx];

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
    fn auto_save_interval(&self) -> std::time::Duration {
        // Only save on app exit, not during runtime (avoids stuttering on slider drag)
        std::time::Duration::from_secs(u64::MAX)
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let state = SessionState {
            file_paths: self.data_store.files.iter().map(|f| f.path.clone()).collect(),
            active_file: self.data_store.active_file,
            selected_variables: self.data_store.files.iter().map(|f| f.selected_variable).collect(),
            view_mode: self.ui_state.view_mode,
            colormap: self.ui_state.colormap,
            map_projection: self.ui_state.map_projection,
            interpolated: self.ui_state.interpolated,
            time_index: self.ui_state.time_index,
            level_index: self.ui_state.level_index,
            range_mode: self.ui_state.range_mode,
            manual_min: self.ui_state.manual_min,
            manual_max: self.ui_state.manual_max,
            globe_cam_lon: self.globe_renderer.cam_lon,
            globe_cam_lat: self.globe_renderer.cam_lat,
            globe_zoom: self.globe_renderer.zoom,
            map_pan_x: self.map_renderer.pan_x,
            map_pan_y: self.map_renderer.pan_y,
            map_zoom: self.map_renderer.zoom,
            vector_overlay_enabled: self.ui_state.vector_overlay_enabled,
            vector_u_var: self.ui_state.vector_u_var,
            vector_v_var: self.ui_state.vector_v_var,
            vector_density: self.ui_state.vector_density,
            vector_scale: self.ui_state.vector_scale,
            contour_enabled: self.ui_state.contour_enabled,
            contour_levels: self.ui_state.contour_levels,
            streamline_enabled: self.ui_state.streamline_enabled,
            profile_mode: self.ui_state.profile_mode,
            profile_split: self.ui_state.profile_split,
            profile_point: self.ui_state.profile_point,
            trajectory_enabled: self.ui_state.trajectory_enabled,
            trajectory_trail_length: self.ui_state.trajectory_trail_length,
        };
        eframe::set_value(storage, APP_KEY, &state);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Re-apply theme on first update (overrides system theme detection)
        if !self.theme_applied {
            apply_theme(ctx);
            self.theme_applied = true;
        }

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

        // Capture sidebar state before borrowing ui_state
        let left_panel_open = self.ui_state.left_panel_open;
        let right_panel_open = self.ui_state.right_panel_open;

        let mut tab_viewer = GeoScopeTabViewer {
            data_store: &mut self.data_store,
            globe_renderer: &mut self.globe_renderer,
            map_renderer: &mut self.map_renderer,
            hovmoller_renderer: &mut self.hovmoller_renderer,
            spectrum_renderer: &mut self.spectrum_renderer,
            cross_section_renderer: &mut self.cross_section_renderer,
            vector_overlay: &mut self.vector_overlay,
            profile_renderer: &mut self.profile_renderer,
            contour_overlay: &mut self.contour_overlay,
            streamline_overlay: &mut self.streamline_overlay,
            trajectory_overlay: &mut self.trajectory_overlay,
            ui_state: &mut self.ui_state,
            data_generation: &mut self.data_generation,
            open_file_request: &mut self.open_file_request,
            lut_cache: &self.lut_cache,
        };

        // Top bar
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(egui::Margin::symmetric(12, 4)))
            .exact_height(32.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(egui::RichText::new("GeoScope").color(PRIMARY).strong().size(15.0));
                    if let Some(fi) = tab_viewer.data_store.active_file {
                        if let Some(file) = tab_viewer.data_store.files.get(fi) {
                            let name = std::path::Path::new(&file.path)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            ui.label(egui::RichText::new("/").color(TEXT_CAPTION).size(14.0));
                            ui.label(egui::RichText::new(name).size(12.0).color(TEXT_SECONDARY));
                        }
                    }
                });
            });

        // Status bar — show inference info or error/export status
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(egui::Margin::symmetric(12, 2)))
            .exact_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // Check for error/export status first
                    let is_special = tab_viewer.ui_state.status_text.starts_with("Error")
                        || tab_viewer.ui_state.status_text.contains("error")
                        || tab_viewer.ui_state.status_text.starts_with("Exported");

                    if is_special {
                        let status_color = if tab_viewer.ui_state.status_text.contains("rror") {
                            ACCENT_ERROR
                        } else {
                            ACCENT_SUCCESS
                        };
                        ui.label(egui::RichText::new(&tab_viewer.ui_state.status_text).size(11.0).color(status_color));
                    } else {
                        // Show inference-based status
                        let inference_text = if let Some(fi) = tab_viewer.data_store.active_file {
                            if let Some(file) = tab_viewer.data_store.files.get(fi) {
                                if let Some(vi) = file.selected_variable {
                                    let var = &file.variables[vi];
                                    let inf = crate::data::inference::infer_variable(var, file.field_data.as_ref());
                                    let cm = tab_viewer.ui_state.colormap;
                                    Some(format!("Detected: {} ({}, {})", inf.description, cm.label(), cm.description()))
                                } else { None }
                            } else { None }
                        } else { None };

                        if let Some(text) = inference_text {
                            ui.label(egui::RichText::new("💡").size(11.0));
                            ui.label(egui::RichText::new(&text).size(11.0).color(TEXT_SECONDARY));
                        } else {
                            ui.label(egui::RichText::new(&tab_viewer.ui_state.status_text).size(11.0).color(TEXT_SECONDARY));
                        }
                    }
                });
            });

        // --- Collapsible side panels (Web-style) ---

        // Left panel: DataBrowser
        if left_panel_open {
            egui::SidePanel::left("data_browser_panel")
                .resizable(true)
                .default_width(160.0)
                .min_width(120.0)
                .max_width(300.0)
                .frame(egui::Frame::new().fill(BG_PANEL).inner_margin(egui::Margin::same(4)))
                .show_animated(ctx, true, |ui| {
                    // Header: Data + open-file button + collapse
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Data").strong().size(13.0).color(TEXT_HEADING));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(
                                egui::RichText::new("\u{2039}").size(22.0).color(TEXT_SECONDARY)
                            ).frame(false)).on_hover_text("Hide sidebar [").clicked() {
                                tab_viewer.ui_state.left_panel_open = false;
                            }
                            if ui.button(egui::RichText::new("+").size(14.0)).clicked() {
                                let paths = rfd::FileDialog::new()
                                    .add_filter("NetCDF", &["nc", "nc4", "netcdf"])
                                    .pick_files()
                                    .unwrap_or_default();
                                tab_viewer.open_file_request.extend(paths);
                            }
                        });
                    });
                    ui.separator();
                    tab_viewer.data_browser_ui(ui);
                });
        } else {
            // Collapsed: thin strip with open button
            egui::SidePanel::left("data_browser_collapsed")
                .resizable(false)
                .exact_width(28.0)
                .frame(egui::Frame::new().fill(BG_DARK).inner_margin(egui::Margin::same(0)))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(
                            egui::RichText::new("\u{203A}").size(22.0).color(TEXT_SECONDARY)
                        ).frame(false)).on_hover_text("Show Data [").clicked() {
                            tab_viewer.ui_state.left_panel_open = true;
                        }
                    });
                });
        }

        // Right panel: Inspector / Code (sub-tabs)
        if right_panel_open {
            egui::SidePanel::right("inspector_panel")
                .resizable(true)
                .default_width(260.0)
                .min_width(200.0)
                .max_width(400.0)
                .frame(egui::Frame::new().fill(BG_PANEL).inner_margin(egui::Margin::same(4)))
                .show_animated(ctx, true, |ui| {
                    // Header: sub-tabs + close button
                    ui.horizontal(|ui| {
                        if ui.selectable_label(tab_viewer.ui_state.right_panel_tab == 0, "Inspector").clicked() {
                            tab_viewer.ui_state.right_panel_tab = 0;
                        }
                        if ui.selectable_label(tab_viewer.ui_state.right_panel_tab == 1, "Code").clicked() {
                            tab_viewer.ui_state.right_panel_tab = 1;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(
                                egui::RichText::new("\u{203A}").size(22.0).color(TEXT_SECONDARY)
                            ).frame(false)).on_hover_text("Hide sidebar ]").clicked() {
                                tab_viewer.ui_state.right_panel_open = false;
                            }
                        });
                    });
                    ui.separator();
                    match tab_viewer.ui_state.right_panel_tab {
                        0 => tab_viewer.inspector_ui(ui),
                        _ => tab_viewer.code_panel_ui(ui),
                    }
                });
        } else {
            // Collapsed: thin strip with open button
            egui::SidePanel::right("inspector_collapsed")
                .resizable(false)
                .exact_width(28.0)
                .frame(egui::Frame::new().fill(BG_DARK).inner_margin(egui::Margin::same(0)))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        if ui.add(egui::Button::new(
                            egui::RichText::new("\u{2039}").size(22.0).color(TEXT_SECONDARY)
                        ).frame(false)).on_hover_text("Show Inspector ]").clicked() {
                            tab_viewer.ui_state.right_panel_open = true;
                        }
                    });
                });
        }

        // Central: Viewport dock
        DockArea::new(&mut self.dock_state)
            .style(dock_style(ctx))
            .show(ctx, &mut tab_viewer);

        // Export dialog
        if self.ui_state.export_dialog_open {
            use crate::renderer::export::ExportFormat;
            let mut open = true;
            let mut do_export = false;
            let is_gif = self.ui_state.export_settings.format == ExportFormat::Gif;
            let dialog_title = if is_gif { "Export GIF" } else { "Export PNG" };
            egui::Window::new(dialog_title)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(280.0);

                    // Format selector
                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        ui.selectable_value(
                            &mut self.ui_state.export_settings.format,
                            ExportFormat::Png,
                            "PNG",
                        );
                        ui.selectable_value(
                            &mut self.ui_state.export_settings.format,
                            ExportFormat::Gif,
                            "GIF",
                        );
                    });

                    ui.add_space(4.0);

                    // Title
                    ui.horizontal(|ui| {
                        ui.label("Title:");
                        ui.text_edit_singleline(&mut self.ui_state.export_settings.title);
                    });

                    ui.add_space(4.0);

                    // Resolution
                    ui.horizontal(|ui| {
                        ui.label("Resolution:");
                        for s in [1u32, 2, 4] {
                            ui.selectable_value(
                                &mut self.ui_state.export_settings.scale,
                                s,
                                format!("{}x", s),
                            );
                        }
                    });

                    // Show output size
                    if let Some(field) = self.data_store.active_field() {
                        let s = self.ui_state.export_settings.scale;
                        ui.label(
                            egui::RichText::new(format!(
                                "  {}x{} px",
                                field.width as u32 * s,
                                field.height as u32 * s
                            ))
                            .size(10.0)
                            .color(egui::Color32::from_gray(120)),
                        );
                    }

                    ui.add_space(4.0);

                    // Colorbar toggle
                    ui.checkbox(&mut self.ui_state.export_settings.colorbar, "Include colorbar");

                    // Publication quality toggle (PNG only)
                    if self.ui_state.export_settings.format != ExportFormat::Gif {
                        ui.checkbox(
                            &mut self.ui_state.export_settings.publication,
                            "Publication quality",
                        );
                    }

                    // GIF-specific options
                    if self.ui_state.export_settings.format == ExportFormat::Gif {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("FPS:");
                            ui.add(egui::Slider::new(&mut self.ui_state.export_settings.gif_fps, 1..=30));
                        });

                        // Show time step count
                        if let Some(file_idx) = self.data_store.active_file {
                            if let Some(n) = self.data_store.files[file_idx].time_steps {
                                ui.label(
                                    egui::RichText::new(format!("  {} frames", n))
                                        .size(10.0)
                                        .color(egui::Color32::from_gray(120)),
                                );
                            }
                        }
                    }

                    ui.add_space(4.0);

                    // Preview colormap bar
                    let bar_w = ui.available_width() - 8.0;
                    let bar_h = 16.0;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(bar_w, bar_h), egui::Sense::hover());
                    let lut = &self.lut_cache[&self.ui_state.colormap];
                    let painter = ui.painter();
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

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Export button
                    let btn_label = if self.ui_state.export_settings.format == ExportFormat::Gif {
                        "Save GIF"
                    } else {
                        "Save PNG"
                    };
                    if ui.add(egui::Button::new(
                        egui::RichText::new(btn_label).color(egui::Color32::WHITE).size(12.0)
                    ).fill(PRIMARY)).clicked() {
                        do_export = true;
                    }
                });
            if !open {
                self.ui_state.export_dialog_open = false;
            }
            if do_export {
                let format = self.ui_state.export_settings.format;
                match format {
                    ExportFormat::Png => {
                        if let Some(field) = self.data_store.active_field().cloned() {
                            let (display_min, display_max) = match self.ui_state.range_mode {
                                crate::ui::RangeMode::Slice => (field.min, field.max),
                                crate::ui::RangeMode::Global => {
                                    self.global_range_cache.unwrap_or((field.min, field.max))
                                }
                                crate::ui::RangeMode::Manual => {
                                    let rmin = self.ui_state.manual_min;
                                    let rmax = self.ui_state.manual_max;
                                    if (rmax - rmin).abs() > f32::EPSILON { (rmin, rmax) } else { (field.min, field.max) }
                                }
                            };

                            let default_name = format!("{}.png", self.ui_state.export_settings.title);
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG", &["png"])
                                .set_file_name(&default_name)
                                .save_file()
                            {
                                let result = if self.ui_state.export_settings.publication {
                                    crate::renderer::export::export_publication_png(
                                        &field,
                                        self.ui_state.colormap,
                                        display_min,
                                        display_max,
                                        &self.ui_state.export_settings,
                                        &path,
                                    )
                                } else {
                                    crate::renderer::export::export_png_with_settings(
                                        &field,
                                        self.ui_state.colormap,
                                        display_min,
                                        display_max,
                                        &self.ui_state.export_settings,
                                        &path,
                                    )
                                };
                                match result {
                                    Ok(()) => {
                                        let s = self.ui_state.export_settings.scale;
                                        self.ui_state.status_text = format!(
                                            "Exported {}x: {}",
                                            s,
                                            path.display()
                                        );
                                        self.ui_state.export_dialog_open = false;
                                    }
                                    Err(e) => {
                                        self.ui_state.status_text = format!("Export error: {e}");
                                    }
                                }
                            }
                        }
                    }
                    ExportFormat::Gif => {
                        if let Some(file_idx) = self.data_store.active_file {
                            if let Some(var_idx) = self.data_store.files[file_idx].selected_variable {
                                let default_name = format!("{}.gif", self.ui_state.export_settings.title);
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("GIF", &["gif"])
                                    .set_file_name(&default_name)
                                    .save_file()
                                {
                                    self.ui_state.status_text = "Exporting GIF...".to_string();
                                    let level_idx = self.ui_state.level_index;
                                    let settings = self.ui_state.export_settings.clone();
                                    let colormap = self.ui_state.colormap;
                                    let range_mode = self.ui_state.range_mode;
                                    let manual_min = self.ui_state.manual_min;
                                    let manual_max = self.ui_state.manual_max;
                                    let global_range = self.global_range_cache;

                                    match crate::renderer::export::export_gif(
                                        &mut self.data_store,
                                        file_idx,
                                        var_idx,
                                        level_idx,
                                        &settings,
                                        colormap,
                                        &range_mode,
                                        manual_min,
                                        manual_max,
                                        global_range,
                                        &path,
                                    ) {
                                        Ok(n_frames) => {
                                            self.ui_state.status_text = format!(
                                                "Exported GIF ({} frames): {}",
                                                n_frames,
                                                path.display()
                                            );
                                            self.ui_state.export_dialog_open = false;
                                        }
                                        Err(e) => {
                                            self.ui_state.status_text = format!("GIF export error: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Handle file open requests from UI
        if !self.open_file_request.is_empty() {
            let paths: Vec<_> = std::mem::take(&mut self.open_file_request);
            for path in &paths {
                if let Err(e) = self.open_file(path) {
                    self.ui_state.status_text = format!("Error: {e}");
                }
            }
        }

        // Handle Code Panel "Run" request (reverse sync: code → GUI)
        if self.ui_state.code_panel_run_request {
            self.ui_state.code_panel_run_request = false;
            let parsed = crate::codegen::parser::parse_python(&self.ui_state.code_panel_text);
            let changes = crate::codegen::parser::apply_to_ui_state(
                &parsed,
                &mut self.ui_state,
                &mut self.data_store,
            );
            if changes.is_empty() {
                self.ui_state.code_panel_status = "No changes detected".to_string();
            } else {
                self.ui_state.code_panel_status = format!("Applied: {}", changes.join(", "));
                self.data_generation += 1;
            }
        }

        // Detect colormap change
        if self.ui_state.colormap != self.last_colormap {
            self.last_colormap = self.ui_state.colormap;
            self.data_generation += 1;
        }

        // Upload field data to GPU when it changes
        if self.data_generation != self.gpu_generation {
            // Compute global range on-demand: only when Global mode is active
            if self.ui_state.range_mode == crate::ui::RangeMode::Global {
                if let Some(file_idx) = self.data_store.active_file {
                    if let Some(file) = self.data_store.files.get(file_idx) {
                        if let Some(var_idx) = file.selected_variable {
                            let key = (file_idx, var_idx);
                            if self.global_range_var != Some(key) {
                                if let Ok((gmin, gmax)) = self.data_store.compute_global_range(file_idx, var_idx) {
                                    self.global_range_cache = Some((gmin, gmax));
                                    self.global_range_var = Some(key);
                                    self.ui_state.global_range = Some((gmin, gmax));
                                }
                            }
                        }
                    }
                }
            }

            if let Some(field) = self.data_store.active_field().cloned() {
                // Apply wavenumber filter if enabled
                let field = if self.ui_state.wavenumber_filter_enabled {
                    if let Some(n_trunc) = crate::data::spectral_filter::detect_n_trunc(field.width, field.height) {
                        let lat_s2n = self.data_store.files
                            .get(self.data_store.active_file.unwrap_or(0))
                            .map(|f| crate::data::spectral_filter::is_lat_south_to_north(f.grid.lat.as_deref()))
                            .unwrap_or(true);
                        let cutoff = self.ui_state.wavenumber_cutoff.min(n_trunc);
                        if let Some(filtered) = crate::data::spectral_filter::wavenumber_filter(
                            &field.values, field.width, field.height, n_trunc, cutoff, lat_s2n,
                        ) {
                            let min = filtered.iter().copied().fold(f32::INFINITY, f32::min);
                            let max = filtered.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                            crate::data::FieldData {
                                values: filtered,
                                width: field.width,
                                height: field.height,
                                min,
                                max,
                            }
                        } else {
                            field
                        }
                    } else {
                        field
                    }
                } else {
                    field
                };

                if let Some(render_state) = frame.wgpu_render_state() {
                    // Determine display range based on range mode
                    let (display_min, display_max) = match self.ui_state.range_mode {
                        crate::ui::RangeMode::Slice => {
                            // Auto-scale from the current slice
                            (field.min, field.max)
                        }
                        crate::ui::RangeMode::Global => {
                            // Use cached global min/max
                            self.global_range_cache.unwrap_or((field.min, field.max))
                        }
                        crate::ui::RangeMode::Manual => {
                            let rmin = self.ui_state.manual_min;
                            let rmax = self.ui_state.manual_max;
                            if (rmax - rmin).abs() > f32::EPSILON {
                                (rmin, rmax)
                            } else {
                                (field.min, field.max)
                            }
                        }
                    };

                    self.globe_renderer.upload_field_data_with_range(
                        render_state,
                        &field.values,
                        field.width,
                        field.height,
                        display_min,
                        display_max,
                        self.ui_state.colormap,
                        self.ui_state.interpolated,
                    );
                    self.map_renderer.ensure_initialized(render_state);
                    self.map_renderer.upload_field_data_with_range(
                        render_state,
                        &field.values,
                        field.width,
                        field.height,
                        display_min,
                        display_max,
                        self.ui_state.colormap,
                        self.ui_state.interpolated,
                    );
                    self.gpu_generation = self.data_generation;

                    if let Some(file_idx) = self.data_store.active_file {
                        if let Some(file) = self.data_store.files.get(file_idx) {
                            if let Some(var_idx) = file.selected_variable {
                                let var = &file.variables[var_idx];
                                let mut parts = vec![
                                    format!("{}: {}x{}", var.name, field.width, field.height),
                                ];
                                // Time info
                                if let Some(time_dim) = var.dimensions.iter().find(|(n, _)| {
                                    matches!(n.as_str(), "time" | "t" | "Time" | "TIME")
                                }) {
                                    parts.push(format!("t={}/{}", self.ui_state.time_index, time_dim.1));
                                }
                                // Level info
                                if let Some(lev_dim) = var.dimensions.iter().find(|(n, _)| {
                                    matches!(n.as_str(), "level" | "lev" | "z" | "depth" | "plev" | "sigma")
                                }) {
                                    parts.push(format!("lev={}/{}", self.ui_state.level_index, lev_dim.1));
                                }
                                parts.push(format!("[{:.4e}, {:.4e}]", field.min, field.max));
                                self.ui_state.status_text = parts.join("  ");
                            }
                        }
                    }
                }
            }
        }

        // Lazy Hovmoller data loading — only when view is active and data has changed
        if self.ui_state.view_mode == crate::ui::ViewMode::Hovmoller
            && self.hovmoller_generation != self.data_generation
        {
            if let Some(file_idx) = self.data_store.active_file {
                if let Some(file) = self.data_store.files.get(file_idx) {
                    if let Some(var_idx) = file.selected_variable {
                        if let Some(ref field) = file.field_data {
                            let lat_idx = field.height / 2;
                            if let Ok(hov_data) =
                                self.data_store.load_hovmoller_data(file_idx, var_idx, lat_idx)
                            {
                                self.hovmoller_renderer
                                    .set_data(&hov_data, self.ui_state.colormap);
                            }
                        }
                    }
                }
            }
            self.hovmoller_generation = self.data_generation;
        }

        // Lazy cross-section data loading
        if self.ui_state.view_mode == crate::ui::ViewMode::CrossSection
            && self.cross_section_generation != self.data_generation
        {
            if let Some(file_idx) = self.data_store.active_file {
                if let Some(file) = self.data_store.files.get(file_idx) {
                    if let Some(var_idx) = file.selected_variable {
                        let time_idx = self.ui_state.time_index;
                        let axis = self.ui_state.cross_section_axis;
                        let fixed_idx = self.ui_state.cross_section_idx;
                        if let Ok(cs_data) = self.data_store.load_cross_section(
                            file_idx, var_idx, time_idx, axis, fixed_idx,
                        ) {
                            self.cross_section_renderer
                                .set_data(&cs_data, self.ui_state.colormap);
                        }
                    }
                }
            }
            self.cross_section_generation = self.data_generation;
        }

        // Map projection switching
        if self.ui_state.map_projection != self.last_map_projection {
            if let Some(render_state) = frame.wgpu_render_state() {
                self.map_renderer.set_projection(self.ui_state.map_projection, render_state);
            }
            self.last_map_projection = self.ui_state.map_projection;
        }

        // Vector overlay data loading
        if self.ui_state.vector_overlay_enabled
            && self.vector_generation != self.data_generation
        {
            if let Some(file_idx) = self.data_store.active_file {
                if let (Some(u_idx), Some(v_idx)) = (self.ui_state.vector_u_var, self.ui_state.vector_v_var) {
                    // Save original selection before load_vector_field overwrites it
                    let orig_var = self.data_store.files[file_idx].selected_variable;
                    let orig_field = self.data_store.files[file_idx].field_data.clone();

                    let time_idx = self.ui_state.time_index;
                    let level_idx = self.ui_state.level_index;
                    if let Ok(vec_data) = self.data_store.load_vector_field(
                        file_idx, u_idx, v_idx, time_idx, level_idx,
                    ) {
                        self.vector_overlay.density = self.ui_state.vector_density;
                        self.vector_overlay.scale = self.ui_state.vector_scale;
                        self.vector_overlay.set_data(vec_data);
                    }

                    // Restore original selected variable and field data
                    self.data_store.files[file_idx].selected_variable = orig_var;
                    self.data_store.files[file_idx].field_data = orig_field;
                }
            }
            self.vector_generation = self.data_generation;
        } else if !self.ui_state.vector_overlay_enabled && self.vector_overlay.has_data() {
            self.vector_overlay.clear();
        }

        // Sync vector overlay density/scale from UI
        self.vector_overlay.density = self.ui_state.vector_density;
        self.vector_overlay.scale = self.ui_state.vector_scale;

        // Profile data loading — triggered by Profile view OR split view with a picked point
        // During animation, skip reload unless the point or mode changed (user interaction).
        let need_profile = self.ui_state.view_mode == crate::ui::ViewMode::Profile
            || (self.ui_state.profile_split && self.ui_state.profile_point.is_some());
        let current_var = self.data_store.active_file.and_then(|fi|
            self.data_store.files.get(fi).and_then(|f| f.selected_variable.map(|vi| (fi, vi)))
        );
        let profile_input_changed = self.ui_state.profile_point != self.last_profile_point
            || self.ui_state.profile_mode != self.last_profile_mode
            || current_var != self.last_profile_var;
        let profile_stale = self.profile_generation != self.data_generation;
        if need_profile && (profile_input_changed || (profile_stale && !self.ui_state.playing)) {
            self.last_profile_point = self.ui_state.profile_point;
            self.last_profile_mode = self.ui_state.profile_mode;
            self.last_profile_var = current_var;
            if let Some(file_idx) = self.data_store.active_file {
                if let Some(file) = self.data_store.files.get(file_idx) {
                    if let Some(var_idx) = file.selected_variable {
                        if let Some(ref field) = file.field_data {
                            let (lon_idx, lat_idx) = self.ui_state.profile_point
                                .unwrap_or((field.width / 2, field.height / 2));
                            let time_idx = self.ui_state.time_index;
                            // Get lon/lat degree strings for title
                            let lon_str = self.data_store.files.get(file_idx)
                                .and_then(|f| f.grid.lon.as_ref())
                                .and_then(|lons| lons.get(lon_idx))
                                .map(|&v| format!("{v:.1}"))
                                .unwrap_or_else(|| format!("{lon_idx}"));
                            let lat_str = self.data_store.files.get(file_idx)
                                .and_then(|f| f.grid.lat.as_ref())
                                .and_then(|lats| lats.get(lat_idx))
                                .map(|&v| format!("{v:.1}"))
                                .unwrap_or_else(|| format!("{lat_idx}"));
                            let mode = self.ui_state.profile_mode;
                            self.profile_is_time_series = false;

                            // Compute display range respecting range_mode
                            let display_range = match self.ui_state.range_mode {
                                crate::ui::RangeMode::Global => self.global_range_cache,
                                crate::ui::RangeMode::Manual => {
                                    let rmin = self.ui_state.manual_min;
                                    let rmax = self.ui_state.manual_max;
                                    if (rmax - rmin).abs() > f32::EPSILON {
                                        Some((rmin, rmax))
                                    } else {
                                        None
                                    }
                                }
                                crate::ui::RangeMode::Slice => None, // use data's own range
                            };

                            // Vertical profile: swap axes (y=level, x=value); others: standard
                            self.profile_renderer.swap_axes = mode == crate::ui::ProfileMode::Vertical;

                            match mode {
                                crate::ui::ProfileMode::TimeLevelHeatmap => {
                                    self.profile_is_time_series = true; // enable playhead
                                    if let Some(tl_data) = self.data_store.load_time_level_data(
                                        file_idx, var_idx, lon_idx, lat_idx,
                                    ) {
                                        let var_name = file.variables[var_idx].name.clone();
                                        self.profile_renderer.set_title(format!(
                                            "{} (lon={}, lat={})", var_name, lon_str, lat_str
                                        ));
                                        self.profile_renderer.set_heatmap_data_with_range(
                                            tl_data, self.ui_state.colormap, display_range,
                                        );
                                    } else {
                                        self.profile_renderer.clear();
                                    }
                                }
                                crate::ui::ProfileMode::TimeSeries => {
                                    self.profile_renderer.clear_heatmap();
                                    let level_idx = self.ui_state.level_index;
                                    if let Some(ts) = self.data_store.load_time_series_data(
                                        file_idx, var_idx, level_idx, lon_idx, lat_idx,
                                    ) {
                                        self.profile_is_time_series = true;
                                        let var_name = file.variables[var_idx].name.clone();
                                        self.profile_renderer.set_title(format!(
                                            "{} time series (lon={}, lat={})", var_name, lon_str, lat_str
                                        ));
                                        self.profile_renderer.set_data(ts);
                                    } else if let Some(profile) = self.data_store.load_profile_data(
                                        file_idx, var_idx, time_idx, lon_idx, lat_idx,
                                    ) {
                                        let var_name = file.variables[var_idx].name.clone();
                                        self.profile_renderer.set_title(format!(
                                            "{} (lon={}, lat={})", var_name, lon_str, lat_str
                                        ));
                                        self.profile_renderer.set_data(profile);
                                        self.profile_renderer.set_current_index(None);
                                    } else {
                                        self.profile_renderer.clear();
                                    }
                                }
                                crate::ui::ProfileMode::Vertical => {
                                    self.profile_renderer.clear_heatmap();
                                    if let Some(profile) = self.data_store.load_profile_data(
                                        file_idx, var_idx, time_idx, lon_idx, lat_idx,
                                    ) {
                                        let var_name = file.variables[var_idx].name.clone();
                                        self.profile_renderer.set_title(format!(
                                            "{} (lon={}, lat={})", var_name, lon_str, lat_str
                                        ));
                                        self.profile_renderer.set_data(profile);
                                        self.profile_renderer.set_current_index(None);
                                    } else {
                                        // Fallback to time series (no level dim) — use standard axes
                                        self.profile_renderer.swap_axes = false;
                                        self.profile_is_time_series = true;
                                        let level_idx = self.ui_state.level_index;
                                        if let Some(ts) = self.data_store.load_time_series_data(
                                            file_idx, var_idx, level_idx, lon_idx, lat_idx,
                                        ) {
                                            let var_name = file.variables[var_idx].name.clone();
                                            self.profile_renderer.set_title(format!(
                                                "{} time series (lon={}, lat={})", var_name, lon_str, lat_str
                                            ));
                                            self.profile_renderer.set_data(ts);
                                        } else {
                                            self.profile_renderer.clear();
                                        }
                                    }
                                }
                            }

                            // Apply display range to profile line graph (Vertical/TimeSeries)
                            if let Some((dmin, dmax)) = display_range {
                                self.profile_renderer.set_display_range(dmin, dmax);
                            }
                        }
                    }
                }
            }
            self.profile_generation = self.data_generation;
        }

        // Sync playheads: profile, hovmoller, cross-section
        // Use profile mode (not stale flag) to determine which index to show
        match self.ui_state.profile_mode {
            crate::ui::ProfileMode::TimeSeries => {
                if self.profile_is_time_series {
                    self.profile_renderer.set_current_index(Some(self.ui_state.time_index));
                }
                self.profile_renderer.set_level_index(None);
            }
            crate::ui::ProfileMode::Vertical => {
                if self.profile_is_time_series {
                    self.profile_renderer.set_current_index(Some(self.ui_state.time_index));
                } else {
                    self.profile_renderer.set_current_index(Some(self.ui_state.level_index));
                }
                self.profile_renderer.set_level_index(None);
            }
            crate::ui::ProfileMode::TimeLevelHeatmap => {
                self.profile_renderer.set_current_index(Some(self.ui_state.time_index));
                self.profile_renderer.set_level_index(Some(self.ui_state.level_index));
            }
        }
        self.hovmoller_renderer.current_time = Some(self.ui_state.time_index);
        self.cross_section_renderer.current_level = Some(self.ui_state.level_index);

        // Contour overlay data update
        if self.ui_state.contour_enabled && self.contour_generation != self.data_generation {
            if let Some(field) = self.data_store.active_field().cloned() {
                self.contour_overlay.update_data(&field, self.ui_state.contour_levels);
            }
            self.contour_generation = self.data_generation;
        } else if !self.ui_state.contour_enabled {
            self.contour_overlay.clear();
        }

        // Streamline overlay data update
        if self.ui_state.streamline_enabled && self.streamline_generation != self.data_generation {
            if let Some(file_idx) = self.data_store.active_file {
                if let (Some(u_idx), Some(v_idx)) = (self.ui_state.vector_u_var, self.ui_state.vector_v_var) {
                    let orig_var = self.data_store.files[file_idx].selected_variable;
                    let orig_field = self.data_store.files[file_idx].field_data.clone();

                    let time_idx = self.ui_state.time_index;
                    let level_idx = self.ui_state.level_index;
                    if let Ok(vec_data) = self.data_store.load_vector_field(
                        file_idx, u_idx, v_idx, time_idx, level_idx,
                    ) {
                        self.streamline_overlay.set_data(vec_data);
                    }

                    self.data_store.files[file_idx].selected_variable = orig_var;
                    self.data_store.files[file_idx].field_data = orig_field;
                }
            }
            self.streamline_generation = self.data_generation;
        } else if !self.ui_state.streamline_enabled && self.streamline_overlay.has_data() {
            self.streamline_overlay.clear();
        }

        // External trajectory file loading (JSON/CSV)
        if self.ui_state.trajectory_external_request {
            self.ui_state.trajectory_external_request = false;
            if let Some(path) = self.ui_state.trajectory_external_path.clone() {
                match crate::data::trajectory_loader::load_trajectory_from_file(&path) {
                    Ok(traj_data) => {
                        self.ui_state.trajectory_enabled = true;
                        self.trajectory_overlay.set_data(traj_data);
                        self.trajectory_generation = self.data_generation;
                        self.ui_state.status_text = format!("Loaded trajectory: {}", path);
                        log::info!("Loaded external trajectory from {path}");
                    }
                    Err(e) => {
                        log::error!("Failed to load trajectory: {e}");
                        self.ui_state.status_text = format!("Error: {e}");
                    }
                }
            }
        }

        // Trajectory overlay data loading (NetCDF)
        if self.ui_state.trajectory_enabled
            && self.trajectory_generation != self.data_generation
        {
            if let Some(file_idx) = self.data_store.active_file {
                if let (Some(lon_idx), Some(lat_idx)) = (
                    self.ui_state.trajectory_lon_var,
                    self.ui_state.trajectory_lat_var,
                ) {
                    if let Ok(traj_data) =
                        self.data_store.load_trajectory_data(file_idx, lon_idx, lat_idx)
                    {
                        self.trajectory_overlay.set_data(traj_data);
                    }
                }
            }
            self.trajectory_generation = self.data_generation;
        } else if !self.ui_state.trajectory_enabled && self.trajectory_overlay.has_data() {
            self.trajectory_overlay.clear();
        }

        // Sync trajectory time
        if self.ui_state.trajectory_enabled {
            self.trajectory_overlay.set_current_time(self.ui_state.time_index);
            self.trajectory_overlay.set_trail_length(self.ui_state.trajectory_trail_length);
        }
    }
}
