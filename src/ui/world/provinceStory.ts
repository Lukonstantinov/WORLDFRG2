/** Province presentation helpers, shared by the Provinces browser (`ProvincePanel`)
 *  and the click-to-open `ProvinceInspector`. Mirrors the `settlementStory.ts`
 *  convention: deterministic prose + formatting derived from the province's own
 *  data, so both views always say the same thing about the same land. */

import { GOOD_DEFS } from "@goods";
import { koppenName } from "@ui/world/climate";
import { BORDER_LAKE, BORDER_RIDGE, BORDER_RIVER } from "@types";
import type { Province } from "@types";

export const ELEV_WORD = ["lowland", "hill country", "upland"];

/** 0..1 → a five-star bar. */
export function stars(q: number): string {
  const n = Math.max(0, Math.min(5, Math.round(q * 5)));
  return "★★★★★".slice(0, n) + "☆☆☆☆☆".slice(0, 5 - n);
}

export function goodLabel(g: number): string { return GOOD_DEFS[g]?.label ?? `good ${g}`; }
export function goodEmoji(g: number): string { return GOOD_DEFS[g]?.emoji ?? "•"; }

/** Icon + phrase for what physically divides two provinces. */
export function borderKind(kind: number): { icon: string; label: string } {
  switch (kind) {
    case BORDER_RIDGE: return { icon: "⛰", label: "mountain ridge" };
    case BORDER_RIVER: return { icon: "〜", label: "river" };
    case BORDER_LAKE:  return { icon: "🌊", label: "lakeshore" };
    default:           return { icon: "·", label: "open country" };
  }
}

/** Cell counts → a readable distance along a frontier/coast. `cellKm` comes from the
 *  world width (≈40075 km of circumference spread over the grid). */
export function cellsToKm(cells: number, cellKm: number): string {
  const km = Math.round(cells * cellKm);
  return km >= 1000 ? `${(km / 1000).toFixed(1)}k km` : `${km} km`;
}

/** A short deterministic account of a province — terrain, people, signature good,
 *  and whether it is a frontier or a settled heartland. */
export function provinceHistory(p: Province, urban: number): string {
  const terr = ELEV_WORD[p.elevation_class] ?? "country";
  const clim = koppenName(p.koppen).toLowerCase();
  const top = p.goods[0] ? goodLabel(p.goods[0].good).toLowerCase() : "mixed produce";
  const coast = p.coastal ? "coast-fronting " : "";
  const status = p.settlements.length === 0
    ? `It stands as open frontier, its ${terr} still unclaimed by any town.`
    : urban > p.rural_pop
      ? `A crowded ${p.settlements.length > 1 ? "heartland of several towns" : "town-centred region"}, its people press on the land.`
      : `A rural ${terr === "country" ? "province" : terr}, its folk work the fields and pastures.`;
  return `${p.name} is a ${coast}${clim} ${terr} of the ${p.culture} people, long known for its ${top}. ${status}`;
}

/** A sentence naming the natural features that actually enclose this province —
 *  built from the border provenance the partition records, so it can only ever say
 *  what the map really shows. Empty when the world predates border provenance. */
export function provinceFrontiers(p: Province): string {
  const nd = p.neighbors_detail;
  if (!nd || nd.length === 0) return "";
  const tally = new Map<number, number>();
  for (const b of nd) tally.set(b.kind, (tally.get(b.kind) ?? 0) + b.cells);
  const total = [...tally.values()].reduce((a, b) => a + b, 0);
  if (total === 0) return "";
  const parts: string[] = [];
  for (const kind of [BORDER_RIDGE, BORDER_RIVER, BORDER_LAKE]) {
    const share = (tally.get(kind) ?? 0) / total;
    if (share >= 0.15) parts.push(`${Math.round(share * 100)}% ${borderKind(kind).label}`);
  }
  if (p.coastal && (p.coast_cells ?? 0) > 0) parts.push("open sea");
  if (parts.length === 0) return "Its landward borders run through open country.";
  return `Bounded by ${parts.join(", ")}.`;
}
