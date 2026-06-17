/** Deterministic heraldic emblems. A house/guild's tinctures, division, ordinary
 *  and charge are derived from its name hash, so a given holder always shows the
 *  same arms. Two systems:
 *    • GUILDS  — the original simple geometric charge on a divided field (a civic
 *                trade-guild mark, not a bloodline). Unchanged "old" design.
 *    • HOUSES  — a richer dynastic system: metal-on-colour tinctures, more
 *                divisions + ordinaries, and a figurative CHARGE (lion, eagle,
 *                fleur-de-lis, tower, …) in the manner of the great royal houses.
 *  Purely decorative SVG. */

const TINCTURES = [
  "#b32d2d", // gules (red)
  "#2a5fa0", // azure (blue)
  "#2f7d44", // vert (green)
  "#6a3d9a", // purpure (purple)
  "#1d2733", // sable (near-black)
  "#c9a227", // or (gold)
  "#c8ced6", // argent (silver)
  "#b5651d", // tenné (orange-brown)
];
// Heraldic "metals" (light) vs "colours" (dark) — a charge must contrast its field
// (the rule of tincture), so a colour field takes a metal charge and vice-versa.
const METALS = ["#c9a227", "#c8ced6", "#e8e2d0"];
const COLOURS = ["#b32d2d", "#2a5fa0", "#2f7d44", "#6a3d9a", "#1d2733", "#b5651d"];

/** The 16 figurative house charges, as emoji glyphs (instantly legible and
 *  consistent with the app's iconography). Index 3 ("double eagle") renders two. */
const HOUSE_CHARGES = [
  "🦁", // 0 lion rampant
  "🐆", // 1 leopard / lion passant
  "🦅", // 2 eagle displayed
  "🦅", // 3 double-headed eagle (rendered twice)
  "🐺", // 4 wolf
  "🐗", // 5 boar
  "🦌", // 6 stag / hart
  "🐻", // 7 bear
  "🐎", // 8 horse
  "🐉", // 9 dragon / wyvern
  "🐬", // 10 dolphin
  "🐍", // 11 serpent
  "🏰", // 12 tower / castle
  "⚜️", // 13 fleur-de-lis
  "🐂", // 14 bull
  "🦢", // 15 swan
];
const HOUSE_CHARGE_NAMES = [
  "lion rampant", "leopard", "eagle displayed", "double-headed eagle", "wolf",
  "boar", "hart", "bear", "horse", "wyvern", "dolphin", "serpent", "tower",
  "fleur-de-lis", "bull", "swan",
];

function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** The holder's primary (field) tincture — the same colour as its shield field.
 *  Shared so the map control overlay and the settlement pie chart colour each
 *  holder identically to its coat of arms. */
export function houseColor(name: string): string {
  return TINCTURES[hash(name) % TINCTURES.length];
}

/** A short heraldic blazon-ish description of a HOUSE's arms (for tooltips). */
export function blazonOf(name: string): string {
  const h = hash(name);
  return HOUSE_CHARGE_NAMES[(h >> 23) % HOUSE_CHARGE_NAMES.length];
}

// ── Guild charges: the original simple geometric set ──
function guildCharge(kind: number, color: string): JSX.Element {
  const c = color;
  switch (kind % 6) {
    case 0: // roundel
      return <circle cx={16} cy={18} r={6} fill={c} />;
    case 1: // mullet (star)
      return <path d="M16 11 l1.8 4 4.4.3 -3.4 2.8 1.1 4.3 -3.9-2.4 -3.9 2.4 1.1-4.3 -3.4-2.8 4.4-.3z" fill={c} />;
    case 2: // lozenge
      return <path d="M16 10 L22 18 L16 26 L10 18 Z" fill={c} />;
    case 3: // cross
      return <path d="M14 11 h4 v5 h5 v4 h-5 v5 h-4 v-5 h-5 v-4 h5 z" fill={c} />;
    case 4: // chevron
      return <path d="M7 24 L16 13 L25 24 L21 24 L16 18 L11 24 Z" fill={c} />;
    default: // bend (diagonal bar)
      return <path d="M8 10 L12 10 L24 26 L20 26 Z" fill={c} />;
  }
}

// ── Ordinaries (metal bars/crosses) overlaid on a HOUSE field ──
function ordinary(kind: number, color: string): JSX.Element | null {
  const c = color;
  switch (kind) {
    case 0: return null; // no ordinary (charge alone)
    case 1: return <rect x="0" y="15" width="32" height="7" fill={c} />;              // fess
    case 2: return <rect x="13" y="0" width="7" height="38" fill={c} />;              // pale
    case 3: return <path d="M0 4 L4 0 L32 28 L32 36 Z" fill={c} />;                   // bend
    case 4: return <path d="M0 38 L16 18 L32 38 L26 38 L16 26 L6 38 Z" fill={c} />;   // chevron
    default: return (                                                                  // cross
      <g fill={c}><rect x="13" y="0" width="6" height="38" /><rect x="0" y="14" width="32" height="6" /></g>
    );
  }
}

export function CoatOfArms({ name, size = 26, guild = false }: { name: string; size?: number; guild?: boolean }) {
  const h = hash(name);
  // Shield outline (heater shape) in a 32×38 box.
  const shield = "M3 3 H29 V20 Q29 32 16 37 Q3 32 3 20 Z";
  const sid = `s${guild ? "g" : "h"}${h}`;

  // Field + (for houses) a contrasting metal charge/ordinary per the rule of tincture.
  const field = TINCTURES[h % TINCTURES.length];
  const fieldIsMetal = METALS.includes(field);
  const metal = METALS[(h >> 5) % METALS.length];
  const colour = COLOURS[(h >> 7) % COLOURS.length];
  // The figurative bits sit in a contrasting tincture (metal on colour / colour on metal).
  const chargeTint = fieldIsMetal ? colour : metal;
  let field2 = TINCTURES[(h >> 17) % TINCTURES.length];
  if (field2 === field) field2 = (fieldIsMetal ? colour : metal);

  if (guild) {
    // ── Old design: geometric charge on a simply divided field ──
    const division = (h >> 11) % 4; // 0 plain · 1 per pale · 2 per fess · 3 per bend
    return (
      <svg width={size} height={(size * 38) / 32} viewBox="0 0 32 38" style={{ display: "block", flex: "0 0 auto" }}>
        <defs><clipPath id={sid}><path d={shield} /></clipPath></defs>
        <g clipPath={`url(#${sid})`}>
          <rect x="0" y="0" width="32" height="38" fill={field} />
          {division === 1 && <rect x="16" y="0" width="16" height="38" fill={field2} />}
          {division === 2 && <rect x="0" y="19" width="32" height="19" fill={field2} />}
          {division === 3 && <path d="M0 0 L32 0 L32 38 Z" fill={field2} />}
          {guildCharge(h >> 23, chargeTint)}
        </g>
        <path d={shield} fill="none" stroke="#0c1118" strokeWidth="1.6" />
        <path d={shield} fill="none" stroke="#c9a227" strokeWidth="0.6" opacity="0.6" />
      </svg>
    );
  }

  // ── New house design: divisions + optional ordinary + figurative charge ──
  const division = (h >> 11) % 8;
  // 0 plain · 1 per pale · 2 per fess · 3 per bend · 4 quarterly · 5 per saltire
  // 6 chief (top band) · 7 semé (field strewn with small charges)
  const ord = (h >> 14) % 6; // ordinary (0 = none)
  const chargeIdx = (h >> 23) % HOUSE_CHARGES.length;
  const semeGlyph = ["✦", "⚜", "●", "✚"][(h >> 20) % 4];

  return (
    <svg width={size} height={(size * 38) / 32} viewBox="0 0 32 38" style={{ display: "block", flex: "0 0 auto" }}>
      <defs><clipPath id={sid}><path d={shield} /></clipPath></defs>
      <g clipPath={`url(#${sid})`}>
        <rect x="0" y="0" width="32" height="38" fill={field} />
        {/* Divisions */}
        {division === 1 && <rect x="16" y="0" width="16" height="38" fill={field2} />}
        {division === 2 && <rect x="0" y="19" width="32" height="19" fill={field2} />}
        {division === 3 && <path d="M0 0 L32 0 L32 38 Z" fill={field2} />}
        {division === 4 && (<g fill={field2}><rect x="16" y="0" width="16" height="19" /><rect x="0" y="19" width="16" height="19" /></g>)}
        {division === 5 && (<g fill={field2}><path d="M16 19 L32 0 L32 19 Z" /><path d="M16 19 L0 38 L16 38 Z" /></g>)}
        {division === 6 && <rect x="0" y="0" width="32" height="9" fill={field2} />}
        {division === 7 && (
          <text x="0" y="0" fill={chargeTint} opacity="0.5" fontSize="6"
            style={{ fontFamily: "serif" }}>
            {[6, 16, 26].map((py) => [4, 13, 22].map((px, j) => (
              <tspan key={`${py}-${j}`} x={px} y={py}>{semeGlyph}</tspan>
            )))}
          </text>
        )}
        {/* Ordinary (metal bar/cross), skipped on quarterly/semé to avoid clutter */}
        {division !== 4 && division !== 7 && ordinary(ord, metal)}
        {/* Figurative charge — emoji glyph(s), centred */}
        {chargeIdx === 3 ? (
          <text textAnchor="middle" dominantBaseline="central" fontSize="11">
            <tspan x="11" y="20">🦅</tspan><tspan x="21" y="20">🦅</tspan>
          </text>
        ) : (
          <text x="16" y="20" textAnchor="middle" dominantBaseline="central" fontSize="15">
            {HOUSE_CHARGES[chargeIdx]}
          </text>
        )}
      </g>
      {/* A house wears a richer border: dark edge + inner gold fillet. */}
      <path d={shield} fill="none" stroke="#0c1118" strokeWidth="1.8" />
      <path d={shield} fill="none" stroke="#c9a227" strokeWidth="0.9" opacity="0.85" />
    </svg>
  );
}
