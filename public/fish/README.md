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

**Mediterranean (Cs):** adriatic-trout · iberian-minnow · spined-loach ·
med-bullhead · iberian-barbel · southern-nase · med-chub · pardilla-roach ·
fartet-toothcarp · flathead-mullet · sand-smelt · estuary-eel

**Tropical:** emberscale-tetra · cascade-danio · stone-pleco · hillstream-loach ·
golden-pacu · sabertooth-payara · spotted-wolffish · tigerfish · king-arapaima ·
emperor-cichlid · whiskered-redtail · silver-arowana

**Boreal / subarctic:** silverpike-taimen · broad-whitefish · arctic-grayling ·
alpine-bullhead · boreal-burbot · northern-pike · amur-ide · siberian-dace ·
blackfin-char · sheefish-inconnu · arctic-cisco · sterlet-sturgeon

**Arid / desert:** wadi-killifish · relict-desert-trout · spring-dace ·
desert-chub · desert-barb · oasis-tilapia · desert-snakehead · sailfin-molly ·
sand-catfish · saltcreek-pupfish · wadi-mullet · desert-goby
