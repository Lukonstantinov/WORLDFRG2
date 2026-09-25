// HOUSES_GUILDS_AND_MARKET_PLAN.md S10 — helpers shared between the Houses
// browser (HousesPanel.tsx) and the House Dossier (HouseDossier.tsx) now that
// the dossier is its own file. Split out rather than duplicated or cross-
// imported (which would create a HousesPanel <-> HouseDossier import cycle).
import { GoodIcon } from "@ui/goods/GoodIcon";
import type { HouseBrief, KinBrief } from "@types";

export const goodIcon = (name: string) => <GoodIcon name={name} size={14} style={{ display: "inline-block", verticalAlign: "middle" }} />;

/** House tiers (`HOUSE_PEOPLE_AND_TIERS.md` §1) — a rank BAND among live peers, not an
 *  absolute score. Tier 1 may be empty on a young world; that's meaningful, not a bug. */
export const TIER_META: Record<number, { glyph: string; name: string; band: string }> = {
  1: { glyph: "⬤", name: "Great Houses", band: "dominate trade" },
  2: { glyph: "◐", name: "Major Houses", band: "a power in their quarter" },
  3: { glyph: "◔", name: "Lesser Houses", band: "trade, matter locally" },
  4: { glyph: "○", name: "Marginal", band: "negligible influence" },
};
/** A house whose tier isn't computed yet (founded since the last monthly pass) reads
 *  as Marginal — it hasn't proven anything yet, which is the honest starting point. */
export const tierOf = (h: HouseBrief) => (h.tier && h.tier >= 1 && h.tier <= 4 ? h.tier : 4);

/** Desaturate a house colour toward grey — guilds read DULL vs the vivid private
 *  houses, so the civic bodies are visually distinct. */
export function dull(hex: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex || "");
  if (!m) return "#7a8694";
  const n = parseInt(m[1], 16);
  const mix = (c: number) => Math.round(c * 0.45 + 0x86 * 0.55);
  const r = mix((n >> 16) & 255), g = mix((n >> 8) & 255), b = mix(n & 255);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
}

/** Phase 2.2 · holdings authorship — is this holding run by a POSTED kin (family) or
 *  a hired factor? Returns the kin's given name if family-run, "" if hired (silent —
 *  "hired" is the unremarkable default, same discipline as everywhere else here).
 *  A snapshot from the roster's last generation (see `Kin.posted`'s own doc comment),
 *  so it can occasionally lag a holding gained since — cosmetic, not a bug. */
export function familyRunAt(kin: KinBrief[], cityName: string): string {
  const k = kin.find((x) => x.role === "factor" && x.posted_name === cityName);
  return k ? k.name.split(" ")[0] : "";
}
