/** Deterministic heraldic emblems. A house/guild's tinctures, division, ordinary
 *  and charge are derived from its name hash, so a given holder always shows the
 *  same arms. Two systems:
 *    • GUILDS  — the original simple geometric charge on a divided field (a civic
 *                trade-guild mark, not a bloodline). Unchanged "old" design.
 *    • HOUSES  — a richer dynastic system: metal-on-colour tinctures, multiple
 *                shield SHAPES, heraldic FURS (ermine/vair), more divisions +
 *                ordinaries, and a figurative VECTOR CHARGE (lion, eagle, tower,
 *                fleur-de-lis, …) drawn as crisp, tintable SVG (was emoji glyphs).
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

/** Shield SHAPES (paths in the 32×38 box). Houses pick one by hash; guilds keep
 *  the classic heater. */
const SHIELD_SHAPES = [
  "M3 3 H29 V20 Q29 32 16 37 Q3 32 3 20 Z",                       // 0 heater
  "M3 3 H29 V22 Q29 35 16 37 Q3 35 3 22 Z",                       // 1 Iberian (round base)
  "M4 3 H28 V28 Q28 33 23 35 L16 37 L9 35 Q4 33 4 28 Z",          // 2 French (square + point)
  "M5 3 Q3 3 3 6 V21 Q3 32 16 37 Q29 32 29 21 V6 Q29 3 27 3 Z",   // 3 Swiss (curved sides)
  "M16 2 Q29 2 29 19 Q29 36 16 37 Q3 36 3 19 Q3 2 16 2 Z",        // 4 cartouche (oval)
];
const HEATER = SHIELD_SHAPES[0];

/** The 16 figurative house charges, now as crisp VECTOR art (were emoji). */
const HOUSE_CHARGE_NAMES = [
  "lion", "eagle", "tower", "fleur-de-lis", "crown", "mullet", "sun", "crescent",
  "rose", "garb", "escallop", "sword", "key", "anchor", "boar", "trefoil",
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

const INK = "#0c1118";
function starPoints(r: number, pts = 5): string {
  let s = "";
  for (let i = 0; i < pts * 2; i++) {
    const rr = i % 2 === 0 ? r : r * 0.45;
    const a = -Math.PI / 2 + (i * Math.PI) / pts;
    s += `${(Math.cos(a) * rr).toFixed(1)},${(Math.sin(a) * rr).toFixed(1)} `;
  }
  return s.trim();
}

/** A figurative house charge (vector), drawn centred on the shield (16,19). */
function houseCharge(idx: number, fill: string): JSX.Element {
  let inner: JSX.Element;
  switch (idx % 16) {
    case 0: { // lion (head affronté)
      const mane = Array.from({ length: 12 }, (_, i) => {
        const a = i * 0.52;
        return <polygon key={i}
          points={`0,0 ${(Math.cos(a) * 10).toFixed(1)},${(Math.sin(a) * 10).toFixed(1)} ${(Math.cos(a + 0.4) * 10).toFixed(1)},${(Math.sin(a + 0.4) * 10).toFixed(1)}`}
          fill={fill} transform="translate(0,-1)" />;
      });
      inner = (<g>{mane}<circle cx={0} cy={-1} r={6} fill={fill} />
        <circle cx={-2.2} cy={-2} r={0.9} fill={INK} /><circle cx={2.2} cy={-2} r={0.9} fill={INK} />
        <path d="M-2,1.5 Q0,3.5 2,1.5" stroke={INK} strokeWidth={1} fill="none" /></g>);
      break;
    }
    case 1: // eagle displayed
      inner = (<g fill={fill}><circle cx={0} cy={-7} r={2.4} /><polygon points="2,-7 6,-8 5,-6" />
        <path d="M0,-4 Q-2,2 -1,8 H1 Q2,2 0,-4 Z" />
        <path d="M-1,-3 Q-9,-6 -11,2 Q-6,0 -2,4 Z" /><path d="M1,-3 Q9,-6 11,2 Q6,0 2,4 Z" /></g>);
      break;
    case 2: // tower
      inner = (<g fill={fill}><rect x={-6} y={-4} width={12} height={13} />
        <rect x={-6} y={-7} width={3} height={3} /><rect x={-1.5} y={-7} width={3} height={3} /><rect x={3} y={-7} width={3} height={3} />
        <path d="M-2,9 V2 A2,2 0 0 1 2,2 V9 Z" fill={INK} /></g>);
      break;
    case 3: // fleur-de-lis
      inner = (<g fill={fill}>
        <path d="M0 -10 C 2 -6 1 -4 0 -3 C -1 -4 -2 -6 0 -10 Z" />
        <path d="M0 -3 C 4 -8 8 -2 4 2 C 2 4 0 2 0 0 C 0 2 -2 4 -4 2 C -8 -2 -4 -8 0 -3 Z" />
        <path d="M-5 2 L5 2 L4 4 L-4 4 Z" /><path d="M-1 4 H1 V9 H-1 Z" />
        <path d="M-3 6 C -2 9 2 9 3 6 Z" /></g>);
      break;
    case 4: // crown
      inner = (<g fill={fill}><path d="M-8,6 L-8,-3 L-4,1 L0,-6 L4,1 L8,-3 L8,6 Z" />
        <circle cx={-8} cy={-4} r={1.5} /><circle cx={0} cy={-7.5} r={1.7} /><circle cx={8} cy={-4} r={1.5} />
        <rect x={-8} y={6} width={16} height={2.4} /></g>);
      break;
    case 5: // mullet (star)
      inner = <polygon points={starPoints(9, 5)} fill={fill} />;
      break;
    case 6: // sun
      inner = (<g>{Array.from({ length: 8 }, (_, i) => (
        <polygon key={i} points="0,-11 1.6,-7 -1.6,-7" fill={fill} transform={`rotate(${i * 45})`} />))}
        <circle r={5} fill={fill} /></g>);
      break;
    case 7: // crescent
      inner = <path d="M-2,-8 A8,8 0 1 0 -2,8 A6,6 0 1 1 -2,-8 Z" fill={fill} />;
      break;
    case 8: // rose
      inner = (<g>{Array.from({ length: 5 }, (_, i) => (
        <ellipse key={i} cx={0} cy={-5.5} rx={2.6} ry={3.4} fill={fill} transform={`rotate(${i * 72})`} />))}
        <circle r={2.2} fill={fill} /></g>);
      break;
    case 9: // garb (wheat sheaf)
      inner = (<g>{[-3, -1.2, 0, 1.2, 3].map((dx, i) => (
        <line key={i} x1={dx} y1={9} x2={dx * 1.6} y2={-9} stroke={fill} strokeWidth={1.5} />))}
        {[-3, -1.2, 0, 1.2, 3].map((dx, i) => <circle key={`e${i}`} cx={dx * 1.6} cy={-9} r={1.3} fill={fill} />)}
        <rect x={-5} y={2} width={10} height={2.4} fill={fill} /></g>);
      break;
    case 10: // escallop (scallop shell)
      inner = <path d="M0,8 A9,8 0 0 1 -9,-1 Q0,-11 9,-1 A9,8 0 0 1 0,8 Z" fill={fill} />;
      break;
    case 11: // sword
      inner = (<g fill={fill}><polygon points="0,-10 1.4,-6 1.4,5 -1.4,5 -1.4,-6" />
        <rect x={-4.5} y={5} width={9} height={1.8} /><rect x={-1} y={6.8} width={2} height={3.5} /></g>);
      break;
    case 12: // key
      inner = (<g fill={fill}><circle cx={0} cy={-5} r={3.6} /><circle cx={0} cy={-5} r={1.5} fill={INK} />
        <rect x={-1} y={-2} width={2} height={11} /><rect x={1} y={6} width={3.5} height={2} /><rect x={1} y={9} width={3} height={2} /></g>);
      break;
    case 13: // anchor
      inner = (<g fill="none" stroke={fill} strokeWidth={2}><circle cx={0} cy={-8} r={2.2} />
        <line x1={0} y1={-6} x2={0} y2={9} /><line x1={-5} y1={-1} x2={5} y2={-1} />
        <path d="M-7,4 Q-7,9 0,9 Q7,9 7,4" /></g>);
      break;
    case 14: // boar
      inner = (<g fill={fill}><ellipse cx={0} cy={0} rx={9} ry={4.5} />
        <polygon points="7,-2 13,-3 12,2 7,3" /><polygon points="11,-1 14,-2 13,0" />
        <rect x={-6} y={3} width={1.8} height={6} /><rect x={3} y={3} width={1.8} height={6} /></g>);
      break;
    default: // 15 trefoil
      inner = (<g fill={fill}><circle cx={0} cy={-4} r={3.2} /><circle cx={-3.4} cy={1} r={3.2} /><circle cx={3.4} cy={1} r={3.2} />
        <rect x={-0.8} y={2} width={1.6} height={7} /></g>);
  }
  return <g transform="translate(16,19)">{inner}</g>;
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
  switch (kind % 10) {
    case 0: return null; // no ordinary (charge alone)
    case 1: return <rect x="0" y="15" width="32" height="7" fill={c} />;              // fess
    case 2: return <rect x="13" y="0" width="7" height="38" fill={c} />;              // pale
    case 3: return <path d="M0 4 L4 0 L32 28 L32 36 Z" fill={c} />;                   // bend
    case 4: return <path d="M0 38 L16 18 L32 38 L26 38 L16 26 L6 38 Z" fill={c} />;   // chevron
    case 5: return (                                                                  // cross
      <g fill={c}><rect x="13" y="0" width="6" height="38" /><rect x="0" y="14" width="32" height="6" /></g>
    );
    case 6: return (                                                                  // saltire
      <g fill={c}><path d="M0 2 L3 0 L32 34 L29 37 Z" /><path d="M29 0 L32 2 L3 37 L0 34 Z" /></g>
    );
    case 7: return <rect x="0" y="0" width="32" height="9" fill={c} />;               // chief
    case 8: return <path d="M0 0 L32 0 L16 29 Z" fill={c} />;                         // pile
    default: return (                                                                 // bordure
      <path d="M3 3 H29 V20 Q29 32 16 37 Q3 32 3 20 Z M6 7 H26 V20 Q26 30 16 34 Q6 30 6 20 Z"
        fill={c} fillRule="evenodd" />
    );
  }
}

export function CoatOfArms({ name, size = 26, guild = false }: { name: string; size?: number; guild?: boolean }) {
  const h = hash(name);
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
        <defs><clipPath id={sid}><path d={HEATER} /></clipPath></defs>
        <g clipPath={`url(#${sid})`}>
          <rect x="0" y="0" width="32" height="38" fill={field} />
          {division === 1 && <rect x="16" y="0" width="16" height="38" fill={field2} />}
          {division === 2 && <rect x="0" y="19" width="32" height="19" fill={field2} />}
          {division === 3 && <path d="M0 0 L32 0 L32 38 Z" fill={field2} />}
          {guildCharge(h >> 23, chargeTint)}
        </g>
        <path d={HEATER} fill="none" stroke="#0c1118" strokeWidth="1.6" />
        <path d={HEATER} fill="none" stroke="#c9a227" strokeWidth="0.6" opacity="0.6" />
      </svg>
    );
  }

  // ── House design: shape + furs + divisions + optional ordinary + vector charge ──
  const shapePath = SHIELD_SHAPES[(h >> 3) % SHIELD_SHAPES.length];
  // Occasionally the field is a heraldic FUR (ermine / vair) instead of a tincture.
  const furKind = ((h >> 9) % 5) === 0 ? ((h >> 10) & 1 ? "vair" : "ermine") : null;
  const fieldFill = furKind ? `url(#fur${sid})` : field;

  const division = (h >> 11) % 11;
  // 0 plain · 1 per pale · 2 per fess · 3 per bend · 4 quarterly · 5 per saltire
  // 6 chief · 7 semé · 8 per chevron · 9 paly · 10 barry
  const ord = (h >> 14) % 10; // ordinary (0 = none)
  const chargeIdx = (h >> 23) % 16;
  const semeGlyph = ["✦", "⚜", "●", "✚"][(h >> 20) % 4];

  return (
    <svg width={size} height={(size * 38) / 32} viewBox="0 0 32 38" style={{ display: "block", flex: "0 0 auto" }}>
      <defs>
        <clipPath id={sid}><path d={shapePath} /></clipPath>
        {furKind === "ermine" && (
          <pattern id={`fur${sid}`} width="10" height="12" patternUnits="userSpaceOnUse">
            <rect width="10" height="12" fill="#eef0f2" />
            <path d="M5,3 q-1.6,2 -0.6,4 h1.2 q1,-2 -0.6,-4 Z" fill="#1d2733" />
            <circle cx="3.4" cy="2.2" r="0.5" fill="#1d2733" /><circle cx="6.6" cy="2.2" r="0.5" fill="#1d2733" /><circle cx="5" cy="3.4" r="0.5" fill="#1d2733" />
          </pattern>
        )}
        {furKind === "vair" && (
          <pattern id={`fur${sid}`} width="8" height="10" patternUnits="userSpaceOnUse">
            <rect width="8" height="10" fill="#c8ced6" />
            <path d="M0,0 Q4,7 8,0 Z" fill="#2a5fa0" /><path d="M-4,5 Q0,12 4,5 Z" fill="#2a5fa0" /><path d="M4,5 Q8,12 12,5 Z" fill="#2a5fa0" />
          </pattern>
        )}
      </defs>
      <g clipPath={`url(#${sid})`}>
        <rect x="0" y="0" width="32" height="38" fill={fieldFill} />
        {/* Divisions */}
        {division === 1 && <rect x="16" y="0" width="16" height="38" fill={field2} />}
        {division === 2 && <rect x="0" y="19" width="32" height="19" fill={field2} />}
        {division === 3 && <path d="M0 0 L32 0 L32 38 Z" fill={field2} />}
        {division === 4 && (<g fill={field2}><rect x="16" y="0" width="16" height="19" /><rect x="0" y="19" width="16" height="19" /></g>)}
        {division === 5 && (<g fill={field2}><path d="M16 19 L32 0 L32 19 Z" /><path d="M16 19 L0 38 L16 38 Z" /></g>)}
        {division === 6 && <rect x="0" y="0" width="32" height="9" fill={field2} />}
        {division === 7 && (
          <text x="0" y="0" fill={chargeTint} opacity="0.5" fontSize="6" style={{ fontFamily: "serif" }}>
            {[6, 16, 26].map((py) => [4, 13, 22].map((px, j) => (
              <tspan key={`${py}-${j}`} x={px} y={py}>{semeGlyph}</tspan>
            )))}
          </text>
        )}
        {division === 8 && <path d="M0 37 L16 20 L32 37 Z" fill={field2} />}
        {division === 9 && (<g fill={field2}>{[0, 1, 2, 3].map((i) => <rect key={i} x={i * 8} y="0" width="4" height="38" />)}</g>)}
        {division === 10 && (<g fill={field2}>{[0, 1, 2, 3, 4].map((i) => <rect key={i} x="0" y={i * 8} width="32" height="4" />)}</g>)}
        {/* Ordinary (metal bar/cross), skipped on quarterly/semé to avoid clutter */}
        {division !== 4 && division !== 7 && ordinary(ord, metal)}
        {/* Figurative VECTOR charge, centred */}
        {houseCharge(chargeIdx, chargeTint)}
      </g>
      {/* A house wears a richer border: dark edge + inner gold fillet. */}
      <path d={shapePath} fill="none" stroke="#0c1118" strokeWidth="1.8" />
      <path d={shapePath} fill="none" stroke="#c9a227" strokeWidth="0.9" opacity="0.85" />
    </svg>
  );
}
