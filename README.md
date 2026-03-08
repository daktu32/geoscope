# GeoScope

<p align="center"><img src="docs/logo.svg" width="320" alt="GeoScope logo"></p>

<p align="center"><strong>Open. See. Explore.</strong> — GFD Data Visualization, Reimagined.</p>

<p align="center">
<img src="https://img.shields.io/badge/status-v0.2%20active-2AA198" alt="Status">
<img src="https://img.shields.io/badge/language-Rust-orange" alt="Language">
<img src="https://img.shields.io/badge/license-TBD-lightgrey" alt="License">
</p>

---

A next-generation GFD (Geophysical Fluid Dynamics) data visualization desktop app.
Drop a NetCDF file, get an instant 3D globe with smart variable inference.
A modern replacement for GrADS / Panoply / ncview.

## Features

### Implemented

- **Globe View** — wgpu 3D sphere with data texture, camera rotation & zoom, atmospheric glow
- **Map View** — Equirectangular & Mollweide projections, pan & zoom
- **Hovmoller Diagram** — Time-longitude heatmap
- **Cross-Section** — Level x Lat/Lon vertical slice heatmap
- **E(n) Spectrum** — Log-log energy spectrum plot
- **Vector Overlay** — Wind arrows on Globe/Map with auto u/v detection
- **5 Colormaps** — Viridis, RdBu_r, Plasma, Inferno, Coolwarm
- **3-Stage Variable Inference** — CF standard_name → name heuristics → dimension structure
- **Time Animation** — Play/pause with adjustable speed (1–60 fps)
- **Level Selection** — Vertical level slider for 3D data
- **Colormap Range Control** — Slice / Global / Manual scaling modes
- **Multi-File Support** — Open multiple NetCDF files, switch between them
- **PNG Export** — Save current view with file dialog
- **Drag & Drop** — Drop NetCDF files to open instantly
- **Dark Theme UI** — Custom-styled egui panels with docking layout

### Planned

- Code Panel (GUI actions → reproducible Python/Rhai scripts)
- LLM Copilot (physical explanations, natural language commands)
- Comparison mode (side-by-side datasets)
- WebAssembly + WebGPU browser version

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| GUI | eframe + egui_dock | 0.33 / 0.18 |
| Rendering | wgpu | 27 |
| Data I/O | netcdf-rs | 0.12 |
| PNG Export | image | 0.25 |
| File Dialog | rfd | 0.15 |
| HDF5 | hdf5 (Homebrew) | 1.10 |

## Quick Start

```bash
# Prerequisites: Rust toolchain, HDF5 1.10 (brew install hdf5@1.10)
cargo run -- path/to/data.nc
```

Or launch without arguments and use the **Open** button in the Data Browser.

## Roadmap

| Version | Goal | Status |
|---------|------|--------|
| **v0.1a** | Tech PoC: wgpu + egui + globe rendering | Done |
| **v0.1b** | Usable minimum: Map, Hovmoller, Spectrum, Export | Done |
| **v0.2 P1** | Mollweide, Cross-Section, Vector Overlay | Done |
| **v0.2 P2** | Level slider, Range control, Multi-file, Colormaps | Done |
| **v0.3** | Code Panel, LLM Copilot | Planned |
| **Future** | Browser version (WebAssembly + WebGPU) | Planned |

## Relationship to dcmodel

GeoScope is part of the [dcmodel](https://www.gfd-dennou.org/) family — numerical models and libraries for GFD developed by the GFD Dennou Club.

```
ispack-rs (spectral transforms)
    |
spmodel-rs (spectral models)
    |
GeoScope (visualization) <-- you are here
```

Primary use case: visualizing output from ispack-rs / spmodel-rs.
Any CF-compliant NetCDF data is supported.

## Documentation

- [`docs/PRD.md`](docs/PRD.md) — Product Requirements Document (v0.3.0)
- [`docs/DESIGN_REVIEW.md`](docs/DESIGN_REVIEW.md) — Expert panel design review
- [`docs/MOCKUP_GUIDE.md`](docs/MOCKUP_GUIDE.md) — Mockup screen map & persona validation
- [`PROGRESS.md`](PROGRESS.md) — Development progress log

## Acknowledgments

- [GFD Dennou Club](https://www.gfd-dennou.org/) — the research community behind dcmodel
- [ISPACK](https://www.gfd-dennou.org/arch/ispack/) by K. Ishioka — the spectral transform library that started it all
