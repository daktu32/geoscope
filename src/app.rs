use eframe::CreationContext;
use egui_dock::{DockArea, DockState, NodeIndex};

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

        let globe_renderer = GlobeRenderer::new(cc);

        let mut dock_state = DockState::new(vec![Tab::Viewport]);
        let surface = dock_state.main_surface_mut();
        surface.split_left(NodeIndex::root(), 0.15, vec![Tab::DataBrowser]);
        surface.split_right(NodeIndex::root(), 0.80, vec![Tab::Inspector, Tab::CodePanel]);

        Self {
            dock_state,
            data_store: DataStore::new(),
            globe_renderer,
            map_renderer: MapRenderer::new(),
            hovmoller_renderer: HovmollerRenderer::new(),
            spectrum_renderer: SpectrumRenderer::new(),
            cross_section_renderer: CrossSectionRenderer::new(),
            vector_overlay: VectorOverlay::new(),
            profile_renderer: ProfileRenderer::new(),
            contour_overlay: ContourOverlay::new(),
            streamline_overlay: StreamlineOverlay::new(),
            trajectory_overlay: TrajectoryOverlay::new(),
            ui_state: crate::ui::UiState::default(),
            data_generation: 0,
            gpu_generation: 0,
            last_colormap: crate::ui::Colormap::default(),
            hovmoller_generation: 0,
            cross_section_generation: 0,
            vector_generation: 0,
            profile_generation: 0,
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
    style.tab_bar.height = 28.0;

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

        DockArea::new(&mut self.dock_state)
            .style(dock_style(ctx))
            .show(ctx, &mut tab_viewer);

        // Export dialog
        if self.ui_state.export_dialog_open {
            let mut open = true;
            let mut do_export = false;
            egui::Window::new("Export PNG")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_min_width(280.0);

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
                    if ui.add(egui::Button::new(
                        egui::RichText::new("Save PNG").color(egui::Color32::WHITE).size(12.0)
                    ).fill(PRIMARY)).clicked() {
                        do_export = true;
                    }
                });
            if !open {
                self.ui_state.export_dialog_open = false;
            }
            if do_export {
                if let Some(field) = self.data_store.active_field().cloned() {
                    // Determine display range
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
                        match crate::renderer::export::export_png_with_settings(
                            &field,
                            self.ui_state.colormap,
                            display_min,
                            display_max,
                            &self.ui_state.export_settings,
                            &path,
                        ) {
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

        // Profile data loading
        if self.ui_state.view_mode == crate::ui::ViewMode::Profile
            && self.profile_generation != self.data_generation
        {
            if let Some(file_idx) = self.data_store.active_file {
                if let Some(file) = self.data_store.files.get(file_idx) {
                    if let Some(var_idx) = file.selected_variable {
                        if let Some(ref field) = file.field_data {
                            let lon_idx = field.width / 2;
                            let lat_idx = field.height / 2;
                            let time_idx = self.ui_state.time_index;
                            if let Some(profile) = self.data_store.load_profile_data(
                                file_idx, var_idx, time_idx, lon_idx, lat_idx,
                            ) {
                                let var_name = file.variables[var_idx].name.clone();
                                self.profile_renderer.set_title(format!(
                                    "{} (lon={}, lat={})", var_name, lon_idx, lat_idx
                                ));
                                self.profile_renderer.set_data(profile);
                            } else {
                                // Fall back to time series
                                let level_idx = self.ui_state.level_index;
                                if let Some(ts) = self.data_store.load_time_series_data(
                                    file_idx, var_idx, level_idx, lon_idx, lat_idx,
                                ) {
                                    let var_name = file.variables[var_idx].name.clone();
                                    self.profile_renderer.set_title(format!(
                                        "{} time series (lon={}, lat={})", var_name, lon_idx, lat_idx
                                    ));
                                    self.profile_renderer.set_data(ts);
                                } else {
                                    self.profile_renderer.clear();
                                }
                            }
                        }
                    }
                }
            }
            self.profile_generation = self.data_generation;
        }

        // Sync profile playhead with current time index (every frame, cheap)
        if self.ui_state.view_mode == crate::ui::ViewMode::Profile {
            self.profile_renderer.set_current_index(Some(self.ui_state.time_index));
        }

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

        // Trajectory overlay data loading
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
