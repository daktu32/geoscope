// renderer/common.rs — Shared types and utilities for all renderers

use crate::ui::Colormap;

// ---------------------------------------------------------------------------
// Vertex type shared across GPU renderers
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

// ---------------------------------------------------------------------------
// Camera uniform sent to the GPU
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub data_range: [f32; 2], // [min, max]
    pub _padding: [f32; 2],
}

// ---------------------------------------------------------------------------
// Matrix math
// ---------------------------------------------------------------------------

pub fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                out[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    out
}

pub fn identity_mat4() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn build_view_proj(
    cam_lon: f32,
    cam_lat: f32,
    zoom: f32,
    rect: egui::Rect,
) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
    let (sin_lon, cos_lon) = cam_lon.sin_cos();
    let (sin_lat, cos_lat) = cam_lat.sin_cos();

    let rot_y = [
        [cos_lon, 0.0, sin_lon, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [-sin_lon, 0.0, cos_lon, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let rot_x = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cos_lat, -sin_lat, 0.0],
        [0.0, sin_lat, cos_lat, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    let view = mat4_mul(&rot_x, &rot_y);

    let aspect = rect.width() / rect.height().max(1.0);
    let scale = zoom;
    let (sx, sy) = if aspect > 1.0 {
        (scale / aspect, scale)
    } else {
        (scale, scale * aspect)
    };

    let proj = [
        [sx, 0.0, 0.0, 0.0],
        [0.0, sy, 0.0, 0.0],
        [0.0, 0.0, 0.5, 0.5],
        [0.0, 0.0, 0.0, 1.0],
    ];

    (view, mat4_mul(&proj, &view))
}

// ---------------------------------------------------------------------------
// Colormap LUT generation (256 entries, RGBA8)
// ---------------------------------------------------------------------------

pub fn generate_viridis_lut() -> Vec<u8> {
    let stops: [(f32, [u8; 3]); 5] = [
        (0.0, [68, 1, 84]),
        (0.25, [59, 82, 139]),
        (0.5, [33, 145, 140]),
        (0.75, [94, 201, 98]),
        (1.0, [253, 231, 37]),
    ];
    interpolate_lut(&stops)
}

pub fn generate_rdbu_r_lut() -> Vec<u8> {
    let stops: [(f32, [u8; 3]); 5] = [
        (0.0, [5, 48, 97]),
        (0.25, [67, 147, 195]),
        (0.5, [247, 247, 247]),
        (0.75, [214, 96, 77]),
        (1.0, [178, 24, 43]),
    ];
    interpolate_lut(&stops)
}

pub fn interpolate_lut(stops: &[(f32, [u8; 3]); 5]) -> Vec<u8> {
    let mut data = Vec::with_capacity(256 * 4);
    for i in 0..256 {
        let t = i as f32 / 255.0;
        let mut seg = 0;
        for s in 0..4 {
            if t >= stops[s].0 && t <= stops[s + 1].0 {
                seg = s;
                break;
            }
        }
        let t0 = stops[seg].0;
        let t1 = stops[seg + 1].0;
        let frac = if (t1 - t0).abs() < 1e-6 {
            0.0
        } else {
            (t - t0) / (t1 - t0)
        };
        let c0 = stops[seg].1;
        let c1 = stops[seg + 1].1;
        let r = (c0[0] as f32 + (c1[0] as f32 - c0[0] as f32) * frac) as u8;
        let g = (c0[1] as f32 + (c1[1] as f32 - c0[1] as f32) * frac) as u8;
        let b = (c0[2] as f32 + (c1[2] as f32 - c0[2] as f32) * frac) as u8;
        data.extend_from_slice(&[r, g, b, 255]);
    }
    data
}

/// CPU-side colormap: map a value in [min, max] to RGBA using the given colormap.
pub fn colormap_rgba(value: f32, min: f32, max: f32, colormap: Colormap) -> [u8; 4] {
    let range = max - min;
    let normalized = if range.abs() < 1e-10 {
        0.5
    } else {
        ((value - min) / range).clamp(0.0, 1.0)
    };

    let lut = match colormap {
        Colormap::Viridis => generate_viridis_lut(),
        Colormap::RdBuR => generate_rdbu_r_lut(),
    };

    let idx = (normalized * 255.0) as usize;
    let base = idx * 4;
    [lut[base], lut[base + 1], lut[base + 2], lut[base + 3]]
}

/// Generate a colormap LUT for the given colormap selection.
pub fn colormap_lut(colormap: Colormap) -> Vec<u8> {
    match colormap {
        Colormap::Viridis => generate_viridis_lut(),
        Colormap::RdBuR => generate_rdbu_r_lut(),
    }
}
