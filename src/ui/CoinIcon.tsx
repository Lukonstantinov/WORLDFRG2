import { CoatOfArms } from "./CoatOfArms";

/** DLC 3.5 · a minted coin rendered as a gold disc with the issuer's coat of arms
 *  embossed in the centre. The issuer is the city's council house (its heraldry
 *  rides the coin, à la a Venetian ducat); falls back to a generic mint mark when
 *  no issuer is known. `value` (>0) tints the rim — a strong agio glows brighter.
 */
export function CoinIcon({ issuer, size = 22, value, title }: {
  issuer?: string;
  size?: number;
  value?: number;
  title?: string;
}) {
  const inner = Math.round(size * 0.62);
  // Rim brightness scales with the coin's value (≈1.2 strong → bright gold).
  const strong = (value ?? 1) >= 1.05;
  const rim = strong ? "#f0d061" : "#c9a227";
  const rim2 = strong ? "#9c7a16" : "#7c6111";
  return (
    <span
      title={title}
      style={{
        display: "inline-flex", alignItems: "center", justifyContent: "center",
        width: size, height: size, borderRadius: "50%", flex: "0 0 auto",
        background: `radial-gradient(circle at 38% 32%, ${rim} 0%, ${rim} 45%, ${rim2} 100%)`,
        border: `1px solid ${rim2}`,
        boxShadow: "inset 0 0 2px rgba(0,0,0,0.5), 0 1px 2px rgba(0,0,0,0.4)",
      }}
    >
      {issuer
        ? <CoatOfArms name={issuer} size={inner} />
        : <span style={{ fontSize: inner * 0.8, lineHeight: 1, color: "#5c4a10" }}>✦</span>}
    </span>
  );
}
