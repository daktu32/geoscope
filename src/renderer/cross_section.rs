// renderer/cross_section.rs — Vertical cross-section (level x lat/lon) using egui 2D drawing

use crate::data::CrossSectionData;
use crate::renderer::common::{colormap_lut, colormap_rgba_with_lut};
use crate::ui::Colormap;

/// Renders a vertical cross-section as an egui texture.
pub struct CrossSectionRenderer {
    texture: Option<egui::TextureHandle>,
    pending_image: Option<egui::ColorImage>,
    n_spatial: usize,
    n_levels: usize,
    is_lat_axis: bool,
    level_values: Vec<f64>,
    /// Current level index for playhead line
    pub current_level: Option<usize>,
}

impl CrossSectionRenderer {
    pub fn new() -> Self {
        Self {
            texture: None,
            pending_image: None,
            n_spatial: 0,
            n_levels: 0,
            is_lat_axis: false,
            level_values: Vec::new(),
            current_level: None,
        }
    }

    /// Convert cross-section data to a ColorImage.
    pub fn set_data(&mut self, data: &CrossSectionData, colormap: Colormap) {
        let w = data.n_spatial;
        let h = data.n_levels;
        let lut = colormap_lut(colormap);
        let mut pixels = Vec::with_capacity(w * h * 4);

        // Data is [level][spatial], level 0 = lowest → render top-to-bottom inverted
        // so high levels appear at top of image
        for lev in (0..h).rev() {
            for x in 0..w {
                let val = data.values[lev * w + x];
                let [r, g, b, a] = colormap_rgba_with_lut(val, data.min, data.max, &lut);
                pixels.extend_from_slice(&[r, g, b, a]);
            }
        }

        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
        self.pending_image = Some(image);
        self.texture = None;
        self.n_spatial = w;
        self.n_levels = h;
        self.is_lat_axis = data.axis == crate::data::CrossSectionAxis::Latitude;
        self.level_values = data.level_values.clone();
    }

    /// Draw the cross-section filling the available space with axis labels.
    pub fn paint(&mut self, ui: &mut egui::Ui) {
        let bg_rect = ui.available_rect_before_wrap();
        super::globe::paint_viewport_background(ui.painter(), bg_rect);

        // Lazily create texture from pending image
        if let Some(image) = self.pending_image.take() {
            self.texture = Some(ui.ctx().load_texture(
                "cross_section",
                image,
                egui::TextureOptions::LINEAR,
            ));
        }

        let Some(tex) = &self.texture else {
            ui.centered_and_justified(|ui| {
                ui.label(crate::i18n::t("no_cross_section"));
            });
            return;
        };

        let margin_left = 50.0;
        let margin_bottom = 30.0;
        let margin_top = 10.0;
        let margin_right = 10.0;

        let available = ui.available_rect_before_wrap();
        let plot_rect = egui::Rect::from_min_max(
            egui::pos2(available.min.x + margin_left, available.min.y + margin_top),
            egui::pos2(available.max.x - margin_right, available.max.y - margin_bottom),
        );

        if plot_rect.width() < 10.0 || plot_rect.height() < 10.0 {
            return;
        }

        // Draw the texture stretched to fill the plot area
        let tex_id = tex.id();
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(tex_id, plot_rect, uv, egui::Color32::WHITE);

        let painter = ui.painter();
        let label_color = egui::Color32::GRAY;
        let font = egui::FontId::proportional(11.0);

        // X-axis labels
        if self.is_lat_axis {
            // Fixed lat, X = longitude
            let lon_labels = [
                (0.0, "0"),
                (0.25, "90"),
                (0.5, "180"),
                (0.75, "270"),
                (1.0, "360"),
            ];
            for &(frac, label) in &lon_labels {
                let x = plot_rect.min.x + frac as f32 * plot_rect.width();
                painter.text(
                    egui::pos2(x, plot_rect.max.y + 4.0),
                    egui::Align2::CENTER_TOP,
                    label,
                    font.clone(),
                    label_color,
                );
                painter.line_segment(
                    [egui::pos2(x, plot_rect.max.y), egui::pos2(x, plot_rect.max.y + 3.0)],
                    egui::Stroke::new(1.0, label_color),
                );
            }
            painter.text(
                egui::pos2(plot_rect.center().x, available.max.y - 2.0),
                egui::Align2::CENTER_BOTTOM,
                "Longitude",
                font.clone(),
                label_color,
            );
        } else {
            // Fixed lon, X = latitude
            let lat_labels = [
                (0.0, "-90"),
                (0.25, "-45"),
                (0.5, "0"),
                (0.75, "45"),
                (1.0, "90"),
            ];
            for &(frac, label) in &lat_labels {
                let x = plot_rect.min.x + frac as f32 * plot_rect.width();
                painter.text(
                    egui::pos2(x, plot_rect.max.y + 4.0),
                    egui::Align2::CENTER_TOP,
                    label,
                    font.clone(),
                    label_color,
                );
                painter.line_segment(
                    [egui::pos2(x, plot_rect.max.y), egui::pos2(x, plot_rect.max.y + 3.0)],
                    egui::Stroke::new(1.0, label_color),
                );
            }
            painter.text(
                egui::pos2(plot_rect.center().x, available.max.y - 2.0),
                egui::Align2::CENTER_BOTTOM,
                "Latitude",
                font.clone(),
                label_color,
            );
        }

        // Y-axis: Level coordinate values (top = high level, bottom = level 0)
        let has_coord = !self.level_values.is_empty() && self.level_values.len() == self.n_levels;
        let n_ticks = 5.min(self.n_levels).max(2);
        for i in 0..n_ticks {
            let frac = i as f32 / (n_ticks - 1) as f32;
            let y = plot_rect.min.y + frac * plot_rect.height();
            // top of image = highest level index, bottom = level 0
            let level_idx = ((1.0 - frac) * (self.n_levels - 1) as f32).round() as usize;
            let label_text = if has_coord {
                let v = self.level_values[level_idx];
                if v.abs() >= 100.0 || (v.abs() < 0.01 && v != 0.0) {
                    format!("{v:.2e}")
                } else {
                    format!("{v:.3}")
                }
            } else {
                format!("{level_idx}")
            };
            painter.text(
                egui::pos2(plot_rect.min.x - 4.0, y),
                egui::Align2::RIGHT_CENTER,
                label_text,
                font.clone(),
                label_color,
            );
            painter.line_segment(
                [egui::pos2(plot_rect.min.x - 3.0, y), egui::pos2(plot_rect.min.x, y)],
                egui::Stroke::new(1.0, label_color),
            );
        }

        painter.text(
            egui::pos2(available.min.x + 2.0, plot_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Level",
            font.clone(),
            label_color,
        );

        // Level playhead line (top = highest level, bottom = level 0)
        if let Some(lev_idx) = self.current_level {
            if self.n_levels > 1 {
                let frac = 1.0 - lev_idx as f32 / (self.n_levels - 1) as f32;
                let y = plot_rect.min.y + frac * plot_rect.height();
                let playhead_color = egui::Color32::from_rgba_premultiplied(255, 200, 60, 180);
                painter.line_segment(
                    [egui::pos2(plot_rect.left(), y), egui::pos2(plot_rect.right(), y)],
                    egui::Stroke::new(1.5, playhead_color),
                );
            }
        }

        // Border
        painter.rect_stroke(
            plot_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            egui::epaint::StrokeKind::Outside,
        );

        ui.allocate_rect(available, egui::Sense::hover());
    }
}
