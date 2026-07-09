// Per-settlement narrative — a short, deterministic account of what a town stands
// on and lives by: the river/reach or lake it sits beside, its trade role, and,
// for a delta town, a vignette on the abundance of life at the river's mouth.
// Generated on the frontend from the already-loaded rivers / toponyms / lakes so
// it works before a campaign, mirroring the river/lake NatGeo stories.

import type { Settlement, RiverData, Toponym, LakeData } from "./types";

/** Deterministic PRNG (mulberry32) so a settlement's story is stable per seed. */
function mulberry32(seed: number) {
  let a = seed >>> 0;
  return function () {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const pick = <T>(r: () => number, arr: T[]): T => arr[Math.floor(r() * arr.length) % arr.length];

const SIZE_WORD: Record<Settlement["size"], string> = {
  capital: "great city", city: "city", town: "town", village: "village", outpost: "waystation",
};

export interface SettlementStory {
  role: string;      // short role label ("delta entrepôt", "river port", …)
  river: string;     // river name if on a river, else ""
  reach: "" | "upper" | "middle" | "lower" | "delta" | "estuary";
  text: string;      // the full multi-sentence account
  isDelta: boolean;
}

/** Build a settlement's story from the world's rivers / toponyms / lakes. */
export function settlementStory(
  s: Settlement, rivers: RiverData[], toponyms: Toponym[], lakes: LakeData[], worldW = 0,
): SettlementStory {
  const half = worldW > 0 ? worldW / 2 : Infinity;
  const ddx = (a: number, b: number) => {
    let d = Math.abs(a - b);
    if (half !== Infinity && d > half) d = worldW - d;
    return d;
  };

  // ── Nearest river reach ──
  let best: { ri: number; frac: number; d2: number } | null = null;
  for (let ri = 0; ri < rivers.length; ri++) {
    const pts = rivers[ri].points;
    const n = pts.length;
    if (n < 2) continue;
    for (let k = 0; k < n; k++) {
      const dx = ddx(pts[k][0], s.x), dy = pts[k][1] - s.y;
      const d2 = dx * dx + dy * dy;
      if (!best || d2 < best.d2) best = { ri, frac: k / (n - 1), d2 };
    }
  }
  const onRiver = !!best && best.d2 <= 9; // within ~3 cells of the channel
  const rv = onRiver ? rivers[best!.ri] : null;
  const mouthKind = rv?.mouth_kind ?? 0;
  const frac = best?.frac ?? 0;

  // River name from the nearest river toponym (else a generic "the river").
  let river = "";
  if (onRiver) {
    let bt = 100; // within ~10 cells
    for (const t of toponyms) {
      if (t.kind !== "river") continue;
      const dx = ddx(t.x, s.x), dy = t.y - s.y, d2 = dx * dx + dy * dy;
      if (d2 < bt) { bt = d2; river = t.name; }
    }
  }

  // ── Nearest lake (fresh vs salt) ──
  let onLake = false, saltLake = false;
  for (const lk of lakes) {
    for (const [lx, ly] of lk.cells) {
      const dx = ddx(lx, s.x), dy = ly - s.y;
      if (dx * dx + dy * dy <= 9) {
        onLake = true;
        if (lk.endorheic || (lk.salinity_ppt ?? 0) >= 35) saltLake = true;
        break;
      }
    }
    if (onLake && saltLake) break;
  }

  const coast = s.site === "coast";
  // Reach classification (points run source→mouth).
  let reach: SettlementStory["reach"] = "";
  if (onRiver) {
    if (frac >= 0.72 && mouthKind === 1) reach = "delta";
    else if (frac >= 0.72 && mouthKind === 2) reach = "estuary";
    else if (frac >= 0.72) reach = "lower";
    else if (frac <= 0.33) reach = "upper";
    else reach = "middle";
  }
  const isDelta = reach === "delta";

  // ── Role ──
  const role =
    reach === "delta" ? "delta entrepôt"
    : reach === "estuary" ? "estuary port"
    : coast && onRiver ? "river-mouth port"
    : coast ? "coastal port"
    : saltLake ? "salt-trading town"
    : onLake ? "lakeside town"
    : onRiver && rv?.navigable ? "river port"
    : onRiver ? "river town"
    : s.site === "hills" ? "hill town"
    : "inland market town";

  // ── Compose the prose ──
  const seed = (s.x * 73856093) ^ (s.y * 19349663) ^ (s.name.length * 2654435761);
  const r = mulberry32(seed >>> 0);
  const sizeWord = SIZE_WORD[s.size] ?? "town";
  const riverPhrase = river || "the river";
  const out: string[] = [];

  out.push(pick(r, [
    `${s.name} is a ${sizeWord}, a ${role} of the region.`,
    `${s.name}, a ${sizeWord}, grew up as a ${role}.`,
    `A ${sizeWord} and ${role}, ${s.name} owes its place to its setting.`,
  ]));

  if (onRiver) {
    const reachPhrase =
      reach === "delta" ? `where ${riverPhrase} fans into its delta` :
      reach === "estuary" ? `on the tidal estuary of ${riverPhrase}` :
      reach === "lower" ? `on the broad lower ${riverPhrase}` :
      reach === "upper" ? `on the swift upper ${riverPhrase}` :
      `on the middle ${riverPhrase}`;
    out.push(pick(r, [
      `It stands ${reachPhrase}, whose water works its mills, floats its barges and fills its wells.`,
      `The town sits ${reachPhrase} — its lifeline for water, fish and trade alike.`,
      `Built ${reachPhrase}, it looks to the river for water, carriage and catch.`,
    ]));
  } else if (saltLake) {
    out.push("It rings a terminal salt lake — undrinkable brine, but a wealth of salt cut from the shore and carried inland along the salt roads.");
  } else if (onLake) {
    out.push("It sits on the shore of a freshwater lake, drawing on its fish and calm water.");
  } else if (coast) {
    out.push("It faces the open sea, a harbour town living by boat and net and the coasting trade.");
  } else {
    out.push(pick(r, [
      "It lies in open country, a market where the roads of the district meet.",
      "Set back from the water, it thrives as a crossroads market for the country around.",
    ]));
  }

  if (isDelta) {
    // The delta "abundance of life" vignette.
    out.push(pick(r, [
      `Here the river breaks into a hundred channels through reed and silt. The delta teems: its marshes are a nursery where young fish shelter and clouds of wildfowl settle each season, and sturgeon and shad run up from the sea to spawn — the fishing and fowling feed ${s.name} the year round.`,
      `The delta is a world of its own — braided water, tidal flats and endless reed-beds alive with fish, eels and wading birds. Migratory flocks darken the sky in their season, and the rich silt makes the fields around ${s.name} the envy of the land.`,
    ]));
  } else if (reach === "estuary") {
    out.push("Salt water meets fresh at its wharves, and diadromous fish — salmon, shad, eel — pass through on their runs, so the nets are seldom empty.");
  }

  if (s.culture) {
    out.push(`Its people are ${s.culture}${s.region ? ` of ${s.region}` : ""}.`);
  }

  return { role, river, reach, text: out.join(" "), isDelta };
}
