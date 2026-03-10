// renderer/export.rs — PNG / GIF export for field data

use std::path::Path;

use crate::data::FieldData;
use crate::renderer::common::{colormap_lut, colormap_rgba_with_lut};
use crate::ui::Colormap;

/// Export format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Png,
    Gif,
}

/// Export settings.
#[derive(Debug, Clone)]
pub struct ExportSettings {
    pub scale: u32,           // 1, 2, or 4
    pub colorbar: bool,       // include colorbar
    pub title: String,        // title text (empty = no title)
    pub format: ExportFormat,
    pub gif_fps: u32,         // frames per second for GIF (1-30)
    pub publication: bool,    // publication-quality layout with axes and labels
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            scale: 2,
            colorbar: true,
            title: String::new(),
            format: ExportFormat::Png,
            gif_fps: 10,
            publication: false,
        }
    }
}

/// Render a single frame as RGBA buffer. Returns (rgba, width, height).
fn render_frame(
    field: &FieldData,
    lut: &[u8],
    display_min: f32,
    display_max: f32,
    settings: &ExportSettings,
) -> (Vec<u8>, u32, u32) {
    let s = settings.scale;
    let data_w = field.width as u32 * s;
    let data_h = field.height as u32 * s;

    let title_h: u32 = if settings.title.is_empty() { 0 } else { 28 * s };
    let bar_h: u32 = if settings.colorbar { 40 * s } else { 0 };
    let margin = 8 * s;
    let bar_label_w = if settings.colorbar { 60 * s } else { 0 };

    let total_w = data_w + if settings.colorbar { margin + bar_label_w } else { 0 };
    let total_h = title_h + data_h + if settings.colorbar { bar_h } else { 0 };

    let bg = [24u8, 24, 24, 255];
    let mut rgba = vec![0u8; (total_w * total_h * 4) as usize];

    // Fill background
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&bg);
    }

    // Draw data (nearest-neighbor upscale)
    for dy in 0..data_h {
        let src_y = (dy / s) as usize;
        for dx in 0..data_w {
            let src_x = (dx / s) as usize;
            let value = field.values[src_y * field.width + src_x];
            let pixel = colormap_rgba_with_lut(value, display_min, display_max, lut);
            let out_y = title_h + dy;
            let idx = ((out_y * total_w + dx) * 4) as usize;
            rgba[idx..idx + 4].copy_from_slice(&pixel);
        }
    }

    // Draw colorbar (horizontal, below data)
    if settings.colorbar {
        let bar_top = title_h + data_h + 4 * s;
        let bar_bottom = bar_top + 12 * s;
        let bar_left = 0u32;
        let bar_right = data_w;

        for y in bar_top..bar_bottom.min(total_h) {
            for x in bar_left..bar_right {
                let t = (x - bar_left) as f32 / (bar_right - bar_left) as f32;
                let val = display_min + t * (display_max - display_min);
                let pixel = colormap_rgba_with_lut(val, display_min, display_max, lut);
                let idx = ((y * total_w + x) * 4) as usize;
                if idx + 4 <= rgba.len() {
                    rgba[idx..idx + 4].copy_from_slice(&pixel);
                }
            }
        }
    }

    (rgba, total_w, total_h)
}

/// Export a 2D field as a PNG image with options.
pub fn export_png_with_settings(
    field: &FieldData,
    colormap: Colormap,
    display_min: f32,
    display_max: f32,
    settings: &ExportSettings,
    path: &Path,
) -> Result<(), String> {
    let lut = colormap_lut(colormap);
    let (rgba, total_w, total_h) = render_frame(field, &lut, display_min, display_max, settings);

    image::save_buffer(path, &rgba, total_w, total_h, image::ColorType::Rgba8)
        .map_err(|e| format!("Failed to save PNG: {e}"))
}

/// Export all time steps as an animated GIF.
pub fn export_gif(
    data_store: &mut crate::data::DataStore,
    file_idx: usize,
    var_idx: usize,
    level_idx: usize,
    settings: &ExportSettings,
    colormap: Colormap,
    range_mode: &crate::ui::RangeMode,
    manual_min: f32,
    manual_max: f32,
    global_range: Option<(f32, f32)>,
    path: &Path,
) -> Result<usize, String> {
    let n_time = data_store.files[file_idx]
        .time_steps
        .unwrap_or(1);

    if n_time == 0 {
        return Err("No time steps to export".to_string());
    }

    let lut = colormap_lut(colormap);

    // Determine the color range to use for all frames
    let (range_min, range_max) = match range_mode {
        crate::ui::RangeMode::Slice => {
            // For GIF, compute global range so all frames use the same scale
            data_store.compute_global_range(file_idx, var_idx)?
        }
        crate::ui::RangeMode::Global => {
            global_range.unwrap_or_else(|| {
                data_store
                    .compute_global_range(file_idx, var_idx)
                    .unwrap_or((0.0, 1.0))
            })
        }
        crate::ui::RangeMode::Manual => {
            if (manual_max - manual_min).abs() > f32::EPSILON {
                (manual_min, manual_max)
            } else {
                data_store.compute_global_range(file_idx, var_idx)?
            }
        }
    };

    // Save original time index to restore later
    let original_time = data_store.files[file_idx].current_time;
    let original_field = data_store.files[file_idx].field_data.clone();

    // Create GIF file
    let file = std::fs::File::create(path)
        .map_err(|e| format!("Failed to create GIF file: {e}"))?;

    // Render first frame to get dimensions
    data_store.load_field_at(file_idx, var_idx, 0, level_idx)?;
    let first_field = data_store.files[file_idx]
        .field_data
        .as_ref()
        .ok_or_else(|| "Failed to load first frame".to_string())?;
    let (_, frame_w, frame_h) = render_frame(first_field, &lut, range_min, range_max, settings);

    let mut encoder = gif::Encoder::new(file, frame_w as u16, frame_h as u16, &[])
        .map_err(|e| format!("Failed to create GIF encoder: {e}"))?;

    // Set infinite loop
    encoder
        .set_repeat(gif::Repeat::Infinite)
        .map_err(|e| format!("Failed to set GIF repeat: {e}"))?;

    // Frame delay in centiseconds (GIF uses 1/100s units)
    let delay = (100 / settings.gif_fps.max(1).min(30)) as u16;

    for t in 0..n_time {
        data_store.load_field_at(file_idx, var_idx, t, level_idx)?;
        let field = data_store.files[file_idx]
            .field_data
            .as_ref()
            .ok_or_else(|| format!("Failed to load frame {t}"))?;

        let (mut rgba, w, h) = render_frame(field, &lut, range_min, range_max, settings);

        let mut frame = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut rgba, 10);
        frame.delay = delay;

        encoder
            .write_frame(&frame)
            .map_err(|e| format!("Failed to write GIF frame {t}: {e}"))?;
    }

    // Restore original state
    data_store.files[file_idx].current_time = original_time;
    data_store.files[file_idx].field_data = original_field;

    Ok(n_time)
}

/// Simple export (backwards-compatible).
#[allow(dead_code)]
pub fn export_png(field: &FieldData, colormap: Colormap, path: &Path) -> Result<(), String> {
    export_png_with_settings(
        field,
        colormap,
        field.min,
        field.max,
        &ExportSettings { scale: 1, colorbar: false, title: String::new(), ..Default::default() },
        path,
    )
}

// ---------------------------------------------------------------------------
// Publication-quality export
// ---------------------------------------------------------------------------

use crate::renderer::bitmap_font;

fn draw_hline(buf: &mut [u8], buf_w: usize, x1: usize, x2: usize, y: usize, color: [u8; 4]) {
    for x in x1..=x2 {
        let idx = (y * buf_w + x) * 4;
        if idx + 4 <= buf.len() {
            buf[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

fn draw_vline(buf: &mut [u8], buf_w: usize, x: usize, y1: usize, y2: usize, color: [u8; 4]) {
    for y in y1..=y2 {
        let idx = (y * buf_w + x) * 4;
        if idx + 4 <= buf.len() {
            buf[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

fn draw_rect_outline(
    buf: &mut [u8],
    buf_w: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: [u8; 4],
) {
    if w == 0 || h == 0 {
        return;
    }
    draw_hline(buf, buf_w, x, x + w - 1, y, color);
    draw_hline(buf, buf_w, x, x + w - 1, y + h - 1, color);
    draw_vline(buf, buf_w, x, y, y + h - 1, color);
    draw_vline(buf, buf_w, x + w - 1, y, y + h - 1, color);
}

/// Format a floating-point value compactly for axis labels.
fn format_value(v: f32) -> String {
    let abs = v.abs();
    if abs == 0.0 {
        "0".to_string()
    } else if abs >= 1000.0 || abs < 0.01 {
        format!("{:.2e}", v)
    } else if abs >= 1.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.3}", v)
    }
}

/// Export a publication-quality PNG with axis labels, tick marks, and colorbar.
pub fn export_publication_png(
    field: &FieldData,
    colormap: Colormap,
    display_min: f32,
    display_max: f32,
    settings: &ExportSettings,
    path: &Path,
) -> Result<(), String> {
    let lut = colormap_lut(colormap);
    let s = settings.scale;
    let su = s as usize;
    let data_w = field.width as u32 * s;
    let data_h = field.height as u32 * s;

    // Layout margins (all multiplied by scale)
    let margin_left = 60 * su;
    let margin_right = 80 * su;
    let margin_top = if settings.title.is_empty() { 16 * su } else { 40 * su };
    let margin_bottom = 50 * su;
    let cbar_w = 20 * su;
    let cbar_gap = 10 * su;

    let total_w = margin_left + data_w as usize + margin_right;
    let total_h = margin_top + data_h as usize + margin_bottom;

    let bg = [24u8, 24, 24, 255];
    let fg = [220u8, 220, 220, 255];
    let tick_color = [180u8, 180, 180, 255];

    let mut rgba = vec![0u8; total_w * total_h * 4];

    // Fill background
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&bg);
    }

    // --- Title ---
    if !settings.title.is_empty() {
        let title_scale = s * 2;
        let tw = bitmap_font::text_width(&settings.title, title_scale);
        let tx = if tw < total_w { (total_w - tw) / 2 } else { margin_left };
        let ty = (margin_top - bitmap_font::text_height(title_scale)) / 2;
        bitmap_font::draw_text(&mut rgba, total_w, tx, ty, &settings.title, fg, title_scale);
    }

    // --- Data area ---
    let data_x0 = margin_left;
    let data_y0 = margin_top;

    for dy in 0..data_h {
        let src_y = (dy / s) as usize;
        for dx in 0..data_w {
            let src_x = (dx / s) as usize;
            let value = field.values[src_y * field.width + src_x];
            let pixel = colormap_rgba_with_lut(value, display_min, display_max, &lut);
            let px = data_x0 + dx as usize;
            let py = data_y0 + dy as usize;
            let idx = (py * total_w + px) * 4;
            if idx + 4 <= rgba.len() {
                rgba[idx..idx + 4].copy_from_slice(&pixel);
            }
        }
    }

    // --- Border around data ---
    draw_rect_outline(&mut rgba, total_w, data_x0, data_y0, data_w as usize, data_h as usize, fg);

    // --- X-axis ticks (longitude) ---
    let x_ticks: &[(f64, &str)] = &[
        (0.0, "0\u{00B0}"),
        (90.0, "90\u{00B0}"),
        (180.0, "180\u{00B0}"),
        (270.0, "270\u{00B0}"),
        (360.0, "360\u{00B0}"),
    ];
    let tick_len = 5 * su;
    let font_scale = s;

    for &(lon, label) in x_ticks {
        let frac = lon / 360.0;
        let px = data_x0 + (frac * data_w as f64) as usize;
        if px <= data_x0 + data_w as usize {
            let tick_y0 = data_y0 + data_h as usize;
            let tick_y1 = tick_y0 + tick_len;
            draw_vline(&mut rgba, total_w, px, tick_y0, tick_y1.min(total_h - 1), tick_color);

            let lw = bitmap_font::text_width(label, font_scale);
            let lx = px.saturating_sub(lw / 2);
            let ly = tick_y1 + 2 * su;
            bitmap_font::draw_text(&mut rgba, total_w, lx, ly, label, fg, font_scale);
        }
    }

    // X-axis label
    {
        let x_label = "Longitude";
        let lw = bitmap_font::text_width(x_label, font_scale);
        let lx = data_x0 + (data_w as usize).saturating_sub(lw) / 2;
        let ly = total_h - bitmap_font::text_height(font_scale) - 2 * su;
        bitmap_font::draw_text(&mut rgba, total_w, lx, ly, x_label, fg, font_scale);
    }

    // --- Y-axis ticks (latitude) ---
    let y_ticks: &[(f64, &str)] = &[
        (-90.0, "-90\u{00B0}"),
        (-45.0, "-45\u{00B0}"),
        (0.0, "0\u{00B0}"),
        (45.0, "45\u{00B0}"),
        (90.0, "90\u{00B0}"),
    ];

    for &(lat, label) in y_ticks {
        let frac = (90.0 - lat) / 180.0;
        let py = data_y0 + (frac * data_h as f64) as usize;
        if py >= data_y0 && py <= data_y0 + data_h as usize {
            let tick_x1 = data_x0;
            let tick_x0 = tick_x1.saturating_sub(tick_len);
            draw_hline(&mut rgba, total_w, tick_x0, tick_x1, py, tick_color);

            let lw = bitmap_font::text_width(label, font_scale);
            let lx = tick_x0.saturating_sub(lw + 2 * su);
            let ly = py.saturating_sub(bitmap_font::text_height(font_scale) / 2);
            bitmap_font::draw_text(&mut rgba, total_w, lx, ly, label, fg, font_scale);
        }
    }

    // Y-axis label (vertical)
    {
        let y_label = "Latitude";
        let label_h = bitmap_font::text_width(y_label, font_scale);
        let lx = 2 * su;
        let ly_center = data_y0 + data_h as usize / 2;
        let ly_bottom = ly_center + label_h / 2;
        bitmap_font::draw_text_vertical(&mut rgba, total_w, lx, ly_bottom, y_label, fg, font_scale);
    }

    // --- Colorbar (vertical, right of data) ---
    if settings.colorbar {
        let cbar_x0 = data_x0 + data_w as usize + cbar_gap;
        let cbar_y0 = data_y0;
        let cbar_h = data_h as usize;

        // Draw gradient
        for row in 0..cbar_h {
            let t = 1.0 - row as f32 / cbar_h as f32;
            let val = display_min + t * (display_max - display_min);
            let pixel = colormap_rgba_with_lut(val, display_min, display_max, &lut);
            for col in 0..cbar_w {
                let px = cbar_x0 + col;
                let py = cbar_y0 + row;
                let idx = (py * total_w + px) * 4;
                if idx + 4 <= rgba.len() {
                    rgba[idx..idx + 4].copy_from_slice(&pixel);
                }
            }
        }

        // Border around colorbar
        draw_rect_outline(&mut rgba, total_w, cbar_x0, cbar_y0, cbar_w, cbar_h, fg);

        // Max label (top)
        let max_label = format_value(display_max);
        let lx = cbar_x0 + cbar_w + 3 * su;
        let ly = cbar_y0;
        bitmap_font::draw_text(&mut rgba, total_w, lx, ly, &max_label, fg, font_scale);

        // Min label (bottom)
        let min_label = format_value(display_min);
        let ly = cbar_y0 + cbar_h - bitmap_font::text_height(font_scale);
        bitmap_font::draw_text(&mut rgba, total_w, lx, ly, &min_label, fg, font_scale);

        // Mid label
        let mid_val = (display_min + display_max) / 2.0;
        let mid_label = format_value(mid_val);
        let ly = cbar_y0 + cbar_h / 2 - bitmap_font::text_height(font_scale) / 2;
        bitmap_font::draw_text(&mut rgba, total_w, lx, ly, &mid_label, fg, font_scale);

        // Tick marks on colorbar
        for frac in &[0.0_f64, 0.25, 0.5, 0.75, 1.0] {
            let py = cbar_y0 + ((1.0 - frac) * cbar_h as f64) as usize;
            let tx0 = cbar_x0 + cbar_w;
            let tx1 = tx0 + 3 * su;
            draw_hline(&mut rgba, total_w, tx0, tx1.min(total_w - 1), py.min(total_h - 1), tick_color);
        }
    }

    image::save_buffer(path, &rgba, total_w as u32, total_h as u32, image::ColorType::Rgba8)
        .map_err(|e| format!("Failed to save PNG: {e}"))
}
