// data/inference.rs — Variable inference engine (3-level fallback)

use super::{ColormapHint, FieldData, VariableInfo};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InferenceResult {
    pub category: VariableCategory,
    pub suggested_colormap: ColormapHint,
    pub description: String,
    pub confidence: InferenceLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum VariableCategory {
    Vorticity,
    Divergence,
    WindSpeed,
    Temperature,
    Pressure,
    Geopotential,
    Humidity,
    StreamFunction,
    VelocityPotential,
    SurfaceHeight,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceLevel {
    L1StandardName,
    L2NamePattern,
    L3Statistics,
}

/// Infer variable category and colormap from metadata and data statistics.
///
/// Uses a 3-level fallback:
/// - L1: CF standard_name (highest confidence)
/// - L2: Variable name pattern + long_name + units
/// - L3: Data statistics (min/max sign)
pub fn infer_variable(var: &VariableInfo, field: Option<&FieldData>) -> InferenceResult {
    // L1: CF standard_name
    if let Some(ref sn) = var.standard_name {
        if let Some(result) = try_standard_name(sn) {
            return result;
        }
    }

    // L2: Name pattern + long_name + units
    if let Some(result) = try_name_pattern(var) {
        return result;
    }

    // L3: Statistics fallback
    infer_from_statistics(field)
}

fn try_standard_name(standard_name: &str) -> Option<InferenceResult> {
    let sn = standard_name.to_ascii_lowercase();
    let (cat, hint, desc) = match sn.as_str() {
        "atmosphere_relative_vorticity" => (
            VariableCategory::Vorticity,
            ColormapHint::Diverging,
            "Relative vorticity (CF)",
        ),
        "divergence_of_wind" => (
            VariableCategory::Divergence,
            ColormapHint::Diverging,
            "Wind divergence (CF)",
        ),
        "air_temperature" => (
            VariableCategory::Temperature,
            ColormapHint::Sequential,
            "Air temperature (CF)",
        ),
        "surface_air_pressure" | "air_pressure" => (
            VariableCategory::Pressure,
            ColormapHint::Sequential,
            "Air pressure (CF)",
        ),
        "geopotential_height" => (
            VariableCategory::Geopotential,
            ColormapHint::Sequential,
            "Geopotential height (CF)",
        ),
        "specific_humidity" | "relative_humidity" => (
            VariableCategory::Humidity,
            ColormapHint::Sequential,
            "Humidity (CF)",
        ),
        "atmosphere_horizontal_streamfunction" => (
            VariableCategory::StreamFunction,
            ColormapHint::Diverging,
            "Stream function (CF)",
        ),
        "atmosphere_horizontal_velocity_potential" => (
            VariableCategory::VelocityPotential,
            ColormapHint::Diverging,
            "Velocity potential (CF)",
        ),
        _ => return None,
    };

    Some(InferenceResult {
        category: cat,
        suggested_colormap: hint,
        description: desc.to_string(),
        confidence: InferenceLevel::L1StandardName,
    })
}

fn try_name_pattern(var: &VariableInfo) -> Option<InferenceResult> {
    let name = var.name.to_ascii_lowercase();
    let long = var
        .long_name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let units = var.units.as_deref().unwrap_or("").to_ascii_lowercase();

    let (cat, hint, desc) = if name.starts_with("vor") || long.contains("vorticity") {
        (
            VariableCategory::Vorticity,
            ColormapHint::Diverging,
            "Vorticity (name pattern)",
        )
    } else if name.starts_with("div") || long.contains("divergence") {
        (
            VariableCategory::Divergence,
            ColormapHint::Diverging,
            "Divergence (name pattern)",
        )
    } else if name.starts_with("temp")
        || (name == "t" && units.contains('k'))
        || long.contains("temperature")
    {
        (
            VariableCategory::Temperature,
            ColormapHint::Sequential,
            "Temperature (name pattern)",
        )
    } else if name.starts_with("ps") || name == "pressure" || long.contains("pressure") {
        (
            VariableCategory::Pressure,
            ColormapHint::Sequential,
            "Pressure (name pattern)",
        )
    } else if name.contains("stream") || name == "psi" || long.contains("stream") {
        (
            VariableCategory::StreamFunction,
            ColormapHint::Diverging,
            "Stream function (name pattern)",
        )
    } else if name.contains("chi") || name == "velocity_potential" || long.contains("velocity potential") {
        (
            VariableCategory::VelocityPotential,
            ColormapHint::Diverging,
            "Velocity potential (name pattern)",
        )
    } else if name.contains("height")
        || name.contains("phi")
        || name.contains("geopot")
        || long.contains("geopotential")
    {
        (
            VariableCategory::Geopotential,
            ColormapHint::Sequential,
            "Geopotential (name pattern)",
        )
    } else if name.contains("wind") || name.contains("speed") || name == "u" || name == "v" {
        (
            VariableCategory::WindSpeed,
            ColormapHint::Sequential,
            "Wind speed (name pattern)",
        )
    } else if name.contains("humid") || name == "q" || long.contains("humidity") {
        (
            VariableCategory::Humidity,
            ColormapHint::Sequential,
            "Humidity (name pattern)",
        )
    } else {
        return None;
    };

    Some(InferenceResult {
        category: cat,
        suggested_colormap: hint,
        description: desc.to_string(),
        confidence: InferenceLevel::L2NamePattern,
    })
}

fn infer_from_statistics(field: Option<&FieldData>) -> InferenceResult {
    if let Some(f) = field {
        if f.min * f.max < 0.0 {
            return InferenceResult {
                category: VariableCategory::Generic,
                suggested_colormap: ColormapHint::Diverging,
                description: "Data spans zero (statistics)".to_string(),
                confidence: InferenceLevel::L3Statistics,
            };
        }
    }

    InferenceResult {
        category: VariableCategory::Generic,
        suggested_colormap: ColormapHint::Sequential,
        description: "Generic field (statistics)".to_string(),
        confidence: InferenceLevel::L3Statistics,
    }
}

/// Detect a wind u/v pair from the variable list.
/// Returns (u_var_idx, v_var_idx) if found.
///
/// L1: standard_name "eastward_wind" / "northward_wind"
/// L2: name patterns: "u"/"v", "u_cos"/"v_cos", "uwnd"/"vwnd"
/// Condition: both must have the same dimension structure.
pub fn detect_wind_pair(variables: &[VariableInfo]) -> Option<(usize, usize)> {
    // L1: standard_name
    let u_sn = variables.iter().position(|v| {
        v.standard_name.as_deref() == Some("eastward_wind")
    });
    let v_sn = variables.iter().position(|v| {
        v.standard_name.as_deref() == Some("northward_wind")
    });
    if let (Some(ui), Some(vi)) = (u_sn, v_sn) {
        if variables[ui].dimensions == variables[vi].dimensions {
            return Some((ui, vi));
        }
    }

    // L2: name patterns
    let u_patterns = ["u", "u_cos", "uwnd"];
    let v_patterns = ["v", "v_cos", "vwnd"];

    for (up, vp) in u_patterns.iter().zip(v_patterns.iter()) {
        let u_idx = variables.iter().position(|v| {
            v.name.eq_ignore_ascii_case(up)
        });
        let v_idx = variables.iter().position(|v| {
            v.name.eq_ignore_ascii_case(vp)
        });
        if let (Some(ui), Some(vi)) = (u_idx, v_idx) {
            if variables[ui].dimensions == variables[vi].dimensions {
                return Some((ui, vi));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_var(name: &str) -> VariableInfo {
        VariableInfo {
            name: name.to_string(),
            long_name: None,
            units: None,
            standard_name: None,
            dimensions: vec![],
        }
    }

    #[test]
    fn l1_standard_name_vorticity() {
        let mut var = make_var("x");
        var.standard_name = Some("atmosphere_relative_vorticity".to_string());
        let r = infer_variable(&var, None);
        assert_eq!(r.category, VariableCategory::Vorticity);
        assert_eq!(r.suggested_colormap, ColormapHint::Diverging);
        assert_eq!(r.confidence, InferenceLevel::L1StandardName);
    }

    #[test]
    fn l2_name_pattern_temperature() {
        let mut var = make_var("T");
        var.units = Some("K".to_string());
        let r = infer_variable(&var, None);
        assert_eq!(r.category, VariableCategory::Temperature);
        assert_eq!(r.confidence, InferenceLevel::L2NamePattern);
    }

    #[test]
    fn l2_name_pattern_vorticity() {
        let var = make_var("vor");
        let r = infer_variable(&var, None);
        assert_eq!(r.category, VariableCategory::Vorticity);
        assert_eq!(r.suggested_colormap, ColormapHint::Diverging);
    }

    #[test]
    fn l3_statistics_diverging() {
        let var = make_var("unknown_field");
        let field = FieldData {
            values: vec![-1.0, 0.0, 1.0],
            width: 3,
            height: 1,
            min: -1.0,
            max: 1.0,
        };
        let r = infer_variable(&var, Some(&field));
        assert_eq!(r.category, VariableCategory::Generic);
        assert_eq!(r.suggested_colormap, ColormapHint::Diverging);
        assert_eq!(r.confidence, InferenceLevel::L3Statistics);
    }

    #[test]
    fn l3_statistics_sequential() {
        let var = make_var("unknown_field");
        let field = FieldData {
            values: vec![1.0, 2.0, 3.0],
            width: 3,
            height: 1,
            min: 1.0,
            max: 3.0,
        };
        let r = infer_variable(&var, Some(&field));
        assert_eq!(r.suggested_colormap, ColormapHint::Sequential);
    }
}
