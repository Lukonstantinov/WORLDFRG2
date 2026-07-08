# Fish species illustrations

Drop generated species plates here as PNGs named by the species **slug**:

```
public/fish/silverfin-trout.png
public/fish/frostscale-grayling.png
public/fish/goldvein-barbel.png
...
```

The Hydrology panel loads `/fish/<slug>.png` for each signature species on a
river reach and falls back to a placeholder tile when the file is absent, so the
app works with or without the artwork.

Recommended: transparent or dark-charcoal background, side profile facing left,
~600×340 px (matches the panel's ~16:9 plate). Slugs are defined in
`src-tauri/src/sim/aquatic.rs` (`SIGNATURE_FISH`). Current slugs:

**Temperate:** silverfin-trout · frostscale-grayling · stonecling-bullhead ·
torrent-loach · ribbon-dace · redflank-char · goldvein-barbel · bronze-chub ·
sailback-asp · whiskered-wels · marbled-perch · gravel-nase · broadscale-bream ·
marsh-carp · reedwater-zander · silt-sturgeon · silverback-eel · tidewater-shad

**Tropical:** emberscale-tetra · golden-pacu · sabertooth-payara · king-arapaima ·
emperor-cichlid · whiskered-redtail

**Boreal / subarctic:** silverpike-taimen · broad-whitefish · arctic-grayling ·
boreal-burbot · northern-pike · blackfin-char

**Arid / desert:** wadi-killifish · desert-barb · oasis-tilapia · sand-catfish ·
saltcreek-pupfish · wadi-mullet
