use ispack_rs::Sphere;

/// Apply wavenumber truncation filter to a 2D field.
///
/// Zeroes out all spherical harmonic coefficients with n > n_cutoff.
/// Input/output: row-major `[lat][lon]` grid data.
///
/// Returns `None` if the grid dimensions are incompatible with ispack
/// (e.g., non-Gaussian grid, odd n_lat, n_lon too small).
pub fn wavenumber_filter(
    values: &[f32],
    n_lon: usize,
    n_lat: usize,
    n_trunc: usize,
    n_cutoff: usize,
    lat_south_to_north: bool,
) -> Option<Vec<f32>> {
    if values.len() != n_lon * n_lat {
        return None;
    }
    if n_cutoff >= n_trunc {
        // No filtering needed
        return Some(values.to_vec());
    }
    if n_lat % 2 != 0 || n_lon < 2 * n_trunc + 2 {
        return None;
    }

    // Build Sphere
    let sphere = Sphere::gaussian(n_trunc, n_lon, n_lat);

    // Convert f32 -> f64, reorder rows if needed (ispack expects south-to-north)
    let mut data_f64 = Vec::with_capacity(n_lon * n_lat);
    for j in 0..n_lat {
        let src_j = if lat_south_to_north { j } else { n_lat - 1 - j };
        let row_start = src_j * n_lon;
        for i in 0..n_lon {
            data_f64.push(values[row_start + i] as f64);
        }
    }

    let grid_field = sphere.grid_from_data(data_f64);
    let mut spec_field = grid_field.to_spectral();

    // Zero out coefficients where n > n_cutoff
    for m in 0..=n_trunc {
        for n in m..=n_trunc {
            if n > n_cutoff {
                spec_field.set_coefficient(n, m, num_complex::Complex::new(0.0, 0.0));
            }
        }
    }

    let filtered_grid = spec_field.to_grid();
    let filtered_data = filtered_grid.data();

    // Convert f64 -> f32, reorder rows back if needed
    let mut result = vec![0.0f32; n_lon * n_lat];
    for j in 0..n_lat {
        let dst_j = if lat_south_to_north { j } else { n_lat - 1 - j };
        let src_start = j * n_lon;
        let dst_start = dst_j * n_lon;
        for i in 0..n_lon {
            result[dst_start + i] = filtered_data[src_start + i] as f32;
        }
    }

    Some(result)
}

/// Detect truncation wavenumber from grid dimensions.
/// For a Gaussian grid: n_trunc = n_lon / 3 (approximately).
/// Returns None if the grid is too small for spectral transforms.
pub fn detect_n_trunc(n_lon: usize, n_lat: usize) -> Option<usize> {
    if n_lat < 4 || n_lon < 6 || n_lat % 2 != 0 {
        return None;
    }
    // Standard rule: n_trunc = (n_lon - 2) / 2 gives maximum, but
    // the common convention is n_trunc ~ n_lon/3 for anti-aliased transforms.
    // We use the more conservative n_lat * 2 / 3, capped by (n_lon - 2) / 2.
    let from_lat = n_lat * 2 / 3;
    let from_lon = (n_lon - 2) / 2;
    let n_trunc = from_lat.min(from_lon);
    if n_trunc == 0 {
        return None;
    }
    // Validate: n_lon must be >= 2*n_trunc + 2 for ispack
    if n_lon < 2 * n_trunc + 2 {
        return None;
    }
    Some(n_trunc)
}

/// Detect whether latitudes run south-to-north (ascending) from grid info.
pub fn is_lat_south_to_north(lat_values: Option<&[f64]>) -> bool {
    match lat_values {
        Some(lats) if lats.len() >= 2 => lats[1] > lats[0],
        _ => true, // assume south-to-north if unknown
    }
}
