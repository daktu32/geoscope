// renderer/profile.rs — 1D line graph (vertical profile / time series) + time-level heatmap

use crate::data::ProfileData;
use crate::data::TimeLevelData;
use crate::renderer::common::{colormap_lut, colormap_rgba_with_lut};
use crate::ui::Colormap;

/// CPU-side profile renderer using egui painter.
pub struct ProfileRenderer {
    data: Option<ProfileData>,
    /// Title text displayed above the plot (e.g., variable name + coordinates).
    title: String,
    /// Current time/level index to highlight on the plot (playhead marker).
    current_index: Option<usize>,
    /// Time × Level heatmap data + texture
    heatmap_data: Option<TimeLevelData>,
    heatmap_texture: Option<egui::TextureHandle>,
    heatmap_pending: Option<egui::ColorImage>,
}

impl ProfileRenderer {
    pub fn new() -> Self {
        Self {
            data: None,
            title: String::new(),
            current_index: None,
            heatmap_data: None,
            heatmap_texture: None,
            heatmap_pending: None,
        }
    }

    pub fn set_data(&mut self, data: ProfileData) {
        self.data = Some(data);
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn set_current_index(&mut self, index: Option<usize>) {
        self.current_index = index;
    }

    pub fn set_heatmap_data(&mut self, data: TimeLevelData, colormap: Colormap) {
        let w = data.n_time;
        let h = data.n_level;
        let lut = colormap_lut(colormap);
        let mut pixels = Vec::with_capacity(w * h * 4);
        // Row = level (top=0), Col = time
        for lev in 0..h {
            for t in 0..w {
                let val = data.values[t * h + lev]; // data is [time][level]
                let [r, g, b, a] = colormap_rgba_with_lut(val, data.min, data.max, &lut);
                pixels.extend_from_slice(&[r, g, b, a]);
            }
        }
        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
        self.heatmap_pending = Some(image);
        self.heatmap_texture = None;
        self.heatmap_data = Some(data);
    }

    pub fn clear(&mut self) {
        self.data = None;
        self.title.clear();
        self.current_index = None;
        self.heatmap_data = None;
        self.heatmap_texture = None;
        self.heatmap_pending = None;
    }

    pub fn clear_heatmap(&mut self) {
        self.heatmap_data = None;
        self.heatmap_texture = None;
        self.heatmap_pending = None;
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn has_heatmap(&self) -> bool {
        self.heatmap_data.is_some()
    }

    pub fn paint(&self, ui: &mut egui::Ui) {
        let Some(data) = &self.data else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Select a point on Globe/Map to view profile")
                        .color(crate::app::TEXT_CAPTION),
                );
            });
            return;
        };

        if data.values.is_empty() {
            return;
        }

        let rect = ui.available_rect_before_wrap();
        crate::renderer::globe::paint_viewport_background(ui.painter(), rect);

        let margin_left = 70.0;
        let margin_right = 20.0;
        let margin_top = if self.title.is_empty() { 20.0 } else { 36.0 };
        let margin_bottom = 40.0;

        let plot = egui::Rect::from_min_max(
            egui::pos2(rect.left() + margin_left, rect.top() + margin_top),
            egui::pos2(rect.right() - margin_right, rect.bottom() - margin_bottom),
        );
        if plot.width() < 20.0 || plot.height() < 20.0 {
            return;
        }

        let n = data.values.len();
        let range = data.max - data.min;

        let axis_min = data.axis_values.first().copied().unwrap_or(0.0);
        let axis_max = data.axis_values.last().copied().unwrap_or(1.0);
        let axis_range = axis_max - axis_min;

        // Compute points for the data line
        let points: Option<Vec<egui::Pos2>> = if n >= 2 && range.abs() > 1e-20 {
            Some(
                (0..n)
                    .map(|i| {
                        let t = if axis_range.abs() > 1e-30 {
                            ((data.axis_values[i] - axis_min) / axis_range) as f32
                        } else {
                            i as f32 / (n - 1) as f32
                        };
                        let v = (data.values[i] - data.min) / range;
                        egui::pos2(
                            plot.left() + t * plot.width(),
                            plot.bottom() - v * plot.height(),
                        )
                    })
                    .collect(),
            )
        } else {
            None
        };

        // Draw axes, labels, data line using a scoped painter borrow
        {
            let painter = ui.painter();

            // --- Title ---
            if !self.title.is_empty() {
                painter.text(
                    egui::pos2(plot.center().x, rect.top() + 8.0),
                    egui::Align2::CENTER_TOP,
                    &self.title,
                    egui::FontId::monospace(11.0),
                    crate::app::TEXT_HEADING,
                );
            }

            // --- Axes ---
            let axis_color = crate::app::TEXT_CAPTION;
            let axis_stroke = egui::Stroke::new(1.0, axis_color);
            painter.line_segment([plot.left_bottom(), plot.right_bottom()], axis_stroke);
            painter.line_segment([plot.left_bottom(), plot.left_top()], axis_stroke);

            // --- X-axis ticks (axis_values) ---
            let x_tick_count = compute_tick_count(n, 7);
            let tick_font = egui::FontId::monospace(9.0);

            for i in 0..x_tick_count {
                let frac = i as f32 / (x_tick_count - 1).max(1) as f32;
                let x = plot.left() + frac * plot.width();
                let val = axis_min + frac as f64 * axis_range;

                painter.line_segment(
                    [egui::pos2(x, plot.bottom()), egui::pos2(x, plot.bottom() + 3.0)],
                    axis_stroke,
                );
                painter.text(
                    egui::pos2(x, plot.bottom() + 5.0),
                    egui::Align2::CENTER_TOP,
                    format_tick_value(val),
                    tick_font.clone(),
                    axis_color,
                );
            }

            // X-axis label
            painter.text(
                egui::pos2(plot.center().x, rect.bottom() - 4.0),
                egui::Align2::CENTER_BOTTOM,
                &data.axis_label,
                egui::FontId::monospace(10.0),
                crate::app::TEXT_SECONDARY,
            );

            // --- Y-axis ticks (value range) ---
            let y_tick_count = compute_tick_count(n, 6);

            for i in 0..y_tick_count {
                let frac = i as f32 / (y_tick_count - 1).max(1) as f32;
                let y = plot.bottom() - frac * plot.height();
                let val = data.min as f64 + frac as f64 * range as f64;

                painter.line_segment(
                    [egui::pos2(plot.left() - 3.0, y), egui::pos2(plot.left(), y)],
                    axis_stroke,
                );
                painter.text(
                    egui::pos2(plot.left() - 5.0, y),
                    egui::Align2::RIGHT_CENTER,
                    format_tick_value(val),
                    tick_font.clone(),
                    axis_color,
                );

                if i > 0 && i < y_tick_count - 1 {
                    let grid_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 20);
                    painter.line_segment(
                        [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
                        egui::Stroke::new(0.5, grid_color),
                    );
                }
            }

            // Y-axis label
            painter.text(
                egui::pos2(rect.left() + 4.0, plot.center().y),
                egui::Align2::LEFT_CENTER,
                &data.value_label,
                egui::FontId::monospace(10.0),
                crate::app::TEXT_SECONDARY,
            );

            // --- Data line and markers ---
            if let Some(ref points) = points {
                let line_color = crate::app::PRIMARY;
                for w in points.windows(2) {
                    painter.line_segment([w[0], w[1]], egui::Stroke::new(1.5, line_color));
                }
                for &pt in points {
                    painter.circle_filled(pt, 2.0, line_color);
                }
            }

            // --- Playhead: current time/level marker ---
            if let (Some(points), Some(idx)) = (&points, self.current_index) {
                if idx < points.len() && idx < data.values.len() {
                    let pt = points[idx];
                    let val = data.values[idx];

                    // Vertical line (full height, semi-transparent)
                    let playhead_color = egui::Color32::from_rgba_premultiplied(255, 200, 60, 140);
                    painter.line_segment(
                        [egui::pos2(pt.x, plot.top()), egui::pos2(pt.x, plot.bottom())],
                        egui::Stroke::new(1.0, playhead_color),
                    );

                    // Highlighted dot
                    painter.circle_filled(pt, 5.0, playhead_color);
                    painter.circle_stroke(pt, 5.0, egui::Stroke::new(1.0, egui::Color32::WHITE));

                    // Value label near the dot
                    let label = format_tick_value(val as f64);
                    let label_offset = if pt.y - plot.top() > 30.0 {
                        egui::vec2(8.0, -16.0) // above
                    } else {
                        egui::vec2(8.0, 10.0)  // below
                    };
                    let label_pos = pt + label_offset;
                    let label_font = egui::FontId::monospace(10.0);
                    let galley = painter.layout_no_wrap(
                        label.clone(),
                        label_font,
                        crate::app::TEXT_HEADING,
                    );
                    let bg = egui::Rect::from_min_size(label_pos, galley.size()).expand(3.0);
                    painter.rect_filled(bg, 2.0, egui::Color32::from_rgba_unmultiplied(15, 15, 23, 210));
                    painter.galley(label_pos, galley, crate::app::TEXT_HEADING);
                }
            }
        } // painter borrow ends here

        // Allocate rect for hover sensing
        let response = ui.allocate_rect(rect, egui::Sense::hover());

        // --- Hover crosshair ---
        if let Some(ref points) = points {
            if let Some(hover_pos) = response.hover_pos() {
                if plot.contains(hover_pos) {
                    let painter = ui.painter();

                    // Crosshair lines
                    let crosshair_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 60);
                    let crosshair_stroke = egui::Stroke::new(0.5, crosshair_color);
                    painter.line_segment(
                        [egui::pos2(hover_pos.x, plot.top()), egui::pos2(hover_pos.x, plot.bottom())],
                        crosshair_stroke,
                    );
                    painter.line_segment(
                        [egui::pos2(plot.left(), hover_pos.y), egui::pos2(plot.right(), hover_pos.y)],
                        crosshair_stroke,
                    );

                    // Find nearest data point
                    let mut best_idx = 0;
                    let mut best_dist = f32::INFINITY;
                    for (i, &pt) in points.iter().enumerate() {
                        let dx = (pt.x - hover_pos.x).abs();
                        if dx < best_dist {
                            best_dist = dx;
                            best_idx = i;
                        }
                    }

                    // Tooltip
                    let axis_val = data.axis_values[best_idx];
                    let data_val = data.values[best_idx];
                    let tooltip_text = format!(
                        "{}: {}\n{}: {}",
                        data.axis_label,
                        format_tick_value(axis_val),
                        data.value_label,
                        format_tick_value(data_val as f64),
                    );

                    // Position tooltip
                    let tooltip_offset = egui::vec2(12.0, -20.0);
                    let tooltip_pos = hover_pos + tooltip_offset;

                    // Background rect for tooltip
                    let tooltip_font = egui::FontId::monospace(10.0);
                    let galley = painter.layout_no_wrap(
                        tooltip_text.clone(),
                        tooltip_font.clone(),
                        crate::app::TEXT_HEADING,
                    );
                    let text_rect = egui::Rect::from_min_size(tooltip_pos, galley.size());
                    let bg_rect = text_rect.expand(4.0);
                    painter.rect_filled(bg_rect, 3.0, crate::app::BG_WIDGET);
                    painter.rect_stroke(
                        bg_rect,
                        3.0,
                        egui::Stroke::new(0.5, crate::app::DIVIDER),
                        egui::epaint::StrokeKind::Outside,
                    );
                    painter.galley(tooltip_pos, galley, crate::app::TEXT_HEADING);

                    // Highlight nearest point
                    painter.circle_stroke(
                        points[best_idx],
                        4.0,
                        egui::Stroke::new(1.5, egui::Color32::WHITE),
                    );
                }
            }
        }
    }

    /// Paint a time × level heatmap.
    pub fn paint_heatmap(&mut self, ui: &mut egui::Ui) {
        let bg_rect = ui.available_rect_before_wrap();
        super::globe::paint_viewport_background(ui.painter(), bg_rect);

        // Lazily create texture
        if let Some(image) = self.heatmap_pending.take() {
            self.heatmap_texture = Some(ui.ctx().load_texture(
                "time_level_heatmap",
                image,
                egui::TextureOptions::LINEAR,
            ));
        }

        let Some(tex) = &self.heatmap_texture else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No time-level data (needs both time and level dimensions)")
                        .color(crate::app::TEXT_CAPTION),
                );
            });
            return;
        };
        let Some(data) = &self.heatmap_data else { return; };

        let margin_left = 50.0;
        let margin_bottom = 30.0;
        let margin_top = 30.0;
        let margin_right = 10.0;

        let available = ui.available_rect_before_wrap();
        let plot_rect = egui::Rect::from_min_max(
            egui::pos2(available.min.x + margin_left, available.min.y + margin_top),
            egui::pos2(available.max.x - margin_right, available.max.y - margin_bottom),
        );
        if plot_rect.width() < 10.0 || plot_rect.height() < 10.0 { return; }

        // Title
        let painter = ui.painter();
        painter.text(
            egui::pos2(plot_rect.center().x, available.min.y + 4.0),
            egui::Align2::CENTER_TOP,
            &self.title,
            egui::FontId::proportional(12.0),
            crate::app::TEXT_HEADING,
        );

        // Draw texture
        let tex_id = tex.id();
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(tex_id, plot_rect, uv, egui::Color32::WHITE);

        let label_color = egui::Color32::GRAY;
        let font = egui::FontId::proportional(10.0);

        // X-axis: time
        let n_ticks = 5.min(data.n_time);
        for i in 0..n_ticks {
            let frac = if n_ticks > 1 { i as f32 / (n_ticks - 1) as f32 } else { 0.5 };
            let idx = ((frac * (data.n_time - 1) as f32) as usize).min(data.n_time - 1);
            let x = plot_rect.min.x + frac * plot_rect.width();
            let label = format_tick_value(data.time_values[idx]);
            painter.text(egui::pos2(x, plot_rect.max.y + 3.0), egui::Align2::CENTER_TOP, &label, font.clone(), label_color);
            painter.line_segment(
                [egui::pos2(x, plot_rect.max.y), egui::pos2(x, plot_rect.max.y + 2.0)],
                egui::Stroke::new(0.5, label_color),
            );
        }
        // X-axis label
        painter.text(
            egui::pos2(plot_rect.center().x, plot_rect.max.y + 16.0),
            egui::Align2::CENTER_TOP,
            &data.time_label,
            font.clone(),
            label_color,
        );

        // Y-axis: level
        let n_lev_ticks = 5.min(data.n_level);
        for i in 0..n_lev_ticks {
            let frac = if n_lev_ticks > 1 { i as f32 / (n_lev_ticks - 1) as f32 } else { 0.5 };
            let idx = ((frac * (data.n_level - 1) as f32) as usize).min(data.n_level - 1);
            let y = plot_rect.min.y + frac * plot_rect.height();
            let label = format_tick_value(data.level_values[idx]);
            painter.text(egui::pos2(plot_rect.min.x - 4.0, y), egui::Align2::RIGHT_CENTER, &label, font.clone(), label_color);
            painter.line_segment(
                [egui::pos2(plot_rect.min.x - 2.0, y), egui::pos2(plot_rect.min.x, y)],
                egui::Stroke::new(0.5, label_color),
            );
        }
        // Y-axis label
        painter.text(
            egui::pos2(available.min.x + 4.0, plot_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &data.level_label,
            font.clone(),
            label_color,
        );

        // Playhead line (current time)
        if let Some(t_idx) = self.current_index {
            if data.n_time > 1 {
                let frac = t_idx as f32 / (data.n_time - 1) as f32;
                let x = plot_rect.min.x + frac * plot_rect.width();
                painter.line_segment(
                    [egui::pos2(x, plot_rect.top()), egui::pos2(x, plot_rect.bottom())],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 200, 50)),
                );
            }
        }

        // Plot border
        painter.rect_stroke(
            plot_rect,
            0.0,
            egui::Stroke::new(0.5, egui::Color32::from_gray(80)),
            egui::epaint::StrokeKind::Outside,
        );
    }

    /// Paint a 3D surface plot using isometric projection.
    /// Axes: x=time (depth), y=value (lateral displacement), z=level (vertical, screen up/down).
    /// This keeps the vertical axis aligned with the physical height dimension.
    pub fn paint_surface3d(&self, ui: &mut egui::Ui, colormap: Colormap) {
        let bg_rect = ui.available_rect_before_wrap();
        super::globe::paint_viewport_background(ui.painter(), bg_rect);

        let Some(data) = &self.heatmap_data else {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("No time-level data (needs both time and level dimensions)")
                        .color(crate::app::TEXT_CAPTION),
                );
            });
            return;
        };

        let painter = ui.painter();
        let available = ui.available_rect_before_wrap();

        // Title
        painter.text(
            egui::pos2(available.center().x, available.min.y + 4.0),
            egui::Align2::CENTER_TOP,
            &self.title,
            egui::FontId::proportional(12.0),
            crate::app::TEXT_HEADING,
        );

        let margin = 40.0;
        let plot_rect = egui::Rect::from_min_max(
            egui::pos2(available.min.x + margin, available.min.y + margin),
            egui::pos2(available.max.x - margin * 0.5, available.max.y - margin * 0.5),
        );
        if plot_rect.width() < 50.0 || plot_rect.height() < 50.0 { return; }

        // Downsample for performance
        let step_t = (data.n_time / 40).max(1);
        let step_l = (data.n_level / 20).max(1);
        let nt = (data.n_time + step_t - 1) / step_t;
        let nl = (data.n_level + step_l - 1) / step_l;

        // Isometric projection:
        //   x-axis = time  → goes into the screen (right-down in iso)
        //   y-axis = value → lateral displacement (left-right, perpendicular to time)
        //   z-axis = level → vertical (screen up = higher level index)
        let cx = plot_rect.center().x - plot_rect.width() * 0.1;
        let cy = plot_rect.center().y + plot_rect.height() * 0.05;

        let scale_t = plot_rect.width() * 0.35 / nt.max(1) as f32;
        let val_range = (data.max - data.min).max(1e-10);
        let scale_val = plot_rect.width() * 0.2;  // max lateral displacement
        let scale_lev = plot_rect.height() * 0.7 / nl.max(1) as f32;

        // Isometric basis vectors
        let at = egui::vec2(0.87 * scale_t, 0.30 * scale_t);    // time → right-down (depth)
        let av = egui::vec2(-0.87, 0.30);                         // value → left-down (lateral)
        let al = egui::vec2(0.0, -scale_lev);                     // level → straight up

        let lut = colormap_lut(colormap);

        // Project: (time_i, level_i) → screen pos, also returns value
        let project = |ti: usize, li: usize| -> (egui::Pos2, f32) {
            let t_idx = (ti * step_t).min(data.n_time - 1);
            let l_idx = (li * step_l).min(data.n_level - 1);
            let val = data.values[t_idx * data.n_level + l_idx];
            let val_norm = (val - data.min) / val_range; // [0, 1]
            // level goes up, time goes into depth, value displaces laterally
            let sx = cx + at.x * ti as f32 + av.x * val_norm * scale_val;
            let sy = cy + at.y * ti as f32 + av.y * val_norm * scale_val + al.y * li as f32;
            (egui::pos2(sx, sy), val)
        };

        // Draw back-to-front: far time first, top level first
        for ti in 0..nt.saturating_sub(1) {
            for li in (0..nl.saturating_sub(1)).rev() {
                let (p00, v00) = project(ti, li);
                let (p10, _) = project(ti + 1, li);
                let (p11, _) = project(ti + 1, li + 1);
                let (p01, _) = project(ti, li + 1);

                let [r, g, b, _] = colormap_rgba_with_lut(v00, data.min, data.max, &lut);
                let fill = egui::Color32::from_rgb(r, g, b);

                // Filled quad
                let mesh = egui::Mesh {
                    indices: vec![0, 1, 2, 0, 2, 3],
                    vertices: vec![
                        egui::epaint::Vertex { pos: p00, uv: egui::epaint::WHITE_UV, color: fill },
                        egui::epaint::Vertex { pos: p10, uv: egui::epaint::WHITE_UV, color: fill },
                        egui::epaint::Vertex { pos: p11, uv: egui::epaint::WHITE_UV, color: fill },
                        egui::epaint::Vertex { pos: p01, uv: egui::epaint::WHITE_UV, color: fill },
                    ],
                    texture_id: egui::TextureId::default(),
                };
                painter.add(egui::Shape::mesh(mesh));

                // Wireframe
                let wire_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 50);
                let wire = egui::Stroke::new(0.3, wire_color);
                painter.line_segment([p00, p10], wire);
                painter.line_segment([p00, p01], wire);
            }
        }

        // Draw axes from bottom-near corner
        let origin = egui::pos2(cx, cy);
        let axis_color = egui::Color32::from_gray(120);
        let axis_stroke = egui::Stroke::new(1.0, axis_color);
        let font = egui::FontId::proportional(10.0);

        // Time axis (depth)
        let t_end = egui::pos2(
            origin.x + at.x * (nt - 1) as f32,
            origin.y + at.y * (nt - 1) as f32,
        );
        painter.line_segment([origin, t_end], axis_stroke);
        painter.text(t_end + egui::vec2(8.0, 4.0), egui::Align2::LEFT_TOP,
            &data.time_label, font.clone(), axis_color);

        // Level axis (vertical, up)
        let l_end = egui::pos2(
            origin.x + al.x * (nl - 1) as f32,
            origin.y + al.y * (nl - 1) as f32,
        );
        painter.line_segment([origin, l_end], axis_stroke);
        painter.text(l_end + egui::vec2(-8.0, -4.0), egui::Align2::RIGHT_BOTTOM,
            &data.level_label, font.clone(), axis_color);

        // Value axis (lateral)
        let v_end = egui::pos2(
            origin.x + av.x * scale_val,
            origin.y + av.y * scale_val,
        );
        painter.line_segment([origin, v_end], axis_stroke);
        painter.text(v_end + egui::vec2(-8.0, 4.0), egui::Align2::RIGHT_TOP,
            &data.value_label, font.clone(), axis_color);

        // Playhead: highlight current time slice as a vertical line of segments
        if let Some(t_idx) = self.current_index {
            let ti = t_idx / step_t;
            if ti < nt {
                for li in 0..nl.saturating_sub(1) {
                    let (p0, _) = project(ti, li);
                    let (p1, _) = project(ti, li + 1);
                    painter.line_segment([p0, p1],
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 200, 50)));
                }
            }
        }
    }
}

/// Compute a reasonable number of ticks (between 2 and max_ticks).
fn compute_tick_count(n_data: usize, max_ticks: usize) -> usize {
    if n_data <= 2 {
        2
    } else {
        max_ticks.min(n_data).max(2)
    }
}

/// Format a tick value: use scientific notation for very large or very small values.
fn format_tick_value(val: f64) -> String {
    let abs = val.abs();
    if abs == 0.0 {
        "0".to_string()
    } else if abs >= 1e4 || abs < 1e-2 {
        format!("{:.2e}", val)
    } else {
        format!("{:.3}", val)
    }
}
