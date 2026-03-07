// renderer/spectrum.rs — E(n) energy spectrum log-log plot using egui 2D drawing

/// Energy spectrum data: E(n) for n = 0, 1, ..., n_max.
#[derive(Debug, Clone)]
pub struct SpectrumData {
    pub energy: Vec<f64>, // E(n) for n=0,1,...,n_max
    pub n_max: usize,
}

/// Renders a log-log spectrum plot with egui painter primitives.
pub struct SpectrumRenderer {
    data: Option<SpectrumData>,
}

impl SpectrumRenderer {
    pub fn new() -> Self {
        Self { data: None }
    }

    pub fn set_data(&mut self, data: SpectrumData) {
        self.data = Some(data);
    }

    /// Draw the log-log E(n) spectrum plot.
    pub fn paint(&mut self, ui: &mut egui::Ui) {
        let Some(data) = &self.data else {
            ui.centered_and_justified(|ui| {
                ui.label("No spectral data available");
            });
            return;
        };

        // Collect plottable points: skip n=0 and E(n)<=0
        let points: Vec<(f64, f64)> = data
            .energy
            .iter()
            .enumerate()
            .filter(|&(n, &e)| n >= 1 && e > 0.0)
            .map(|(n, &e)| (n as f64, e))
            .collect();

        if points.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No positive spectral data to plot");
            });
            return;
        }

        let margin_left = 60.0;
        let margin_bottom = 40.0;
        let margin_top = 15.0;
        let margin_right = 15.0;

        let available = ui.available_rect_before_wrap();
        let plot_rect = egui::Rect::from_min_max(
            egui::pos2(available.min.x + margin_left, available.min.y + margin_top),
            egui::pos2(available.max.x - margin_right, available.max.y - margin_bottom),
        );

        if plot_rect.width() < 20.0 || plot_rect.height() < 20.0 {
            return;
        }

        // Compute log10 ranges
        let log_n_min = (points.first().unwrap().0).log10();
        let log_n_max = (points.last().unwrap().0).log10();
        let log_e_min = points.iter().map(|&(_, e)| e.log10()).fold(f64::INFINITY, f64::min);
        let log_e_max = points.iter().map(|&(_, e)| e.log10()).fold(f64::NEG_INFINITY, f64::max);

        // Add small padding to ranges
        let x_range = (log_n_max - log_n_min).max(1.0);
        let y_range = (log_e_max - log_e_min).max(1.0);
        let x_lo = log_n_min - 0.05 * x_range;
        let x_hi = log_n_max + 0.05 * x_range;
        let y_lo = log_e_min - 0.05 * y_range;
        let y_hi = log_e_max + 0.05 * y_range;

        let to_screen = |log_n: f64, log_e: f64| -> egui::Pos2 {
            let fx = ((log_n - x_lo) / (x_hi - x_lo)) as f32;
            let fy = 1.0 - ((log_e - y_lo) / (y_hi - y_lo)) as f32; // y-axis inverted
            egui::pos2(
                plot_rect.min.x + fx * plot_rect.width(),
                plot_rect.min.y + fy * plot_rect.height(),
            )
        };

        let painter = ui.painter();
        let grid_color = egui::Color32::from_gray(60);
        let primary = egui::Color32::from_rgb(0, 164, 154);
        let label_color = egui::Color32::GRAY;
        let font = egui::FontId::proportional(11.0);

        // Grid lines at integer powers of 10
        // X-axis grid (n)
        let ix_lo = log_n_min.floor() as i32;
        let ix_hi = log_n_max.ceil() as i32;
        for p in ix_lo..=ix_hi {
            let log_n = p as f64;
            if log_n < x_lo || log_n > x_hi {
                continue;
            }
            let top = to_screen(log_n, y_hi);
            let bot = to_screen(log_n, y_lo);
            painter.line_segment([top, bot], egui::Stroke::new(0.5, grid_color));
            // Label
            let label = if p >= 0 {
                format!("{}", 10_u64.pow(p as u32))
            } else {
                format!("1e{p}")
            };
            painter.text(
                egui::pos2(bot.x, plot_rect.max.y + 4.0),
                egui::Align2::CENTER_TOP,
                label,
                font.clone(),
                label_color,
            );
        }

        // Y-axis grid (E)
        let iy_lo = log_e_min.floor() as i32;
        let iy_hi = log_e_max.ceil() as i32;
        for p in iy_lo..=iy_hi {
            let log_e = p as f64;
            if log_e < y_lo || log_e > y_hi {
                continue;
            }
            let left = to_screen(x_lo, log_e);
            let right = to_screen(x_hi, log_e);
            painter.line_segment([left, right], egui::Stroke::new(0.5, grid_color));
            let label = format!("1e{p}");
            painter.text(
                egui::pos2(plot_rect.min.x - 4.0, left.y),
                egui::Align2::RIGHT_CENTER,
                label,
                font.clone(),
                label_color,
            );
        }

        // Data line
        let screen_points: Vec<egui::Pos2> = points
            .iter()
            .map(|&(n, e)| to_screen(n.log10(), e.log10()))
            .collect();

        for pair in screen_points.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                egui::Stroke::new(1.5, primary),
            );
        }

        // Data points (small circles)
        for &pt in &screen_points {
            painter.circle_filled(pt, 2.5, primary);
        }

        // Axis labels
        painter.text(
            egui::pos2(plot_rect.center().x, available.max.y - 2.0),
            egui::Align2::CENTER_BOTTOM,
            "n",
            egui::FontId::proportional(13.0),
            label_color,
        );
        painter.text(
            egui::pos2(available.min.x + 2.0, plot_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "E(n)",
            egui::FontId::proportional(13.0),
            label_color,
        );

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
