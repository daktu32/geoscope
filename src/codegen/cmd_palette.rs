// codegen/cmd_palette.rs — Command palette: local pattern matching + action dispatch

use crate::data::DataStore;
use crate::ui::{Colormap, UiState, ViewMode};
use crate::renderer::map::MapProjection;

#[derive(Clone, Debug)]
pub struct PaletteCommand {
    pub icon: &'static str,
    pub label: String,
    pub action: PaletteAction,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum PaletteAction {
    SetVariable(String),
    SetColormap(String),
    SetViewMode(String),
    SetProjection(String),
    SetTimeIndex(usize),
    SetLevelIndex(usize),
    ToggleContours,
    ToggleVectors,
    ToggleStreamlines,
    ToggleWavenumberFilter,
    ExportPng,
    ExportGif,
}

/// Match user input against known commands. Returns sorted matches.
pub fn match_commands(input: &str, _ui_state: &UiState, data_store: &DataStore) -> Vec<PaletteCommand> {
    let input_lower = input.to_lowercase();
    let mut results = Vec::new();

    // Static commands (always available)
    let static_commands: Vec<(&str, &str, &str, PaletteAction)> = vec![
        // View modes
        ("globe", "🌐", "Switch to Globe view", PaletteAction::SetViewMode("Globe".into())),
        ("map", "🗺", "Switch to Map view", PaletteAction::SetViewMode("Map".into())),
        ("hovmoller", "📊", "Switch to Hovmoller view", PaletteAction::SetViewMode("Hovmoller".into())),
        ("spectrum", "📈", "Switch to Spectrum view", PaletteAction::SetViewMode("Spectrum".into())),
        ("section", "🔪", "Switch to CrossSection view", PaletteAction::SetViewMode("CrossSection".into())),
        ("profile", "📍", "Switch to Profile view", PaletteAction::SetViewMode("Profile".into())),
        // Colormaps
        ("viridis", "🎨", "Set colormap: Viridis", PaletteAction::SetColormap("Viridis".into())),
        ("rdbu", "🎨", "Set colormap: RdBu_r", PaletteAction::SetColormap("RdBu_r".into())),
        ("plasma", "🎨", "Set colormap: Plasma", PaletteAction::SetColormap("Plasma".into())),
        ("inferno", "🎨", "Set colormap: Inferno", PaletteAction::SetColormap("Inferno".into())),
        ("magma", "🎨", "Set colormap: Magma", PaletteAction::SetColormap("Magma".into())),
        ("cividis", "🎨", "Set colormap: Cividis", PaletteAction::SetColormap("Cividis".into())),
        ("turbo", "🎨", "Set colormap: Turbo", PaletteAction::SetColormap("Turbo".into())),
        ("coolwarm", "🎨", "Set colormap: Coolwarm", PaletteAction::SetColormap("Coolwarm".into())),
        ("spectral colormap", "🎨", "Set colormap: Spectral", PaletteAction::SetColormap("Spectral".into())),
        ("brbg", "🎨", "Set colormap: BrBG", PaletteAction::SetColormap("BrBG".into())),
        // Projections
        ("equirect", "🗺", "Projection: Equirectangular", PaletteAction::SetProjection("Equirectangular".into())),
        ("mollweide", "🗺", "Projection: Mollweide", PaletteAction::SetProjection("Mollweide".into())),
        ("polar north", "🗺", "Projection: Polar North", PaletteAction::SetProjection("PolarNorth".into())),
        ("polar south", "🗺", "Projection: Polar South", PaletteAction::SetProjection("PolarSouth".into())),
        // Toggles
        ("contour", "📐", "Toggle contour lines", PaletteAction::ToggleContours),
        ("vector", "➡", "Toggle vector overlay", PaletteAction::ToggleVectors),
        ("streamline", "🌊", "Toggle streamlines", PaletteAction::ToggleStreamlines),
        ("wavenumber", "🔬", "Toggle wavenumber filter", PaletteAction::ToggleWavenumberFilter),
        // Export
        ("export png", "💾", "Export as PNG", PaletteAction::ExportPng),
        ("export gif", "🎬", "Export as GIF", PaletteAction::ExportGif),
    ];

    for (keyword, icon, label, action) in static_commands {
        if keyword.contains(&input_lower) || input_lower.contains(keyword) || fuzzy_match(&input_lower, keyword) {
            results.push(PaletteCommand { icon, label: label.to_string(), action });
        }
    }

    // Dynamic commands: variable names from loaded files
    if let Some(fi) = data_store.active_file {
        if let Some(file) = data_store.files.get(fi) {
            for var in &file.variables {
                if var.name.to_lowercase().contains(&input_lower) || input_lower.contains(&var.name.to_lowercase()) {
                    results.push(PaletteCommand {
                        icon: "🔍",
                        label: format!("Select variable: {}", var.name),
                        action: PaletteAction::SetVariable(var.name.clone()),
                    });
                }
            }
        }
    }

    // Limit to 8 results
    results.truncate(8);
    results
}

fn fuzzy_match(input: &str, target: &str) -> bool {
    let mut target_chars = target.chars();
    for c in input.chars() {
        if c == ' ' { continue; }
        loop {
            match target_chars.next() {
                Some(tc) if tc == c => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Apply a palette action to UI state. Returns description of what was done.
pub fn apply_action(action: &PaletteAction, ui_state: &mut UiState, data_store: &mut DataStore) -> Option<String> {
    match action {
        PaletteAction::SetVariable(name) => {
            if let Some(fi) = data_store.active_file {
                if let Some(file) = data_store.files.get_mut(fi) {
                    if let Some(idx) = file.variables.iter().position(|v| v.name == *name) {
                        if file.selected_variable != Some(idx) {
                            file.selected_variable = Some(idx);
                            let _ = data_store.load_field(fi, idx);
                            return Some(format!("Variable: {}", name));
                        }
                    }
                }
            }
            None
        }
        PaletteAction::SetColormap(cmap_str) => {
            if let Some(cm) = name_to_colormap(cmap_str) {
                if ui_state.colormap != cm {
                    ui_state.colormap = cm;
                    return Some(format!("Colormap: {}", cmap_str));
                }
            }
            None
        }
        PaletteAction::SetViewMode(vm) => {
            if let Some(mode) = name_to_view_mode(vm) {
                if ui_state.view_mode != mode {
                    ui_state.view_mode = mode;
                    return Some(format!("View: {}", vm));
                }
            }
            None
        }
        PaletteAction::SetProjection(proj) => {
            if let Some(p) = name_to_projection(proj) {
                if ui_state.map_projection != p {
                    ui_state.map_projection = p;
                    return Some(format!("Projection: {}", proj));
                }
            }
            None
        }
        PaletteAction::SetTimeIndex(t) => {
            let t = *t;
            if ui_state.time_index != t {
                ui_state.time_index = t;
                return Some(format!("Time: {}", t));
            }
            None
        }
        PaletteAction::SetLevelIndex(l) => {
            let l = *l;
            if ui_state.level_index != l {
                ui_state.level_index = l;
                return Some(format!("Level: {}", l));
            }
            None
        }
        PaletteAction::ToggleContours => {
            ui_state.contour_enabled = !ui_state.contour_enabled;
            let state = if ui_state.contour_enabled { "ON" } else { "OFF" };
            Some(format!("Contours: {}", state))
        }
        PaletteAction::ToggleVectors => {
            ui_state.vector_overlay_enabled = !ui_state.vector_overlay_enabled;
            let state = if ui_state.vector_overlay_enabled { "ON" } else { "OFF" };
            Some(format!("Vectors: {}", state))
        }
        PaletteAction::ToggleStreamlines => {
            ui_state.streamline_enabled = !ui_state.streamline_enabled;
            let state = if ui_state.streamline_enabled { "ON" } else { "OFF" };
            Some(format!("Streamlines: {}", state))
        }
        PaletteAction::ToggleWavenumberFilter => {
            ui_state.wavenumber_filter_enabled = !ui_state.wavenumber_filter_enabled;
            let state = if ui_state.wavenumber_filter_enabled { "ON" } else { "OFF" };
            Some(format!("Wavenumber filter: {}", state))
        }
        PaletteAction::ExportPng => {
            ui_state.export_settings.format = crate::renderer::export::ExportFormat::Png;
            ui_state.export_dialog_open = true;
            Some("Opening PNG export dialog".to_string())
        }
        PaletteAction::ExportGif => {
            ui_state.export_settings.format = crate::renderer::export::ExportFormat::Gif;
            ui_state.export_dialog_open = true;
            Some("Opening GIF export dialog".to_string())
        }
    }
}

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

fn name_to_projection(name: &str) -> Option<MapProjection> {
    match name {
        "Equirectangular" | "equirectangular" | "PlateCarree" => Some(MapProjection::Equirectangular),
        "Mollweide" | "mollweide" => Some(MapProjection::Mollweide),
        "PolarNorth" | "NorthPolarStereo" => Some(MapProjection::PolarNorth),
        "PolarSouth" | "SouthPolarStereo" => Some(MapProjection::PolarSouth),
        _ => None,
    }
}
