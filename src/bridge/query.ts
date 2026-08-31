// Split from the former monolithic src/bridge/tauri.ts (invoke wrappers, one per Rust command).
import { invoke } from "@tauri-apps/api/core";
import type { CultureRegion, EconomySnapshot, GoodBeltMask, Itinerary, PoliticalCenter, ProvinceGoodMask, Settlement, SharkZone, TradeMatrix } from "@types";
import type { FisheryBank, OverlaysResult, TradeRoute } from "./types";

/** Compute trade routes between the current settlements (pass the store list).
 *  Rivers feed inland routes; `reach`/`maxCrossing` cap open-water crossings
 *  (reach: 0 = global, 1 = coastal+short, 2 = continental only). `desertRoutes`
 *  is the Silk-Road mode: caravans prefer overland steppe corridors & deserts
 *  when seas are dangerous. */
export async function computeTradeRoutes(
  settlements: { x: number; y: number; score: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
  piracy: number,
  season: number,
  months: number,
): Promise<TradeRoute[]> {
  return invoke("compute_trade_routes", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    piracy,
    season,
    months,
  });
}

/** Travel-time / itinerary between two world cells over the shared coarse cost
 *  grid (#23). `reach`: 2 = continental (no open-sea crossings), else sea allowed. */
export async function computeItinerary(
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
  rivers: { points: [number, number][] }[],
  reach: number,
  desertRoutes: boolean,
): Promise<import("@types").Itinerary> {
  return invoke("compute_itinerary", {
    fromX, fromY, toX, toY,
    riversJson: JSON.stringify(rivers),
    reach,
    desertRoutes,
  });
}

/** Compute circular "grand bank" fishing-ground zones from the fishery field. */
export async function computeFisheryBanks(): Promise<FisheryBank[]> {
  return invoke("compute_fishery_banks");
}

/** Cluster the highest-risk shark-infested water into danger zones. */
export async function computeSharkZones(): Promise<SharkZone[]> {
  return invoke("compute_shark_zones");
}

/** Cluster the highest-risk shipworm (Teredo) hull-hazard water into zones. */
export async function computeShipwormZones(): Promise<SharkZone[]> {
  return invoke("compute_shipworm_zones");
}

/** Cluster the highest-risk storm/cyclone water into danger zones. `month <= 0`
 *  is the combined annual extent; 1..months applies the seasonal phase. */
export async function computeStormZones(month: number, months: number): Promise<SharkZone[]> {
  return invoke("compute_storm_zones", { month, months });
}

/** Cluster the highest-risk reef/shoal wreck water into danger zones. */
export async function computeReefZones(): Promise<SharkZone[]> {
  return invoke("compute_reef_zones");
}

/** Cluster the land under a monsoon-type climate into wet-season flood regions
 *  (Natural Disasters overlay, alongside the hurricane/storm zones). */
export async function computeMonsoonZones(): Promise<SharkZone[]> {
  return invoke("compute_monsoon_zones");
}

/** CLAUDE.md §8.19 (goods localities, shipped) Slice 5 · the FULL-RESOLUTION belt mask for each named
 *  good — the payload that replaces `GoodRegion`'s coarse blocks for the FILL, so a
 *  belt meeting the sea ends on the coastline instead of spilling into it (F4/D3).
 *
 *  Takes an explicit good list rather than returning every good, because a mask is a
 *  real payload and only the toggled goods are ever drawn. `compute_good_regions`
 *  still supplies the label centroid, medallion and sublabel for every good. */
export async function computeGoodBeltMasks(goods: string[]): Promise<GoodBeltMask[]> {
  return invoke("compute_good_belt_masks", { goods });
}

/** One good's belt SAMPLED to a single province, at the SAME resolution the relief
 *  plate crops at — the province-plate counterpart of {@link computeGoodBeltMasks}.
 *  Reads the goods TILE column directly, so it works on any world (no localities, no
 *  running campaign) and lets the province survey plate draw belt AREAS + a QUALITY
 *  wash at the fidelity the ground under them is drawn at (F1 · slice 1). `maxDim`
 *  defaults to 130 server-side — pass the same value `getProvinceTerrainCrop` uses so
 *  the two plates line up. */
export async function provinceGoodBeltMasks(
  provinceId: number, goods: string[], maxDim?: number,
): Promise<ProvinceGoodMask[]> {
  return invoke("province_good_belt_masks", { provinceId, goods, maxDim });
}

/** Latitude bands of the general circulation for the Climate Bands overlay: the
 *  ITCZ rain line (per-column, migrates over land) plus the subtropical-high
 *  (desert) and polar-front (storm-track) belt latitudes, which move with the
 *  planet's rotation/greenhouse (Planet panel). */
export interface ClimateBands {
  width: number;
  itcz: number[];       // per-column ITCZ latitude (°N), annual mean, length = width
  // The convergence zone at its two SEASONAL EXTREMES (°N per column). These are
  // the exact lines the seasonal wind belts are displaced about, so the band
  // between them is the land that changes circulation regime between seasons —
  // which is what a monsoon climate is.
  itcz_july: number[];
  itcz_january: number[];
  hadley_edge: number;  // subtropical-high latitude (°), ~30 on Earth
  polar_front: number;  // polar-front / storm-track latitude (°), ~60 on Earth
  cells: number;        // circulation cells per hemisphere
}

export async function computeClimateBands(): Promise<ClimateBands> {
  return invoke("compute_climate_bands");
}

/** One-shot fetch of the *static* map overlays (wind/current vectors +
 *  streamlines, fishery banks, shark/shipworm/reef zones, trade-good regions) in
 *  a single IPC round-trip backed by one shared tile-cache read. Storm zones are
 *  fetched separately (`computeStormZones`) because they depend on the month. */
export async function computeOverlays(): Promise<OverlaysResult> {
  return invoke("compute_overlays");
}

/** The organic culture territories of the active world (Peoples overlay). */
export async function computeCultureRegions(): Promise<CultureRegion[]> {
  return invoke("compute_culture_regions");
}

/** Build the region↔region trade matrix (routed + bundled flows). */
export async function computeTradeMatrix(
  settlements: { x: number; y: number; score: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
  luxuryBias: number,
  piracy: number,
): Promise<TradeMatrix> {
  return invoke("compute_trade_matrix", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    luxuryBias,
    piracy,
  });
}

/** Re-rank settlements by trade power and return political influence centers. */
export async function computePolitical(
  settlements: { x: number; y: number; score: number; population: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
  piracy: number,
): Promise<PoliticalCenter[]> {
  return invoke("compute_political", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    piracy,
  });
}

/** Build + persist the economy snapshot (hubs, quality-graded production,
 *  cost-aware flows with per-hop prices, wealth, chokepoints). */
export async function computeEconomy(
  settlements: { x: number; y: number; score: number; population: number }[],
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
  desertRoutes: boolean,
  economicRegions: number,
  luxuryBias: number,
  piracy: number,
  season: number,
  months: number,
): Promise<EconomySnapshot> {
  return invoke("compute_economy", {
    settlementsJson: JSON.stringify(settlements),
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
    desertRoutes,
    economicRegions,
    luxuryBias,
    piracy,
    season,
    months,
  });
}

/** Trade-development feedback: grow each settlement by its hub's trade wealth
 *  (one-way, bounded). Returns the updated settlement list. */
export async function computeSettlementDevelopment(
  settlements: import("@types").Settlement[],
): Promise<import("@types").Settlement[]> {
  return invoke("compute_settlement_development", {
    settlementsJson: JSON.stringify(settlements),
  });
}
