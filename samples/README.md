# Sample Data

Full-resolution sample NetCDF files are hosted on [GitHub Releases](https://github.com/daktu32/geoscope/releases/tag/v0.4-beta).

## Download

```bash
# Using the download script (requires GitHub CLI)
./samples/download.sh

# Or manually with gh
gh release download v0.4-beta -R daktu32/geoscope -p "*.nc" -D samples/
```

## Files

| File | Size | Description | Features demonstrated |
|------|------|-------------|----------------------|
| `rossby_haurwitz.nc` | 62 MB | Rossby-Haurwitz wave (Williamson test case 6), 1001 time steps | Globe, Map, contour, vector overlay, streamlines |
| `beta_gyre.nc` | 158 MB | Beta-plane gyre with trajectory tracking, 1001 time steps | Trajectory overlay, time animation |
| `held_suarez.nc` | 26 MB | Held-Suarez (1994) atmospheric benchmark, 10 sigma levels, 101 time steps | Cross-section, level selection, profile |

## What to Try

### rossby_haurwitz.nc
- Variables: `vort` (vorticity), `phi` (geopotential), `u_cos`, `v_cos` (wind)
- Grid: 128 x 64 (lon x lat), 1001 time steps
- Try: Globe view + contour overlay (`C`) + streamlines (`V`) + animation (`Space`)

### beta_gyre.nc
- Variables: `vort` (vorticity), `u_cos`, `v_cos` (wind), `gyre_lon`, `gyre_lat` (trajectory)
- Grid: 128 x 64 (lon x lat), 1001 time steps
- Try: Globe view + trajectory overlay (`T`) + time animation (`Space`)

### held_suarez.nc
- Variables: `vort` (vorticity), `temp` (temperature), `ucos`, `vcos` (wind), `lnps` (log surface pressure)
- Grid: 64 x 32 x 10 levels (lon x lat x lev), 101 time steps
- Try: Cross-section view (`6`) + level slider + profile view (`5`)

## Generate Your Own

Full-resolution data can also be generated with [spmodel-rs](https://github.com/daktu32/spmodel-rs):

```bash
cargo run --release --example rossby_haurwitz   # → output/rossby_haurwitz.nc
cargo run --release --example beta_gyre         # → output/beta_gyre.nc
cargo run --release --example held_suarez       # → output/held_suarez.nc
```
