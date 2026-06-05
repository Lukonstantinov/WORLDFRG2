# WorldForge 2 — Trade Cartography & Good-Flow Spec

Feature spec for the route/price/days visualization, directional trade flow, and
chokepoint refinement. Builds on the existing economy snapshot (`compute_economy`,
`EconChain`, `EconChokepoint`) and goods overlay (`compute_good_regions`).

Status: **approved scope** (decisions locked below). Not yet implemented.

---

## Locked decisions

| Area | Decision |
|---|---|
| Travel days | Mode **+ terrain**: caravan ~30, ship ~120, river ~40 km/day, slowed by mountains/desert via the existing coarse cost grid. `km/cell = 40075 / grid_width`. |
| Cargo value | **Derived**: value = volume × delivered price (desire × scarcity × transport). No new per-good field. |
| Direction | **Both directions** drawn; arrow direction flips **only at hubs/emporiums** (each hub→hub segment has one net direction). |
| Hubs | Highlighted; **adjustable size + intensity** via Toolbar overlay sliders. |
| Diversity | Ranked good list + **distinct-good count**; plus **two side-by-side vertical stacked columns** (one per direction, goods top→bottom by value). |
| Chokepoints | **Both** geographic straits (land-on-both-sides geometry) **and** emporium cities (high routed-volume pass-through). |
| Click-a-good | **Multi-line** price/days graph (all routes for that good). |
| Panel layout | Road **list + price graph together** in the main movable panel; **cargo two-column view opens as a small secondary popup** when a road is selected. |
| Graph x-axis | **Cumulative travel days** from origin, each hub a marked dot. |

---

## Feature 1 — Travel-days on routes

**Backend.** Add per-edge physical metrics to the trade graph. For each coarse path
segment, sum cell lengths (orthogonal = 1 cell, diagonal = √2), convert to km
(`× km_per_cell`), then to days per the segment's dominant mode:

```
mode      base km/day   terrain modifier
caravan   30            × cost-grid relief/desert factor (slower in mountains/desert)
river     40            (river cells, cheap & fast)
ship      120           coastal; × (1 + sea_hazard) slowdown for storms/reefs
```

Mode per segment is already derivable (`TradeRoute.kind`: 0 overland / 1 sea / 2 river).
Emit `length_km` and `days` on each `EconChain` stop (cumulative) and each leg.

**Frontend.** Show per-leg + total days on routes and in the graph axis.

## Feature 2 — Click-a-good → movable trade panel

**Trigger.** Goods are already drawn (`compute_good_regions`). Make a good's
region clickable on the map (hit-test against the good cell-mask) → open `GoodFlowPanel`.

**Panel (movable, draggable):**
- **Road list** — every route carrying this good (from economy `chains` filtered by
  good): columns = destination hub, total value, distinct-good count, total days,
  direction glyph.
- **Price/days graph** — multi-line (one line per route); x = cumulative days, y =
  price multiplier; hub dots; hover shows hub name + price + day count.
- Selecting a road → **highlights that road on the map** with **directional arrows**
  + **highlights its hubs**, and opens the **cargo popup**.
- **Cargo popup (secondary, small)** — two vertical stacked columns side by side:
  left = forward direction, right = reverse; each stacks the corridor's goods
  top→bottom by value, labelled with good icon + value; header shows distinct count.

## Feature 3 — Directional trade flow + value + diversity

**Backend.** The economy flow solver already walks `si → di` and accumulates per
coarse-edge volume. Extend the accumulation to also store, **per hub→hub graph edge**:
- directional value (`fwd_value`, `bwd_value`) = Σ amount × delivered price by direction,
- `by_good` value map (for diversity + cargo columns),
- distinct-good count.

Net direction per segment = sign(fwd_value − bwd_value); arrows rendered per
hub→hub edge so they only flip at hubs.

**Frontend.** Corridor arrows sized by total value; both directions available to the
cargo popup.

## Feature 4 — Chokepoints = straits AND emporiums

Replace the current "cluster busiest edges" classifier with two explicit detectors:
- **Straits** — scan sea coarse cells for a *narrow water gap*: land within a short
  radius on two opposing sides (N/S or E/W) with open water between. Score = routed
  volume threading the cell. Emit as `Strait`.
- **Emporiums** — hub cities whose **pass-through** routed volume (transit, not
  origin/destination) exceeds a threshold. Score = transit volume. Emit as `Emporium`.

Keep the volume ranking; drop the generic `Passage`/`Pass` mid-route artifacts.

## Adjustable hub display

Toolbar overlay section: **hub size** slider + **highlight intensity** slider (and
on/off), stored in `uiStore` (e.g. `hubDisplay: { size, intensity, on }`),
consumed by the overlay renderer.

---

## Build plan (phased)

1. **Backend metrics** — add `km_per_cell` helper; per-leg `length_km`/`days` on
   `EconChain`; directional value + `by_good` value + diversity per graph edge.
   (`commands/query_commands.rs`, `compute_economy`.)
2. **Chokepoint detectors** — strait geometry scan + emporium transit-volume;
   replace classifier. (`compute_economy`.)
3. **Bridge + types** — extend the economy types in `bridge/tauri.ts` / `types.ts`.
4. **GoodFlowPanel** — new movable panel (list + multi-line graph) + cargo popup.
   (`src/ui/GoodFlowPanel.tsx`.)
5. **Map interaction** — good-region hit-test → open panel; selected-road highlight
   with direction arrows + hub highlight. (`OverlayManager.ts`, `MapCanvas.tsx`.)
6. **Hub display controls** — Toolbar sliders + `uiStore` state + renderer wiring.

Phases 1–3 are backend/data (verifiable with `cargo check`); 4–6 are UI.
