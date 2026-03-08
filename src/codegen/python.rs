// codegen/python.rs — Generate Python (xarray + cartopy + matplotlib) code from UI state

use crate::data::DataStore;
use crate::renderer::map::MapProjection;
use crate::ui::{Colormap, RangeMode, UiState, ViewMode};

/// Generate a Python script that reproduces the current visualization.
pub fn generate_python(ui_state: &UiState, data_store: &DataStore) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("import xarray as xr".to_string());
    lines.push("import matplotlib.pyplot as plt".to_string());

    let needs_cartopy = matches!(ui_state.view_mode, ViewMode::Globe | ViewMode::Map);
    if needs_cartopy {
        lines.push("import cartopy.crs as ccrs".to_string());
    }
    lines.push(String::new());

    // Open dataset
    let file_path = data_store
        .active_file
        .and_then(|fi| data_store.files.get(fi))
        .map(|f| f.path.clone())
        .unwrap_or_else(|| "path/to/data.nc".to_string());

    lines.push(format!("ds = xr.open_dataset(\"{}\")", file_path));

    // Select variable
    let (var_name, has_time, has_level) = if let Some(fi) = data_store.active_file {
        if let Some(file) = data_store.files.get(fi) {
            if let Some(vi) = file.selected_variable {
                let var = &file.variables[vi];
                let has_t = var.dimensions.iter().any(|(n, _)| n == "time" || n == "t");
                let has_l = var.dimensions.iter().any(|(n, _)| {
                    matches!(
                        n.to_ascii_lowercase().as_str(),
                        "level" | "lev" | "z" | "sigma" | "depth" | "plev"
                    )
                });
                (var.name.clone(), has_t, has_l)
            } else {
                ("var_name".to_string(), false, false)
            }
        } else {
            ("var_name".to_string(), false, false)
        }
    } else {
        ("var_name".to_string(), false, false)
    };

    lines.push(format!("da = ds[\"{}\"]", var_name));

    // Slice selection
    let mut isel_parts = Vec::new();
    if has_time {
        isel_parts.push(format!("time={}", ui_state.time_index));
    }
    if has_level {
        isel_parts.push(format!("level={}", ui_state.level_index));
    }
    if !isel_parts.is_empty() {
        lines.push(format!("da = da.isel({})", isel_parts.join(", ")));
    }

    lines.push(String::new());

    // Colormap
    let cmap_name = colormap_to_matplotlib(ui_state.colormap);

    match ui_state.view_mode {
        ViewMode::Globe => {
            lines.push("# Globe (Orthographic projection)".to_string());
            lines.push("fig = plt.figure(figsize=(8, 8))".to_string());
            lines.push(
                "ax = fig.add_subplot(111, projection=ccrs.Orthographic(0, 0))".to_string(),
            );
            lines.push("ax.set_global()".to_string());
            append_pcolormesh(&mut lines, cmap_name, ui_state);
            lines.push("ax.coastlines()".to_string());
        }
        ViewMode::Map => {
            let proj_str = match ui_state.map_projection {
                MapProjection::Equirectangular => "ccrs.PlateCarree()".to_string(),
                MapProjection::Mollweide => "ccrs.Mollweide()".to_string(),
                MapProjection::PolarNorth => {
                    "ccrs.NorthPolarStereo()".to_string()
                }
                MapProjection::PolarSouth => {
                    "ccrs.SouthPolarStereo()".to_string()
                }
            };
            let proj_name = match ui_state.map_projection {
                MapProjection::Equirectangular => "Equirectangular (PlateCarree)",
                MapProjection::Mollweide => "Mollweide",
                MapProjection::PolarNorth => "North Polar Stereographic",
                MapProjection::PolarSouth => "South Polar Stereographic",
            };
            lines.push(format!("# Map ({} projection)", proj_name));
            lines.push("fig = plt.figure(figsize=(12, 6))".to_string());
            lines.push(format!(
                "ax = fig.add_subplot(111, projection={})",
                proj_str
            ));
            lines.push("ax.set_global()".to_string());
            append_pcolormesh(&mut lines, cmap_name, ui_state);
            lines.push("ax.coastlines()".to_string());
        }
        ViewMode::Hovmoller => {
            lines.push("# Hovmoller diagram".to_string());
            lines.push("fig, ax = plt.subplots(figsize=(12, 6))".to_string());
            lines.push(format!(
                "da.plot(ax=ax, cmap=\"{}\")",
                cmap_name
            ));
            lines.push("ax.set_xlabel(\"Longitude\")".to_string());
            lines.push("ax.set_ylabel(\"Time\")".to_string());
        }
        _ => {
            lines.push("fig, ax = plt.subplots(figsize=(10, 6))".to_string());
            lines.push(format!("da.plot(ax=ax, cmap=\"{}\")", cmap_name));
        }
    }

    // Overlays
    if ui_state.contour_enabled
        && matches!(ui_state.view_mode, ViewMode::Globe | ViewMode::Map)
    {
        lines.push(String::new());
        lines.push("# Contour overlay".to_string());
        lines.push(format!(
            "da.plot.contour(ax=ax, levels={}, colors=\"k\", linewidths=0.5, transform=ccrs.PlateCarree())",
            ui_state.contour_levels
        ));
    }

    if ui_state.vector_overlay_enabled
        && matches!(ui_state.view_mode, ViewMode::Globe | ViewMode::Map)
    {
        if let Some(fi) = data_store.active_file {
            if let Some(file) = data_store.files.get(fi) {
                let u_name = ui_state
                    .vector_u_var
                    .and_then(|i| file.variables.get(i))
                    .map(|v| v.name.as_str())
                    .unwrap_or("u");
                let v_name = ui_state
                    .vector_v_var
                    .and_then(|i| file.variables.get(i))
                    .map(|v| v.name.as_str())
                    .unwrap_or("v");
                lines.push(String::new());
                lines.push("# Vector overlay".to_string());
                lines.push(format!("u = ds[\"{}\"]", u_name));
                lines.push(format!("v = ds[\"{}\"]", v_name));
                if has_time {
                    lines.push(format!(
                        "u = u.isel(time={})",
                        ui_state.time_index
                    ));
                    lines.push(format!(
                        "v = v.isel(time={})",
                        ui_state.time_index
                    ));
                }
                if has_level {
                    lines.push(format!(
                        "u = u.isel(level={})",
                        ui_state.level_index
                    ));
                    lines.push(format!(
                        "v = v.isel(level={})",
                        ui_state.level_index
                    ));
                }
                let skip = (20 / ui_state.vector_density).max(1);
                lines.push(format!(
                    "ax.quiver(u.lon[::{}], u.lat[::{}], u[::{},::{} ], v[::{},::{} ], transform=ccrs.PlateCarree())",
                    skip, skip, skip, skip, skip, skip
                ));
            }
        }
    }

    lines.push(String::new());
    lines.push(format!("plt.title(\"{}\")", var_name));
    lines.push("plt.tight_layout()".to_string());
    lines.push("plt.show()".to_string());

    lines.join("\n")
}

fn append_pcolormesh(lines: &mut Vec<String>, cmap_name: &str, ui_state: &UiState) {
    let mut kwargs = vec![
        "ax=ax".to_string(),
        format!("cmap=\"{}\"", cmap_name),
        "transform=ccrs.PlateCarree()".to_string(),
    ];

    match ui_state.range_mode {
        RangeMode::Manual => {
            kwargs.push(format!("vmin={}", ui_state.manual_min));
            kwargs.push(format!("vmax={}", ui_state.manual_max));
        }
        _ => {}
    }

    lines.push(format!("da.plot.pcolormesh({})", kwargs.join(", ")));
}

fn colormap_to_matplotlib(cm: Colormap) -> &'static str {
    match cm {
        Colormap::Viridis => "viridis",
        Colormap::Plasma => "plasma",
        Colormap::Inferno => "inferno",
        Colormap::Magma => "magma",
        Colormap::Cividis => "cividis",
        Colormap::Turbo => "turbo",
        Colormap::RdBuR => "RdBu_r",
        Colormap::Coolwarm => "coolwarm",
        Colormap::Spectral => "Spectral",
        Colormap::BrBG => "BrBG",
    }
}
