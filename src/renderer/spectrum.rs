// renderer/spectrum.rs — E(n)/E(m) energy spectrum plot using egui 2D drawing

/// Energy spectrum data: both E(n) and E(m).
#[derive(Debug, Clone)]
pub struct SpectrumData {
    /// E(n) for n=0,1,...,n_trunc (total wavenumber spectrum)
    pub energy_n: Vec<f64>,
    /// E(m) for m=0,1,...,m_trunc (zonal wavenumber spectrum)
    pub energy_m: Vec<f64>,
    pub n_trunc: usize,
    pub m_trunc: usize,
}

/// Which spectrum to display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpectrumDisplayMode {
    TotalWavenumber,    // E(n)
    ZonalWavenumber,    // E(m)
    TemporalFrequency,  // E(ω)
}

/// Renders spectrum plots.
///
/// - E(n): log-log line plot with reference slopes (n⁻³, n⁻⁵ᐟ³)
/// - E(m): linear-x, log-y bar chart (clearly shows discrete wavenumber structure)
///
/// The Y-axis range is "sticky" per mode: it only expands (never shrinks) across
/// successive `set_data` calls, so the axis stays stable during animation.
/// Call `reset_range()` when switching variables to start fresh.
pub struct SpectrumRenderer {
    data: Option<SpectrumData>,
    /// Accumulated Y-axis envelope per mode (log10 scale)
    y_envelope_n: Option<(f64, f64)>,
    y_envelope_m: Option<(f64, f64)>,
    /// Temporal spectrum data (E(ω))
    pub temporal_data: Option<crate::data::temporal_filter::TemporalSpectrumData>,
    y_envelope_omega: Option<(f64, f64)>,
}

impl SpectrumRenderer {
    pub fn new() -> Self {
        Self {
            data: None,
            y_envelope_n: None,
            y_envelope_m: None,
            temporal_data: None,
            y_envelope_omega: None,
        }
    }

    pub fn set_data(&mut self, data: SpectrumData) {
        // Expand Y envelope for E(n)
        Self::expand_envelope(&mut self.y_envelope_n, &data.energy_n);
        // Expand Y envelope for E(m)
        Self::expand_envelope(&mut self.y_envelope_m, &data.energy_m);
        self.data = Some(data);
    }

    fn expand_envelope(envelope: &mut Option<(f64, f64)>, energy: &[f64]) {
        let (emin, emax) = energy
            .iter()
            .copied()
            .filter(|&e| e > 0.0)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), e| {
                (lo.min(e), hi.max(e))
            });
        if emin.is_finite() && emax.is_finite() {
            let (lmin, lmax) = (emin.log10(), emax.log10());
            *envelope = Some(match *envelope {
                Some((prev_lo, prev_hi)) => (prev_lo.min(lmin), prev_hi.max(lmax)),
                None => (lmin, lmax),
            });
        }
    }

    /// Set temporal spectrum data for E(ω) display.
    pub fn set_temporal_data(&mut self, data: crate::data::temporal_filter::TemporalSpectrumData) {
        Self::expand_envelope(&mut self.y_envelope_omega, &data.energy);
        self.temporal_data = Some(data);
    }

    /// Reset the sticky Y-axis range (call when variable changes).
    pub fn reset_range(&mut self) {
        self.y_envelope_n = None;
        self.y_envelope_m = None;
        self.y_envelope_omega = None;
        self.temporal_data = None;
    }

    /// Draw the spectrum plot for the given mode.
    pub fn paint(&mut self, ui: &mut egui::Ui, mode: SpectrumDisplayMode) {
        match mode {
            SpectrumDisplayMode::TotalWavenumber => self.paint_loglog(ui, mode),
            SpectrumDisplayMode::ZonalWavenumber => self.paint_bar(ui),
            SpectrumDisplayMode::TemporalFrequency => self.paint_temporal(ui),
        }
    }

    /// E(n): log-log line plot with reference slopes.
    fn paint_loglog(&self, ui: &mut egui::Ui, mode: SpectrumDisplayMode) {
        let bg_rect = ui.available_rect_before_wrap();
        super::globe::paint_viewport_background(ui.painter(), bg_rect);

        let Some(data) = &self.data else {
            ui.centered_and_justified(|ui| {
                ui.label(crate::i18n::t("no_spectrum"));
            });
            return;
        };

        let (energy, y_envelope, x_label) = match mode {
            SpectrumDisplayMode::TotalWavenumber => (&data.energy_n, &self.y_envelope_n, "n"),
            SpectrumDisplayMode::ZonalWavenumber => (&data.energy_m, &self.y_envelope_m, "m"),
            SpectrumDisplayMode::TemporalFrequency => unreachable!(),
        };
        let y_label = match mode {
            SpectrumDisplayMode::TotalWavenumber => "E(n)",
            SpectrumDisplayMode::ZonalWavenumber => "E(m)",
            SpectrumDisplayMode::TemporalFrequency => unreachable!(),
        };

        // Collect plottable points: skip index 0, E<=0, and machine-precision noise
        let e_max = energy.iter().copied().fold(0.0_f64, f64::max);
        let noise_floor = e_max * 1e-20;
        let points: Vec<(f64, f64)> = energy
            .iter()
            .enumerate()
            .filter(|&(k, &e)| k >= 1 && e > noise_floor)
            .map(|(k, &e)| (k as f64, e))
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
            egui::pos2(
                available.min.x + margin_left,
                available.min.y + margin_top,
            ),
            egui::pos2(
                available.max.x - margin_right,
                available.max.y - margin_bottom,
            ),
        );

        if plot_rect.width() < 20.0 || plot_rect.height() < 20.0 {
            return;
        }

        // X range: log10
        let log_k_min = (points.first().unwrap().0).log10();
        let log_k_max = (points.last().unwrap().0).log10();
        let x_range = (log_k_max - log_k_min).max(0.5);
        let x_lo = log_k_min - 0.05 * x_range;
        let x_hi = log_k_max + 0.05 * x_range;

        // Y range: sticky envelope, rounded to integer powers of 10
        let (raw_y_lo, raw_y_hi) = y_envelope.unwrap_or_else(|| {
            let lo = points
                .iter()
                .map(|&(_, e)| e.log10())
                .fold(f64::INFINITY, f64::min);
            let hi = points
                .iter()
                .map(|&(_, e)| e.log10())
                .fold(f64::NEG_INFINITY, f64::max);
            (lo, hi)
        });
        let y_lo = raw_y_lo.floor() - 0.5;
        let y_hi = raw_y_hi.ceil() + 0.5;

        let to_screen = |log_k: f64, log_e: f64| -> egui::Pos2 {
            let fx = ((log_k - x_lo) / (x_hi - x_lo)) as f32;
            let fy = 1.0 - ((log_e - y_lo) / (y_hi - y_lo)) as f32;
            egui::pos2(
                plot_rect.min.x + fx * plot_rect.width(),
                plot_rect.min.y + fy * plot_rect.height(),
            )
        };

        let painter = ui.painter();
        let grid_color = egui::Color32::from_gray(60);
        let primary = egui::Color32::from_rgb(0, 200, 190);
        let label_color = egui::Color32::GRAY;
        let font = egui::FontId::proportional(11.0);

        // --- X-axis grid and tick labels ---
        // Background gridlines at powers of 10
        let ix_lo = log_k_min.floor() as i32;
        let ix_hi = log_k_max.ceil() as i32;
        for p in ix_lo..=ix_hi {
            let log_k = p as f64;
            if log_k < x_lo || log_k > x_hi {
                continue;
            }
            let top = to_screen(log_k, y_hi);
            let bot = to_screen(log_k, y_lo);
            painter.line_segment([top, bot], egui::Stroke::new(0.5, grid_color));
        }

        // Tick labels at data points (the actual n values)
        let min_label_spacing = 28.0_f32;
        let mut last_label_x = f32::NEG_INFINITY;
        for &(k, _) in &points {
            let n = k as usize;
            let pos = to_screen(k.log10(), y_lo);
            let sx = pos.x;
            if sx - last_label_x < min_label_spacing {
                continue;
            }
            // Small tick mark
            painter.line_segment(
                [
                    egui::pos2(sx, plot_rect.max.y),
                    egui::pos2(sx, plot_rect.max.y + 3.0),
                ],
                egui::Stroke::new(1.0, label_color),
            );
            painter.text(
                egui::pos2(sx, plot_rect.max.y + 4.0),
                egui::Align2::CENTER_TOP,
                format!("{n}"),
                font.clone(),
                primary,
            );
            last_label_x = sx;
        }

        // --- Y-axis grid ---
        let iy_lo = y_lo.ceil() as i32;
        let iy_hi = y_hi.floor() as i32;
        for p in iy_lo..=iy_hi {
            let log_e = p as f64;
            let left = to_screen(x_lo, log_e);
            let right = to_screen(x_hi, log_e);
            painter.line_segment([left, right], egui::Stroke::new(0.5, grid_color));
            painter.text(
                egui::pos2(plot_rect.min.x - 4.0, left.y),
                egui::Align2::RIGHT_CENTER,
                format!("1e{p}"),
                font.clone(),
                label_color,
            );
        }

        // --- Reference slopes (n⁻³ and n⁻⁵/³) ---
        let anchor_idx = points.len() / 2;
        let anchor_log_k = points[anchor_idx].0.log10();
        let anchor_log_e = points[anchor_idx].1.log10();
        let ref_slopes: &[(f64, f64, &str, egui::Color32)] = &[
            (
                -3.0,
                1.0,
                "n⁻³",
                egui::Color32::from_rgba_premultiplied(255, 120, 80, 180),
            ),
            (
                -5.0 / 3.0,
                -1.0,
                "n⁻⁵ᐟ³",
                egui::Color32::from_rgba_premultiplied(120, 180, 255, 180),
            ),
        ];

        for &(slope, y_offset, label, color) in ref_slopes {
            let ref_anchor_e = anchor_log_e + y_offset;
            let ref_k_start = log_k_min.max(x_lo);
            let ref_k_end = log_k_max.min(x_hi);
            let e_start = ref_anchor_e + slope * (ref_k_start - anchor_log_k);
            let e_end = ref_anchor_e + slope * (ref_k_end - anchor_log_k);

            let p0 = to_screen(ref_k_start, e_start);
            let p1 = to_screen(ref_k_end, e_end);
            painter.line_segment([p0, p1], egui::Stroke::new(1.0, color));
            painter.text(
                egui::pos2(p1.x + 3.0, p1.y),
                egui::Align2::LEFT_CENTER,
                label,
                font.clone(),
                color,
            );
        }

        // --- Data line ---
        let screen_points: Vec<egui::Pos2> = points
            .iter()
            .map(|&(k, e)| to_screen(k.log10(), e.log10()))
            .collect();

        for pair in screen_points.windows(2) {
            painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.5, primary));
        }

        // Data points (circles)
        for &pt in &screen_points {
            painter.circle_filled(pt, 2.5, primary);
        }

        // --- Axis labels ---
        Self::draw_axis_labels(painter, &available, &plot_rect, x_label, y_label, label_color);

        // Border
        painter.rect_stroke(
            plot_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            egui::epaint::StrokeKind::Outside,
        );

        ui.allocate_rect(available, egui::Sense::hover());
    }

    /// E(m): linear-x, log-y bar chart for discrete wavenumber structure.
    fn paint_bar(&self, ui: &mut egui::Ui) {
        let bg_rect = ui.available_rect_before_wrap();
        super::globe::paint_viewport_background(ui.painter(), bg_rect);

        let Some(data) = &self.data else {
            ui.centered_and_justified(|ui| {
                ui.label(crate::i18n::t("no_spectrum"));
            });
            return;
        };

        let energy = &data.energy_m;
        let y_envelope = &self.y_envelope_m;

        // All points including zero-energy (we'll draw bars only for E > 0)
        let e_max = energy.iter().copied().fold(0.0_f64, f64::max);
        let noise_floor = e_max * 1e-20;

        if e_max <= 0.0 {
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
            egui::pos2(
                available.min.x + margin_left,
                available.min.y + margin_top,
            ),
            egui::pos2(
                available.max.x - margin_right,
                available.max.y - margin_bottom,
            ),
        );

        if plot_rect.width() < 20.0 || plot_rect.height() < 20.0 {
            return;
        }

        // X range: linear, from 0 to m_trunc
        let m_max = data.m_trunc;
        let x_lo = -0.5_f64;
        let x_hi = m_max as f64 + 0.5;

        // Y range: log10, sticky envelope
        let (raw_y_lo, raw_y_hi) = y_envelope.unwrap_or_else(|| {
            let positive: Vec<f64> = energy
                .iter()
                .copied()
                .filter(|&e| e > noise_floor)
                .collect();
            if positive.is_empty() {
                return (-20.0, 0.0);
            }
            let lo = positive
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min)
                .log10();
            let hi = positive
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
                .log10();
            (lo, hi)
        });
        let y_lo = raw_y_lo.floor() - 0.5;
        let y_hi = raw_y_hi.ceil() + 0.5;

        let to_screen_x = |m: f64| -> f32 {
            let fx = ((m - x_lo) / (x_hi - x_lo)) as f32;
            plot_rect.min.x + fx * plot_rect.width()
        };

        let to_screen_y = |log_e: f64| -> f32 {
            let fy = 1.0 - ((log_e - y_lo) / (y_hi - y_lo)) as f32;
            plot_rect.min.y + fy * plot_rect.height()
        };

        let painter = ui.painter();
        let grid_color = egui::Color32::from_gray(60);
        let primary = egui::Color32::from_rgb(0, 200, 190);
        let bar_fill = egui::Color32::from_rgba_premultiplied(0, 200, 190, 100);
        let label_color = egui::Color32::GRAY;
        let font = egui::FontId::proportional(11.0);

        // --- X-axis ticks: label every bar that has data, thin gridlines at regular intervals ---
        // Collect m values with non-zero energy for tick labels
        let active_ms: Vec<usize> = energy
            .iter()
            .enumerate()
            .filter(|&(m, &e)| m >= 1 && e > noise_floor)
            .map(|(m, _)| m)
            .collect();

        // Draw faint background gridlines at regular intervals
        let tick_step = if m_max > 30 { 10 } else { 5 };
        for m in (0..=m_max).step_by(tick_step) {
            let sx = to_screen_x(m as f64);
            if sx < plot_rect.min.x || sx > plot_rect.max.x {
                continue;
            }
            painter.line_segment(
                [
                    egui::pos2(sx, plot_rect.min.y),
                    egui::pos2(sx, plot_rect.max.y),
                ],
                egui::Stroke::new(0.5, grid_color),
            );
        }

        // Draw tick labels at active m values (where bars exist)
        // Thin out labels if they'd overlap: require min pixel spacing
        let min_label_spacing = 28.0_f32;
        let mut last_label_x = f32::NEG_INFINITY;
        for &m in &active_ms {
            let sx = to_screen_x(m as f64);
            if sx - last_label_x < min_label_spacing {
                continue;
            }
            // Small tick mark
            painter.line_segment(
                [
                    egui::pos2(sx, plot_rect.max.y),
                    egui::pos2(sx, plot_rect.max.y + 3.0),
                ],
                egui::Stroke::new(1.0, label_color),
            );
            painter.text(
                egui::pos2(sx, plot_rect.max.y + 4.0),
                egui::Align2::CENTER_TOP,
                format!("{m}"),
                font.clone(),
                primary,
            );
            last_label_x = sx;
        }

        // Also label m=0 for reference
        {
            let sx = to_screen_x(0.0);
            if sx >= plot_rect.min.x && (active_ms.is_empty() || to_screen_x(active_ms[0] as f64) - sx >= min_label_spacing) {
                painter.text(
                    egui::pos2(sx, plot_rect.max.y + 4.0),
                    egui::Align2::CENTER_TOP,
                    "0".to_string(),
                    font.clone(),
                    label_color,
                );
            }
        }

        // --- Y-axis grid ---
        let iy_lo = y_lo.ceil() as i32;
        let iy_hi = y_hi.floor() as i32;
        for p in iy_lo..=iy_hi {
            let sy = to_screen_y(p as f64);
            painter.line_segment(
                [
                    egui::pos2(plot_rect.min.x, sy),
                    egui::pos2(plot_rect.max.x, sy),
                ],
                egui::Stroke::new(0.5, grid_color),
            );
            painter.text(
                egui::pos2(plot_rect.min.x - 4.0, sy),
                egui::Align2::RIGHT_CENTER,
                format!("1e{p}"),
                font.clone(),
                label_color,
            );
        }

        // --- Bars ---
        let bar_half_width = (plot_rect.width() / (x_hi - x_lo) as f32 * 0.35).min(6.0);
        let baseline_y = to_screen_y(y_lo);

        for (m, &e) in energy.iter().enumerate() {
            if m == 0 || e <= noise_floor {
                continue;
            }
            let log_e = e.log10();
            let sx = to_screen_x(m as f64);
            let sy = to_screen_y(log_e);

            // Filled bar
            let bar_rect = egui::Rect::from_min_max(
                egui::pos2(sx - bar_half_width, sy),
                egui::pos2(sx + bar_half_width, baseline_y),
            );
            painter.rect_filled(bar_rect, 0.0, bar_fill);
            painter.rect_stroke(
                bar_rect,
                0.0,
                egui::Stroke::new(1.0, primary),
                egui::epaint::StrokeKind::Outside,
            );

            // Circle at the top of each bar
            painter.circle_filled(egui::pos2(sx, sy), 2.5, primary);
        }

        // --- Axis labels ---
        Self::draw_axis_labels(painter, &available, &plot_rect, "m", "E(m)", label_color);

        // Border
        painter.rect_stroke(
            plot_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            egui::epaint::StrokeKind::Outside,
        );

        ui.allocate_rect(available, egui::Sense::hover());
    }

    /// E(ω): log-log line plot for temporal frequency spectrum (no reference slopes).
    fn paint_temporal(&self, ui: &mut egui::Ui) {
        let bg_rect = ui.available_rect_before_wrap();
        super::globe::paint_viewport_background(ui.painter(), bg_rect);

        let Some(data) = &self.temporal_data else {
            ui.centered_and_justified(|ui| {
                ui.label(crate::i18n::t("click_for_spectrum"));
            });
            return;
        };

        let y_envelope = &self.y_envelope_omega;

        // Collect plottable points: skip k=0 (DC), skip E<=0
        let e_max = data.energy.iter().copied().fold(0.0_f64, f64::max);
        let noise_floor = e_max * 1e-20;
        let points: Vec<(f64, f64)> = data
            .energy
            .iter()
            .enumerate()
            .filter(|&(k, &e)| k >= 1 && e > noise_floor)
            .map(|(k, &e)| (k as f64, e))
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
            egui::pos2(
                available.min.x + margin_left,
                available.min.y + margin_top,
            ),
            egui::pos2(
                available.max.x - margin_right,
                available.max.y - margin_bottom,
            ),
        );

        if plot_rect.width() < 20.0 || plot_rect.height() < 20.0 {
            return;
        }

        // X range: log10 of frequency index
        let log_k_min = (points.first().unwrap().0).log10();
        let log_k_max = (points.last().unwrap().0).log10();
        let x_range = (log_k_max - log_k_min).max(0.5);
        let x_lo = log_k_min - 0.05 * x_range;
        let x_hi = log_k_max + 0.05 * x_range;

        // Y range: sticky envelope
        let (raw_y_lo, raw_y_hi) = y_envelope.unwrap_or_else(|| {
            let lo = points.iter().map(|&(_, e)| e.log10()).fold(f64::INFINITY, f64::min);
            let hi = points.iter().map(|&(_, e)| e.log10()).fold(f64::NEG_INFINITY, f64::max);
            (lo, hi)
        });
        let y_lo = raw_y_lo.floor() - 0.5;
        let y_hi = raw_y_hi.ceil() + 0.5;

        let to_screen = |log_k: f64, log_e: f64| -> egui::Pos2 {
            let fx = ((log_k - x_lo) / (x_hi - x_lo)) as f32;
            let fy = 1.0 - ((log_e - y_lo) / (y_hi - y_lo)) as f32;
            egui::pos2(
                plot_rect.min.x + fx * plot_rect.width(),
                plot_rect.min.y + fy * plot_rect.height(),
            )
        };

        let painter = ui.painter();
        let grid_color = egui::Color32::from_gray(60);
        let primary = egui::Color32::from_rgb(220, 160, 60); // warm amber for temporal
        let label_color = egui::Color32::GRAY;
        let font = egui::FontId::proportional(11.0);

        // --- X-axis grid ---
        let ix_lo = log_k_min.floor() as i32;
        let ix_hi = log_k_max.ceil() as i32;
        for p in ix_lo..=ix_hi {
            let log_k = p as f64;
            if log_k < x_lo || log_k > x_hi {
                continue;
            }
            let top = to_screen(log_k, y_hi);
            let bot = to_screen(log_k, y_lo);
            painter.line_segment([top, bot], egui::Stroke::new(0.5, grid_color));
        }

        // Tick labels
        let min_label_spacing = 28.0_f32;
        let mut last_label_x = f32::NEG_INFINITY;
        for &(k, _) in &points {
            let idx = k as usize;
            let pos = to_screen(k.log10(), y_lo);
            let sx = pos.x;
            if sx - last_label_x < min_label_spacing {
                continue;
            }
            painter.line_segment(
                [
                    egui::pos2(sx, plot_rect.max.y),
                    egui::pos2(sx, plot_rect.max.y + 3.0),
                ],
                egui::Stroke::new(1.0, label_color),
            );
            // Show physical frequency if dt is meaningful
            let label_text = if data.dt > 0.0 && data.dt != 1.0 {
                let freq = idx as f64 / (data.energy.len() as f64 * data.dt);
                if freq < 0.01 {
                    format!("{:.1e}", freq)
                } else {
                    format!("{:.3}", freq)
                }
            } else {
                format!("{idx}")
            };
            painter.text(
                egui::pos2(sx, plot_rect.max.y + 4.0),
                egui::Align2::CENTER_TOP,
                label_text,
                font.clone(),
                primary,
            );
            last_label_x = sx;
        }

        // --- Y-axis grid ---
        let iy_lo = y_lo.ceil() as i32;
        let iy_hi = y_hi.floor() as i32;
        for p in iy_lo..=iy_hi {
            let log_e = p as f64;
            let left = to_screen(x_lo, log_e);
            let right = to_screen(x_hi, log_e);
            painter.line_segment([left, right], egui::Stroke::new(0.5, grid_color));
            painter.text(
                egui::pos2(plot_rect.min.x - 4.0, left.y),
                egui::Align2::RIGHT_CENTER,
                format!("1e{p}"),
                font.clone(),
                label_color,
            );
        }

        // --- Data line (no reference slopes for temporal) ---
        let screen_points: Vec<egui::Pos2> = points
            .iter()
            .map(|&(k, e)| to_screen(k.log10(), e.log10()))
            .collect();

        for pair in screen_points.windows(2) {
            painter.line_segment([pair[0], pair[1]], egui::Stroke::new(1.5, primary));
        }

        for &pt in &screen_points {
            painter.circle_filled(pt, 2.5, primary);
        }

        // --- Axis labels ---
        let x_label = if data.dt > 0.0 && data.dt != 1.0 { "ω" } else { "k" };
        Self::draw_axis_labels(painter, &available, &plot_rect, x_label, "E(ω)", label_color);

        // Border
        painter.rect_stroke(
            plot_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
            egui::epaint::StrokeKind::Outside,
        );

        ui.allocate_rect(available, egui::Sense::hover());
    }

    fn draw_axis_labels(
        painter: &egui::Painter,
        available: &egui::Rect,
        plot_rect: &egui::Rect,
        x_label: &str,
        y_label: &str,
        label_color: egui::Color32,
    ) {
        painter.text(
            egui::pos2(plot_rect.center().x, available.max.y - 2.0),
            egui::Align2::CENTER_BOTTOM,
            x_label,
            egui::FontId::proportional(13.0),
            label_color,
        );
        painter.text(
            egui::pos2(available.min.x + 2.0, plot_rect.center().y),
            egui::Align2::LEFT_CENTER,
            y_label,
            egui::FontId::proportional(13.0),
            label_color,
        );
    }
}
