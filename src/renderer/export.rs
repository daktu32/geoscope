// renderer/export.rs — PNG export for field data

use std::path::Path;

use crate::data::FieldData;
use crate::renderer::common::colormap_rgba;
use crate::ui::Colormap;

/// Export a 2D field as a PNG image with the given colormap.
pub fn export_png(field: &FieldData, colormap: Colormap, path: &Path) -> Result<(), String> {
    let width = field.width as u32;
    let height = field.height as u32;

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for &value in &field.values {
        let pixel = colormap_rgba(value, field.min, field.max, colormap);
        rgba.extend_from_slice(&pixel);
    }

    image::save_buffer(path, &rgba, width, height, image::ColorType::Rgba8)
        .map_err(|e| format!("Failed to save PNG: {e}"))
}
