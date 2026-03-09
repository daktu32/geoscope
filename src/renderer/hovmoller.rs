// renderer/hovmoller.rs — Hovmoller diagram (time x longitude) using egui 2D drawing

use crate::renderer::common::{colormap_lut, colormap_rgba_with_lut};
use crate::ui::Colormap;

/// Row-major [time][lon] data for a Hovmoller diagram.
#[derive(Debug, Clone)]
pub struct HovmollerData {
    pub values: Vec<f32>, // row-major [time][lon]
    pub n_lon: usize,
    pub n_time: usize,
    pub min: f32,
    pub max: f32,
}

/// Renders a Hovmoller diagram as an egui texture.
pub struct HovmollerRenderer {
    texture: Option<egui::TextureHandle>,
    pending_image: Option<egui::ColorImage>,
    n_lon: usize,
    n_time: usize,
    /// Current time index for playhead line
    pub current_time: Option<usize>,
}

impl HovmollerRenderer {
    pub fn new() -> Self {
        Self {
            texture: None,
            pending_image: None,
            n_lon: 0,
            n_time: 0,
            current_time: None,
        }
    }

    /// Convert data to a ColorImage. The texture handle is created lazily in `paint()`.
    pub fn set_data(&mut self, data: &HovmollerData, colormap: Colormap) {
        let w = data.n_lon;
        let h = data.n_time;
        let lut = colormap_lut(colormap);
        let mut pixels = Vec::with_capacity(w * h * 4);

        for t in 0..h {
            for x in 0..w {
                let val = data.values[t * w + x];
                let [r, g, b, a] = colormap_rgba_with_lut(val, data.min, data.max, &lut);
                pixels.extend_from_slice(&[r, g, b, a]);
            }
        }

        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
        self.pending_image = Some(image);
        self.texture = None; // force re-upload
        self.n_lon = w;
        self.n_time = h;
    }

    /// Draw the Hovmoller diagram filling the available space with axis labels.
    pub fn paint(&mut self, ui: &mut egui::Ui) {
        // Stylish background
        let bg_rect = ui.available_rect_before_wrap();
        super::globe::paint_viewport_background(ui.painter(), bg_rect);

        // Lazily create texture from pending image
        if let Some(image) = self.pending_image.take() {
            self.texture = Some(ui.ctx().load_texture(
                "hovmoller",
                image,
                egui::TextureOptions::LINEAR,
            ));
        }

        let Some(tex) = &self.texture else {
            ui.centered_and_justified(|ui| {
                ui.label("No Hovmoller data available");
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
        ui.painter().image(
            tex_id,
            plot_rect,
            uv,
            egui::Color32::WHITE,
        );

        let painter = ui.painter();
        let label_color = egui::Color32::GRAY;
        let font = egui::FontId::proportional(11.0);

        // X-axis: Longitude labels (0, 90, 180, 270, 360)
        let lon_labels = [
            (0.0, "0"),
            (0.25, "90"),
            (0.5, "180"),
            (0.75, "270"),
            (1.0, "360"),
        ];
        for &(frac, label) in &lon_labels {
            let x = plot_rect.min.x + frac as f32 * plot_rect.width();
            let y = plot_rect.max.y + 4.0;
            painter.text(
                egui::pos2(x, y),
                egui::Align2::CENTER_TOP,
                label,
                font.clone(),
                label_color,
            );
            // Tick mark
            painter.line_segment(
                [egui::pos2(x, plot_rect.max.y), egui::pos2(x, plot_rect.max.y + 3.0)],
                egui::Stroke::new(1.0, label_color),
            );
        }

        // X-axis title
        painter.text(
            egui::pos2(plot_rect.center().x, available.max.y - 2.0),
            egui::Align2::CENTER_BOTTOM,
            "Longitude",
            font.clone(),
            label_color,
        );

        // Y-axis: Time step labels (evenly spaced, ~5 ticks)
        let n_ticks = 5.min(self.n_time).max(2);
        for i in 0..n_ticks {
            let frac = i as f32 / (n_ticks - 1) as f32;
            let y = plot_rect.min.y + frac * plot_rect.height();
            let time_val = (frac * (self.n_time - 1) as f32).round() as usize;
            painter.text(
                egui::pos2(plot_rect.min.x - 4.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{time_val}"),
                font.clone(),
                label_color,
            );
            painter.line_segment(
                [egui::pos2(plot_rect.min.x - 3.0, y), egui::pos2(plot_rect.min.x, y)],
                egui::Stroke::new(1.0, label_color),
            );
        }

        // Y-axis title
        painter.text(
            egui::pos2(available.min.x + 2.0, plot_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Time",
            font.clone(),
            label_color,
        );

        // Time playhead line
        if let Some(t_idx) = self.current_time {
            if self.n_time > 1 {
                let frac = t_idx as f32 / (self.n_time - 1) as f32;
                let y = plot_rect.min.y + frac * plot_rect.height();
                let playhead_color = egui::Color32::from_rgba_premultiplied(255, 200, 60, 180);
                painter.line_segment(
                    [egui::pos2(plot_rect.left(), y), egui::pos2(plot_rect.right(), y)],
                    egui::Stroke::new(1.5, playhead_color),
                );
            }
        }

        // Border around plot area
        painter.rect_stroke(
            plot_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            egui::epaint::StrokeKind::Outside,
        );

        // Reserve the full available space so layout advances past this widget
        ui.allocate_rect(available, egui::Sense::hover());
    }
}
