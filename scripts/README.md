# scripts/

One-off developer tooling — **not** part of the app runtime or build. Assets they
generate are committed under `public/`, so you only re-run these when regenerating art.

| Script | Purpose | Output |
|--------|---------|--------|
| `gen_city_sprites.mjs` | Generate isometric city-building sprites | `public/city-sprites/*.svg` + `_peaks.json` |
| `gen_fish_gallery.mjs` | Build the fish reference gallery | fish art |
| `gen_fish_crops.mjs` | Crop fish source images | `public/fish/*.png` |
| `place_fish.ps1`, `crop.ps1`, `crop2.ps1` | Windows image-crop helpers (PowerShell) | cropped PNGs |
| `fish_catalogue.json` | Source data for the fish generators | (data) |

Run with `node scripts/<name>.mjs` (or PowerShell for `*.ps1`).
