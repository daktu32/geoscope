use std::path::Path;

/// Metadata for a single variable in a NetCDF file.
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub long_name: Option<String>,
    pub units: Option<String>,
    pub dimensions: Vec<(String, usize)>,
}

/// Hint for which colormap family to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColormapHint {
    /// Diverging colormap (e.g. RdBu_r) — for fields with positive and negative values.
    Diverging,
    /// Sequential colormap (e.g. viridis) — for non-negative or monotone fields.
    Sequential,
}

/// Coordinate values extracted from a NetCDF file.
#[derive(Debug, Clone)]
pub struct GridInfo {
    pub lon: Option<Vec<f64>>,
    pub lat: Option<Vec<f64>>,
}

/// Metadata and data for an opened NetCDF file.
#[derive(Debug)]
pub struct OpenFile {
    pub path: String,
    pub variables: Vec<VariableInfo>,
    pub selected_variable: Option<usize>,
    pub field_data: Option<FieldData>,
    /// Index of the time dimension (-1 means none detected).
    pub time_steps: Option<usize>,
    pub current_time: usize,
    /// Number of vertical levels (None if not detected).
    pub level_count: Option<usize>,
    /// Coordinate grid information.
    pub grid: GridInfo,
}

/// 2D field data ready for rendering.
#[derive(Debug, Clone)]
pub struct FieldData {
    pub values: Vec<f32>,
    pub width: usize,
    pub height: usize,
    pub min: f32,
    pub max: f32,
}

/// Central data store for the application.
#[derive(Debug, Default)]
pub struct DataStore {
    pub files: Vec<OpenFile>,
    pub active_file: Option<usize>,
}

// ---------------------------------------------------------------------------
// File extension filter for drag & drop
// ---------------------------------------------------------------------------

/// Accepted NetCDF file extensions.
const ACCEPTED_EXTENSIONS: &[&str] = &["nc", "nc4", "netcdf"];

/// Returns `Ok(())` if the path has an accepted NetCDF extension, otherwise an
/// error message suitable for display.
pub fn validate_netcdf_path(path: &Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ACCEPTED_EXTENSIONS.iter().any(|&a| a.eq_ignore_ascii_case(ext)) {
        Ok(())
    } else {
        Err(format!(
            "Unsupported file type '.{ext}'. Only .nc / .nc4 / .netcdf files are accepted."
        ))
    }
}

// ---------------------------------------------------------------------------
// Colormap inference (PRD 4.1)
// ---------------------------------------------------------------------------

/// Infer a recommended colormap family from variable metadata and data range.
///
/// Rules (evaluated in order):
/// 1. Name starts with `vor` / `div` / `d` → Diverging
/// 2. Name starts with `temp` or equals `T` with units containing `K` → Sequential
/// 3. min * max < 0 (data spans zero) → Diverging
/// 4. Otherwise → Sequential
pub fn infer_colormap(var: &VariableInfo, field: Option<&FieldData>) -> ColormapHint {
    let name_lower = var.name.to_ascii_lowercase();

    // Rule 1: vorticity / divergence style
    if name_lower.starts_with("vor")
        || name_lower.starts_with("div")
        || (name_lower.starts_with('d') && name_lower.len() <= 4)
    {
        return ColormapHint::Diverging;
    }

    // Rule 2: temperature
    if name_lower.starts_with("temp")
        || (var.name == "T"
            && var
                .units
                .as_ref()
                .is_some_and(|u| u.contains('K')))
    {
        return ColormapHint::Sequential;
    }

    // Rule 3: data spans zero
    if let Some(f) = field {
        if f.min * f.max < 0.0 {
            return ColormapHint::Diverging;
        }
    }

    ColormapHint::Sequential
}

// ---------------------------------------------------------------------------
// Dimension detection helpers
// ---------------------------------------------------------------------------

const TIME_NAMES: &[&str] = &["time", "t"];
const LEVEL_NAMES: &[&str] = &["level", "lev", "z", "sigma"];
const LON_NAMES: &[&str] = &["lon", "longitude", "x"];
const LAT_NAMES: &[&str] = &["lat", "latitude", "y"];

fn is_name_in(name: &str, candidates: &[&str]) -> bool {
    let lower = name.to_ascii_lowercase();
    candidates.iter().any(|&c| c == lower)
}

/// Find the position and size of a named dimension in a variable's dimension list.
fn find_dim(dims: &[(String, usize)], candidates: &[&str]) -> Option<(usize, usize)> {
    dims.iter()
        .enumerate()
        .find(|(_, (name, _))| is_name_in(name, candidates))
        .map(|(pos, (_, size))| (pos, *size))
}

// ---------------------------------------------------------------------------
// DataStore implementation
// ---------------------------------------------------------------------------

impl DataStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_file(&mut self, path: &Path) -> Result<(), String> {
        // D&D filtering
        validate_netcdf_path(path)?;

        let path_str = path.display().to_string();
        log::info!("Opening file: {path_str}");

        let file = netcdf::open(path)
            .map_err(|e| format!("Failed to open NetCDF file: {e}"))?;

        let mut variables = Vec::new();
        for var in file.variables() {
            let long_name = var
                .attribute_value("long_name")
                .and_then(|v| v.ok())
                .and_then(|v| match v {
                    netcdf::AttributeValue::Str(s) => Some(s),
                    _ => None,
                });
            let units = var
                .attribute_value("units")
                .and_then(|v| v.ok())
                .and_then(|v| match v {
                    netcdf::AttributeValue::Str(s) => Some(s),
                    _ => None,
                });
            let dimensions = var
                .dimensions()
                .iter()
                .map(|d| (d.name().to_string(), d.len()))
                .collect();

            variables.push(VariableInfo {
                name: var.name().to_string(),
                long_name,
                units,
                dimensions,
            });
        }

        // Detect time dimension from any variable that has one
        let time_steps = variables
            .iter()
            .filter_map(|v| find_dim(&v.dimensions, TIME_NAMES).map(|(_, size)| size))
            .next();

        // Detect level dimension
        let level_count = variables
            .iter()
            .filter_map(|v| find_dim(&v.dimensions, LEVEL_NAMES).map(|(_, size)| size))
            .next();

        // Read coordinate values
        let grid = GridInfo {
            lon: read_coord_var(&file, LON_NAMES),
            lat: read_coord_var(&file, LAT_NAMES),
        };

        let open_file = OpenFile {
            path: path_str,
            variables,
            selected_variable: None,
            field_data: None,
            time_steps,
            current_time: 0,
            level_count,
            grid,
        };

        self.files.push(open_file);
        self.active_file = Some(self.files.len() - 1);
        Ok(())
    }

    /// Load a 2D slice of data for the selected variable.
    /// Uses the first time step and first level if the variable has those dimensions.
    pub fn load_field(&mut self, file_idx: usize, var_idx: usize) -> Result<(), String> {
        self.load_field_at(file_idx, var_idx, 0, 0)
    }

    /// Load a 2D slice at the given time and level indices.
    pub fn load_field_at(
        &mut self,
        file_idx: usize,
        var_idx: usize,
        time_idx: usize,
        level_idx: usize,
    ) -> Result<(), String> {
        let file_entry = &self.files[file_idx];
        let var_info = &file_entry.variables[var_idx];

        let file = netcdf::open(&file_entry.path)
            .map_err(|e| format!("Failed to reopen file: {e}"))?;

        let var = file
            .variable(&var_info.name)
            .ok_or_else(|| format!("Variable '{}' not found", var_info.name))?;

        let dims = &var_info.dimensions;
        let ndim = dims.len();

        if ndim < 2 {
            return Err("Variable must have at least 2 dimensions".to_string());
        }

        // Last two dimensions are spatial (lat, lon)
        let height = dims[ndim - 2].1;
        let width = dims[ndim - 1].1;

        // Build an extents slice for netcdf reading.
        // Leading dimensions are indexed by time_idx / level_idx as appropriate.
        let time_pos = find_dim(dims, TIME_NAMES).map(|(p, _)| p);
        let level_pos = find_dim(dims, LEVEL_NAMES).map(|(p, _)| p);

        // Construct start/count vectors
        let mut start = vec![0usize; ndim];
        let mut count: Vec<usize> = dims.iter().map(|(_, s)| *s).collect();

        if let Some(tp) = time_pos {
            start[tp] = time_idx;
            count[tp] = 1;
        }
        if let Some(lp) = level_pos {
            start[lp] = level_idx;
            count[lp] = 1;
        }

        // Ensure spatial dimensions are fully read
        start[ndim - 2] = 0;
        count[ndim - 2] = height;
        start[ndim - 1] = 0;
        count[ndim - 1] = width;

        let extents: Vec<std::ops::Range<usize>> = start
            .iter()
            .zip(count.iter())
            .map(|(&s, &c)| s..s + c)
            .collect();

        let values_f64: Vec<f64> = var
            .get_values(extents.as_slice())
            .map_err(|e| format!("Failed to read data: {e}"))?;

        let values: Vec<f32> = values_f64.iter().map(|&v| v as f32).collect();

        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        let file_entry = &mut self.files[file_idx];
        file_entry.selected_variable = Some(var_idx);
        file_entry.current_time = time_idx;
        file_entry.field_data = Some(FieldData {
            values,
            width,
            height,
            min,
            max,
        });

        Ok(())
    }

    pub fn active_field(&self) -> Option<&FieldData> {
        let file = self.files.get(self.active_file?)?;
        file.field_data.as_ref()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Try to read a 1D coordinate variable by checking several candidate names.
fn read_coord_var(file: &netcdf::File, candidates: &[&str]) -> Option<Vec<f64>> {
    for &name in candidates {
        if let Some(var) = file.variable(name) {
            if let Ok(vals) = var.get_values::<f64, _>(..) {
                return Some(vals);
            }
        }
    }
    None
}
