// renderer/profile.rs — 1D line graph (vertical profile / time series)

use crate::data::ProfileData;

/// CPU-side profile renderer using egui painter.
pub struct ProfileRenderer {
    data: Option<ProfileData>,
    /// Title text displayed above the plot (e.g., variable name + coordinates).
    title: String,
    /// Current time/level index to highlight on the plot (playhead marker).
    current_index: Option<usize>,
}

impl ProfileRenderer {
    pub fn new() -> Self {
        Self {
            data: None,
            title: String::new(),
            current_index: None,
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

    pub fn clear(&mut self) {
        self.data = None;
        self.title.clear();
        self.current_index = None;
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
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
