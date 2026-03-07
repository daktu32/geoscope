// renderer/vector_overlay.rs — Vector field overlay (wind arrows) on Globe/Map views

use crate::data::VectorFieldData;

/// CPU-side vector overlay renderer using egui painter line drawing.
/// Draws arrows on top of Globe or Map views by sampling the grid at intervals.
pub struct VectorOverlay {
    data: Option<VectorFieldData>,
    pub density: usize,
    pub scale: f32,
}

impl VectorOverlay {
    pub fn new() -> Self {
        Self {
            data: None,
            density: 8,
            scale: 1.0,
        }
    }

    pub fn set_data(&mut self, data: VectorFieldData) {
        self.data = Some(data);
    }

    pub fn clear(&mut self) {
        self.data = None;
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    /// Paint arrows on the equirectangular map view.
    pub fn paint_on_map(
        &self,
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

        let arrow_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 200);
        let arrow_stroke = egui::Stroke::new(1.2, arrow_color);

        let aspect = plot_rect.width() / plot_rect.height().max(1.0);
        let (sx, sy) = if aspect > 1.0 {
            (zoom / aspect, zoom)
        } else {
            (zoom, zoom * aspect)
        };

        // Arrow length scaling: pixels per unit of max_magnitude
        let base_arrow_len = plot_rect.width().min(plot_rect.height()) * 0.03 * self.scale;

        for j in (0..data.height).step_by(self.density) {
            for i in (0..data.width).step_by(self.density) {
                let idx = j * data.width + i;
                let u = data.u_values[idx];
                let v = data.v_values[idx];
                let mag = (u * u + v * v).sqrt();
                if mag < data.max_magnitude * 0.01 {
                    continue;
                }

                // Grid position in NDC [-1, 1]
                let ndc_x = -1.0 + 2.0 * (i as f32 / data.width as f32);
                let ndc_y = 1.0 - 2.0 * (j as f32 / data.height as f32);

                // Apply pan + zoom (same transform as map.rs build_ortho_view_proj)
                let screen_x = (ndc_x * sx - pan_x * sx) * 0.5 + 0.5;
                let screen_y = (-ndc_y * sy - pan_y * sy) * 0.5 + 0.5;

                // Map to plot_rect
                let px = plot_rect.min.x + screen_x * plot_rect.width();
                let py = plot_rect.min.y + screen_y * plot_rect.height();

                if !plot_rect.contains(egui::pos2(px, py)) {
                    continue;
                }

                // Direction and length
                let norm_mag = mag / data.max_magnitude;
                let arrow_len = base_arrow_len * norm_mag;
                let dx = (u / mag) * arrow_len;
                let dy = -(v / mag) * arrow_len; // screen y is inverted

                let start = egui::pos2(px - dx * 0.5, py - dy * 0.5);
                let end = egui::pos2(px + dx * 0.5, py + dy * 0.5);

                // Arrow line
                painter.line_segment([start, end], arrow_stroke);

                // Arrowhead
                if arrow_len > 3.0 {
                    let head_len = arrow_len * 0.3;
                    let angle = dy.atan2(dx);
                    let ha1 = angle + 2.5;
                    let ha2 = angle - 2.5;
                    let h1 = egui::pos2(end.x + ha1.cos() * head_len, end.y + ha1.sin() * head_len);
                    let h2 = egui::pos2(end.x + ha2.cos() * head_len, end.y + ha2.sin() * head_len);
                    painter.line_segment([end, h1], arrow_stroke);
                    painter.line_segment([end, h2], arrow_stroke);
                }
            }
        }
    }

    /// Paint arrows on the globe view.
    pub fn paint_on_globe(
        &self,
        painter: &egui::Painter,
        plot_rect: egui::Rect,
        view: &[[f32; 4]; 4],
        view_proj: &[[f32; 4]; 4],
    ) {
        let Some(data) = &self.data else { return };
        if data.max_magnitude < 1e-20 {
            return;
        }

        let arrow_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 200);
        let arrow_stroke = egui::Stroke::new(1.2, arrow_color);
        let base_arrow_len = plot_rect.width().min(plot_rect.height()) * 0.025 * self.scale;

        for j in (0..data.height).step_by(self.density) {
            for i in (0..data.width).step_by(self.density) {
                let idx = j * data.width + i;
                let u_val = data.u_values[idx];
                let v_val = data.v_values[idx];
                let mag = (u_val * u_val + v_val * v_val).sqrt();
                if mag < data.max_magnitude * 0.01 {
                    continue;
                }

                // Grid to spherical coordinates
                let u_frac = i as f32 / data.width as f32;
                let v_frac = j as f32 / data.height as f32;
                let theta = 2.0 * std::f32::consts::PI * u_frac; // longitude
                let phi = std::f32::consts::PI * v_frac; // colatitude

                let sin_phi = phi.sin();
                let cos_phi = phi.cos();
                let sin_theta = theta.sin();
                let cos_theta = theta.cos();

                // Position on unit sphere
                let pos = [sin_phi * cos_theta, cos_phi, sin_phi * sin_theta];

                // Check if on front face: view-space z = dot(pos, row 2 of view)
                // WGSL does vec4*M where M_wgsl = M_rust^T, so view-z = dot(pos, view[2])
                let vz = view[2][0] * pos[0] + view[2][1] * pos[1] + view[2][2] * pos[2];
                if vz < 0.0 {
                    continue; // backface
                }

                // Project to clip space
                let clip = mat4_mul_vec4(view_proj, [pos[0], pos[1], pos[2], 1.0]);
                if clip[3].abs() < 1e-6 {
                    continue;
                }
                let ndc_x = clip[0] / clip[3];
                let ndc_y = clip[1] / clip[3];

                // NDC to screen
                let screen_x = (ndc_x + 1.0) * 0.5;
                let screen_y = (1.0 - ndc_y) * 0.5;
                let px = plot_rect.min.x + screen_x * plot_rect.width();
                let py = plot_rect.min.y + screen_y * plot_rect.height();

                if !plot_rect.contains(egui::pos2(px, py)) {
                    continue;
                }

                // Tangent vectors on sphere for east/north directions
                // East: d/d(theta) normalized
                let east = [-sin_theta, 0.0, cos_theta];
                // North: -d/d(phi) normalized (pointing toward north pole)
                let north = [-cos_phi * cos_theta, sin_phi, -cos_phi * sin_theta];

                // Offset position by u/v in tangent plane
                let scale_fac = 0.02; // small offset for direction
                let offset_pos = [
                    pos[0] + scale_fac * (u_val * east[0] + v_val * north[0]) / mag,
                    pos[1] + scale_fac * (u_val * east[1] + v_val * north[1]) / mag,
                    pos[2] + scale_fac * (u_val * east[2] + v_val * north[2]) / mag,
                ];

                let clip2 = mat4_mul_vec4(view_proj, [offset_pos[0], offset_pos[1], offset_pos[2], 1.0]);
                if clip2[3].abs() < 1e-6 {
                    continue;
                }
                let ndc2_x = clip2[0] / clip2[3];
                let ndc2_y = clip2[1] / clip2[3];
                let px2 = plot_rect.min.x + (ndc2_x + 1.0) * 0.5 * plot_rect.width();
                let py2 = plot_rect.min.y + (1.0 - ndc2_y) * 0.5 * plot_rect.height();

                // Direction on screen
                let dir_x = px2 - px;
                let dir_y = py2 - py;
                let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
                if dir_len < 0.5 {
                    continue;
                }

                let norm_mag = mag / data.max_magnitude;
                let arrow_len = base_arrow_len * norm_mag;
                let dx = (dir_x / dir_len) * arrow_len;
                let dy = (dir_y / dir_len) * arrow_len;

                let start = egui::pos2(px - dx * 0.5, py - dy * 0.5);
                let end = egui::pos2(px + dx * 0.5, py + dy * 0.5);

                painter.line_segment([start, end], arrow_stroke);

                if arrow_len > 3.0 {
                    let head_len = arrow_len * 0.3;
                    let angle = dy.atan2(dx);
                    let ha1 = angle + 2.5;
                    let ha2 = angle - 2.5;
                    let h1 = egui::pos2(end.x + ha1.cos() * head_len, end.y + ha1.sin() * head_len);
                    let h2 = egui::pos2(end.x + ha2.cos() * head_len, end.y + ha2.sin() * head_len);
                    painter.line_segment([end, h1], arrow_stroke);
                    painter.line_segment([end, h2], arrow_stroke);
                }
            }
        }
    }
}

/// Multiply matrix by vector: M_rust * v.
/// This matches the WGSL `vec4 * mat4x4` because WGSL reads columns from
/// Rust rows, effectively transposing: v * M_wgsl = v * M_rust^T = M_rust * v.
fn mat4_mul_vec4(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2] + m[0][3] * v[3],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2] + m[1][3] * v[3],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2] + m[2][3] * v[3],
        m[3][0] * v[0] + m[3][1] * v[1] + m[3][2] * v[2] + m[3][3] * v[3],
    ]
}
