/** Panel SCHEMATIC harness — renders the REAL panel components against a stubbed
 *  IPC layer, so what you see is the shipping code and not a mockup (CLAUDE.md §2.2
 *  forbids mockups precisely because they drift from the app).
 *
 *  It is also the closest thing the frontend has to a test: SCOREBOARD lists "the
 *  entire frontend, 33k lines, zero tests" as unmeasured, and this at least proves
 *  these panels MOUNT and RENDER against realistic payloads.
 *
 *  Run:  npx vite --config dev/vite.schematic.ts
 *  Shot: node dev/shoot.mjs
 */
import React from "react";
import { createRoot } from "react-dom/client";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { HouseStandingView, FeudsView } from "@ui/campaign/HouseDossier";
import { ProvinceInspector } from "@ui/world/ProvinceInspector";
import { ProvinceMiniMap } from "@ui/world/ProvinceMiniMap";
import type { FeudRow, HouseStability, ProvinceLand, Province } from "@types";

// ── Stub the Tauri IPC bridge. The panels call `invoke` through @bridge; shimming
//    the internal transport means the components themselves run UNMODIFIED. ──
const YEARS = 180;
const hist = Array.from({ length: YEARS }, (_, i) => {
  const t = i / (YEARS - 1);
  return {
    year: 120 + i,
    rural: 41_000 + 18_000 * t - 6_000 * Math.sin(t * 7),
    urban: 9_000 + 26_000 * t,
    // The whole point of the layer: woodland falls, arable rises, soil wears.
    forest: 0.62 - 0.34 * t,
    arable: 0.14 + 0.29 * t,
    pasture: 0.13 + 0.04 * t,
    irrigated: t > 0.55 ? 0.25 : 0,
    soil: 0.86 - 0.30 * t + 0.05 * Math.sin(t * 9),
    unrest: Math.max(0, 0.10 + 0.45 * t - 0.3 * Math.sin(t * 5)),
    surplus: 5_200 + 3_400 * t - 2_600 * Math.abs(Math.sin(t * 6)),
  };
});
const last = hist[hist.length - 1];

const LAND: ProvinceLand = {
  id: 7,
  forest: last.forest, arable: last.arable, pasture: last.pasture,
  irrigated: last.irrigated, waste: 1 - last.forest - last.arable - last.pasture,
  soil: last.soil, unrest: last.unrest,
  rural: last.rural, rural_cap: 58_000, urban: last.urban,
  saturation: last.rural / 58_000,
  surplus: last.surplus, revenue: 742, arrears: 318,
  tax_rate: 0.19, tax_max: 0.35,
  tenure: [0.21, 0.38, 0.09, 0.32],
  holders: [
    { house: 3, name: "House Vareni", color: "#d1603d", estates: 4 },
    { house: 11, name: "House Okkath", color: "#3d8dd1", estates: 1 },
  ],
  holder_hub: 2, holder_name: "Ostrahn",
  works: [
    { kind: 2, label: "irrigation", progress: 0.62, years_left: 3, yearly_cost: 55,
      funder: "Ostrahn", stalled: false },
    { kind: 3, label: "road", progress: 0.15, years_left: 4.2, yearly_cost: 45,
      funder: "Ostrahn", stalled: true },
  ],
  history: hist, 
  events: [
    { year: 297, kind: "revolt", text: "The countryside rose against Ostrahn — dues went uncollected" },
    { year: 288, kind: "irrigation", text: "irrigation completed in the Ostrahn country" },
    { year: 271, kind: "dearth", text: "A failed harvest in the Ostrahn country — the land fed no-one but itself" },
    { year: 264, kind: "tax", text: "Dues raised to 19% of the surplus in the Ostrahn country" },
    { year: 236, kind: "clearance", text: "clearance completed in the Ostrahn country" },
  ],
};

const STABILITY: HouseStability = {
  idx: 3, name: "House Vareni",
  gauges: [
    { key: "solvency", label: "Solvency", score: 0.18, pips: 2,
      phrase: "in the red 7 of 12 months — ruin in 5", warn: true },
    { key: "liquidity", label: "Liquidity", score: 0.22, pips: 2,
      phrase: "4 months of runway", warn: true },
    { key: "exposure", label: "Exposure", score: 0.41, pips: 3,
      phrase: "78% of its living from silk", warn: false },
    { key: "succession", label: "Succession", score: 0.24, pips: 2,
      phrase: "head 41 years in, late in life", warn: true },
    { key: "cohesion", label: "Cohesion", score: 0.74, pips: 4,
      phrase: "holds together", warn: false },
  ],
  monthly_burn: 41.6, liquid: -88.2,
  liabilities: [
    { label: "Warehouse upkeep", amount: 18.4, note: "3 depots" },
    { label: "Fleet upkeep", amount: 14.7, note: "6 sea · 0 river · 2 caravan" },
    { label: "Estate depots", amount: 5.2, note: "5 holdings" },
    { label: "Office rents", amount: 3.3, note: "4 leased" },
    { label: "Bank debt outstanding", amount: 640.0, note: "principal still to repay" },
    { label: "Lent to city treasuries", amount: 1180.0,
      note: "2 borrowers at war — a default would land here" },
    { label: "Contract penalties at risk", amount: 96.0, note: "3 live contracts" },
  ],
  liabilities_total: 1957.6,
  debt_months: 7, debt_limit: 12,
  top_good_share: 0.78, top_good: "silk",
  top_city_share: 0.54, top_city: "Ostrahn",
  head_years: 41, head_span_years: 44,
  feuds_live: 3, feuds_hot: 2,
};

const FEUDS: FeudRow[] = [
  { a: 3, b: 11, a_name: "House Vareni", b_name: "House Okkath",
    a_color: "#d1603d", b_color: "#3d8dd1",
    cause: "the same trade", stage: "vendetta", stage_idx: 3, intensity: 0.91,
    good: "silk", city: "Ostrahn", started_year: 264, years: 33, flares: 22,
    damage_a: 412.5, damage_b: 96.0, outcome: "running", running: true, ended_year: 0,
    log: [
      { year: 296, stage: "vendetta", loser: "House Vareni", cost: 61.2,
        text: "House Okkath forces House Vareni's counting-house in Kelmar to close" },
      { year: 293, stage: "vendetta", loser: "House Vareni", cost: 44.8,
        text: "House Okkath has a ship of House Vareni taken at sea" },
      { year: 289, stage: "trade war", loser: "House Vareni", cost: 30.1,
        text: "House Okkath bars House Vareni from the market of Ostrahn" },
    ] },
  { a: 3, b: 5, a_name: "House Vareni", b_name: "House Sedhri",
    a_color: "#d1603d", b_color: "#6ec06a",
    cause: "a contested council", stage: "trade war", stage_idx: 2, intensity: 0.66,
    good: "dyestuff", city: "Kelmar", started_year: 281, years: 16, flares: 9,
    damage_a: 88.0, damage_b: 121.4, outcome: "running", running: true, ended_year: 0,
    log: [
      { year: 295, stage: "trade war", loser: "House Sedhri", cost: 22.6,
        text: "House Vareni drives House Sedhri out of the Kelmar trade" },
    ] },
  { a: 3, b: 8, a_name: "House Vareni", b_name: "House Tavric",
    a_color: "#d1603d", b_color: "#c9a24a",
    cause: "a broken match", stage: "open feud", stage_idx: 1, intensity: 0.34,
    good: "", city: "Ostrahn", started_year: 290, years: 7, flares: 3,
    damage_a: 12.0, damage_b: 19.8, outcome: "running", running: true, ended_year: 0, log: [] },
  { a: 5, b: 11, a_name: "House Sedhri", b_name: "House Okkath",
    a_color: "#6ec06a", b_color: "#3d8dd1",
    cause: "a closed market", stage: "trade war", stage_idx: 2, intensity: 0.12,
    good: "amber", city: "Kelmar", started_year: 240, years: 28, flares: 14,
    damage_a: 240.0, damage_b: 61.0, outcome: "arbitrated", running: false, ended_year: 268,
    log: [{ year: 268, stage: "trade war", loser: "House Sedhri", cost: 0,
            text: "The council of Kelmar imposes a settlement" }] },
  { a: 8, b: 11, a_name: "House Tavric", b_name: "House Okkath",
    a_color: "#c9a24a", b_color: "#3d8dd1",
    cause: "the same trade", stage: "cold rivalry", stage_idx: 0, intensity: 0.05,
    good: "furs", city: "", started_year: 250, years: 12, flares: 2,
    damage_a: 8.0, damage_b: 4.0, outcome: "sealed by marriage", running: false, ended_year: 262,
    log: [] },
];

const PROVINCE: Province = {
  id: 7, name: "Vaskeld", seat_x: 96, seat_y: 60, cells: 812, area_km2: 41_200,
  island: 0, neighbors: [4, 9, 12],
  koppen: 18, elevation_class: 1, mean_fertility: 0.58, coastal: true,
  goods: [{ good: 12, quality: 0.86, rank: 3, of: 41 }, { good: 4, quality: 0.62, rank: 0, of: 0 }],
  culture: "Ashkar", rural_pop: 44_000,
  analog: "the Baltic hinterland — cool, wooded, river-threaded",
  settlements: ["s1", "s2", "s3"],
  koppen_shares: [[18, 0.61], [17, 0.24], [26, 0.15]],
  elev_min_m: 4, elev_mean_m: 310, elev_max_m: 1240, relief_m: 1236,
  temp_mean: 7.4, precip_mean: 690, season_amp: 11.2, arid_frac: 0.04,
  disease_mean: 0.11, coast_cells: 62, river_cells: 148, navigable_river: true,
  lake_cells: 21,
  culture_shares: [["Ashkar", 0.72], ["Velin", 0.19], ["Okkath", 0.09]],
  food_capacity: 58_000, rural_cap: 58_000,
  neighbors_detail: [
    { neighbor: 4, cells: 43, kind: 1 },
    { neighbor: 9, cells: 28, kind: 2 },
    { neighbor: 12, cells: 9, kind: 0 },
  ],
};

// A blobby province footprint for the plate.
const GW = 200, GH = 140;
const raster = new Uint32Array(GW * GH).fill(0xffffffff);
for (let y = 0; y < GH; y++) {
  for (let x = 0; x < GW; x++) {
    const dx = (x - 96) / 46, dy = (y - 62) / 34;
    const wobble = 0.24 * Math.sin(x * 0.19) + 0.18 * Math.cos(y * 0.23);
    if (dx * dx + dy * dy < 1 + wobble) raster[y * GW + x] = 7;
  }
}

const RESP: Record<string, unknown> = {
  campaign_house_stability: STABILITY,
  campaign_get_feuds: FEUDS,
  campaign_province_land: LAND,
  campaign_province_land_all: [LAND],
  campaign_province_state: [{ id: 7, rural_pop: LAND.rural, urban_pop: LAND.urban, hub_count: 3, net_migration: -820 }],
  campaign_province_detail: {
    id: 7, rural_pop: LAND.rural, urban_pop: LAND.urban, net_migration: -820,
    settlements: [
      { name: "Ostrahn", x: 96, y: 60, population: 21_400, seat: true, hub_class: 2, dev_tier: 3 },
      { name: "Kelmar", x: 74, y: 74, population: 9_100, seat: false, hub_class: 1, dev_tier: 2 },
      { name: "Vist", x: 118, y: 48, population: 2_300, seat: false, hub_class: 0, dev_tier: 1 },
    ],
    buildings: [
      { kind: 0, name: "Vareni silk estate", x: 88, y: 66, stats: [["Good", "silk"], ["Quality", "86%"]].map(([label, value]) => ({ label, value })) },
      { kind: 1, name: "Ostrahn dyeworks", x: 99, y: 57, stats: [["Output", "cloth"]].map(([label, value]) => ({ label, value })) },
      { kind: 3, name: "Banco Vareni", x: 95, y: 61, stats: [["Reserves", "1.2k"]].map(([label, value]) => ({ label, value })) },
      { kind: 2, name: "Kelmar depot", x: 76, y: 72, stats: [["Capacity", "800"]].map(([label, value]) => ({ label, value })) },
    ],
  },
};
(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  transformCallback: (cb: unknown) => cb,
  invoke: (cmd: string) => Promise.resolve(RESP[cmd] ?? null),
};

useWorldStore.setState({
  provinces: [PROVINCE],
  provinceRaster: { data: raster, w: GW, h: GH, gridW: GW, gridH: GH },
  settlements: [],
  meta: { grid_width: GW, grid_height: GH } as never,
});
useUIStore.setState({ showProvinceInspector: true, selectedProvince: 7 });

function Slot({ label, children }: { label: string; children: React.ReactNode }) {
  return <div><div className="cap">{label}</div><div className="slot">{children}</div></div>;
}

/** The dossier views are normally hosted inside HouseDetail's subtabs; the harness
 *  gives them the same bronze card so the schematic matches what ships. */
function DossierCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div data-draggable style={{
      position: "absolute", width: 460, background: "rgba(28,20,12,0.97)",
      border: "1px solid #2a3a4e", borderRadius: 8, padding: "9px 11px",
      font: "12px/1.4 system-ui, sans-serif", color: "#cfe0f4",
      boxShadow: "0 8px 30px rgba(0,0,0,.5)",
    }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 2 }}>
        <span style={{ width: 10, height: 10, borderRadius: 2, background: "#d1603d", alignSelf: "center" }} />
        <span style={{ fontSize: 11 }}>🏦</span>
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13 }}>House Vareni</span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#7090b0", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>
      <div style={{ color: "#9ab0c8", fontSize: 10 }}>Ilvar Vareni · of Ostrahn · gen 4</div>
      <div style={{ color: "#c9a227", fontSize: 11, fontWeight: 700 }}>wealth −88.2</div>
      <div style={{ display: "flex", gap: 4, margin: "7px 0 5px", borderBottom: "1px solid #1a2a3e" }}>
        {["Summary", "⚖ Standing", "⚔ Feuds (3)", "🏦 Bank", "📒 Accountant"].map((t) => (
          <div key={t} style={{
            fontSize: 10, padding: "3px 9px",
            color: t === title ? "#e8dcc0" : "#7090b0", fontWeight: t === title ? 700 : 400,
            borderBottom: t === title ? "2px solid #c9a227" : "2px solid transparent",
          }}>{t}</div>
        ))}
      </div>
      {children}
    </div>
  );
}

/** CAMPAIGN TIMELINE — the same province plate at several years of one campaign.
 *  This is the thing the layer exists to make visible: at year 120 the country is
 *  wooded and the soil is in good heart; by year 300 the wood has been cleared for
 *  the plough, the soil is worn, and the tenure has passed to the houses. Before the
 *  land layer these four plates would have been pixel-identical. */
function Timeline() {
  const shots = [0, 60, 120, YEARS - 1].map((i) => hist[i]);
  const mini = RESP.campaign_province_detail as { settlements: never[]; buildings: never[] };
  return (
    <div>
      <div className="cap">Campaign timeline · one province, four moments (land use plate)</div>
      <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
        {shots.map((sm) => (
          <div key={sm.year} style={{ width: 210 }}>
            <div style={{ color: "#e6f2ea", fontSize: 12, fontWeight: 600, marginBottom: 3 }}>
              year {sm.year}
            </div>
            <ProvinceMiniMap
              province={PROVINCE}
              raster={{ data: raster, w: GW, h: GH, gridW: GW, gridH: GH }}
              settlements={mini.settlements}
              buildings={[]}
              land={LAND}
              sample={sm}
              plates={["landuse", "water", "borders"]}
              width={200}
              riverCells={PROVINCE.river_cells}
            />
            <div style={{ color: "#9ab0c8", fontSize: 11, marginTop: 4, lineHeight: 1.35 }}>
              wood {Math.round(sm.forest * 100)}% · crop {Math.round(sm.arable * 100)}%<br />
              soil {sm.soil.toFixed(2)} · unrest {Math.round(sm.unrest * 100)}%<br />
              rural {Math.round(sm.rural).toLocaleString()} · urban {Math.round(sm.urban).toLocaleString()}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(
  <div className="stage">
    <Slot label="House Dossier · ⚖ Standing">
      <DossierCard title="⚖ Standing"><HouseStandingView idx={3} /></DossierCard>
    </Slot>
    <Slot label="House Dossier · ⚔ Feuds">
      <DossierCard title="⚔ Feuds (3)"><FeudsView house={3} /></DossierCard>
    </Slot>
    <Slot label="Province Inspector · Land"><ProvinceInspector /></Slot>
    <div style={{ flexBasis: "100%" }}><Timeline /></div>
  </div>,
);
