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
    /// [0]: interpolated flag (0.0 = nearest/grid, 1.0 = bilinear/smooth)
    /// [1]: unused
    pub params: [f32; 2],
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

fn interpolate_lut(stops: &[(f32, [u8; 3])]) -> Vec<u8> {
    assert!(stops.len() >= 2);
    let n_segs = stops.len() - 1;
    let mut data = Vec::with_capacity(256 * 4);
    for i in 0..256 {
        let t = i as f32 / 255.0;
        let mut seg = n_segs - 1;
        for s in 0..n_segs {
            if t <= stops[s + 1].0 {
                seg = s;
                break;
            }
        }
        let t0 = stops[seg].0;
        let t1 = stops[seg + 1].0;
        let frac = if (t1 - t0).abs() < 1e-6 {
            0.0
        } else {
            ((t - t0) / (t1 - t0)).clamp(0.0, 1.0)
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

// --- Sequential colormaps (9 stops for accuracy) ---

fn lut_viridis() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [68, 1, 84]),
        (0.125, [72, 35, 116]),
        (0.250, [64, 67, 135]),
        (0.375, [52, 94, 141]),
        (0.500, [33, 145, 140]),
        (0.625, [53, 183, 121]),
        (0.750, [109, 205, 89]),
        (0.875, [180, 222, 44]),
        (1.000, [253, 231, 37]),
    ])
}

fn lut_plasma() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [13, 8, 135]),
        (0.125, [75, 3, 161]),
        (0.250, [126, 3, 168]),
        (0.375, [168, 34, 150]),
        (0.500, [204, 71, 120]),
        (0.625, [230, 111, 82]),
        (0.750, [248, 149, 64]),
        (0.875, [249, 199, 42]),
        (1.000, [240, 249, 33]),
    ])
}

fn lut_inferno() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [0, 0, 4]),
        (0.125, [40, 11, 84]),
        (0.250, [101, 21, 110]),
        (0.375, [159, 42, 99]),
        (0.500, [212, 72, 66]),
        (0.625, [245, 125, 21]),
        (0.750, [250, 175, 8]),
        (0.875, [245, 224, 76]),
        (1.000, [252, 255, 164]),
    ])
}

fn lut_magma() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [0, 0, 4]),
        (0.125, [28, 16, 68]),
        (0.250, [79, 18, 123]),
        (0.375, [136, 34, 142]),
        (0.500, [183, 54, 121]),
        (0.625, [227, 89, 101]),
        (0.750, [249, 142, 109]),
        (0.875, [254, 205, 170]),
        (1.000, [252, 253, 191]),
    ])
}

fn lut_cividis() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [0, 32, 77]),
        (0.125, [0, 56, 108]),
        (0.250, [59, 77, 107]),
        (0.375, [95, 97, 104]),
        (0.500, [123, 118, 105]),
        (0.625, [153, 140, 96]),
        (0.750, [185, 163, 78]),
        (0.875, [218, 189, 49]),
        (1.000, [253, 216, 2]),
    ])
}

fn lut_turbo() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [48, 18, 59]),
        (0.100, [70, 68, 202]),
        (0.200, [62, 131, 252]),
        (0.300, [18, 190, 209]),
        (0.400, [17, 232, 150]),
        (0.500, [83, 253, 74]),
        (0.600, [177, 237, 17]),
        (0.700, [237, 195, 11]),
        (0.800, [252, 136, 12]),
        (0.900, [222, 63, 10]),
        (1.000, [122, 4, 3]),
    ])
}

// --- Diverging colormaps ---

fn lut_rdbu_r() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [5, 48, 97]),
        (0.125, [33, 102, 172]),
        (0.250, [67, 147, 195]),
        (0.375, [146, 197, 222]),
        (0.500, [247, 247, 247]),
        (0.625, [244, 165, 130]),
        (0.750, [214, 96, 77]),
        (0.875, [178, 24, 43]),
        (1.000, [103, 0, 31]),
    ])
}

fn lut_coolwarm() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [59, 76, 192]),
        (0.125, [98, 130, 234]),
        (0.250, [141, 176, 254]),
        (0.375, [184, 208, 249]),
        (0.500, [221, 221, 221]),
        (0.625, [245, 186, 152]),
        (0.750, [238, 138, 105]),
        (0.875, [213, 75, 60]),
        (1.000, [180, 4, 38]),
    ])
}

fn lut_spectral() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [158, 1, 66]),
        (0.125, [213, 62, 79]),
        (0.250, [244, 109, 67]),
        (0.375, [253, 174, 97]),
        (0.500, [255, 255, 191]),
        (0.625, [171, 221, 164]),
        (0.750, [102, 194, 165]),
        (0.875, [50, 136, 189]),
        (1.000, [94, 79, 162]),
    ])
}

fn lut_brbg() -> Vec<u8> {
    interpolate_lut(&[
        (0.000, [84, 48, 5]),
        (0.125, [140, 81, 10]),
        (0.250, [191, 129, 45]),
        (0.375, [223, 194, 125]),
        (0.500, [245, 245, 245]),
        (0.625, [128, 205, 193]),
        (0.750, [53, 151, 143]),
        (0.875, [1, 102, 94]),
        (1.000, [0, 60, 48]),
    ])
}

/// CPU-side colormap: map a value in [min, max] to RGBA using a pre-generated LUT.
pub fn colormap_rgba_with_lut(value: f32, min: f32, max: f32, lut: &[u8]) -> [u8; 4] {
    let range = max - min;
    let normalized = if range.abs() < 1e-10 {
        0.5
    } else {
        ((value - min) / range).clamp(0.0, 1.0)
    };

    let idx = (normalized * 255.0) as usize;
    let base = idx * 4;
    [lut[base], lut[base + 1], lut[base + 2], lut[base + 3]]
}

/// Generate a colormap LUT for the given colormap selection.
pub fn colormap_lut(colormap: Colormap) -> Vec<u8> {
    match colormap {
        // Sequential
        Colormap::Viridis => lut_viridis(),
        Colormap::Plasma => lut_plasma(),
        Colormap::Inferno => lut_inferno(),
        Colormap::Magma => lut_magma(),
        Colormap::Cividis => lut_cividis(),
        Colormap::Turbo => lut_turbo(),
        // Diverging
        Colormap::RdBuR => lut_rdbu_r(),
        Colormap::Coolwarm => lut_coolwarm(),
        Colormap::Spectral => lut_spectral(),
        Colormap::BrBG => lut_brbg(),
    }
}
