// mcp/mod.rs — MCP bridge: TCP command API for external MCP server communication

pub mod listener;

use serde::{Deserialize, Serialize};

/// Commands sent from the MCP bridge to GeoScope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum McpCommand {
    #[serde(rename = "get_status")]
    GetStatus,
    #[serde(rename = "open_file")]
    OpenFile { path: String },
    #[serde(rename = "list_variables")]
    ListVariables,
    #[serde(rename = "set_variable")]
    SetVariable { name: String },
    #[serde(rename = "set_view")]
    SetView { mode: String },
    #[serde(rename = "set_colormap")]
    SetColormap { name: String },
    #[serde(rename = "set_time")]
    SetTime { index: usize },
    #[serde(rename = "set_level")]
    SetLevel { index: usize },
    #[serde(rename = "set_projection")]
    SetProjection { name: String },
    #[serde(rename = "set_range")]
    SetRange { min: f64, max: f64 },
    #[serde(rename = "toggle_overlay")]
    ToggleOverlay { name: String, enabled: bool },
    #[serde(rename = "play")]
    Play,
    #[serde(rename = "pause")]
    Pause,
}

/// Response from GeoScope to MCP bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub success: bool,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl McpResponse {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data,
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: serde_json::Value::Null,
            error: Some(msg.into()),
        }
    }
}

/// Build a status JSON from the current app state.
pub fn build_status(
    ui_state: &crate::ui::UiState,
    data_store: &crate::data::DataStore,
) -> serde_json::Value {
    let (file_name, var_name, dims, time_steps, level_count, data_range) =
        if let Some(fi) = data_store.active_file {
            if let Some(file) = data_store.files.get(fi) {
                let fname = std::path::Path::new(&file.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file.path.clone());
                let (vname, dims_str) = if let Some(vi) = file.selected_variable {
                    if let Some(var) = file.variables.get(vi) {
                        let dims = var
                            .dimensions
                            .iter()
                            .map(|(n, s)| format!("{}({})", n, s))
                            .collect::<Vec<_>>()
                            .join(" × ");
                        (Some(var.name.clone()), Some(dims))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };
                let dr = file.field_data.as_ref().map(|f| (f.min, f.max));
                (
                    Some(fname),
                    vname,
                    dims_str,
                    file.time_steps,
                    file.level_count,
                    dr,
                )
            } else {
                (None, None, None, None, None, None)
            }
        } else {
            (None, None, None, None, None, None)
        };

    serde_json::json!({
        "file": file_name,
        "variable": var_name,
        "dimensions": dims,
        "time_index": ui_state.time_index,
        "time_steps": time_steps,
        "level_index": ui_state.level_index,
        "level_count": level_count,
        "view_mode": format!("{:?}", ui_state.view_mode),
        "colormap": ui_state.colormap.label(),
        "projection": format!("{:?}", ui_state.map_projection),
        "range_mode": format!("{:?}", ui_state.range_mode),
        "data_range": data_range.map(|(a, b)| vec![a, b]),
        "interpolated": ui_state.interpolated,
        "overlays": {
            "contour": ui_state.contour_enabled,
            "vector": ui_state.vector_overlay_enabled,
            "streamline": ui_state.streamline_enabled,
            "trajectory": ui_state.trajectory_enabled,
            "wavenumber_filter": ui_state.wavenumber_filter_enabled,
        },
    })
}

/// Build a variable list JSON from the current file.
pub fn build_variable_list(data_store: &crate::data::DataStore) -> serde_json::Value {
    if let Some(fi) = data_store.active_file {
        if let Some(file) = data_store.files.get(fi) {
            let vars: Vec<serde_json::Value> = file
                .variables
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    serde_json::json!({
                        "index": i,
                        "name": v.name,
                        "long_name": v.long_name,
                        "units": v.units,
                        "standard_name": v.standard_name,
                        "dimensions": v.dimensions.iter()
                            .map(|(n, s)| format!("{}({})", n, s))
                            .collect::<Vec<_>>(),
                        "selected": file.selected_variable == Some(i),
                    })
                })
                .collect();
            return serde_json::json!({
                "file": file.path,
                "variables": vars,
            });
        }
    }
    serde_json::json!({ "variables": [] })
}

/// Process a command that modifies UiState via ScriptSettings (reusing Rhai engine path).
pub fn build_script_settings(cmd: &McpCommand) -> Option<crate::codegen::rhai_engine::ScriptSettings> {
    use crate::codegen::rhai_engine::ScriptSettings;

    let mut s = ScriptSettings::default();
    match cmd {
        McpCommand::SetVariable { name } => {
            s.variable_name = Some(name.clone());
        }
        McpCommand::SetView { mode } => {
            s.view_mode = Some(mode.clone());
        }
        McpCommand::SetColormap { name } => {
            s.colormap = Some(name.clone());
        }
        McpCommand::SetTime { index } => {
            s.time_index = Some(*index as i64);
        }
        McpCommand::SetLevel { index } => {
            s.level_index = Some(*index as i64);
        }
        McpCommand::SetProjection { name } => {
            s.projection = Some(name.clone());
        }
        McpCommand::SetRange { min, max } => {
            s.vmin = Some(*min);
            s.vmax = Some(*max);
        }
        McpCommand::ToggleOverlay { name, enabled } => match name.as_str() {
            "contour" | "contours" => s.contour_enabled = Some(*enabled),
            "vector" | "vectors" => s.vector_enabled = Some(*enabled),
            "streamline" | "streamlines" => s.streamline_enabled = Some(*enabled),
            _ => return None,
        },
        _ => return None,
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_deserialization() {
        let json = r#"{"command":"get_status"}"#;
        let cmd: McpCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, McpCommand::GetStatus));

        let json = r#"{"command":"set_variable","name":"zeta"}"#;
        let cmd: McpCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, McpCommand::SetVariable { name } if name == "zeta"));

        let json = r#"{"command":"set_range","min":-5e-5,"max":5e-5}"#;
        let cmd: McpCommand = serde_json::from_str(json).unwrap();
        assert!(matches!(cmd, McpCommand::SetRange { min, max } if (min + 5e-5).abs() < 1e-10 && (max - 5e-5).abs() < 1e-10));
    }

    #[test]
    fn test_response_serialization() {
        let resp = McpResponse::ok(serde_json::json!({"status": "ok"}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(!json.contains("\"error\""));

        let resp = McpResponse::err("not found");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("not found"));
    }

    #[test]
    fn test_build_script_settings() {
        let cmd = McpCommand::SetColormap {
            name: "RdBu_r".to_string(),
        };
        let s = build_script_settings(&cmd).unwrap();
        assert_eq!(s.colormap.as_deref(), Some("RdBu_r"));

        let cmd = McpCommand::ToggleOverlay {
            name: "contour".to_string(),
            enabled: true,
        };
        let s = build_script_settings(&cmd).unwrap();
        assert_eq!(s.contour_enabled, Some(true));

        // GetStatus doesn't produce ScriptSettings
        let cmd = McpCommand::GetStatus;
        assert!(build_script_settings(&cmd).is_none());
    }
}
