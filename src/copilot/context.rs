pub fn build_system_prompt() -> String {
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
    data_store: &crate::data::DataStore,
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
                }
            }
        }
    }

    ctx.push_str(&format!("- View: {:?}\n", ui_state.view_mode));
    ctx.push_str(&format!("- Colormap: {:?}\n", ui_state.colormap));
    ctx.push_str(&format!("- Time index: {}\n", ui_state.time_index));
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
