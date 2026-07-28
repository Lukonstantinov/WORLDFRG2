import type { ZonalProfile } from "@types";

/**
 * The TIER-1 preview: a latitude strip showing what the planetary knobs do
 * before anything is generated.
 *
 * Latitude runs down the vertical axis (90 N at top, 90 S at bottom) so it reads
 * the same way as the map beside it. Three things are layered:
 *
 *  1. A **climate band** column — the indicative Köppen main class per latitude.
 *  2. The **temperature curve** for this world, with Earth's ghosted behind it,
 *     so what you read is the ANOMALY rather than an absolute number.
 *  3. The **seasonal envelope** — a shaded ribbon between the coldest and
 *     warmest month a continental interior would see at that latitude.
 *
 * Plus belt markers for the Hadley edge and polar front, and a shaded band for
 * the part of the globe the current latitude frame actually shows.
 *
 * Pure SVG, no dependencies, no canvas. Everything is derived from a single
 * cheap backend call, so this redraws freely as the user drags.
 */

const W = 154;   // strip width (fits the workflow panel)
const H = 190;   // strip height
const PAD_L = 20; // room for the latitude labels
const PAD_R = 6;
const PAD_T = 8;
const PAD_B = 12;

/** Köppen main-class colours — same five families as the coarse thumbnail. */
const CLASS_COLOR: Record<string, string> = {
  A: "#187a3c",
  B: "#d6a84a",
  C: "#7ebe5c",
  D: "#5c94b8",
  E: "#e2e9f0",
};

export function ZonalStrip({ profile }: { profile: ZonalProfile | null }) {
  if (!profile || profile.samples.length === 0) {
    return (
      <div style={{ height: H, display: "flex", alignItems: "center", justifyContent: "center",
        color: "#3d5266", fontSize: 9 }}>
        computing…
      </div>
    );
  }

  const plotW = W - PAD_L - PAD_R;
  const plotH = H - PAD_T - PAD_B;

  // Latitude → y. 90 N at the top.
  const latY = (lat: number) => PAD_T + ((90 - lat) / 180) * plotH;

  // Temperature → x. A fixed −60…+40 °C window keeps the curve comparable
  // between settings — an auto-scaled axis would make every world look the same.
  const T_MIN = -60;
  const T_MAX = 40;
  const tempX = (t: number) =>
    PAD_L + ((Math.max(T_MIN, Math.min(T_MAX, t)) - T_MIN) / (T_MAX - T_MIN)) * plotW;

  const line = (pick: (s: ZonalProfile["samples"][0]) => number) =>
    profile.samples.map((s) => `${tempX(pick(s)).toFixed(1)},${latY(s.lat).toFixed(1)}`).join(" ");

  // Seasonal envelope as a closed ribbon: down the winter edge, back up the summer.
  const envelope = [
    ...profile.samples.map((s) => `${tempX(s.winterContinental).toFixed(1)},${latY(s.lat).toFixed(1)}`),
    ...[...profile.samples].reverse().map((s) => `${tempX(s.summerContinental).toFixed(1)},${latY(s.lat).toFixed(1)}`),
  ].join(" ");

  // Class band column, drawn as one rect per sample so it needs no run-merging.
  const bandW = 7;
  const step = plotH / (profile.samples.length - 1);

  const beltMark = (lat: number, color: string, label: string) => (
    <g key={`${label}${lat}`}>
      {[lat, -lat].map((l) => (
        <line key={l} x1={PAD_L} y1={latY(l)} x2={W - PAD_R} y2={latY(l)}
          stroke={color} strokeWidth={0.7} strokeDasharray="3 2" opacity={0.75} />
      ))}
      <text x={W - PAD_R} y={latY(lat) - 1.5} textAnchor="end" fontSize={7} fill={color}>
        {label} {Math.round(lat)}°
      </text>
    </g>
  );

  // The slice of the globe the map actually shows under the current frame.
  const cropTop = latY(Math.min(90, profile.visibleTop));
  const cropBot = latY(Math.max(-90, profile.visibleBottom));

  return (
    <svg width={W} height={H} style={{ display: "block" }}>
      {/* Cropped-out regions: dim whatever the latitude frame hides. */}
      {cropTop > PAD_T && (
        <rect x={PAD_L} y={PAD_T} width={plotW} height={cropTop - PAD_T} fill="#000" opacity={0.45} />
      )}
      {cropBot < PAD_T + plotH && (
        <rect x={PAD_L} y={cropBot} width={plotW} height={PAD_T + plotH - cropBot} fill="#000" opacity={0.45} />
      )}

      {/* Climate band column */}
      {profile.samples.map((s, i) => (
        <rect key={i} x={PAD_L} y={PAD_T + i * step - step / 2} width={bandW} height={step + 0.6}
          fill={CLASS_COLOR[s.zone] ?? "#5a6068"} />
      ))}

      {/* Freezing reference */}
      <line x1={tempX(0)} y1={PAD_T} x2={tempX(0)} y2={PAD_T + plotH}
        stroke="#3f5a72" strokeWidth={0.6} />
      <text x={tempX(0)} y={H - 3} textAnchor="middle" fontSize={7} fill="#4a6a86">0°C</text>

      {/* Equator */}
      <line x1={PAD_L} y1={latY(0)} x2={W - PAD_R} y2={latY(0)}
        stroke="#5d7f9c" strokeWidth={0.8} />

      {/* Belt markers */}
      {beltMark(profile.hadleyEdge, "#d8a24a", "H")}
      {beltMark(profile.polarFront, "#7fb2e0", "PF")}

      {/* Seasonal envelope (continental interior) */}
      <polygon points={envelope} fill="#4a90d0" opacity={0.16} />

      {/* Earth ghost, then this world */}
      <polyline points={line((s) => s.tempEarth)} fill="none"
        stroke="#5a6a7c" strokeWidth={1} strokeDasharray="2 2" opacity={0.8} />
      <polyline points={line((s) => s.temp)} fill="none"
        stroke="#ffcf70" strokeWidth={1.6} />

      {/* Latitude labels */}
      {[90, 60, 30, 0, -30, -60, -90].map((l) => (
        <text key={l} x={PAD_L - 3} y={latY(l) + 2.5} textAnchor="end" fontSize={7} fill="#4a6a86">
          {l > 0 ? `${l}N` : l < 0 ? `${-l}S` : "0"}
        </text>
      ))}
    </svg>
  );
}

/** The legend + numeric readouts that sit under the strip. */
export function ZonalReadout({ profile }: { profile: ZonalProfile | null }) {
  if (!profile) return null;
  const dT = profile.globalMean - profile.globalMeanEarth;
  const warmer = dT >= 0;
  return (
    <div style={{ fontSize: 9, color: "#5a7390", lineHeight: 1.5, marginTop: 3 }}>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 3 }}>
        <span><span style={{ color: "#ffcf70" }}>━</span> this world</span>
        <span><span style={{ color: "#5a6a7c" }}>┅</span> Earth</span>
        <span><span style={{ color: "#4a90d0" }}>▬</span> seasonal range</span>
      </div>
      <div>
        Global mean <b style={{ color: "#a8c0d8" }}>{profile.globalMean.toFixed(1)}°C</b>
        {" "}({warmer ? "+" : "−"}{Math.abs(dT).toFixed(1)} vs Earth)
      </div>
      <div>
        Tropics to <b style={{ color: "#d8a24a" }}>{Math.round(profile.hadleyEdge)}°</b>
        {" · "}storm track <b style={{ color: "#7fb2e0" }}>{Math.round(profile.polarFront)}°</b>
        {" · "}{profile.cells} cell{profile.cells === 1 ? "" : "s"}/hemisphere
      </div>
      <div>
        {profile.iceLine >= 89.5
          ? "No permanent ice anywhere."
          : <>Permanent ice from <b style={{ color: "#cfe0f0" }}>{Math.round(profile.iceLine)}°</b> poleward.</>}
      </div>
      {profile.retrograde && (
        <div style={{ color: "#c8a8e0" }}>
          ↩ Retrograde: trade winds reversed, warm/cold currents on the opposite coasts.
        </div>
      )}
    </div>
  );
}
