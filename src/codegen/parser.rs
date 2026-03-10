// codegen/parser.rs — Parse edited Python code back into UI settings

use crate::data::DataStore;
use crate::renderer::map::MapProjection;
use crate::ui::{Colormap, RangeMode, UiState, ViewMode};

/// Parsed settings extracted from Python code.
#[derive(Debug, Default)]
pub struct ParsedSettings {
    pub variable_name: Option<String>,
    pub time_index: Option<usize>,
    pub level_index: Option<usize>,
    pub colormap: Option<String>,
    pub vmin: Option<f32>,
    pub vmax: Option<f32>,
    pub view_mode: Option<String>,
    pub projection: Option<String>,
    pub camera_lon: Option<f32>,
    pub camera_lat: Option<f32>,
    pub contour_enabled: Option<bool>,
    pub vector_enabled: Option<bool>,
    pub streamline_enabled: Option<bool>,
}

/// Parse a Python code string for known patterns.
pub fn parse_python(code: &str) -> ParsedSettings {
    let mut s = ParsedSettings::default();

    for line in code.lines() {
        let trimmed = line.trim();

        // ds["var_name"] or da = ds["var_name"]
        if let Some(pos) = trimmed.find("ds[\"") {
            let rest = &trimmed[pos + 4..];
            if let Some(end) = rest.find('"') {
                s.variable_name = Some(rest[..end].to_string());
            }
        }

        // .isel(time=N, level=M) or .isel(time=N) or .isel(level=M)
        if let Some(pos) = trimmed.find(".isel(") {
            let rest = &trimmed[pos + 6..];
            if let Some(paren_end) = rest.find(')') {
                let args = &rest[..paren_end];
                for part in args.split(',') {
                    let part = part.trim();
                    if let Some(val) = part.strip_prefix("time=") {
                        if let Ok(n) = val.trim().parse::<usize>() {
                            s.time_index = Some(n);
                        }
                    } else if let Some(val) = part.strip_prefix("level=") {
                        if let Ok(n) = val.trim().parse::<usize>() {
                            s.level_index = Some(n);
                        }
                    }
                }
            }
        }

        // cmap="name"
        if let Some(pos) = trimmed.find("cmap=\"") {
            let rest = &trimmed[pos + 6..];
            if let Some(end) = rest.find('"') {
                s.colormap = Some(rest[..end].to_string());
            }
        }

        // vmin=X
        if let Some(pos) = trimmed.find("vmin=") {
            let rest = &trimmed[pos + 5..];
            let val_str: String = rest.chars().take_while(|c| *c != ',' && *c != ')' && *c != ' ').collect();
            if let Ok(v) = val_str.parse::<f32>() {
                s.vmin = Some(v);
            }
        }

        // vmax=Y
        if let Some(pos) = trimmed.find("vmax=") {
            let rest = &trimmed[pos + 5..];
            let val_str: String = rest.chars().take_while(|c| *c != ',' && *c != ')' && *c != ' ').collect();
            if let Ok(v) = val_str.parse::<f32>() {
                s.vmax = Some(v);
            }
        }

        // ccrs.Orthographic(lon, lat)
        if let Some(pos) = trimmed.find("ccrs.Orthographic(") {
            s.view_mode = Some("Globe".to_string());
            s.projection = Some("Orthographic".to_string());
            let rest = &trimmed[pos + 18..];
            if let Some(paren_end) = rest.find(')') {
                let args = &rest[..paren_end];
                let parts: Vec<&str> = args.split(',').collect();
                if parts.len() >= 2 {
                    if let Ok(lon) = parts[0].trim().parse::<f32>() {
                        s.camera_lon = Some(lon);
                    }
                    if let Ok(lat) = parts[1].trim().parse::<f32>() {
                        s.camera_lat = Some(lat);
                    }
                }
            }
        }

        // ccrs.PlateCarree()
        if trimmed.contains("ccrs.PlateCarree()") && trimmed.contains("projection=") {
            s.view_mode = Some("Map".to_string());
            s.projection = Some("PlateCarree".to_string());
        }

        // ccrs.Mollweide()
        if trimmed.contains("ccrs.Mollweide()") && trimmed.contains("projection=") {
            s.view_mode = Some("Map".to_string());
            s.projection = Some("Mollweide".to_string());
        }

        // ccrs.NorthPolarStereo()
        if trimmed.contains("ccrs.NorthPolarStereo()") && trimmed.contains("projection=") {
            s.view_mode = Some("Map".to_string());
            s.projection = Some("NorthPolarStereo".to_string());
        }

        // ccrs.SouthPolarStereo()
        if trimmed.contains("ccrs.SouthPolarStereo()") && trimmed.contains("projection=") {
            s.view_mode = Some("Map".to_string());
            s.projection = Some("SouthPolarStereo".to_string());
        }

        // ax.contour or .plot.contour
        if trimmed.contains(".contour(") || trimmed.contains(".contour ") {
            s.contour_enabled = Some(true);
        }

        // ax.quiver
        if trimmed.contains(".quiver(") || trimmed.contains(".quiver ") {
            s.vector_enabled = Some(true);
        }

        // ax.streamplot
        if trimmed.contains(".streamplot(") || trimmed.contains(".streamplot ") {
            s.streamline_enabled = Some(true);
        }
    }

    s
}

/// Map a matplotlib colormap name to our Colormap enum.
fn matplotlib_to_colormap(name: &str) -> Option<Colormap> {
    match name {
        "viridis" => Some(Colormap::Viridis),
        "plasma" => Some(Colormap::Plasma),
        "inferno" => Some(Colormap::Inferno),
        "magma" => Some(Colormap::Magma),
        "cividis" => Some(Colormap::Cividis),
        "turbo" => Some(Colormap::Turbo),
        "RdBu_r" => Some(Colormap::RdBuR),
        "coolwarm" => Some(Colormap::Coolwarm),
        "Spectral" => Some(Colormap::Spectral),
        "BrBG" => Some(Colormap::BrBG),
        _ => None,
    }
}

/// Apply parsed settings to UiState. Returns descriptions of applied changes.
pub fn apply_to_ui_state(
    parsed: &ParsedSettings,
    ui_state: &mut UiState,
    data_store: &mut DataStore,
) -> Vec<String> {
    let mut changes = Vec::new();

    // Variable name → find matching variable in active file
    if let Some(ref var_name) = parsed.variable_name {
        if let Some(fi) = data_store.active_file {
            if let Some(file) = data_store.files.get_mut(fi) {
                if let Some(idx) = file.variables.iter().position(|v| v.name == *var_name) {
                    if file.selected_variable != Some(idx) {
                        file.selected_variable = Some(idx);
                        // Reload field data for the new variable
                        let _ = data_store.load_field(fi, idx);
                        changes.push(format!("variable={}", var_name));
                    }
                }
            }
        }
    }

    // Time index
    if let Some(t) = parsed.time_index {
        if ui_state.time_index != t {
            ui_state.time_index = t;
            changes.push(format!("time={}", t));
        }
    }

    // Level index
    if let Some(l) = parsed.level_index {
        if ui_state.level_index != l {
            ui_state.level_index = l;
            changes.push(format!("level={}", l));
        }
    }

    // Colormap
    if let Some(ref cmap_str) = parsed.colormap {
        if let Some(cm) = matplotlib_to_colormap(cmap_str) {
            if ui_state.colormap != cm {
                ui_state.colormap = cm;
                changes.push(format!("colormap={}", cmap_str));
            }
        }
    }

    // vmin/vmax → switch to Manual range mode
    if parsed.vmin.is_some() || parsed.vmax.is_some() {
        if let Some(vmin) = parsed.vmin {
            ui_state.manual_min = vmin;
        }
        if let Some(vmax) = parsed.vmax {
            ui_state.manual_max = vmax;
        }
        ui_state.range_mode = RangeMode::Manual;
        changes.push(format!(
            "range=Manual({}..{})",
            ui_state.manual_min, ui_state.manual_max
        ));
    }

    // View mode / projection
    if let Some(ref vm) = parsed.view_mode {
        match vm.as_str() {
            "Globe" => {
                if ui_state.view_mode != ViewMode::Globe {
                    ui_state.view_mode = ViewMode::Globe;
                    changes.push("view=Globe".to_string());
                }
            }
            "Map" => {
                if ui_state.view_mode != ViewMode::Map {
                    ui_state.view_mode = ViewMode::Map;
                    changes.push("view=Map".to_string());
                }
                if let Some(ref proj) = parsed.projection {
                    let mp = match proj.as_str() {
                        "PlateCarree" => Some(MapProjection::Equirectangular),
                        "Mollweide" => Some(MapProjection::Mollweide),
                        "NorthPolarStereo" => Some(MapProjection::PolarNorth),
                        "SouthPolarStereo" => Some(MapProjection::PolarSouth),
                        _ => None,
                    };
                    if let Some(p) = mp {
                        if ui_state.map_projection != p {
                            ui_state.map_projection = p;
                            changes.push(format!("projection={}", proj));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Contour overlay
    if let Some(enabled) = parsed.contour_enabled {
        if ui_state.contour_enabled != enabled {
            ui_state.contour_enabled = enabled;
            changes.push(format!("contour={}", enabled));
        }
    }

    // Vector overlay
    if let Some(enabled) = parsed.vector_enabled {
        if ui_state.vector_overlay_enabled != enabled {
            ui_state.vector_overlay_enabled = enabled;
            changes.push(format!("vector={}", enabled));
        }
    }

    // Streamline overlay
    if let Some(enabled) = parsed.streamline_enabled {
        if ui_state.streamline_enabled != enabled {
            ui_state.streamline_enabled = enabled;
            changes.push(format!("streamline={}", enabled));
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_variable_name() {
        let code = r#"da = ds["vorticity"]"#;
        let p = parse_python(code);
        assert_eq!(p.variable_name.as_deref(), Some("vorticity"));
    }

    #[test]
    fn parse_isel() {
        let code = "da = da.isel(time=5, level=2)";
        let p = parse_python(code);
        assert_eq!(p.time_index, Some(5));
        assert_eq!(p.level_index, Some(2));
    }

    #[test]
    fn parse_colormap() {
        let code = r#"da.plot.pcolormesh(ax=ax, cmap="RdBu_r", vmin=-5e-5, vmax=5e-5)"#;
        let p = parse_python(code);
        assert_eq!(p.colormap.as_deref(), Some("RdBu_r"));
        assert!(p.vmin.is_some());
        assert!(p.vmax.is_some());
    }

    #[test]
    fn parse_orthographic() {
        let code = "ax = fig.add_subplot(111, projection=ccrs.Orthographic(135, 35))";
        let p = parse_python(code);
        assert_eq!(p.view_mode.as_deref(), Some("Globe"));
        assert_eq!(p.camera_lon, Some(135.0));
        assert_eq!(p.camera_lat, Some(35.0));
    }

    #[test]
    fn parse_plate_carree() {
        let code = "ax = fig.add_subplot(111, projection=ccrs.PlateCarree())";
        let p = parse_python(code);
        assert_eq!(p.view_mode.as_deref(), Some("Map"));
        assert_eq!(p.projection.as_deref(), Some("PlateCarree"));
    }

    #[test]
    fn parse_mollweide() {
        let code = "ax = fig.add_subplot(111, projection=ccrs.Mollweide())";
        let p = parse_python(code);
        assert_eq!(p.view_mode.as_deref(), Some("Map"));
        assert_eq!(p.projection.as_deref(), Some("Mollweide"));
    }

    #[test]
    fn parse_contour_and_quiver() {
        let code = "da.plot.contour(ax=ax, levels=10)\nax.quiver(u, v)";
        let p = parse_python(code);
        assert_eq!(p.contour_enabled, Some(true));
        assert_eq!(p.vector_enabled, Some(true));
    }
}
