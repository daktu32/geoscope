use crate::data::{DataStore, FieldData, GridInfo};

pub fn build_system_prompt() -> String {
    let lang = crate::i18n::current_lang();
    let mut prompt = build_base_system_prompt();
    match lang {
        crate::i18n::Lang::Ja => {
            prompt.push_str(r#"

## Language
Respond in Japanese (日本語) by default. Use natural, concise Japanese suitable for a researcher.
Technical terms (e.g., vorticity, divergence, Rossby wave) may be kept in English or written as カタカナ, whichever is more natural.
However, if the user writes in English or explicitly asks you to respond in English, follow their instruction and respond in English."#);
        }
        crate::i18n::Lang::En => {
            prompt.push_str(r#"

## Language
Respond in English by default.
However, if the user writes in Japanese or explicitly asks you to respond in Japanese, follow their instruction and respond in Japanese."#);
        }
    }
    prompt
}

fn build_base_system_prompt() -> String {
    r#"You are GeoScope Copilot, an AI assistant for geophysical fluid dynamics (GFD) data visualization.

## Your Role
- Explain physical phenomena visible in the current visualization
- Help users understand their data (vorticity, divergence, temperature, etc.)
- Suggest visualization improvements (colormaps, overlays, projections)
- Answer questions about GFD concepts (spectral methods, Rossby waves, etc.)

## Domain Knowledge
- Spherical harmonic spectral methods: data is represented as sum of Y^m_n basis functions
- Vorticity (zeta): curl of velocity, positive = cyclonic (NH), diverging colormap RdBu_r
- Divergence (D): div of velocity, related to vertical motion
- Geopotential (Phi): related to pressure surfaces
- Rossby-Haurwitz waves: large-scale wave patterns on rotating sphere
- Energy spectrum E(n): kinetic energy per total wavenumber n, typically follows n^(-3) in enstrophy cascade range
- Shallow water equations: zeta, D, Phi as prognostic variables on sphere

## Context You Receive
You receive detailed quantitative context including:
- **Field statistics**: mean, std dev, skewness, zonal mean profile, extrema locations
- **Spectral analysis**: E(n) energy spectrum, peak wavenumber, spectral slope
- **Temporal tendency**: time derivative statistics (if time dimension exists)

Use this data to make specific, quantitative observations about the physical phenomena.
For example: "The dominant wavenumber n=4 with E(4)=2.3e-3 is consistent with a Rossby-Haurwitz wave pattern."

## GeoScope Features You Can Reference
- Views: Globe (3D orthographic), Map (Equirect/Mollweide/Polar), Hovmoller, Spectrum, CrossSection, Profile
- Overlays: contour lines, vector arrows, streamlines, trajectory
- Colormaps: viridis, plasma, inferno (sequential), RdBu_r, coolwarm (diverging)
- Wavenumber filter: truncates spectral coefficients to show only large/small scales
- Code Panel: generates Python (xarray + cartopy) code from current view

Keep responses concise. Use markdown formatting. When suggesting actions, be specific about GeoScope settings."#
        .to_string()
}

/// Build context about current visualization state.
pub fn build_view_context(
    ui_state: &crate::ui::UiState,
    data_store: &DataStore,
) -> String {
    let mut ctx = String::new();
    ctx.push_str("\n## Current Visualization State\n");

    // Active file and variable
    if let Some(fi) = data_store.active_file {
        if let Some(file) = data_store.files.get(fi) {
            ctx.push_str(&format!("- File: {}\n", file.path));
            if let Some(vi) = file.selected_variable {
                if let Some(var) = file.variables.get(vi) {
                    let dim_names: Vec<&str> =
                        var.dimensions.iter().map(|(n, _)| n.as_str()).collect();
                    ctx.push_str(&format!(
                        "- Variable: {} ({})\n",
                        var.name,
                        dim_names.join(", ")
                    ));
                    if let Some(ref ln) = var.long_name {
                        ctx.push_str(&format!("- Long name: {}\n", ln));
                    }
                    if let Some(ref units) = var.units {
                        ctx.push_str(&format!("- Units: {}\n", units));
                    }
                }
                if let Some(ref field) = file.field_data {
                    ctx.push_str(&format!(
                        "- Value range: [{:.4e}, {:.4e}]\n",
                        field.min, field.max
                    ));
                    ctx.push_str(&format!(
                        "- Grid size: {} x {}\n",
                        field.width, field.height
                    ));

                    // 1. Field statistics
                    ctx.push_str(&build_field_statistics(field, &file.grid));

                    // 2. Spectral analysis
                    ctx.push_str(&build_spectral_context(field));

                    // 3. Temporal tendency
                    if let Some(n_time) = file.time_steps {
                        if n_time > 1 && vi < file.variables.len() {
                            ctx.push_str(&build_temporal_context(
                                &file.path,
                                &file.variables[vi],
                                ui_state.time_index,
                                ui_state.level_index,
                                field,
                            ));
                        }
                    }
                }
            }

            // Time info
            if let Some(n_time) = file.time_steps {
                ctx.push_str(&format!(
                    "- Time: step {} of {} (playing: {})\n",
                    ui_state.time_index, n_time, ui_state.playing
                ));
            }
        }
    }

    ctx.push_str(&format!("- View: {:?}\n", ui_state.view_mode));
    ctx.push_str(&format!("- Colormap: {:?}\n", ui_state.colormap));
    ctx.push_str(&format!("- Level index: {}\n", ui_state.level_index));

    if ui_state.contour_enabled {
        ctx.push_str("- Contours: enabled\n");
    }
    if ui_state.vector_overlay_enabled {
        ctx.push_str("- Vector overlay: enabled\n");
    }
    if ui_state.streamline_enabled {
        ctx.push_str("- Streamlines: enabled\n");
    }

    ctx
}

// ---------------------------------------------------------------------------
// 1. Field statistics
// ---------------------------------------------------------------------------

fn build_field_statistics(field: &FieldData, grid: &GridInfo) -> String {
    let n = field.values.len();
    if n == 0 {
        return String::new();
    }

    let mut ctx = String::new();
    ctx.push_str("\n### Field Statistics\n");

    // Mean
    let sum: f64 = field.values.iter().map(|&v| v as f64).sum();
    let mean = sum / n as f64;

    // Variance & std
    let var: f64 = field.values.iter().map(|&v| {
        let d = v as f64 - mean;
        d * d
    }).sum::<f64>() / n as f64;
    let std = var.sqrt();

    // Skewness
    let skew = if std > 1e-30 {
        let m3: f64 = field.values.iter().map(|&v| {
            let d = v as f64 - mean;
            d * d * d
        }).sum::<f64>() / n as f64;
        m3 / (std * std * std)
    } else {
        0.0
    };

    ctx.push_str(&format!("- Mean: {:.4e}, Std: {:.4e}, Skewness: {:.2}\n", mean, std, skew));

    // Fraction positive/negative (useful for diverging fields)
    let n_pos = field.values.iter().filter(|&&v| v > 0.0).count();
    let n_neg = field.values.iter().filter(|&&v| v < 0.0).count();
    ctx.push_str(&format!(
        "- Positive: {:.1}%, Negative: {:.1}%\n",
        100.0 * n_pos as f64 / n as f64,
        100.0 * n_neg as f64 / n as f64,
    ));

    // Location of extrema (lat/lon indices)
    let (max_idx, min_idx) = field.values.iter().enumerate().fold(
        (0usize, 0usize),
        |(max_i, min_i), (i, &v)| {
            let max_i = if v > field.values[max_i] { i } else { max_i };
            let min_i = if v < field.values[min_i] { i } else { min_i };
            (max_i, min_i)
        },
    );
    let max_lat_idx = max_idx / field.width;
    let max_lon_idx = max_idx % field.width;
    let min_lat_idx = min_idx / field.width;
    let min_lon_idx = min_idx % field.width;

    // Convert to approximate lat/lon degrees if grid info available
    if let (Some(lats), Some(lons)) = (&grid.lat, &grid.lon) {
        if max_lat_idx < lats.len() && max_lon_idx < lons.len() {
            ctx.push_str(&format!(
                "- Max location: lat={:.1}°, lon={:.1}°\n",
                lats[max_lat_idx], lons[max_lon_idx]
            ));
        }
        if min_lat_idx < lats.len() && min_lon_idx < lons.len() {
            ctx.push_str(&format!(
                "- Min location: lat={:.1}°, lon={:.1}°\n",
                lats[min_lat_idx], lons[min_lon_idx]
            ));
        }
    } else {
        ctx.push_str(&format!(
            "- Max at grid (lat_idx={}, lon_idx={})\n",
            max_lat_idx, max_lon_idx
        ));
        ctx.push_str(&format!(
            "- Min at grid (lat_idx={}, lon_idx={})\n",
            min_lat_idx, min_lon_idx
        ));
    }

    // Zonal mean profile (average over longitude for each latitude)
    let mut zonal_mean = Vec::with_capacity(field.height);
    for j in 0..field.height {
        let row_start = j * field.width;
        let row_end = row_start + field.width;
        let row_sum: f64 = field.values[row_start..row_end].iter().map(|&v| v as f64).sum();
        zonal_mean.push(row_sum / field.width as f64);
    }

    // Report zonal mean at key latitudes (equator, mid-lat, poles)
    // Sample ~5 representative latitudes
    let h = field.height;
    let sample_indices = if h >= 5 {
        vec![0, h / 4, h / 2, 3 * h / 4, h - 1]
    } else {
        (0..h).collect()
    };

    ctx.push_str("- Zonal mean profile (lat_idx → mean):");
    for &j in &sample_indices {
        if let Some(ref lats) = grid.lat {
            if j < lats.len() {
                ctx.push_str(&format!(" [{:.0}°: {:.3e}]", lats[j], zonal_mean[j]));
            }
        } else {
            ctx.push_str(&format!(" [j={}: {:.3e}]", j, zonal_mean[j]));
        }
    }
    ctx.push('\n');

    ctx
}

// ---------------------------------------------------------------------------
// 2. Spectral analysis
// ---------------------------------------------------------------------------

fn build_spectral_context(field: &FieldData) -> String {
    use crate::data::spectral_filter::{detect_n_trunc, is_lat_south_to_north};

    let n_lon = field.width;
    let n_lat = field.height;

    let n_trunc = match detect_n_trunc(n_lon, n_lat) {
        Some(n) => n,
        None => return String::new(),
    };

    let sphere = ispack_rs::Sphere::gaussian(n_trunc, n_lon, n_lat);

    // Convert to f64, ensure south-to-north ordering
    // (GeoScope stores data row-major [lat][lon], typically south-to-north for Gaussian grids)
    let lat_s2n = is_lat_south_to_north(None); // assume south-to-north
    let mut data_f64 = Vec::with_capacity(n_lon * n_lat);
    for j in 0..n_lat {
        let src_j = if lat_s2n { j } else { n_lat - 1 - j };
        let row_start = src_j * n_lon;
        for i in 0..n_lon {
            data_f64.push(field.values[row_start + i] as f64);
        }
    }

    let grid_field = sphere.grid_from_data(data_f64);
    let spec_field = grid_field.to_spectral();

    // Compute E(n) = sum_m |s^m_n|^2
    let mut energy = vec![0.0f64; n_trunc + 1];
    for n in 0..=n_trunc {
        for m in 0..=n {
            let c = spec_field.coefficient(n, m);
            let e = c.norm_sqr(); // |Re|^2 + |Im|^2
            // m=0 counts once, m>0 counts twice (conjugate symmetry)
            if m == 0 {
                energy[n] += e;
            } else {
                energy[n] += 2.0 * e;
            }
        }
    }

    let mut ctx = String::new();
    ctx.push_str("\n### Spectral Analysis (E(n) energy spectrum)\n");

    // Peak wavenumber (n >= 1)
    let (peak_n, peak_e) = energy.iter().enumerate()
        .skip(1) // skip n=0 (mean)
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));
    ctx.push_str(&format!("- Peak wavenumber: n={}, E({})={:.4e}\n", peak_n, peak_n, peak_e));

    // E(0) = mean component
    ctx.push_str(&format!("- E(0) (mean): {:.4e}\n", energy[0]));

    // Report E(n) for key wavenumbers
    let key_ns: Vec<usize> = vec![1, 2, 3, 4, 5, 10, 20]
        .into_iter()
        .filter(|&n| n <= n_trunc)
        .collect();
    ctx.push_str("- E(n) at key wavenumbers:");
    for &n in &key_ns {
        ctx.push_str(&format!(" n={}:{:.2e}", n, energy[n]));
    }
    ctx.push('\n');

    // Spectral slope (linear regression in log-log space for n in [5, n_trunc/2])
    let slope_range_start = 5.min(n_trunc);
    let slope_range_end = (n_trunc / 2).max(slope_range_start + 1);
    if slope_range_end > slope_range_start {
        let mut sum_x = 0.0f64;
        let mut sum_y = 0.0f64;
        let mut sum_xx = 0.0f64;
        let mut sum_xy = 0.0f64;
        let mut count = 0;
        for n in slope_range_start..=slope_range_end {
            if energy[n] > 0.0 {
                let x = (n as f64).ln();
                let y = energy[n].ln();
                sum_x += x;
                sum_y += y;
                sum_xx += x * x;
                sum_xy += x * y;
                count += 1;
            }
        }
        if count >= 2 {
            let slope = (count as f64 * sum_xy - sum_x * sum_y)
                / (count as f64 * sum_xx - sum_x * sum_x);
            ctx.push_str(&format!(
                "- Spectral slope (n={}..{}): {:.1} (n^-3 = enstrophy cascade, n^-5/3 = energy cascade)\n",
                slope_range_start, slope_range_end, slope
            ));
        }
    }

    // Total energy
    let total_e: f64 = energy.iter().skip(1).sum();
    ctx.push_str(&format!("- Total energy (n≥1): {:.4e}\n", total_e));

    // Large-scale vs small-scale partition
    let boundary_n = n_trunc / 3;
    if boundary_n >= 2 {
        let large_scale: f64 = energy[1..=boundary_n].iter().sum();
        let small_scale: f64 = energy[boundary_n + 1..].iter().sum();
        if total_e > 0.0 {
            ctx.push_str(&format!(
                "- Large-scale (n≤{}): {:.1}%, Small-scale (n>{}): {:.1}%\n",
                boundary_n,
                100.0 * large_scale / total_e,
                boundary_n,
                100.0 * small_scale / total_e,
            ));
        }
    }

    ctx
}

// ---------------------------------------------------------------------------
// 3. Temporal tendency
// ---------------------------------------------------------------------------

fn build_temporal_context(
    file_path: &str,
    var_info: &crate::data::VariableInfo,
    time_idx: usize,
    level_idx: usize,
    current_field: &FieldData,
) -> String {
    // Load adjacent time step to compute tendency
    let adjacent_idx = if time_idx > 0 {
        time_idx - 1
    } else {
        time_idx + 1
    };

    let adjacent_field = match load_field_slice(file_path, var_info, adjacent_idx, level_idx) {
        Some(f) => f,
        None => return String::new(),
    };

    if adjacent_field.len() != current_field.values.len() {
        return String::new();
    }

    let n = current_field.values.len();
    let sign = if time_idx > 0 { 1.0 } else { -1.0 }; // ensure tendency = current - previous

    // Compute tendency (difference)
    let tendency: Vec<f64> = current_field.values.iter()
        .zip(adjacent_field.iter())
        .map(|(&c, &a)| sign * (c as f64 - a as f64))
        .collect();

    let tend_mean: f64 = tendency.iter().sum::<f64>() / n as f64;
    let tend_var: f64 = tendency.iter().map(|&t| (t - tend_mean) * (t - tend_mean)).sum::<f64>() / n as f64;
    let tend_std = tend_var.sqrt();
    let tend_max = tendency.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let tend_min = tendency.iter().cloned().fold(f64::INFINITY, f64::min);

    // RMS tendency
    let rms = (tendency.iter().map(|&t| t * t).sum::<f64>() / n as f64).sqrt();

    // Relative change (tendency RMS / field RMS)
    let field_rms = (current_field.values.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / n as f64).sqrt();
    let relative_change = if field_rms > 1e-30 { rms / field_rms } else { 0.0 };

    let mut ctx = String::new();
    ctx.push_str("\n### Temporal Tendency (dt = 1 step)\n");
    ctx.push_str(&format!("- Tendency mean: {:.4e}\n", tend_mean));
    ctx.push_str(&format!("- Tendency std: {:.4e}\n", tend_std));
    ctx.push_str(&format!("- Tendency range: [{:.4e}, {:.4e}]\n", tend_min, tend_max));
    ctx.push_str(&format!("- RMS tendency: {:.4e}\n", rms));
    ctx.push_str(&format!(
        "- Relative change per step: {:.2e} ({:.2}%)\n",
        relative_change,
        relative_change * 100.0
    ));

    // Characterize: growing, decaying, or quasi-steady
    if relative_change < 1e-4 {
        ctx.push_str("- State: quasi-steady (very small tendency)\n");
    } else if tend_mean.abs() > 2.0 * tend_std && tend_mean > 0.0 {
        ctx.push_str("- State: systematically growing (positive mean tendency)\n");
    } else if tend_mean.abs() > 2.0 * tend_std && tend_mean < 0.0 {
        ctx.push_str("- State: systematically decaying (negative mean tendency)\n");
    } else {
        ctx.push_str("- State: dynamically active (spatially varying tendency)\n");
    }

    ctx
}

/// Load a raw 2D field slice from NetCDF without modifying DataStore.
fn load_field_slice(
    file_path: &str,
    var_info: &crate::data::VariableInfo,
    time_idx: usize,
    level_idx: usize,
) -> Option<Vec<f32>> {
    let file = netcdf::open(file_path).ok()?;
    let var = file.variable(&var_info.name)?;
    let dims = &var_info.dimensions;
    let ndim = dims.len();
    if ndim < 2 {
        return None;
    }

    let height = dims[ndim - 2].1;
    let width = dims[ndim - 1].1;

    let time_names: &[&str] = &["time", "t"];
    let level_names: &[&str] = &["level", "lev", "z", "sigma"];

    let time_pos = dims.iter().enumerate()
        .find(|(_, (name, _))| time_names.iter().any(|&c| c == name.to_ascii_lowercase()))
        .map(|(p, _)| p);
    let level_pos = dims.iter().enumerate()
        .find(|(_, (name, _))| level_names.iter().any(|&c| c == name.to_ascii_lowercase()))
        .map(|(p, _)| p);

    let mut start = vec![0usize; ndim];
    let mut count: Vec<usize> = dims.iter().map(|(_, s)| *s).collect();

    if let Some(tp) = time_pos {
        if time_idx < dims[tp].1 {
            start[tp] = time_idx;
            count[tp] = 1;
        } else {
            return None;
        }
    }
    if let Some(lp) = level_pos {
        start[lp] = level_idx;
        count[lp] = 1;
    }
    start[ndim - 2] = 0;
    count[ndim - 2] = height;
    start[ndim - 1] = 0;
    count[ndim - 1] = width;

    let extents: Vec<std::ops::Range<usize>> = start.iter()
        .zip(count.iter())
        .map(|(&s, &c)| s..s + c)
        .collect();

    let values_f64: Vec<f64> = var.get_values(extents.as_slice()).ok()?;
    Some(values_f64.iter().map(|&v| v as f32).collect())
}
