// renderer/vector_overlay.rs — Vector field overlay (wind arrows) on Globe/Map views

use crate::data::VectorFieldData;

/// A cached arrow: start and end positions on screen.
struct CachedArrow {
    start: egui::Pos2,
    end: egui::Pos2,
    head_len: f32,
    angle: f32,
}

#[derive(Clone, PartialEq)]
struct MapCacheKey {
    density: usize,
    scale_bits: u32,
    pan_x_bits: u32,
    pan_y_bits: u32,
    zoom_bits: u32,
    rect: [u32; 4],
}

#[derive(Clone, PartialEq)]
struct GlobeCacheKey {
    density: usize,
    scale_bits: u32,
    view_proj: [[u32; 4]; 4],
    rect: [u32; 4],
}

fn f32_to_bits(v: f32) -> u32 { v.to_bits() }

fn rect_bits(r: egui::Rect) -> [u32; 4] {
    [f32_to_bits(r.min.x), f32_to_bits(r.min.y), f32_to_bits(r.max.x), f32_to_bits(r.max.y)]
}

fn mat4_bits(m: &[[f32; 4]; 4]) -> [[u32; 4]; 4] {
    [
        [m[0][0].to_bits(), m[0][1].to_bits(), m[0][2].to_bits(), m[0][3].to_bits()],
        [m[1][0].to_bits(), m[1][1].to_bits(), m[1][2].to_bits(), m[1][3].to_bits()],
        [m[2][0].to_bits(), m[2][1].to_bits(), m[2][2].to_bits(), m[2][3].to_bits()],
        [m[3][0].to_bits(), m[3][1].to_bits(), m[3][2].to_bits(), m[3][3].to_bits()],
    ]
}

/// CPU-side vector overlay renderer using egui painter line drawing.
/// Arrow screen positions are cached and only recomputed when view parameters change.
pub struct VectorOverlay {
    data: Option<VectorFieldData>,
    pub density: usize,
    pub scale: f32,
    map_cache: Vec<CachedArrow>,
    map_cache_key: Option<MapCacheKey>,
    globe_cache: Vec<CachedArrow>,
    globe_cache_key: Option<GlobeCacheKey>,
}

impl VectorOverlay {
    pub fn new() -> Self {
        Self {
            data: None,
            density: 8,
            scale: 1.0,
            map_cache: Vec::new(),
            map_cache_key: None,
            globe_cache: Vec::new(),
            globe_cache_key: None,
        }
    }

    pub fn set_data(&mut self, data: VectorFieldData) {
        self.data = Some(data);
        self.map_cache_key = None;
        self.globe_cache_key = None;
    }

    pub fn clear(&mut self) {
        self.data = None;
        self.map_cache.clear();
        self.map_cache_key = None;
        self.globe_cache.clear();
        self.globe_cache_key = None;
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    /// Paint arrows on the equirectangular map view.
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

        let key = MapCacheKey {
            density: self.density,
            scale_bits: f32_to_bits(self.scale),
            pan_x_bits: f32_to_bits(pan_x),
            pan_y_bits: f32_to_bits(pan_y),
            zoom_bits: f32_to_bits(zoom),
            rect: rect_bits(plot_rect),
        };

        if self.map_cache_key.as_ref() != Some(&key) {
            self.map_cache = Self::compute_map_arrows(data, self.density, self.scale, plot_rect, pan_x, pan_y, zoom);
            self.map_cache_key = Some(key);
        }

        let stroke = egui::Stroke::new(1.2, egui::Color32::from_rgba_premultiplied(255, 255, 255, 200));
        Self::draw_arrows(painter, &self.map_cache, stroke);
    }

    /// Paint arrows on the globe view.
    pub fn paint_on_globe(
        &mut self,
        painter: &egui::Painter,
        plot_rect: egui::Rect,
        view: &[[f32; 4]; 4],
        view_proj: &[[f32; 4]; 4],
    ) {
        let Some(data) = &self.data else { return };
        if data.max_magnitude < 1e-20 {
            return;
        }

        let key = GlobeCacheKey {
            density: self.density,
            scale_bits: f32_to_bits(self.scale),
            view_proj: mat4_bits(view_proj),
            rect: rect_bits(plot_rect),
        };

        if self.globe_cache_key.as_ref() != Some(&key) {
            self.globe_cache = Self::compute_globe_arrows(data, self.density, self.scale, plot_rect, view, view_proj);
            self.globe_cache_key = Some(key);
        }

        let stroke = egui::Stroke::new(1.2, egui::Color32::from_rgba_premultiplied(255, 255, 255, 200));
        Self::draw_arrows(painter, &self.globe_cache, stroke);
    }

    fn draw_arrows(painter: &egui::Painter, arrows: &[CachedArrow], stroke: egui::Stroke) {
        for a in arrows {
            painter.line_segment([a.start, a.end], stroke);
            if a.head_len > 0.0 {
                let ha1 = a.angle + 2.5;
                let ha2 = a.angle - 2.5;
                let h1 = egui::pos2(a.end.x + ha1.cos() * a.head_len, a.end.y + ha1.sin() * a.head_len);
                let h2 = egui::pos2(a.end.x + ha2.cos() * a.head_len, a.end.y + ha2.sin() * a.head_len);
                painter.line_segment([a.end, h1], stroke);
                painter.line_segment([a.end, h2], stroke);
            }
        }
    }

    fn compute_map_arrows(
        data: &VectorFieldData,
        density: usize,
        scale: f32,
        plot_rect: egui::Rect,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) -> Vec<CachedArrow> {
        let mut arrows = Vec::new();
        let aspect = plot_rect.width() / plot_rect.height().max(1.0);
        let (sx, sy) = if aspect > 1.0 {
            (zoom / aspect, zoom)
        } else {
            (zoom, zoom * aspect)
        };
        let base_arrow_len = plot_rect.width().min(plot_rect.height()) * 0.03 * scale;

        for j in (0..data.height).step_by(density) {
            for i in (0..data.width).step_by(density) {
                let idx = j * data.width + i;
                let u = data.u_values[idx];
                let v = data.v_values[idx];
                let mag = (u * u + v * v).sqrt();
                if mag < data.max_magnitude * 0.01 {
                    continue;
                }

                let ndc_x = -1.0 + 2.0 * (i as f32 / data.width as f32);
                let ndc_y = 1.0 - 2.0 * (j as f32 / data.height as f32);
                let screen_x = (ndc_x * sx - pan_x * sx) * 0.5 + 0.5;
                let screen_y = (-ndc_y * sy - pan_y * sy) * 0.5 + 0.5;
                let px = plot_rect.min.x + screen_x * plot_rect.width();
                let py = plot_rect.min.y + screen_y * plot_rect.height();

                if !plot_rect.contains(egui::pos2(px, py)) {
                    continue;
                }

                let norm_mag = mag / data.max_magnitude;
                let arrow_len = base_arrow_len * norm_mag;
                let dx = (u / mag) * arrow_len;
                let dy = -(v / mag) * arrow_len;

                let start = egui::pos2(px - dx * 0.5, py - dy * 0.5);
                let end = egui::pos2(px + dx * 0.5, py + dy * 0.5);
                let angle = dy.atan2(dx);
                let head_len = if arrow_len > 3.0 { arrow_len * 0.3 } else { 0.0 };

                arrows.push(CachedArrow { start, end, head_len, angle });
            }
        }
        arrows
    }

    fn compute_globe_arrows(
        data: &VectorFieldData,
        density: usize,
        scale: f32,
        plot_rect: egui::Rect,
        view: &[[f32; 4]; 4],
        view_proj: &[[f32; 4]; 4],
    ) -> Vec<CachedArrow> {
        let mut arrows = Vec::new();
        let base_arrow_len = plot_rect.width().min(plot_rect.height()) * 0.025 * scale;

        for j in (0..data.height).step_by(density) {
            for i in (0..data.width).step_by(density) {
                let idx = j * data.width + i;
                let u_val = data.u_values[idx];
                let v_val = data.v_values[idx];
                let mag = (u_val * u_val + v_val * v_val).sqrt();
                if mag < data.max_magnitude * 0.01 {
                    continue;
                }

                let u_frac = i as f32 / data.width as f32;
                let v_frac = j as f32 / data.height as f32;
                let theta = 2.0 * std::f32::consts::PI * u_frac;
                let phi = std::f32::consts::PI * v_frac;

                let (sin_phi, cos_phi) = phi.sin_cos();
                let (sin_theta, cos_theta) = theta.sin_cos();

                let pos = [sin_phi * cos_theta, cos_phi, sin_phi * sin_theta];

                // Backface check
                let vz = view[2][0] * pos[0] + view[2][1] * pos[1] + view[2][2] * pos[2];
                if vz < 0.0 {
                    continue;
                }

                let clip = mat4_mul_vec4(view_proj, [pos[0], pos[1], pos[2], 1.0]);
                if clip[3].abs() < 1e-6 {
                    continue;
                }
                let ndc_x = clip[0] / clip[3];
                let ndc_y = clip[1] / clip[3];

                let px = plot_rect.min.x + (ndc_x + 1.0) * 0.5 * plot_rect.width();
                let py = plot_rect.min.y + (1.0 - ndc_y) * 0.5 * plot_rect.height();

                if !plot_rect.contains(egui::pos2(px, py)) {
                    continue;
                }

                // Tangent vectors
                let east = [-sin_theta, 0.0, cos_theta];
                let north = [-cos_phi * cos_theta, sin_phi, -cos_phi * sin_theta];

                let scale_fac = 0.02;
                let offset_pos = [
                    pos[0] + scale_fac * (u_val * east[0] + v_val * north[0]) / mag,
                    pos[1] + scale_fac * (u_val * east[1] + v_val * north[1]) / mag,
                    pos[2] + scale_fac * (u_val * east[2] + v_val * north[2]) / mag,
                ];

                let clip2 = mat4_mul_vec4(view_proj, [offset_pos[0], offset_pos[1], offset_pos[2], 1.0]);
                if clip2[3].abs() < 1e-6 {
                    continue;
                }
                let px2 = plot_rect.min.x + (clip2[0] / clip2[3] + 1.0) * 0.5 * plot_rect.width();
                let py2 = plot_rect.min.y + (1.0 - clip2[1] / clip2[3]) * 0.5 * plot_rect.height();

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
                let angle = dy.atan2(dx);
                let head_len = if arrow_len > 3.0 { arrow_len * 0.3 } else { 0.0 };

                arrows.push(CachedArrow { start, end, head_len, angle });
            }
        }
        arrows
    }
}

fn mat4_mul_vec4(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2] + m[0][3] * v[3],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2] + m[1][3] * v[3],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2] + m[2][3] * v[3],
        m[3][0] * v[0] + m[3][1] * v[1] + m[3][2] * v[2] + m[3][3] * v[3],
    ]
}
