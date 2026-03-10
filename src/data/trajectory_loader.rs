// data/trajectory_loader.rs — Load trajectory data from external JSON/CSV files

use super::TrajectoryData;
use std::path::Path;

/// Load trajectory data from an external file (JSON or CSV).
/// Format is detected by file extension.
pub fn load_trajectory_from_file(path: &str) -> Result<TrajectoryData, String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "json" => load_json(path),
        "csv" => load_csv(path),
        _ => Err(format!("Unsupported trajectory file extension '.{ext}'. Use .json or .csv")),
    }
}

/// Load trajectory from JSON.
///
/// Supports two formats:
/// 1. Object with name and points: `{"name": "...", "points": [{"lon": 135.0, "lat": 35.0}, ...]}`
/// 2. Simple array: `[[135.0, 35.0], [136.5, 35.2], ...]`
fn load_json(path: &str) -> Result<TrajectoryData, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {e}"))?;

    match &value {
        serde_json::Value::Object(obj) => {
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("External")
                .to_string();

            let points_val = obj
                .get("points")
                .ok_or_else(|| "JSON object must have a \"points\" field".to_string())?;

            let points = parse_points_array(points_val)?;
            Ok(TrajectoryData { points, name })
        }
        serde_json::Value::Array(_) => {
            let points = parse_points_array(&value)?;
            let name = Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("External")
                .to_string();
            Ok(TrajectoryData { points, name })
        }
        _ => Err("JSON must be an object or array".to_string()),
    }
}

/// Parse a JSON array of points.
/// Supports `[{"lon": ..., "lat": ...}, ...]` or `[[lon, lat], ...]`.
fn parse_points_array(value: &serde_json::Value) -> Result<Vec<(f32, f32)>, String> {
    let arr = value
        .as_array()
        .ok_or_else(|| "Expected a JSON array of points".to_string())?;

    if arr.is_empty() {
        return Err("Points array is empty".to_string());
    }

    let mut points = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let (lon, lat) = match item {
            serde_json::Value::Object(obj) => {
                let lon = obj
                    .get("lon")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| format!("Point {i}: missing or invalid \"lon\""))?;
                let lat = obj
                    .get("lat")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| format!("Point {i}: missing or invalid \"lat\""))?;
                (lon, lat)
            }
            serde_json::Value::Array(pair) => {
                if pair.len() < 2 {
                    return Err(format!("Point {i}: array must have at least 2 elements [lon, lat]"));
                }
                let lon = pair[0]
                    .as_f64()
                    .ok_or_else(|| format!("Point {i}: invalid lon value"))?;
                let lat = pair[1]
                    .as_f64()
                    .ok_or_else(|| format!("Point {i}: invalid lat value"))?;
                (lon, lat)
            }
            _ => return Err(format!("Point {i}: expected object or array")),
        };
        points.push((lon as f32, lat as f32));
    }

    Ok(points)
}

/// Load trajectory from CSV.
/// Expected format: `lon,lat` header (optional), then `lon_value,lat_value` per line.
fn load_csv(path: &str) -> Result<TrajectoryData, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let mut points = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Skip header row
        if i == 0 && line.contains("lon") && line.contains("lat") {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 2 {
            return Err(format!("Line {}: expected at least 2 comma-separated values", i + 1));
        }

        let lon: f64 = parts[0]
            .trim()
            .parse()
            .map_err(|_| format!("Line {}: invalid lon value '{}'", i + 1, parts[0].trim()))?;
        let lat: f64 = parts[1]
            .trim()
            .parse()
            .map_err(|_| format!("Line {}: invalid lat value '{}'", i + 1, parts[1].trim()))?;

        points.push((lon as f32, lat as f32));
    }

    if points.is_empty() {
        return Err("CSV file contains no valid data points".to_string());
    }

    let name = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("External")
        .to_string();

    Ok(TrajectoryData { points, name })
}
