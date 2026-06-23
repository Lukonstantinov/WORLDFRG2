# WorldForge 2 → Victoria 2-style UI / UX Redesign

> **Scope:** *visual & interaction design only.* This proposes the screens,
> layout, components and flows that give WorldForge 2 a Victoria 2 look and feel.
> It designs **chrome over the existing canvas and data** — it does **not**
> design new backend systems. Visual mockup:
> [`docs/mockups/victoria2-redesign.html`](mockups/victoria2-redesign.html).
>
> **Questions asked:** *"Make it more like Victoria 2. Is it possible? Can Claude
> design the whole app?"* → **Yes**, as a UI/UX layer: the campaign sim already
> produces the data a Victoria-style HUD needs (social strata, treasury/tariff,
> wars, prestige-like scores, market prices). We re-present it; we don't rebuild
> the simulation.

---

## 1. The idea in one line
Keep the world generator exactly as it is. Add a **"Play this world"** mode that
swaps the generator chrome for a grand-strategy **Country HUD** — one unchanged
map canvas, two skins.

## 2. Layout: before → after
- **Today (generator):** `WorkflowPanel (left) | Map | Toolbar (right) | StatusBar`.
- **Playing (HUD):** the map goes **full-bleed**; a **country top-bar** sits on
  top (flag, government, Great-Power rank, prestige/industry/military, treasury +
  yearly balance, date + speed). A **tab drawer** overlays the left; a province
  **inspector** slides in from the right; a thin **time/alert bar** sits at the
  bottom. No map re-engineering — only chrome.

## 3. The Country screen — eight tabs (mirrors Victoria 2's main screen)
`Budget · Population · Politics · Technology · Production · Trade · Diplomacy · Military`
- **Budget** — class tax sliders (poor/middle/rich), tariffs, spending sliders
  (education/military/admin/social) with a live coloured ledger.
- **Population** — pop list as proportional bars (farmers, labourers, craftsmen,
  clerks, capitalists, aristocrats…) + a **pop card** with life/everyday/luxury
  **needs meters** and a militancy meter.
- **Politics** — **upper-house** party-support bars + a **reform list** (status
  dot + unlock requirement). Production/Trade/Diplomacy/Military reuse existing
  hub/house/war panels reframed under their tabs.

## 4. Map modes (the biggest "feel" win, cheapest to build)
Recolour passes on the existing `OverlayManager` visibility registry — no new
render pipeline:
`Political · Population · Culture/Religion · RGO/Production · Militancy/Unrest · Sphere/Diplomacy`.
A map-mode icon strip replaces the layer selector; choice is remembered per
session.

## 5. Design tokens & component kit
Reuse the app theme (`bg #0b1420 · panel #13202e · text #cfe2f6 · accent
#3a80c0`; good `#4cae7a`, warn `#d9a441`, bad `#c0573a`, gold `#d8b24a`).
New reusable components, all extending the existing panel style (HubPanel,
HousesPanel, CoinCreditPanel): **stat pill**, **budget slider**, **pop bar**,
**needs meter**, **reform row**, **ledger row**, **map-mode chip**.

## 6. Interaction flows
- **Steer a country:** drag a budget slider → live ledger preview → confirm;
  hover any number for a breakdown tooltip.
- **Inspect a province:** click map → right inspector with pops/RGO/militancy →
  click a pop bar → needs card.
- **Pass a reform:** Politics tab → requirement shown → support crosses the line
  → row lights up → Enact with a militancy-impact preview.

## 7. Rollout — UI only, staged on existing screens
1. **HUD shell** — top-bar + tab drawer + full-bleed skin, toggled by "Play this
   world" (reads the existing campaign snapshot, no new data).
2. **Map-mode strip** — political/population/militancy/production recolours.
3. **Population & pop cards** — render `Society` strata as pop bars + needs meters.
4. **Budget & Politics tabs** — sliders + reform rows wired to existing polis
   policy / `CityFinance`.
5. **Diplomacy & Great-Power bar** — scoreboard + relations over existing
   war/economy data.

## 8. Can Claude design/build it?
- **Design:** yes — this document + the HTML mockup are the design.
- **Build:** yes, incrementally (steps 1→5), each a self-contained front-end
  change shipping its own HTML report (per the project's standing rule).
- **Honest limits:** the live Tauri window needs your machine to view (a headless
  box only runs the type-checks), so verification here is via the mockups +
  `npx tsc --noEmit`. Deeper Victoria *mechanics* — real POP objects, a tech
  tree, full diplomacy/military — are a **separate systems track**, intentionally
  out of scope for this UI-only proposal.

**Recommended first step:** build the **HUD shell + map-mode strip** (rollout 1–2)
— it delivers the Victoria 2 feel immediately on data that already exists.
