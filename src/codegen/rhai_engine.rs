// codegen/rhai_engine.rs — Rhai scripting engine for GeoScope

use std::cell::RefCell;
use std::rc::Rc;

use rhai::{Engine, Scope};

use crate::data::DataStore;
use crate::renderer::map::MapProjection;
use crate::ui::{Colormap, RangeMode, UiState, ViewMode};

/// Settings extracted from a Rhai script execution.
#[derive(Clone, Debug, Default)]
pub struct ScriptSettings {
    pub variable_name: Option<String>,
    pub time_index: Option<i64>,
    pub level_index: Option<i64>,
    pub colormap: Option<String>,
    pub view_mode: Option<String>,
    pub projection: Option<String>,
    pub vmin: Option<f64>,
    pub vmax: Option<f64>,
    pub contour_enabled: Option<bool>,
    pub vector_enabled: Option<bool>,
    pub streamline_enabled: Option<bool>,
    pub wavenumber_cutoff: Option<i64>,
}

/// Create and configure a Rhai engine with GeoScope API functions.
fn create_engine(settings: Rc<RefCell<ScriptSettings>>) -> Engine {
    let mut engine = Engine::new();

    // set_variable("name")
    let s = settings.clone();
    engine.register_fn("set_variable", move |name: &str| {
        s.borrow_mut().variable_name = Some(name.to_string());
    });

    // set_time(index)
    let s = settings.clone();
    engine.register_fn("set_time", move |idx: i64| {
        s.borrow_mut().time_index = Some(idx);
    });

    // set_level(index)
    let s = settings.clone();
    engine.register_fn("set_level", move |idx: i64| {
        s.borrow_mut().level_index = Some(idx);
    });

    // set_colormap("name")
    let s = settings.clone();
    engine.register_fn("set_colormap", move |name: &str| {
        s.borrow_mut().colormap = Some(name.to_string());
    });

    // set_view("Globe" | "Map" | "Hovmoller" | "Spectrum" | "CrossSection" | "Profile")
    let s = settings.clone();
    engine.register_fn("set_view", move |name: &str| {
        s.borrow_mut().view_mode = Some(name.to_string());
    });

    // set_projection("Equirectangular" | "Mollweide" | "PolarNorth" | "PolarSouth")
    let s = settings.clone();
    engine.register_fn("set_projection", move |name: &str| {
        s.borrow_mut().projection = Some(name.to_string());
    });

    // set_range(min, max)
    let s = settings.clone();
    engine.register_fn("set_range", move |vmin: f64, vmax: f64| {
        let mut st = s.borrow_mut();
        st.vmin = Some(vmin);
        st.vmax = Some(vmax);
    });

    // enable_contours() / disable_contours()
    let s = settings.clone();
    engine.register_fn("enable_contours", move || {
        s.borrow_mut().contour_enabled = Some(true);
    });
    let s = settings.clone();
    engine.register_fn("disable_contours", move || {
        s.borrow_mut().contour_enabled = Some(false);
    });

    // enable_vectors() / disable_vectors()
    let s = settings.clone();
    engine.register_fn("enable_vectors", move || {
        s.borrow_mut().vector_enabled = Some(true);
    });
    let s = settings.clone();
    engine.register_fn("disable_vectors", move || {
        s.borrow_mut().vector_enabled = Some(false);
    });

    // enable_streamlines() / disable_streamlines()
    let s = settings.clone();
    engine.register_fn("enable_streamlines", move || {
        s.borrow_mut().streamline_enabled = Some(true);
    });
    let s = settings.clone();
    engine.register_fn("disable_streamlines", move || {
        s.borrow_mut().streamline_enabled = Some(false);
    });

    // set_wavenumber_cutoff(n)
    let s = settings.clone();
    engine.register_fn("set_wavenumber_cutoff", move |n: i64| {
        s.borrow_mut().wavenumber_cutoff = Some(n);
    });

    engine
}

/// Execute a Rhai script and return the collected settings.
pub fn execute_script(code: &str) -> Result<ScriptSettings, String> {
    let settings = Rc::new(RefCell::new(ScriptSettings::default()));
    let engine = create_engine(settings.clone());
    let scope = &mut Scope::new();

    engine
        .eval_with_scope::<()>(scope, code)
        .map_err(|e| e.to_string())?;

    let result = settings.borrow().clone();
    Ok(result)
}

/// Map a colormap name string to our Colormap enum.
fn name_to_colormap(name: &str) -> Option<Colormap> {
    match name {
        "Viridis" | "viridis" => Some(Colormap::Viridis),
        "Plasma" | "plasma" => Some(Colormap::Plasma),
        "Inferno" | "inferno" => Some(Colormap::Inferno),
        "Magma" | "magma" => Some(Colormap::Magma),
        "Cividis" | "cividis" => Some(Colormap::Cividis),
        "Turbo" | "turbo" => Some(Colormap::Turbo),
        "RdBu_r" | "rdbu_r" => Some(Colormap::RdBuR),
        "Coolwarm" | "coolwarm" => Some(Colormap::Coolwarm),
        "Spectral" => Some(Colormap::Spectral),
        "BrBG" => Some(Colormap::BrBG),
        _ => None,
    }
}

/// Map a view mode name string to ViewMode enum.
fn name_to_view_mode(name: &str) -> Option<ViewMode> {
    match name {
        "Globe" | "globe" => Some(ViewMode::Globe),
        "Map" | "map" => Some(ViewMode::Map),
        "Hovmoller" | "hovmoller" => Some(ViewMode::Hovmoller),
        "Spectrum" | "spectrum" => Some(ViewMode::Spectrum),
        "CrossSection" | "cross_section" => Some(ViewMode::CrossSection),
        "Profile" | "profile" => Some(ViewMode::Profile),
        _ => None,
    }
}

/// Map a projection name string to MapProjection enum.
fn name_to_projection(name: &str) -> Option<MapProjection> {
    match name {
        "Equirectangular" | "equirectangular" | "PlateCarree" => {
            Some(MapProjection::Equirectangular)
        }
        "Mollweide" | "mollweide" => Some(MapProjection::Mollweide),
        "PolarNorth" | "NorthPolarStereo" => Some(MapProjection::PolarNorth),
        "PolarSouth" | "SouthPolarStereo" => Some(MapProjection::PolarSouth),
        _ => None,
    }
}

/// Apply script settings to UiState. Returns descriptions of applied changes.
pub fn apply_script_settings(
    settings: &ScriptSettings,
    ui_state: &mut UiState,
    data_store: &mut DataStore,
) -> Vec<String> {
    let mut changes = Vec::new();

    // Variable
    if let Some(ref var_name) = settings.variable_name {
        if let Some(fi) = data_store.active_file {
            if let Some(file) = data_store.files.get_mut(fi) {
                if let Some(idx) = file.variables.iter().position(|v| v.name == *var_name) {
                    if file.selected_variable != Some(idx) {
                        file.selected_variable = Some(idx);
                        let _ = data_store.load_field(fi, idx);
                        changes.push(format!("variable={}", var_name));
                    }
                }
            }
        }
    }

    // Time index
    if let Some(t) = settings.time_index {
        let t = t.max(0) as usize;
        if ui_state.time_index != t {
            ui_state.time_index = t;
            changes.push(format!("time={}", t));
        }
    }

    // Level index
    if let Some(l) = settings.level_index {
        let l = l.max(0) as usize;
        if ui_state.level_index != l {
            ui_state.level_index = l;
            changes.push(format!("level={}", l));
        }
    }

    // Colormap
    if let Some(ref cmap_str) = settings.colormap {
        if let Some(cm) = name_to_colormap(cmap_str) {
            if ui_state.colormap != cm {
                ui_state.colormap = cm;
                changes.push(format!("colormap={}", cmap_str));
            }
        }
    }

    // Range
    if settings.vmin.is_some() || settings.vmax.is_some() {
        if let Some(vmin) = settings.vmin {
            ui_state.manual_min = vmin as f32;
        }
        if let Some(vmax) = settings.vmax {
            ui_state.manual_max = vmax as f32;
        }
        ui_state.range_mode = RangeMode::Manual;
        changes.push(format!(
            "range=Manual({}..{})",
            ui_state.manual_min, ui_state.manual_max
        ));
    }

    // View mode
    if let Some(ref vm) = settings.view_mode {
        if let Some(mode) = name_to_view_mode(vm) {
            if ui_state.view_mode != mode {
                ui_state.view_mode = mode;
                changes.push(format!("view={}", vm));
            }
        }
    }

    // Projection
    if let Some(ref proj) = settings.projection {
        if let Some(p) = name_to_projection(proj) {
            if ui_state.map_projection != p {
                ui_state.map_projection = p;
                changes.push(format!("projection={}", proj));
            }
        }
    }

    // Contour overlay
    if let Some(enabled) = settings.contour_enabled {
        if ui_state.contour_enabled != enabled {
            ui_state.contour_enabled = enabled;
            changes.push(format!("contour={}", enabled));
        }
    }

    // Vector overlay
    if let Some(enabled) = settings.vector_enabled {
        if ui_state.vector_overlay_enabled != enabled {
            ui_state.vector_overlay_enabled = enabled;
            changes.push(format!("vector={}", enabled));
        }
    }

    // Streamline overlay
    if let Some(enabled) = settings.streamline_enabled {
        if ui_state.streamline_enabled != enabled {
            ui_state.streamline_enabled = enabled;
            changes.push(format!("streamline={}", enabled));
        }
    }

    // Wavenumber cutoff
    if let Some(n) = settings.wavenumber_cutoff {
        let n = n.max(0) as usize;
        if ui_state.wavenumber_cutoff != n {
            ui_state.wavenumber_cutoff = n;
            ui_state.wavenumber_filter_enabled = true;
            changes.push(format!("wavenumber_cutoff={}", n));
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_set_variable() {
        let code = r#"set_variable("vorticity");"#;
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.variable_name.as_deref(), Some("vorticity"));
    }

    #[test]
    fn test_execute_set_time_and_level() {
        let code = "set_time(5); set_level(2);";
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.time_index, Some(5));
        assert_eq!(settings.level_index, Some(2));
    }

    #[test]
    fn test_execute_set_colormap_and_range() {
        let code = r#"set_colormap("RdBu_r"); set_range(-5e-5, 5e-5);"#;
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.colormap.as_deref(), Some("RdBu_r"));
        assert_eq!(settings.vmin, Some(-5e-5));
        assert_eq!(settings.vmax, Some(5e-5));
    }

    #[test]
    fn test_execute_view_and_projection() {
        let code = r#"set_view("Globe"); set_projection("Mollweide");"#;
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.view_mode.as_deref(), Some("Globe"));
        assert_eq!(settings.projection.as_deref(), Some("Mollweide"));
    }

    #[test]
    fn test_execute_overlays() {
        let code = "enable_contours(); enable_streamlines();";
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.contour_enabled, Some(true));
        assert_eq!(settings.streamline_enabled, Some(true));
    }

    #[test]
    fn test_execute_disable_overlays() {
        let code = "disable_contours(); disable_vectors();";
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.contour_enabled, Some(false));
        assert_eq!(settings.vector_enabled, Some(false));
    }

    #[test]
    fn test_execute_wavenumber_cutoff() {
        let code = "set_wavenumber_cutoff(21);";
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.wavenumber_cutoff, Some(21));
    }

    #[test]
    fn test_execute_full_script() {
        let code = r#"
set_variable("vorticity");
set_time(5);
set_level(0);
set_colormap("RdBu_r");
set_view("Globe");
set_range(-5e-5, 5e-5);
enable_contours();
enable_streamlines();
set_wavenumber_cutoff(21);
"#;
        let settings = execute_script(code).unwrap();
        assert_eq!(settings.variable_name.as_deref(), Some("vorticity"));
        assert_eq!(settings.time_index, Some(5));
        assert_eq!(settings.level_index, Some(0));
        assert_eq!(settings.colormap.as_deref(), Some("RdBu_r"));
        assert_eq!(settings.view_mode.as_deref(), Some("Globe"));
        assert_eq!(settings.vmin, Some(-5e-5));
        assert_eq!(settings.vmax, Some(5e-5));
        assert_eq!(settings.contour_enabled, Some(true));
        assert_eq!(settings.streamline_enabled, Some(true));
        assert_eq!(settings.wavenumber_cutoff, Some(21));
    }

    #[test]
    fn test_execute_syntax_error() {
        let code = "set_variable(";
        let result = execute_script(code);
        assert!(result.is_err());
    }

    #[test]
    fn test_name_to_colormap() {
        assert_eq!(name_to_colormap("viridis"), Some(Colormap::Viridis));
        assert_eq!(name_to_colormap("RdBu_r"), Some(Colormap::RdBuR));
        assert_eq!(name_to_colormap("unknown"), None);
    }

    #[test]
    fn test_name_to_view_mode() {
        assert_eq!(name_to_view_mode("Globe"), Some(ViewMode::Globe));
        assert_eq!(name_to_view_mode("map"), Some(ViewMode::Map));
        assert_eq!(name_to_view_mode("bad"), None);
    }
}
