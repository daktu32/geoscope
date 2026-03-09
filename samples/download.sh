#!/bin/bash
# Download full-resolution sample NetCDF files from GitHub Release.
# Usage: ./samples/download.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Downloading sample data to ${SCRIPT_DIR}/ ..."

if command -v gh &> /dev/null; then
    gh release download v0.4-beta -R daktu32/geoscope -p "*.nc" -D "$SCRIPT_DIR" --clobber
else
    echo "Error: GitHub CLI (gh) not found. Install it: https://cli.github.com/"
    echo ""
    echo "Or download manually from:"
    echo "  https://github.com/daktu32/geoscope/releases/tag/v0.4-beta"
    exit 1
fi

echo ""
ls -lh "$SCRIPT_DIR"/*.nc
echo ""
echo "Done! Try: cargo run --release -- samples/rossby_haurwitz.nc"
