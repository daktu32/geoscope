use std::path::Path;

pub mod inference;

/// Metadata for a single variable in a NetCDF file.
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub long_name: Option<String>,
    pub units: Option<String>,
    pub standard_name: Option<String>,
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
#[allow(dead_code)]
pub struct GridInfo {
    pub lon: Option<Vec<f64>>,
    pub lat: Option<Vec<f64>>,
}

/// Metadata and data for an opened NetCDF file.
#[derive(Debug)]
#[allow(dead_code)]
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

/// Axis for cross-section slicing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CrossSectionAxis {
    #[default]
    Latitude,
    Longitude,
}

/// 2D cross-section data: level vs lat or lon.
#[derive(Debug, Clone)]
pub struct CrossSectionData {
    pub values: Vec<f32>, // row-major [level][spatial]
    pub n_levels: usize,
    pub n_spatial: usize,
    pub min: f32,
    pub max: f32,
    pub axis: CrossSectionAxis,
}

/// Vector field data (u, v components) for overlay rendering.
#[derive(Debug, Clone)]
pub struct VectorFieldData {
    pub u_values: Vec<f32>,
    pub v_values: Vec<f32>,
    pub width: usize,
    pub height: usize,
    pub max_magnitude: f32,
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
#[allow(dead_code)]
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
            let standard_name = var
                .attribute_value("standard_name")
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
                standard_name,
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

    /// Load a cross-section: all levels at a fixed lat or lon index.
    pub fn load_cross_section(
        &self,
        file_idx: usize,
        var_idx: usize,
        time_idx: usize,
        axis: CrossSectionAxis,
        fixed_idx: usize,
    ) -> Result<CrossSectionData, String> {
        let file_entry = &self.files[file_idx];
        let var_info = &file_entry.variables[var_idx];

        let file = netcdf::open(&file_entry.path)
            .map_err(|e| format!("Failed to reopen file: {e}"))?;

        let var = file
            .variable(&var_info.name)
            .ok_or_else(|| format!("Variable '{}' not found", var_info.name))?;

        let dims = &var_info.dimensions;
        let ndim = dims.len();

        let level_dim = find_dim(dims, LEVEL_NAMES)
            .ok_or_else(|| "Variable has no level dimension".to_string())?;
        let (level_pos, n_levels) = level_dim;

        if ndim < 3 {
            return Err("Variable must have at least 3 dimensions (time/level/lat/lon)".to_string());
        }

        let n_lat = dims[ndim - 2].1;
        let n_lon = dims[ndim - 1].1;
        let time_pos = find_dim(dims, TIME_NAMES).map(|(p, _)| p);

        let n_spatial = match axis {
            CrossSectionAxis::Latitude => n_lon,   // fix lat, vary lon
            CrossSectionAxis::Longitude => n_lat,  // fix lon, vary lat
        };

        let mut all_values: Vec<f32> = Vec::with_capacity(n_levels * n_spatial);

        for lev in 0..n_levels {
            let mut start = vec![0usize; ndim];
            let mut count: Vec<usize> = vec![1; ndim];

            if let Some(tp) = time_pos {
                start[tp] = time_idx;
            }
            start[level_pos] = lev;

            match axis {
                CrossSectionAxis::Latitude => {
                    // fix lat, read all lon
                    start[ndim - 2] = fixed_idx;
                    count[ndim - 2] = 1;
                    start[ndim - 1] = 0;
                    count[ndim - 1] = n_lon;
                }
                CrossSectionAxis::Longitude => {
                    // fix lon, read all lat
                    start[ndim - 2] = 0;
                    count[ndim - 2] = n_lat;
                    start[ndim - 1] = fixed_idx;
                    count[ndim - 1] = 1;
                }
            }

            let extents: Vec<std::ops::Range<usize>> = start
                .iter()
                .zip(count.iter())
                .map(|(&s, &c)| s..s + c)
                .collect();

            let row: Vec<f64> = var
                .get_values(extents.as_slice())
                .map_err(|e| format!("Failed to read level {lev}: {e}"))?;

            all_values.extend(row.iter().map(|&v| v as f32));
        }

        let min = all_values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = all_values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        Ok(CrossSectionData {
            values: all_values,
            n_levels,
            n_spatial,
            min,
            max,
            axis,
        })
    }

    /// Load u and v vector field components at a given time and level.
    pub fn load_vector_field(
        &mut self,
        file_idx: usize,
        u_var_idx: usize,
        v_var_idx: usize,
        time_idx: usize,
        level_idx: usize,
    ) -> Result<VectorFieldData, String> {
        // Load u component
        self.load_field_at(file_idx, u_var_idx, time_idx, level_idx)?;
        let u_field = self.files[file_idx].field_data.clone()
            .ok_or_else(|| "Failed to load u field".to_string())?;

        // Load v component
        self.load_field_at(file_idx, v_var_idx, time_idx, level_idx)?;
        let v_field = self.files[file_idx].field_data.clone()
            .ok_or_else(|| "Failed to load v field".to_string())?;

        if u_field.width != v_field.width || u_field.height != v_field.height {
            return Err("u and v fields have different dimensions".to_string());
        }

        let max_magnitude = u_field.values.iter()
            .zip(v_field.values.iter())
            .map(|(&u, &v)| (u * u + v * v).sqrt())
            .fold(0.0f32, f32::max);

        Ok(VectorFieldData {
            u_values: u_field.values,
            v_values: v_field.values,
            width: u_field.width,
            height: u_field.height,
            max_magnitude,
        })
    }

    pub fn active_field(&self) -> Option<&FieldData> {
        let file = self.files.get(self.active_file?)?;
        file.field_data.as_ref()
    }

    /// Load all time steps for a variable at a specific latitude index,
    /// producing a [time][lon] array suitable for a Hovmoller diagram.
    pub fn load_hovmoller_data(
        &self,
        file_idx: usize,
        var_idx: usize,
        lat_idx: usize,
    ) -> Result<crate::renderer::hovmoller::HovmollerData, String> {
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

        let time_dim = find_dim(dims, TIME_NAMES);
        let n_time = time_dim.map(|(_, s)| s).unwrap_or(1);

        // Last two dims are (lat, lon)
        let n_lat = dims[ndim - 2].1;
        let n_lon = dims[ndim - 1].1;

        if lat_idx >= n_lat {
            return Err(format!(
                "lat_idx {lat_idx} out of range (n_lat = {n_lat})"
            ));
        }

        let level_pos = find_dim(dims, LEVEL_NAMES).map(|(p, _)| p);

        let mut all_values: Vec<f32> = Vec::with_capacity(n_time * n_lon);

        for t in 0..n_time {
            let mut start = vec![0usize; ndim];
            let mut count: Vec<usize> = vec![1; ndim];

            if let Some((tp, _)) = time_dim {
                start[tp] = t;
                count[tp] = 1;
            }
            if let Some(lp) = level_pos {
                start[lp] = 0;
                count[lp] = 1;
            }

            // lat: single index
            start[ndim - 2] = lat_idx;
            count[ndim - 2] = 1;
            // lon: all
            start[ndim - 1] = 0;
            count[ndim - 1] = n_lon;

            let extents: Vec<std::ops::Range<usize>> = start
                .iter()
                .zip(count.iter())
                .map(|(&s, &c)| s..s + c)
                .collect();

            let row: Vec<f64> = var
                .get_values(extents.as_slice())
                .map_err(|e| format!("Failed to read time step {t}: {e}"))?;

            all_values.extend(row.iter().map(|&v| v as f32));
        }

        let min = all_values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = all_values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        Ok(crate::renderer::hovmoller::HovmollerData {
            values: all_values,
            n_lon,
            n_time,
            min,
            max,
        })
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
