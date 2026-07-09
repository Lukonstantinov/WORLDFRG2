import { useId } from "react";
import { CoatOfArms } from "./CoatOfArms";

/** DLC 3.5 · a struck coin rendered as a realistic minted disc: a reeded (milled)
 *  edge, a raised rim lit from the top-left, a domed metallic field, a beaded
 *  inner ring and a specular gloss, with the issuer's coat of arms embossed at
 *  the centre (its heraldry rides the coin, à la a Venetian ducat). `value` (>0)
 *  picks the METAL — a strong agio strikes bright gold, a debased coin a worn,
 *  pale metal — so the money's soundness reads at a glance. */
export function CoinIcon({ issuer, size = 22, value, title }: {
  issuer?: string;
  size?: number;
  value?: number;
  title?: string;
}) {
  // Unique gradient ids per instance (many coins share the document).
  const uid = useId().replace(/:/g, "");
  const gid = (s: string) => `${s}${uid}`;

  // Metal tier from the coin's value/agio: bright gold (hard) → gold → worn pale.
  const v = value ?? 1;
  const M = v >= 1.05
    ? { hi: "#fdec95", mid: "#e8c452", lo: "#aa7a1b", edge: "#7a530f", rimHi: "#ffe98f", rimLo: "#6b4a0d" }
    : v < 0.9
    ? { hi: "#e8ddb4", mid: "#c3b279", lo: "#8b7d49", edge: "#635832", rimHi: "#ece1b8", rimLo: "#574e2e" }
    : { hi: "#f4da77", mid: "#cfa836", lo: "#916e17", edge: "#6d520f", rimHi: "#f7e187", rimLo: "#5c440c" };

  const cx = 50, cy = 50;
  // Reeded (milled) edge — thin radial ticks around the circumference.
  const reeds = Array.from({ length: 48 }, (_, i) => {
    const a = (i / 48) * Math.PI * 2, c = Math.cos(a), s = Math.sin(a);
    return <line key={i} x1={cx + c * 43} y1={cy + s * 43} x2={cx + c * 47.5} y2={cy + s * 47.5}
      stroke={M.rimLo} strokeWidth={1.4} strokeLinecap="round" opacity={0.6} />;
  });
  // Beaded inner ring — the ring of raised dots classic on ducats/florins.
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
          {/* Domed metal: a specular hot-spot up-left falling to a dark edge. */}
          <radialGradient id={gid("face")} cx="38%" cy="30%" r="75%">
            <stop offset="0%" stopColor={M.hi} />
            <stop offset="44%" stopColor={M.mid} />
            <stop offset="82%" stopColor={M.lo} />
            <stop offset="100%" stopColor={M.edge} />
          </radialGradient>
          {/* Raised rim lit top-left → dark bottom-right. */}
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
        {/* Rim/field step — a thin inner shadow that reads as a raised rim */}
        <circle cx={cx} cy={cy} r={39.4} fill="none" stroke="rgba(0,0,0,0.3)" strokeWidth={1.3} />
        <circle cx={cx} cy={cy} r={44.4} fill="none" stroke="rgba(255,255,255,0.22)" strokeWidth={0.8} />
        {/* Beaded inner ring */}
        <g>{beads}</g>
        {/* Top specular gloss */}
        <ellipse cx={40} cy={29} rx={21} ry={11} fill="#ffffff" opacity={0.16} />
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
