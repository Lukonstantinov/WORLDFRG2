/** A simple deterministic heraldic emblem for a merchant house. The shield's
 *  tinctures, division and charge are derived from the house name hash, so a
 *  given house always shows the same arms. Purely decorative (SVG). */

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

function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** The house's primary (field) tincture — the same colour as its shield field.
 *  Shared so the map control overlay and the settlement pie chart colour each
 *  house identically to its coat of arms. */
export function houseColor(name: string): string {
  return TINCTURES[hash(name) % TINCTURES.length];
}

/** A charge (central symbol) drawn in the given color, centered in a ~32-wide field. */
function charge(kind: number, color: string): JSX.Element {
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

export function CoatOfArms({ name, size = 26 }: { name: string; size?: number }) {
  const h = hash(name);
  const field = TINCTURES[h % TINCTURES.length];
  let charge2 = TINCTURES[(h >> 5) % TINCTURES.length];
  if (charge2 === field) charge2 = TINCTURES[(h >> 5 + 3) % TINCTURES.length] ?? "#e8e2d0";
  const division = (h >> 11) % 4; // 0 plain · 1 per pale · 2 per fess · 3 per bend
  const field2 = TINCTURES[(h >> 17) % TINCTURES.length];
  // Shield outline (heater shape) in a 32×38 box.
  const shield = "M3 3 H29 V20 Q29 32 16 37 Q3 32 3 20 Z";
  return (
    <svg width={size} height={(size * 38) / 32} viewBox="0 0 32 38" style={{ display: "block", flex: "0 0 auto" }}>
      <defs>
        <clipPath id={`s${h}`}><path d={shield} /></clipPath>
      </defs>
      <g clipPath={`url(#s${h})`}>
        <rect x="0" y="0" width="32" height="38" fill={field} />
        {division === 1 && <rect x="16" y="0" width="16" height="38" fill={field2} />}
        {division === 2 && <rect x="0" y="19" width="32" height="19" fill={field2} />}
        {division === 3 && <path d="M0 0 L32 0 L32 38 Z" fill={field2} />}
        {charge((h >> 23) , charge2)}
      </g>
      <path d={shield} fill="none" stroke="#0c1118" strokeWidth="1.6" />
      <path d={shield} fill="none" stroke="#c9a227" strokeWidth="0.6" opacity="0.6" />
    </svg>
  );
}
