use eframe::CreationContext;
use egui_dock::{DockArea, DockState, NodeIndex};

use crate::data::DataStore;
use crate::renderer::GlobeRenderer;
use crate::renderer::MapRenderer;
use crate::renderer::cross_section::CrossSectionRenderer;
use crate::renderer::hovmoller::HovmollerRenderer;
use crate::renderer::spectrum::SpectrumRenderer;
use crate::renderer::vector_overlay::VectorOverlay;
use crate::ui::{GeoScopeTabViewer, Tab};

const PRIMARY: egui::Color32 = egui::Color32::from_rgb(0, 164, 154);
const BG_DARK: egui::Color32 = egui::Color32::from_rgb(24, 24, 32);
const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(30, 30, 40);
const BG_WIDGET: egui::Color32 = egui::Color32::from_rgb(42, 42, 55);
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(220, 220, 230);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(130, 130, 150);

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
    ui_state: crate::ui::UiState,
    data_generation: u64,
    gpu_generation: u64,
    last_colormap: crate::ui::Colormap,
    hovmoller_generation: u64,
    cross_section_generation: u64,
    vector_generation: u64,
    last_map_projection: crate::renderer::map::MapProjection,
    theme_applied: bool,
}

impl GeoScopeApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);

        let globe_renderer = GlobeRenderer::new(cc);

        let mut dock_state = DockState::new(vec![Tab::Viewport]);
        let surface = dock_state.main_surface_mut();
        surface.split_left(NodeIndex::root(), 0.15, vec![Tab::DataBrowser]);
        surface.split_right(NodeIndex::root(), 0.80, vec![Tab::Inspector]);

        Self {
            dock_state,
            data_store: DataStore::new(),
            globe_renderer,
            map_renderer: MapRenderer::new(),
            hovmoller_renderer: HovmollerRenderer::new(),
            spectrum_renderer: SpectrumRenderer::new(),
            cross_section_renderer: CrossSectionRenderer::new(),
            vector_overlay: VectorOverlay::new(),
            ui_state: crate::ui::UiState::default(),
            data_generation: 0,
            gpu_generation: 0,
            last_colormap: crate::ui::Colormap::default(),
            hovmoller_generation: 0,
            cross_section_generation: 0,
            vector_generation: 0,
            last_map_projection: crate::renderer::map::MapProjection::default(),
            theme_applied: false,
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
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_DIM);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 50, 65));
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = BG_WIDGET;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 75));
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(55, 55, 72);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, PRIMARY);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.active.bg_fill = PRIMARY;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);

    visuals.widgets.open.bg_fill = BG_WIDGET;
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(4);

    visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(0, 164, 154, 60);
    visuals.selection.stroke = egui::Stroke::new(1.0, PRIMARY);

    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 2],
        blur: 8,
        spread: 0,
        color: egui::Color32::from_black_alpha(80),
    };
    visuals.window_corner_radius = egui::CornerRadius::same(6);

    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.striped = true;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.combo_width = 0.0;

    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(16.0, egui::FontFamily::Proportional),
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
    style.tab.inactive.text_color = TEXT_DIM;
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
            ui_state: &mut self.ui_state,
            data_generation: &mut self.data_generation,
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
                            ui.label(egui::RichText::new("›").color(TEXT_DIM).size(14.0));
                            ui.label(egui::RichText::new(name).size(13.0).color(TEXT_DIM));
                        }
                    }
                });
            });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(egui::Margin::symmetric(12, 2)))
            .exact_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new(&tab_viewer.ui_state.status_text)
                            .size(11.0)
                            .color(TEXT_DIM),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let view_label = match tab_viewer.ui_state.view_mode {
                            crate::ui::ViewMode::Globe => "Globe",
                            crate::ui::ViewMode::Map => "Map",
                            crate::ui::ViewMode::Hovmoller => "Hovmoller",
                            crate::ui::ViewMode::Spectrum => "E(n)",
                            crate::ui::ViewMode::CrossSection => "Section",
                        };
                        ui.label(egui::RichText::new(view_label).size(11.0).color(PRIMARY));
                    });
                });
            });

        DockArea::new(&mut self.dock_state)
            .style(dock_style(ctx))
            .show(ctx, &mut tab_viewer);

        // Detect colormap change
        if self.ui_state.colormap != self.last_colormap {
            self.last_colormap = self.ui_state.colormap;
            self.data_generation += 1;
        }

        // Upload field data to GPU when it changes
        if self.data_generation != self.gpu_generation {
            if let Some(field) = self.data_store.active_field().cloned() {
                if let Some(render_state) = frame.wgpu_render_state() {
                    self.globe_renderer.upload_field_data(
                        render_state,
                        &field.values,
                        field.width,
                        field.height,
                        self.ui_state.colormap,
                        self.ui_state.interpolated,
                    );
                    self.map_renderer.ensure_initialized(render_state);
                    self.map_renderer.upload_field_data(
                        render_state,
                        &field.values,
                        field.width,
                        field.height,
                        self.ui_state.colormap,
                        self.ui_state.interpolated,
                    );
                    self.gpu_generation = self.data_generation;

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
                    if let Ok(vec_data) = self.data_store.load_vector_field(
                        file_idx, u_idx, v_idx, time_idx, 0,
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
    }
}
