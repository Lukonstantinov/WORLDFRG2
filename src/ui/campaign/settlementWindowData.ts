// settlementWindowData.ts — HubDetail (+ the live flows / route / culture reads)
// → what the settlement window draws. Pure functions, no React, no canvas.
//
// The rule every function here keeps: a thing the data does not hold is NOT
// shown. Where the design handoff asks for a number the sim never computes (a
// construction queue, a city's faiths, a warehouse tier this panel cannot see)
// the row is omitted, or — for a tier requirement — marked UNKNOWN rather than
// guessed as met or unmet.

import type { BuildingInfo, CityVessels, EstateRow, Government, HubDetail, JournalEntry } from "@types";
import type { CaravanKind, LandmarkKind, ShipKind, SlotStatus, StyleKey, WaterKind } from "@canvas/cityArt";
import { toHex } from "@canvas/cityArt";

export const TIER_NAMES = ["", "Outpost", "Market", "Guild Town", "Free City", "Emporium"];
export const CIVIC_COLOR = "#7a8aa0";

// ── architectural family ──────────────────────────────────────────────────────

// Culture kit indices (`sim/shared/cultures.rs` / `cultureDress.ts` DRESS_KITS).
const KIT_NORSE = 4, KIT_CELTIC = 5, KIT_SLAVIC = 9;
const KIT_ARAB = 6, KIT_AMAZIGH = 13, KIT_PERSIAN = 3;
const KIT_INDIC = 7, KIT_SINITIC = 8, KIT_YAMATO = 14;
const KIT_TURKIC = 11, KIT_MONGOL = 15, KIT_QUECHUA = 16;
const NOMAD = new Set([KIT_TURKIC, KIT_MONGOL]);
const EAST = new Set([KIT_SINITIC, KIT_YAMATO]);
const DESERT_BUILDERS = new Set([KIT_ARAB, KIT_AMAZIGH, KIT_PERSIAN]);
const NORTHERN = new Set([KIT_NORSE, KIT_CELTIC, KIT_SLAVIC]);

/**
 * Pick one of the eight families from climate first, then let the culture's
 * kit override it WITHIN a compatible climate band (README "Family selection":
 * "let a culture's kit override climate"). The override is bounded on purpose:
 * a nomad people in a rainforest does not pitch yurts, and a Sinitic city on
 * the tundra is still built against the snow.
 *
 * `elevation` is the hub's normalised height where the worldgen snapshot knows
 * it (a campaign-founded town has none, and simply takes the lowland reading).
 */
export function pickFamily(koppen: number, kit: number | undefined, coastal: boolean, elevation?: number): StyleKey {
  const k = kit ?? -1;
  const high = (elevation ?? 0) > 0.35;
  let base: StyleKey;
  switch (koppen) {
    case 1: case 2: case 3: case 23: base = "trop"; break;                       // Af Am Aw As
    case 4: case 5: base = "desert"; break;                                      // BWh BWk
    case 6: case 7: base = NOMAD.has(k) ? "steppe" : (koppen === 7 && !DESERT_BUILDERS.has(k) ? "steppe" : "desert"); break; // BSh BSk
    case 8: case 9: case 10: case 18: case 19: base = "med"; break;              // Cs* Dsa Dsb
    case 11: case 24: base = EAST.has(k) ? "east" : k === KIT_INDIC ? "trop" : "med"; break; // Cfa Cwa
    case 25: case 26: case 32: base = EAST.has(k) ? "east" : "alpine"; break;    // Cwb Cwc H
    case 12: case 13: case 14: case 15: case 27: case 28:                        // Cfb Cfc Dfa Dfb Dwa Dwb
      base = high && !coastal ? "alpine" : EAST.has(k) && (koppen === 27 || koppen === 28) ? "east" : "hansa"; break;
    case 16: case 17: case 20: case 29: case 30: case 31:                        // subarctic
      base = coastal ? "arctic" : "alpine"; break;
    case 21: case 22: base = coastal ? "arctic" : "alpine"; break;               // ET EF
    default: base = "med";
  }
  // Kit overrides, each confined to a band where that building tradition works.
  const tropOrPolar = base === "trop" || base === "arctic";
  if (NOMAD.has(k) && !tropOrPolar && (base === "desert" || base === "hansa" || base === "alpine")) return "steppe";
  if (EAST.has(k) && (base === "med" || base === "hansa")) return "east";
  if (DESERT_BUILDERS.has(k) && base === "med" && (koppen === 8 || koppen === 18)) return "desert";
  if (NORTHERN.has(k) && base === "med" && koppen !== 8) return "hansa";
  if (k === KIT_QUECHUA && (base === "med" || base === "hansa")) return "alpine";
  return base;
}

/** Water the city sits on (README "Water type"): a real sea port → sea; a coastal
 *  hub with no sea access → lake; river-connected → river; a desert town with
 *  no coast → oasis; else none. */
export function waterKind(opts: { coastal: boolean; seaAccess?: boolean; river: boolean; family: StyleKey }): WaterKind {
  if (opts.coastal) return opts.seaAccess === false ? "lake" : "sea";
  if (opts.river) return "river";
  if (opts.family === "desert") return "oasis";
  return "none";
}

/** 0..4 population bucket — also the scene cache key's size component. */
export function popBucket(pop: number): number {
  return pop < 1_000 ? 0 : pop < 4_000 ? 1 : pop < 12_000 ? 2 : pop < 35_000 ? 3 : 4;
}

/** Walls are a sign of a city, not a hamlet — the same threshold the previous
 *  city plan used (pop > 12k), or a real Guild Town and above. */
export function isWalled(pop: number, devTier: number | undefined): boolean {
  return pop > 12_000 || (devTier ?? 0) >= 3;
}

/** Per-family hull NAMES — display only; the sim has no hull types (README). */
export const HULL_NAMES: Record<StyleKey, { sea: string; river: string; caravan: string }> = {
  med: { sea: "Galleys & carracks", river: "River barges", caravan: "Mule trains" },
  hansa: { sea: "Cogs & hulks", river: "River prams", caravan: "Carters' wagons" },
  steppe: { sea: "Sea hulls", river: "Hide rafts", caravan: "Camel & horse strings" },
  desert: { sea: "Dhows", river: "River boats", caravan: "Camel caravans" },
  east: { sea: "Ocean junks", river: "River junks & sampans", caravan: "Pack-horse trains" },
  trop: { sea: "Prahus & outriggers", river: "River dugouts", caravan: "Porters" },
  alpine: { sea: "Lake boats", river: "River barges", caravan: "Mule trains" },
  arctic: { sea: "Knarrs", river: "River boats", caravan: "Sledge trains" },
};
export const VESSEL_ICONS: Record<StyleKey, { sea: ShipKind; river: ShipKind; caravan: CaravanKind }> = {
  med: { sea: "galley", river: "barge", caravan: "horse" },
  hansa: { sea: "cog", river: "barge", caravan: "wagon" },
  steppe: { sea: "raft", river: "raft", caravan: "camel" },
  desert: { sea: "dhow", river: "barge", caravan: "camel" },
  east: { sea: "junk", river: "sampan", caravan: "horse" },
  trop: { sea: "prahu", river: "sampan", caravan: "horse" },
  alpine: { sea: "barge", river: "barge", caravan: "horse" },
  arctic: { sea: "knarr", river: "barge", caravan: "wagon" },
};
/** The caravan drawn outside the walls, per family (none for a stilt coast). */
export const SCENE_CARAVAN: Record<StyleKey, CaravanKind | null> = {
  med: "horse", hansa: "wagon", steppe: "camel", desert: "camel", east: "horse", trop: null, alpine: "horse", arctic: null,
};

// ── building slots ────────────────────────────────────────────────────────────

/** The five structures the sim itself can raise (`structure_label`), in the
 *  order `update_structures` picks the next one. A hub lacking one has a free
 *  site for it — that is a real, sim-defined notion, unlike an arbitrary slot
 *  count. */
const SIM_STRUCTURES = ["Workshop", "Granary", "Shipyard", "Guildhall", "Warehouse"] as const;

export interface SlotRow {
  kind: LandmarkKind;
  status: SlotStatus;
  owner: string;
  color: string;
  /** Status line under the name: the effect for a standing building, the reason
   *  for a site/lock. */
  note: string;
  /** Where a DERIVED landmark comes from (it is not a sim structure). */
  derived?: string;
  building?: BuildingInfo;
}

function holySite(events: JournalEntry[] | undefined): boolean {
  return (events ?? []).some((e) => e.kind === "pilgrimage" || e.kind === "temple");
}

function councilOwner(g: Government | null | undefined): { owner: string; color: string } {
  if (g && g.council && g.council !== "—") return { owner: g.council, color: toHex(g.council_color, CIVIC_COLOR) };
  return { owner: "Council", color: CIVIC_COLOR };
}

/**
 * Every slot the window shows, operating first. Three sources, never mixed up:
 * 1. `detail.buildings` — the sim's real structures + one fondaco per diaspora.
 * 2. DERIVED landmarks that are a direct reading of real state: a harbour for a
 *    real sea port, a council hall for a seated council, a mint for a city that
 *    strikes its own coin, a palace for a principality's ruling house, a temple
 *    where the chronicle records pilgrims. Each carries `derived` naming why.
 * 3. Free sites / locks for the sim's own five structures not yet raised — a
 *    shipyard is locked without a coast or a seated house, which is the sim's
 *    own rule (`update_structures`), not a tier gate invented for the UI.
 * No "Building · N months" row is ever produced: the sim erects a structure in
 * one step and holds no construction queue.
 */
export function deriveSlots(d: HubDetail, seaAccess?: boolean): SlotRow[] {
  const out: SlotRow[] = [];
  const blds = d.buildings ?? [];
  for (const b of blds) {
    out.push({ kind: b.label, status: "op", owner: b.owner, color: toHex(b.color, CIVIC_COLOR), note: b.effect, building: b });
  }
  const g = d.government;
  const seatedCouncil = !!g && ((g.officials?.length ?? 0) > 0 || (!!g.council && g.council !== "—"));
  if (d.coastal && seaAccess !== false) {
    out.push({ kind: "Harbor", status: "op", owner: "Civic", color: CIVIC_COLOR, note: "Quays of a sea port", derived: "a coastal sea port" });
  }
  if (seatedCouncil) {
    const c = councilOwner(g);
    out.push({ kind: "Council Hall", status: "op", ...c, note: g!.govt_type || "Council seated", derived: "a seated council" });
  }
  if (d.coin_name) {
    const c = councilOwner(g);
    out.push({ kind: "Mint", status: "op", ...c, note: `Strikes the ${d.coin_name}`, derived: "its own coinage" });
  }
  if (g?.leader && g.govt_type === "Principality") {
    out.push({ kind: "Palace", status: "op", owner: g.leader.house, color: toHex(g.leader.house_color, CIVIC_COLOR), note: `Court of ${g.leader.head_name}`, derived: "a princely ruling house" });
  }
  if (holySite(d.events)) {
    out.push({ kind: "Temple", status: "op", owner: "Civic", color: CIVIC_COLOR, note: "Draws pilgrims", derived: "pilgrimages in the chronicle" });
  }
  // Free sites / locks for the sim's own structures.
  const have = new Set(blds.map((b) => b.label));
  const seatedHouse = (d.houses ?? []).length > 0;
  let nextMarked = false;
  for (const s of SIM_STRUCTURES) {
    if (have.has(s)) continue;
    if (s === "Shipyard" && !d.coastal) {
      out.push({ kind: s, status: "lock", owner: "", color: CIVIC_COLOR, note: "Needs a coast" });
      continue;
    }
    if (s === "Shipyard" && !seatedHouse) {
      out.push({ kind: s, status: "lock", owner: "", color: CIVIC_COLOR, note: "Needs a seated house" });
      continue;
    }
    out.push({ kind: s, status: "site", owner: "", color: CIVIC_COLOR, note: nextMarked ? "Site free" : "Site free · next to rise" });
    nextMarked = true;
  }
  return out;
}

// ── districts ─────────────────────────────────────────────────────────────────

export interface District { name: string; color: string; works: number }

function worksOf(name: string, estates: EstateRow[]): number {
  if (name === "Civic") return estates.filter((e) => e.owner_is_civic).length;
  return estates.filter((e) => e.owner === name).length;
}

/**
 * Up to four quarters (NW/NE/SW/SE in the scene), one per faction that owns
 * something standing here — the old city plan's ward rule: the distinct owners
 * of the city's buildings, busiest first, topped up with seated houses. "N works"
 * is the owner's real count of estates/manufactories in this city's hinterland.
 */
export function deriveDistricts(d: HubDetail, slots: SlotRow[]): District[] {
  const estates = d.estates_here ?? [];
  const tally = new Map<string, { color: string; n: number }>();
  for (const s of slots) {
    if (s.status !== "op" || !s.owner) continue;
    const e = tally.get(s.owner) ?? { color: s.color, n: 0 };
    e.n++; tally.set(s.owner, e);
  }
  for (const h of d.houses ?? []) {
    if (tally.size >= 4) break;
    if (!tally.has(h.name)) tally.set(h.name, { color: toHex(h.color, CIVIC_COLOR), n: 0 });
  }
  return [...tally.entries()]
    .sort((a, b) => b[1].n - a[1].n || worksOf(b[0], estates) - worksOf(a[0], estates))
    .slice(0, 4)
    .map(([name, v]) => ({ name, color: v.color, works: worksOf(name, estates) }));
}

// ── the tier ladder's checklist ──────────────────────────────────────────────

export type CheckState = "met" | "open" | "unknown";
export interface TierCheck { label: string; state: CheckState; title?: string }

const UNSEEN = "Not visible from this panel — the sim checks it, the window cannot.";

/**
 * The NEXT tier's requirements (`development_tier`, cities.rs; the table in
 * docs/proposals/SETTLEMENT_TIER_REQUIREMENTS.md), each read from what this
 * panel actually has. Three the window cannot see — a house warehouse's tier,
 * a bank stake, a mint with no coin of its own — come back UNKNOWN rather than
 * guessed. "N of M" groups list every member so the reader can count them.
 */
export function tierChecklist(d: HubDetail, hubClass: number, tier: number): { next: string | null; need: string; checks: TierCheck[] } {
  if (tier >= 5) return { next: null, need: "", checks: [] };
  const pop = d.population;
  const g = d.government;
  const stable = d.sent_stability >= 0.5 && (d.society?.unrest ?? 0) < 0.5;
  const govt = !!g && (g.govt_type !== "Merchant Council" || (g.officials?.length ?? 0) > 0);
  const laws = (g?.laws?.length ?? 0) > 0;
  const civic = (d.structures ?? []).length;
  const guild = (d.houses ?? []).some((h) => h.is_guild);
  const traded = (d.bought ?? 0) + (d.sold ?? 0) > 0;
  const tradeHub = hubClass >= 1;
  const ownCoin = !!d.coin_name;
  const health = d.public_health ?? 0;
  const c = (label: string, ok: boolean): TierCheck => ({ label, state: ok ? "met" : "open" });
  const u = (label: string, ok: boolean): TierCheck => ok ? { label, state: "met" } : { label, state: "unknown", title: UNSEEN };
  const fmt = (n: number) => n.toLocaleString("en-US");
  const next = TIER_NAMES[tier + 1];
  switch (tier + 1) {
    case 2: return { next, need: "", checks: [c(`Pop ≥ ${fmt(700)}`, pop >= 700), u("Trade, a depot, or trade-hub rank", traded || tradeHub)] };
    case 3: return { next, need: "2 of the 4 after government", checks: [
      c(`Pop ≥ ${fmt(2000)}`, pop >= 2000), c("Government seated", govt),
      c("Guild", guild), u("Warehouse (depot+)", false), c("Civic building", civic >= 1), c("Trade hub", tradeHub)] };
    case 4: return { next, need: "2 of the 5 after finance", checks: [
      c(`Pop ≥ ${fmt(7000)}`, pop >= 7000), c("Trade hub", tradeHub), u("Finance · coin, mint or bank", ownCoin),
      u("Warehouse tier 3+", false), c("2+ civic buildings", civic >= 2), c("Guild", guild), c("Written laws", laws), c("Stable", stable)] };
    default: return { next, need: "3 of the 7 after finance", checks: [
      c(`Pop ≥ ${fmt(20000)}`, pop >= 20000), c("Trade hub or entrepôt", tradeHub), u("Finance · coin, mint or bank", ownCoin),
      c("Own coinage", ownCoin), u("Warehouse tier 4+", false), c("3+ civic buildings", civic >= 3), c("Written laws", laws),
      c("Stable", stable), c("Public health", health >= 0.3), c("Guild", guild)] };
  }
}

/** Supply-by-sea share, or null when nothing arrived at all. */
export function seaShare(d: HubDetail): number | null {
  const s = d.in_by_sea ?? 0, l = d.in_by_land ?? 0;
  return s + l > 1e-6 ? s / (s + l) : null;
}

/** Hulls drawn on the scene's water, scaled by real registered hulls plus the
 *  cargoes actually inbound (most carriage is ownerless, so a port with no
 *  registered hull still sees traffic). */
export function sceneShips(v: CityVessels | undefined, water: WaterKind, inBySea: number): number {
  if (water === "none" || water === "oasis") return 0;
  const cls = v?.classes ?? [];
  const reg = water === "river" ? (cls.find((c) => c.kind === "river")?.registered ?? 0) : (cls.find((c) => c.kind === "sea")?.registered ?? 0);
  const traffic = reg + Math.ceil((v?.inbound_cargoes ?? 0) / 3);
  return Math.max(inBySea > 0 || traffic > 0 ? 1 : 0, Math.min(9, traffic));
}
