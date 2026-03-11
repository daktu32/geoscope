// renderer/streamline.rs — Streamline overlay for vector fields

use crate::data::VectorFieldData;

/// CPU-side streamline renderer using egui painter.
pub struct StreamlineOverlay {
    data: Option<VectorFieldData>,
    pub density: usize,
    lines_cache: Vec<Vec<egui::Pos2>>,
    cache_valid: bool,
}

impl StreamlineOverlay {
    pub fn new() -> Self {
        Self {
            data: None,
            density: 6,
            lines_cache: Vec::new(),
            cache_valid: false,
        }
    }

    pub fn set_data(&mut self, data: VectorFieldData) {
        self.data = Some(data);
        self.cache_valid = false;
    }

    pub fn clear(&mut self) {
        self.data = None;
        self.lines_cache.clear();
        self.cache_valid = false;
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    /// Invalidate cached streamlines, forcing recomputation on next paint.
    #[allow(dead_code)]
    pub fn invalidate(&mut self) {
        self.cache_valid = false;
    }

    /// Paint streamlines on the equirectangular map view.
    pub fn paint_on_map(
        &mut self,
        painter: &egui::Painter,
        plot_rect: egui::Rect,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) {
        let Some(data) = &self.data else { return };
        if data.max_magnitude < 1e-20 {
            return;
        }

        if !self.cache_valid {
            self.lines_cache = Self::compute_streamlines(data, self.density);
            self.cache_valid = true;
        }

        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(100, 220, 255, 160));
        let arrow_stroke = egui::Stroke::new(1.4, egui::Color32::from_rgba_premultiplied(100, 220, 255, 200));
        let aspect = plot_rect.width() / plot_rect.height().max(1.0);
        let (sx, sy) = if aspect > 1.0 {
            (zoom / aspect, zoom)
        } else {
            (zoom, zoom * aspect)
        };

        let to_screen = |p: &egui::Pos2| -> egui::Pos2 {
            let ndc_x = -1.0 + 2.0 * p.x;
            let ndc_y = 1.0 - 2.0 * p.y;
            let scr_x = (ndc_x * sx - pan_x * sx) * 0.5 + 0.5;
            let scr_y = (-ndc_y * sy - pan_y * sy) * 0.5 + 0.5;
            egui::pos2(
                plot_rect.min.x + scr_x * plot_rect.width(),
                plot_rect.min.y + scr_y * plot_rect.height(),
            )
        };

        for line in &self.lines_cache {
            let screen_pts: Vec<egui::Pos2> = line
                .iter()
                .filter_map(|p| {
                    let sp = to_screen(p);
                    if plot_rect.contains(sp) {
                        Some(sp)
                    } else {
                        None
                    }
                })
                .collect();

            for w in screen_pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }

            // Draw arrowheads every ~20 points
            let arrow_interval = 20;
            for (idx, w) in screen_pts.windows(2).enumerate() {
                if idx % arrow_interval == arrow_interval / 2 && idx > 0 {
                    Self::draw_arrowhead(painter, w[0], w[1], 4.5, arrow_stroke);
                }
            }
        }
    }

    /// Draw a small triangle arrowhead at `tip` pointing in the direction from `from` to `tip`.
    fn draw_arrowhead(
        painter: &egui::Painter,
        from: egui::Pos2,
        tip: egui::Pos2,
        size: f32,
        stroke: egui::Stroke,
    ) {
        let dx = tip.x - from.x;
        let dy = tip.y - from.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return;
        }
        let ux = dx / len;
        let uy = dy / len;
        // Perpendicular
        let px = -uy;
        let py = ux;

        let half = size * 0.5;
        let base_x = tip.x - ux * size;
        let base_y = tip.y - uy * size;

        let p1 = egui::pos2(base_x + px * half, base_y + py * half);
        let p2 = egui::pos2(base_x - px * half, base_y - py * half);

        painter.line_segment([p1, tip], stroke);
        painter.line_segment([p2, tip], stroke);
    }

    /// Compute streamlines using RK4 integration from seed points.
    /// Returns streamlines in UV coordinates [0,1] x [0,1].
    fn compute_streamlines(data: &VectorFieldData, density: usize) -> Vec<Vec<egui::Pos2>> {
        let mut lines = Vec::new();
        let steps = 80;
        let dt_base = 0.005;

        // Compute mean magnitude for adaptive step sizing
        let mean_magnitude = {
            let sum: f32 = data.u_values.iter().zip(data.v_values.iter())
                .map(|(&u, &v)| (u * u + v * v).sqrt())
                .sum();
            let count = data.u_values.len().max(1) as f32;
            (sum / count).max(1e-10)
        };

        for j in (0..data.height).step_by(density) {
            for i in (0..data.width).step_by(density) {
                let mut x = i as f32 / data.width as f32;
                let mut y = j as f32 / data.height as f32;
                let mut pts = vec![egui::pos2(x, y)];

                for _ in 0..steps {
                    // Adaptive dt based on local velocity magnitude
                    let (u0, v0) = Self::sample_velocity_bilinear(data, x, y);
                    let mag = (u0 * u0 + v0 * v0).sqrt();
                    if mag < 1e-20 {
                        break;
                    }
                    let speed_ratio = (mag / mean_magnitude).clamp(0.3, 3.0);
                    let dt = dt_base / speed_ratio;

                    // RK4 integration (direction-normalized)
                    let normalize = |u: f32, v: f32| -> (f32, f32) {
                        let n = (u * u + v * v).sqrt().max(1e-10);
                        (u / n, v / n)
                    };

                    let (k1x, k1y) = normalize(u0, -v0); // v negated: northward vs y-down

                    let x2 = x + 0.5 * dt * k1x;
                    let y2 = y + 0.5 * dt * k1y;
                    let (su2, sv2) = Self::sample_velocity_bilinear(data, wrap_x(x2), y2.clamp(0.0, 1.0));
                    let (k2x, k2y) = normalize(su2, -sv2);

                    let x3 = x + 0.5 * dt * k2x;
                    let y3 = y + 0.5 * dt * k2y;
                    let (su3, sv3) = Self::sample_velocity_bilinear(data, wrap_x(x3), y3.clamp(0.0, 1.0));
                    let (k3x, k3y) = normalize(su3, -sv3);

                    let x4 = x + dt * k3x;
                    let y4 = y + dt * k3y;
                    let (su4, sv4) = Self::sample_velocity_bilinear(data, wrap_x(x4), y4.clamp(0.0, 1.0));
                    let (k4x, k4y) = normalize(su4, -sv4);

                    x += dt / 6.0 * (k1x + 2.0 * k2x + 2.0 * k3x + k4x);
                    y += dt / 6.0 * (k1y + 2.0 * k2y + 2.0 * k3y + k4y);

                    // Periodic boundary in x
                    x = wrap_x(x);
                    // Clamp y
                    if y < 0.0 || y > 1.0 {
                        break;
                    }

                    pts.push(egui::pos2(x, y));
                }

                if pts.len() >= 3 {
                    lines.push(pts);
                }
            }
        }
        lines
    }

    /// Bilinear interpolation of velocity at fractional UV coordinates.
    fn sample_velocity_bilinear(data: &VectorFieldData, u_frac: f32, v_frac: f32) -> (f32, f32) {
        let fx = u_frac * data.width as f32;
        let fy = v_frac * data.height as f32;

        let ix0 = (fx.floor() as isize).max(0) as usize;
        let iy0 = (fy.floor() as isize).max(0) as usize;
        let ix1 = if ix0 + 1 >= data.width { 0 } else { ix0 + 1 }; // periodic in x
        let iy1 = (iy0 + 1).min(data.height - 1);

        let tx = fx - fx.floor();
        let ty = fy - fy.floor();

        let ix0 = ix0.min(data.width - 1);

        let get = |ix: usize, iy: usize| -> (f32, f32) {
            let idx = iy * data.width + ix;
            if idx < data.u_values.len() {
                (data.u_values[idx], data.v_values[idx])
            } else {
                (0.0, 0.0)
            }
        };

        let (u00, v00) = get(ix0, iy0);
        let (u10, v10) = get(ix1, iy0);
        let (u01, v01) = get(ix0, iy1);
        let (u11, v11) = get(ix1, iy1);

        let u = u00 * (1.0 - tx) * (1.0 - ty)
            + u10 * tx * (1.0 - ty)
            + u01 * (1.0 - tx) * ty
            + u11 * tx * ty;
        let v = v00 * (1.0 - tx) * (1.0 - ty)
            + v10 * tx * (1.0 - ty)
            + v01 * (1.0 - tx) * ty
            + v11 * tx * ty;

        (u, v)
    }

    /// Legacy nearest-neighbor sample (kept for backward compat, unused internally).
    #[allow(dead_code)]
    fn sample_velocity(data: &VectorFieldData, u_frac: f32, v_frac: f32) -> (f32, f32) {
        let fx = u_frac * data.width as f32;
        let fy = v_frac * data.height as f32;
        let ix = (fx as usize).min(data.width - 1);
        let iy = (fy as usize).min(data.height - 1);
        let idx = iy * data.width + ix;
        if idx < data.u_values.len() {
            (data.u_values[idx], data.v_values[idx])
        } else {
            (0.0, 0.0)
        }
    }
}

/// Wrap x to [0, 1) for periodic longitude boundary.
fn wrap_x(x: f32) -> f32 {
    let mut x = x;
    if x < 0.0 { x += 1.0; }
    if x >= 1.0 { x -= 1.0; }
    x
}
