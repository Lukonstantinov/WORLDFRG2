import { useId } from "react";
import { CoatOfArms } from "./CoatOfArms";

/** The monetary metal a coin is struck in — chosen from what bullion the issuing
 *  polis can actually reach (gold/silver deposits in its region), so the map's
 *  geology shows in its money. */
export type CoinMetal = "gold" | "silver" | "electrum" | "bronze";

interface Palette { hi: string; mid: string; lo: string; edge: string; rimHi: string; rimLo: string; wear: string }
const METALS: Record<CoinMetal, Palette> = {
  gold:     { hi: "#fdec95", mid: "#e8c452", lo: "#aa7a1b", edge: "#7a530f", rimHi: "#ffe98f", rimLo: "#6b4a0d", wear: "#3a2c0c" },
  silver:   { hi: "#f6f8fb", mid: "#c4cdd8", lo: "#8b98a6", edge: "#5b6773", rimHi: "#fbfdff", rimLo: "#4c5661", wear: "#20262c" },
  electrum: { hi: "#f5edb8", mid: "#dcd083", lo: "#a99a4e", edge: "#736a32", rimHi: "#f8f0be", rimLo: "#645c2b", wear: "#2f2c12" },
  bronze:   { hi: "#efc196", mid: "#c07f45", lo: "#824c22", edge: "#532f14", rimHi: "#f2c99e", rimLo: "#48280f", wear: "#2a1608" },
};

/** DLC 3.5 · a struck coin rendered as a realistic minted disc: a reeded (milled)
 *  edge, a raised rim lit from the top-left, a domed metallic field, a beaded
 *  inner ring and a specular gloss, with the issuer's coat of arms embossed at
 *  the centre. `metal` picks the bullion (gold/silver/electrum/bronze) from what
 *  the polis can reach; `value` (agio) then modulates the strike — a strong coin
 *  gleams, a debased one is worn/tarnished — so soundness reads at a glance. */
export function CoinIcon({ issuer, size = 22, value, metal = "gold", title }: {
  issuer?: string;
  size?: number;
  value?: number;
  metal?: CoinMetal;
  title?: string;
}) {
  const uid = useId().replace(/:/g, "");
  const gid = (s: string) => `${s}${uid}`;
  const M = METALS[metal] ?? METALS.gold;

  // Agio → strike quality: a hard coin gleams brighter, a debased one tarnishes.
  const v = value ?? 1;
  const bright = v >= 1.05;
  const worn = v < 0.9;
  const gloss = bright ? 0.26 : worn ? 0.09 : 0.17;

  const cx = 50, cy = 50;
  const reeds = Array.from({ length: 48 }, (_, i) => {
    const a = (i / 48) * Math.PI * 2, c = Math.cos(a), s = Math.sin(a);
    return <line key={i} x1={cx + c * 43} y1={cy + s * 43} x2={cx + c * 47.5} y2={cy + s * 47.5}
      stroke={M.rimLo} strokeWidth={1.4} strokeLinecap="round" opacity={0.6} />;
  });
  const beads = Array.from({ length: 28 }, (_, i) => {
    const a = (i / 28) * Math.PI * 2;
    return <circle key={i} cx={cx + Math.cos(a) * 35.5} cy={cy + Math.sin(a) * 35.5} r={1.35}
      fill={M.edge} opacity={0.5} />;
  });

  const armSize = Math.max(8, Math.round(size * 0.46));
  return (
    <span title={title} style={{
      position: "relative", display: "inline-flex", alignItems: "center", justifyContent: "center",
      width: size, height: size, flex: "0 0 auto",
    }}>
      <svg width={size} height={size} viewBox="0 0 100 100"
        style={{ position: "absolute", inset: 0, display: "block", filter: "drop-shadow(0 1px 1.5px rgba(0,0,0,0.55))" }}>
        <defs>
          <radialGradient id={gid("face")} cx="38%" cy="30%" r="75%">
            <stop offset="0%" stopColor={M.hi} />
            <stop offset="44%" stopColor={M.mid} />
            <stop offset="82%" stopColor={M.lo} />
            <stop offset="100%" stopColor={M.edge} />
          </radialGradient>
          <linearGradient id={gid("rim")} x1="12%" y1="8%" x2="82%" y2="96%">
            <stop offset="0%" stopColor={M.rimHi} />
            <stop offset="50%" stopColor={M.mid} />
            <stop offset="100%" stopColor={M.rimLo} />
          </linearGradient>
        </defs>
        {/* Milled edge band */}
        <g>{reeds}</g>
        {/* Raised rim */}
        <circle cx={cx} cy={cy} r={45} fill={`url(#${gid("rim")})`} />
        {/* Inner field (the flan) */}
        <circle cx={cx} cy={cy} r={39} fill={`url(#${gid("face")})`} />
        {/* A debased coin wears a dull tarnish veil */}
        {worn && <circle cx={cx} cy={cy} r={39} fill={M.wear} opacity={0.2} />}
        {/* Rim/field step (raised-rim relief) */}
        <circle cx={cx} cy={cy} r={39.4} fill="none" stroke="rgba(0,0,0,0.3)" strokeWidth={1.3} />
        <circle cx={cx} cy={cy} r={44.4} fill="none" stroke="rgba(255,255,255,0.22)" strokeWidth={0.8} />
        {/* Beaded inner ring */}
        <g>{beads}</g>
        {/* Top specular gloss (brighter on a hard coin) */}
        <ellipse cx={40} cy={29} rx={21} ry={11} fill="#ffffff" opacity={gloss} />
      </svg>
      {/* Central device — coat of arms, struck into the metal (drop-shadow = relief). */}
      {issuer
        ? <span style={{ position: "relative", lineHeight: 0, filter: "drop-shadow(0 0.5px 0.4px rgba(0,0,0,0.55))" }}>
            <CoatOfArms name={issuer} size={armSize} />
          </span>
        : <span style={{ position: "relative", fontSize: armSize * 1.0, lineHeight: 1, color: M.edge,
            textShadow: "0 0.5px 0 rgba(255,255,255,0.3)" }}>✦</span>}
    </span>
  );
}
