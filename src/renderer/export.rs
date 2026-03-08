// renderer/export.rs — PNG export for field data

use std::path::Path;

use crate::data::FieldData;
use crate::renderer::common::{colormap_lut, colormap_rgba_with_lut};
use crate::ui::Colormap;

/// Export settings.
#[derive(Debug, Clone)]
pub struct ExportSettings {
    pub scale: u32,           // 1, 2, or 4
    pub colorbar: bool,       // include colorbar
    pub title: String,        // title text (empty = no title)
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            scale: 2,
            colorbar: true,
            title: String::new(),
        }
    }
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
    let s = settings.scale;
    let data_w = field.width as u32 * s;
    let data_h = field.height as u32 * s;

    // Layout: optional title + data + optional colorbar
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
            let pixel = colormap_rgba_with_lut(value, display_min, display_max, &lut);
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
                let pixel = colormap_rgba_with_lut(val, display_min, display_max, &lut);
                let idx = ((y * total_w + x) * 4) as usize;
                if idx + 4 <= rgba.len() {
                    rgba[idx..idx + 4].copy_from_slice(&pixel);
                }
            }
        }
    }

    image::save_buffer(path, &rgba, total_w, total_h, image::ColorType::Rgba8)
        .map_err(|e| format!("Failed to save PNG: {e}"))
}

/// Simple export (backwards-compatible).
#[allow(dead_code)]
pub fn export_png(field: &FieldData, colormap: Colormap, path: &Path) -> Result<(), String> {
    export_png_with_settings(
        field,
        colormap,
        field.min,
        field.max,
        &ExportSettings { scale: 1, colorbar: false, title: String::new() },
        path,
    )
}
