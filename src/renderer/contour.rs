// renderer/contour.rs — Contour (isoline) overlay on Globe/Map views

use crate::data::FieldData;

/// A single contour line segment in UV coordinates.
struct ContourSegment {
    p0: (f32, f32), // (u, v) in [0,1]
    p1: (f32, f32),
    level_index: usize, // which contour level (0-based)
    value: f32,         // the threshold value for this contour
}

/// Cached screen-space contour lines.
struct CachedContour {
    start: egui::Pos2,
    end: egui::Pos2,
    level_index: usize,
    value: f32,
}

#[derive(Clone, PartialEq)]
struct ContourCacheKey {
    n_levels: usize,
    data_min_bits: u32,
    data_max_bits: u32,
    width: usize,
    height: usize,
    // View params
    pan_x_bits: u32,
    pan_y_bits: u32,
    zoom_bits: u32,
    rect: [u32; 4],
}

/// CPU-side contour overlay using Marching Squares + egui painter.
pub struct ContourOverlay {
    uv_segments: Vec<ContourSegment>,
    n_levels: usize,
    map_cache: Vec<CachedContour>,
    map_cache_key: Option<ContourCacheKey>,
    data_generation: u64,
    /// Contour line color (default: white with alpha 120).
    pub color: egui::Color32,
}

impl ContourOverlay {
    pub fn new() -> Self {
        Self {
            uv_segments: Vec::new(),
            n_levels: 0,
            map_cache: Vec::new(),
            map_cache_key: None,
            data_generation: 0,
            color: egui::Color32::from_rgba_premultiplied(255, 255, 255, 120),
        }
    }

    /// Recompute contour lines from field data.
    pub fn update_data(&mut self, data: &FieldData, n_levels: usize) {
        self.uv_segments = Self::marching_squares(data, n_levels);
        self.n_levels = n_levels;
        self.map_cache_key = None; // Invalidate screen cache
        self.data_generation += 1;
    }

    pub fn clear(&mut self) {
        self.uv_segments.clear();
        self.map_cache.clear();
        self.map_cache_key = None;
    }

    /// Paint contour lines on the equirectangular map view.
    pub fn paint_on_map(
        &mut self,
        painter: &egui::Painter,
        plot_rect: egui::Rect,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) {
        if self.uv_segments.is_empty() {
            return;
        }

        let key = ContourCacheKey {
            n_levels: 0,
            data_min_bits: 0,
            data_max_bits: 0,
            width: 0,
            height: 0,
            pan_x_bits: pan_x.to_bits(),
            pan_y_bits: pan_y.to_bits(),
            zoom_bits: zoom.to_bits(),
            rect: [
                plot_rect.min.x.to_bits(),
                plot_rect.min.y.to_bits(),
                plot_rect.max.x.to_bits(),
                plot_rect.max.y.to_bits(),
            ],
        };

        if self.map_cache_key.as_ref() != Some(&key) {
            self.map_cache = self.project_to_map(plot_rect, pan_x, pan_y, zoom);
            self.map_cache_key = Some(key);
        }

        let n = self.n_levels.max(1);
        for seg in &self.map_cache {
            let is_major = n > 1 && seg.level_index % 5 == 0;
            let (width, alpha_boost) = if is_major {
                (1.2, 40)
            } else {
                (0.6, 0)
            };
            let c = self.color;
            let a = (c.a() as u16 + alpha_boost as u16).min(255) as u8;
            let stroke_color = egui::Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), a);
            let stroke = egui::Stroke::new(width, stroke_color);
            painter.line_segment([seg.start, seg.end], stroke);
        }

        // Draw contour labels
        self.paint_labels(painter, plot_rect);
    }

    /// Paint contour value labels along contour lines.
    fn paint_labels(&self, painter: &egui::Painter, plot_rect: egui::Rect) {
        // Group segments by level_index and find good label positions
        // We only label major levels (every 5th) to avoid clutter
        let n = self.n_levels.max(1);
        let min_spacing_sq = 100.0 * 100.0; // minimum ~100px between labels of same level

        // Track label positions per level to enforce spacing
        let mut placed: Vec<Vec<egui::Pos2>> = vec![Vec::new(); n];

        for seg in &self.map_cache {
            if n > 1 && seg.level_index % 5 != 0 {
                continue; // only label major contours
            }

            let mid = egui::pos2(
                (seg.start.x + seg.end.x) * 0.5,
                (seg.start.y + seg.end.y) * 0.5,
            );
            if !plot_rect.contains(mid) {
                continue;
            }

            // Check the segment is roughly horizontal (good for readability)
            let dx = (seg.end.x - seg.start.x).abs();
            let dy = (seg.end.y - seg.start.y).abs();
            if dy > dx * 0.8 {
                continue; // too vertical
            }

            // Check spacing from existing labels of same level
            let li = seg.level_index;
            if li < placed.len() {
                let too_close = placed[li].iter().any(|p| {
                    let d = (p.x - mid.x) * (p.x - mid.x) + (p.y - mid.y) * (p.y - mid.y);
                    d < min_spacing_sq
                });
                if too_close {
                    continue;
                }
                placed[li].push(mid);
            }

            let label = format_compact(seg.value);
            let font = egui::FontId::monospace(9.0);
            let galley = painter.layout_no_wrap(label, font, egui::Color32::WHITE);

            // Semi-transparent background
            let text_rect = egui::Rect::from_min_size(
                egui::pos2(mid.x - galley.size().x * 0.5, mid.y - galley.size().y * 0.5),
                galley.size(),
            ).expand(1.0);
            painter.rect_filled(text_rect, 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 100));
            painter.galley(
                egui::pos2(text_rect.min.x + 1.0, text_rect.min.y + 1.0),
                galley,
                egui::Color32::WHITE,
            );
        }
    }

    /// Paint contour lines on the globe view.
    /// Uses the same coordinate system as the Globe mesh and VectorOverlay:
    /// UV → (theta, phi) → (sin_phi*cos_theta, cos_phi, sin_phi*sin_theta)
    /// and the shared view_proj matrix from build_view_proj.
    pub fn paint_on_globe(
        &mut self,
        painter: &egui::Painter,
        plot_rect: egui::Rect,
        view: &[[f32; 4]; 4],
        view_proj: &[[f32; 4]; 4],
    ) {
        if self.uv_segments.is_empty() {
            return;
        }

        let n = self.n_levels.max(1);

        let project = |u: f32, v: f32| -> Option<egui::Pos2> {
            // UV to spherical (same as Globe mesh: phi = PI * v, theta = TAU * u)
            let theta = u * std::f32::consts::TAU;
            let phi = v * std::f32::consts::PI;
            let (sin_phi, cos_phi) = phi.sin_cos();
            let (sin_theta, cos_theta) = theta.sin_cos();

            let pos = [sin_phi * cos_theta, cos_phi, sin_phi * sin_theta];

            // Back-face culling using view matrix
            let vz = view[2][0] * pos[0] + view[2][1] * pos[1] + view[2][2] * pos[2];
            if vz < 0.0 {
                return None;
            }

            // Project using view_proj matrix
            let w = view_proj[3][0] * pos[0] + view_proj[3][1] * pos[1] + view_proj[3][2] * pos[2] + view_proj[3][3];
            if w.abs() < 1e-6 {
                return None;
            }
            let ndc_x = (view_proj[0][0] * pos[0] + view_proj[0][1] * pos[1] + view_proj[0][2] * pos[2] + view_proj[0][3]) / w;
            let ndc_y = (view_proj[1][0] * pos[0] + view_proj[1][1] * pos[1] + view_proj[1][2] * pos[2] + view_proj[1][3]) / w;

            let sx = plot_rect.min.x + (ndc_x + 1.0) * 0.5 * plot_rect.width();
            let sy = plot_rect.min.y + (1.0 - ndc_y) * 0.5 * plot_rect.height();
            Some(egui::pos2(sx, sy))
        };

        for seg in &self.uv_segments {
            let Some(p0) = project(seg.p0.0, seg.p0.1) else { continue };
            let Some(p1) = project(seg.p1.0, seg.p1.1) else { continue };

            if !plot_rect.contains(p0) && !plot_rect.contains(p1) {
                continue;
            }

            let is_major = n > 1 && seg.level_index % 5 == 0;
            let (width, alpha_boost) = if is_major { (1.2, 40) } else { (0.6, 0) };
            let c = self.color;
            let a = (c.a() as u16 + alpha_boost as u16).min(255) as u8;
            let stroke_color = egui::Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), a);
            let stroke = egui::Stroke::new(width, stroke_color);

            painter.line_segment([p0, p1], stroke);
        }
    }

    fn project_to_map(
        &self,
        plot_rect: egui::Rect,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) -> Vec<CachedContour> {
        let aspect = plot_rect.width() / plot_rect.height().max(1.0);
        let (sx, sy) = if aspect > 1.0 {
            (zoom / aspect, zoom)
        } else {
            (zoom, zoom * aspect)
        };

        self.uv_segments
            .iter()
            .filter_map(|seg| {
                let to_screen = |u: f32, v: f32| -> egui::Pos2 {
                    let ndc_x = -1.0 + 2.0 * u;
                    let ndc_y = 1.0 - 2.0 * v;
                    let scr_x = (ndc_x * sx - pan_x * sx) * 0.5 + 0.5;
                    let scr_y = (-ndc_y * sy - pan_y * sy) * 0.5 + 0.5;
                    egui::pos2(
                        plot_rect.min.x + scr_x * plot_rect.width(),
                        plot_rect.min.y + scr_y * plot_rect.height(),
                    )
                };

                let p0 = to_screen(seg.p0.0, seg.p0.1);
                let p1 = to_screen(seg.p1.0, seg.p1.1);

                if plot_rect.contains(p0) || plot_rect.contains(p1) {
                    Some(CachedContour {
                        start: p0,
                        end: p1,
                        level_index: seg.level_index,
                        value: seg.value,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Marching Squares algorithm to extract contour line segments.
    fn marching_squares(data: &FieldData, n_levels: usize) -> Vec<ContourSegment> {
        if data.values.is_empty() || n_levels == 0 {
            return Vec::new();
        }

        let range = data.max - data.min;
        if range.abs() < 1e-20 {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let w = data.width;
        let h = data.height;

        for level_i in 1..n_levels {
            let threshold = data.min + range * (level_i as f32 / n_levels as f32);

            for j in 0..h - 1 {
                for i in 0..w - 1 {
                    let v00 = data.values[j * w + i];
                    let v10 = data.values[j * w + i + 1];
                    let v01 = data.values[(j + 1) * w + i];
                    let v11 = data.values[(j + 1) * w + i + 1];

                    let idx = ((v00 >= threshold) as u8)
                        | (((v10 >= threshold) as u8) << 1)
                        | (((v11 >= threshold) as u8) << 2)
                        | (((v01 >= threshold) as u8) << 3);

                    if idx == 0 || idx == 15 {
                        continue;
                    }

                    let u0 = i as f32 / w as f32;
                    let u1 = (i + 1) as f32 / w as f32;
                    let vv0 = j as f32 / h as f32;
                    let vv1 = (j + 1) as f32 / h as f32;

                    let interp = |a: f32, b: f32| -> f32 {
                        let d = b - a;
                        if d.abs() < 1e-10 { 0.5 } else { (threshold - a) / d }
                    };

                    // Edge midpoints (interpolated)
                    let top = (u0 + interp(v00, v10) * (u1 - u0), vv0);
                    let bottom = (u0 + interp(v01, v11) * (u1 - u0), vv1);
                    let left = (u0, vv0 + interp(v00, v01) * (vv1 - vv0));
                    let right = (u1, vv0 + interp(v10, v11) * (vv1 - vv0));

                    let add_seg = |segs: &mut Vec<ContourSegment>, a: (f32, f32), b: (f32, f32)| {
                        segs.push(ContourSegment {
                            p0: a,
                            p1: b,
                            level_index: level_i,
                            value: threshold,
                        });
                    };

                    match idx {
                        1 | 14 => add_seg(&mut segments, top, left),
                        2 | 13 => add_seg(&mut segments, top, right),
                        3 | 12 => add_seg(&mut segments, left, right),
                        4 | 11 => add_seg(&mut segments, right, bottom),
                        5 => {
                            add_seg(&mut segments, top, right);
                            add_seg(&mut segments, left, bottom);
                        }
                        6 | 9 => add_seg(&mut segments, top, bottom),
                        7 | 8 => add_seg(&mut segments, left, bottom),
                        10 => {
                            add_seg(&mut segments, top, left);
                            add_seg(&mut segments, right, bottom);
                        }
                        _ => {}
                    }
                }
            }
        }

        segments
    }
}

/// Format a float compactly for contour labels.
fn format_compact(v: f32) -> String {
    let abs = v.abs();
    if abs == 0.0 {
        "0".to_string()
    } else if abs >= 1e4 || abs < 1e-2 {
        format!("{:.1e}", v)
    } else if abs >= 100.0 {
        format!("{:.0}", v)
    } else if abs >= 1.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}
