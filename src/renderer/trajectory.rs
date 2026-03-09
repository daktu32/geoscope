// renderer/trajectory.rs — Trajectory overlay on Globe/Map views

use crate::data::TrajectoryData;

/// CPU-side trajectory overlay renderer using egui painter.
pub struct TrajectoryOverlay {
    data: Option<TrajectoryData>,
    current_time: usize,
    trail_length: usize,
    pub color: egui::Color32,
    pub dot_radius: f32,
}

impl TrajectoryOverlay {
    pub fn new() -> Self {
        Self {
            data: None,
            current_time: 0,
            trail_length: 500,
            color: egui::Color32::from_rgb(0, 200, 180), // teal
            dot_radius: 5.0,
        }
    }

    pub fn set_data(&mut self, data: TrajectoryData) {
        self.data = Some(data);
    }

    pub fn clear(&mut self) {
        self.data = None;
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn set_current_time(&mut self, time_idx: usize) {
        self.current_time = time_idx;
    }

    pub fn set_trail_length(&mut self, len: usize) {
        self.trail_length = len;
    }

    /// Paint trajectory on an equirectangular Map view.
    pub fn paint_on_map(
        &self,
        painter: &egui::Painter,
        plot_rect: egui::Rect,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) {
        let data = match &self.data {
            Some(d) if !d.points.is_empty() => d,
            _ => return,
        };

        let n = data.points.len();
        if n == 0 {
            return;
        }

        let current = self.current_time.min(n - 1);

        // Convert lon/lat to screen position (equirectangular UV mapping)
        // Data is latitude-flipped in the texture (row 0 = south at v=0 = top),
        // so negate lat to match the flipped display.
        let to_screen = |lon_deg: f32, lat_deg: f32| -> egui::Pos2 {
            let lon_norm = ((lon_deg % 360.0) + 360.0) % 360.0;
            let u = lon_norm / 360.0;
            let v = (90.0 + lat_deg) / 180.0;

            // UV -> world coords [-1, 1]
            let wx = u * 2.0 - 1.0;
            let wy = 1.0 - v * 2.0;

            // Apply zoom and pan (ortho projection)
            let aspect = plot_rect.width() / plot_rect.height().max(1.0);
            let (sx, sy) = if aspect > 1.0 {
                (zoom / aspect, zoom)
            } else {
                (zoom, zoom * aspect)
            };

            let screen_x = plot_rect.center().x + (wx * sx - pan_x * sx) * plot_rect.width() * 0.5;
            let screen_y = plot_rect.center().y - (wy * sy - pan_y * sy) * plot_rect.height() * 0.5;
            egui::pos2(screen_x, screen_y)
        };

        // Trail: full path from t=0, recent portion brighter
        let trail_len = self.trail_length.max(1) as f32;
        let bright_start = current.saturating_sub(self.trail_length);

        for i in 0..current {
            let (lon0, lat0) = data.points[i];
            let (lon1, lat1) = data.points[i + 1];
            let p0 = to_screen(lon0, lat0);
            let p1 = to_screen(lon1, lat1);

            if (lon1 - lon0).abs() > 180.0 {
                continue;
            }

            let alpha = if i >= bright_start {
                // Recent: fade from 180 (newest) to 80
                let t = (current - i) as f32 / trail_len;
                (180.0 - t * 100.0) as u8
            } else {
                // Old: dim but visible
                40
            };

            let trail_color = egui::Color32::from_rgba_unmultiplied(
                self.color.r(),
                self.color.g(),
                self.color.b(),
                alpha,
            );
            painter.line_segment([p0, p1], egui::Stroke::new(1.5, trail_color));
        }

        // Start marker: small hollow circle at t=0
        {
            let (lon0, lat0) = data.points[0];
            let start_pos = to_screen(lon0, lat0);
            let start_color = egui::Color32::from_rgba_unmultiplied(
                self.color.r(),
                self.color.g(),
                self.color.b(),
                140,
            );
            painter.circle_stroke(start_pos, 3.5, egui::Stroke::new(1.2, start_color));
        }

        // Current position: filled circle + white stroke
        let (lon, lat) = data.points[current];
        let pos = to_screen(lon, lat);
        painter.circle_filled(pos, self.dot_radius, self.color);
        painter.circle_stroke(pos, self.dot_radius, egui::Stroke::new(1.5, egui::Color32::WHITE));
    }

    /// Paint trajectory on a Globe view.
    pub fn paint_on_globe(
        &self,
        painter: &egui::Painter,
        plot_rect: egui::Rect,
        view: &[[f32; 4]; 4],
        view_proj: &[[f32; 4]; 4],
    ) {
        let data = match &self.data {
            Some(d) if !d.points.is_empty() => d,
            _ => return,
        };

        let n = data.points.len();
        if n == 0 {
            return;
        }

        let current = self.current_time.min(n - 1);

        // Convert lon/lat to screen position via 3D sphere projection.
        // Globe mesh maps data row 0 (south) to v=0 (north pole), so the display
        // is latitude-flipped. Negate lat to match the flipped display.
        let to_screen = |lon_deg: f32, lat_deg: f32| -> Option<egui::Pos2> {
            let theta = lon_deg.to_radians();
            let lat_rad = (-lat_deg).to_radians();
            let cos_lat = lat_rad.cos();
            // Globe mesh coords: x = sin_phi*cos_theta = cos_lat*cos_theta
            //                    y = cos_phi = sin_lat
            //                    z = sin_phi*sin_theta = cos_lat*sin_theta
            let x = cos_lat * theta.cos();
            let y = lat_rad.sin();
            let z = cos_lat * theta.sin();

            // Back-face culling (same as VectorOverlay: vz < 0 means facing away)
            let vz = view[2][0] * x + view[2][1] * y + view[2][2] * z;
            if vz < 0.0 {
                return None;
            }

            // Apply view_proj to get clip coords
            let cx = view_proj[0][0] * x + view_proj[0][1] * y + view_proj[0][2] * z + view_proj[0][3];
            let cy = view_proj[1][0] * x + view_proj[1][1] * y + view_proj[1][2] * z + view_proj[1][3];
            let cw = view_proj[3][0] * x + view_proj[3][1] * y + view_proj[3][2] * z + view_proj[3][3];

            if cw.abs() < 1e-6 {
                return None;
            }

            let ndc_x = cx / cw;
            let ndc_y = cy / cw;

            let screen_x = plot_rect.center().x + ndc_x * plot_rect.width() * 0.5;
            let screen_y = plot_rect.center().y - ndc_y * plot_rect.height() * 0.5;
            Some(egui::pos2(screen_x, screen_y))
        };

        // Trail: full path from t=0, recent portion brighter
        let trail_len = self.trail_length.max(1) as f32;
        let bright_start = current.saturating_sub(self.trail_length);

        for i in 0..current {
            let (lon0, lat0) = data.points[i];
            let (lon1, lat1) = data.points[i + 1];

            if let (Some(p0), Some(p1)) = (to_screen(lon0, lat0), to_screen(lon1, lat1)) {
                if (lon1 - lon0).abs() > 180.0 {
                    continue;
                }

                let alpha = if i >= bright_start {
                    let t = (current - i) as f32 / trail_len;
                    (180.0 - t * 100.0) as u8
                } else {
                    40
                };

                let trail_color = egui::Color32::from_rgba_unmultiplied(
                    self.color.r(),
                    self.color.g(),
                    self.color.b(),
                    alpha,
                );
                painter.line_segment([p0, p1], egui::Stroke::new(1.5, trail_color));
            }
        }

        // Start marker: small hollow circle at t=0
        {
            let (lon0, lat0) = data.points[0];
            if let Some(start_pos) = to_screen(lon0, lat0) {
                let start_color = egui::Color32::from_rgba_unmultiplied(
                    self.color.r(),
                    self.color.g(),
                    self.color.b(),
                    140,
                );
                painter.circle_stroke(start_pos, 3.5, egui::Stroke::new(1.2, start_color));
            }
        }

        // Current position
        let (lon, lat) = data.points[current];
        if let Some(pos) = to_screen(lon, lat) {
            painter.circle_filled(pos, self.dot_radius, self.color);
            painter.circle_stroke(pos, self.dot_radius, egui::Stroke::new(1.5, egui::Color32::WHITE));
        }
    }
}
