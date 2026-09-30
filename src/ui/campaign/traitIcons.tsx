import type { CSSProperties } from "react";
import { T, FZ } from "@ui/campaign/chronicleTheme";

/** One shared vocabulary for a person's traits, a seat's office and the path
 *  that put them there — so a trait reads the same icon + colour in the
 *  council chamber, the notable roster and a life story. Names are exactly
 *  `individuals.rs::trait_name`'s. */
type Tone = "virtue" | "vice" | "skill" | "life" | "body" | "repute";
const TONE_COLOR: Record<Tone, string> = {
  virtue: "#7fd0a0", vice: "#e08a6a", skill: "#7ab0e8", life: "#c9a86a", body: "#b09ac8", repute: "#e8c84a",
};

export const TRAIT_META: Record<string, { icon: string; tone: Tone; blurb: string }> = {
  Kind: { icon: "💛", tone: "virtue", blurb: "moved by others' suffering" },
  Cruel: { icon: "🗡", tone: "vice", blurb: "unmoved by it" },
  Brave: { icon: "🦁", tone: "virtue", blurb: "stands firm under threat" },
  Craven: { icon: "🐇", tone: "vice", blurb: "runs from danger" },
  Honest: { icon: "⚖", tone: "virtue", blurb: "keeps faith, refuses bribes" },
  Deceitful: { icon: "🎭", tone: "vice", blurb: "lies when it pays" },
  Proud: { icon: "🦚", tone: "vice", blurb: "cannot bear a slight" },
  Humble: { icon: "🙏", tone: "virtue", blurb: "lets insults pass" },
  Ambitious: { icon: "🧗", tone: "life", blurb: "reaches for more" },
  Content: { icon: "😌", tone: "life", blurb: "happy with what they have" },
  Curious: { icon: "🔭", tone: "virtue", blurb: "drawn to new ideas" },
  Closed: { icon: "🚪", tone: "vice", blurb: "distrusts novelty" },
  Loyal: { icon: "🤝", tone: "virtue", blurb: "stands by friends and patrons" },
  Fickle: { icon: "🍃", tone: "vice", blurb: "changes sides" },
  Temperate: { icon: "🍵", tone: "virtue", blurb: "measured, never rash" },
  Impulsive: { icon: "⚡", tone: "vice", blurb: "acts first" },
  Generous: { icon: "🎁", tone: "virtue", blurb: "gives freely" },
  Greedy: { icon: "💰", tone: "vice", blurb: "keeps everything" },
  Orator: { icon: "🗣", tone: "skill", blurb: "sways a crowd" },
  Strategist: { icon: "♟", tone: "skill", blurb: "wins by the plan" },
  Scholar: { icon: "📚", tone: "skill", blurb: "learned" },
  Administrator: { icon: "📋", tone: "skill", blurb: "runs an office well" },
  Seafarer: { icon: "⚓", tone: "skill", blurb: "at home on deep water" },
  "Physician-trained": { icon: "⚕", tone: "skill", blurb: "trained in medicine" },
  Drunkard: { icon: "🍷", tone: "life", blurb: "drinks deep" },
  Gambler: { icon: "🎲", tone: "life", blurb: "stakes too much" },
  Ascetic: { icon: "🕯", tone: "life", blurb: "lives plainly" },
  Glutton: { icon: "🍖", tone: "life", blurb: "eats too well" },
  Hunter: { icon: "🏹", tone: "life", blurb: "loves the chase" },
  "Patron of the Arts": { icon: "🎨", tone: "life", blurb: "keeps artists" },
  "One-eyed": { icon: "👁", tone: "body", blurb: "lost an eye" },
  Lame: { icon: "🦯", tone: "body", blurb: "walks with a limp" },
  Scarred: { icon: "🩹", tone: "body", blurb: "carries old wounds" },
  Sickly: { icon: "🤒", tone: "body", blurb: "often ill" },
  Robust: { icon: "💪", tone: "body", blurb: "rarely ill" },
  "Maimed hand": { icon: "✋", tone: "body", blurb: "a crippled hand" },
  Hero: { icon: "🏅", tone: "repute", blurb: "hailed for courage" },
  Coward: { icon: "🐔", tone: "vice", blurb: "remembered for fleeing" },
  "Oath-breaker": { icon: "💔", tone: "vice", blurb: "broke a sworn word" },
  Benefactor: { icon: "🏛", tone: "repute", blurb: "fed or funded the city" },
  Bought: { icon: "🪙", tone: "vice", blurb: "known to take bribes" },
  "Kin-slayer": { icon: "☠", tone: "vice", blurb: "shed kin blood" },
};

export const traitColor = (name: string) => TONE_COLOR[TRAIT_META[name]?.tone ?? "life"];

/** A trait as an icon chip; `strength` 2 reads "deeply" (a doubled ring). */
export function TraitChip({ name, strength = 1, compact, style }: { name: string; strength?: number; compact?: boolean; style?: CSSProperties }) {
  const m = TRAIT_META[name] ?? { icon: "•", tone: "life" as Tone, blurb: "" };
  const c = TONE_COLOR[m.tone];
  return (
    <span title={`${strength >= 2 ? "Deeply " : ""}${name}${m.blurb ? ` — ${m.blurb}` : ""}`}
      style={{
        display: "inline-flex", alignItems: "center", gap: 3, whiteSpace: "nowrap",
        fontSize: FZ.micro, color: c, background: `${c}18`,
        border: `1px ${strength >= 2 ? "double" : "solid"} ${c}${strength >= 2 ? "cc" : "55"}`,
        borderWidth: strength >= 2 ? 3 : 1, borderRadius: 999,
        padding: compact ? "0 4px" : "0 6px", lineHeight: compact ? 1.4 : 1.6, ...style,
      }}>
      <span aria-hidden>{m.icon}</span>{!compact && <span>{name}</span>}
    </span>
  );
}

/** A seat's office by role — 0 the head (by government kind), 1 treasury,
 *  2 harbour, 3 justice, 4+ a councillor's seat. */
export function officeIcon(role: number, formKind?: string): string {
  if (role === 0) return formKind === "ruler" ? "👑" : formKind === "assembly" ? "🗳" : "⚜";
  return ["", "💰", "⚓", "⚖", "🪶"][Math.min(role, 4)] ?? "🪶";
}

/** Why they sit — `official_path_name`'s strings. */
export const PATH_META: Record<string, { icon: string; color: string; short: string }> = {
  "kin of a house": { icon: "👪", color: "#c9a227", short: "house kin" },
  "military success": { icon: "⚔", color: "#e07a5a", short: "war hero" },
  "wealth": { icon: "💰", color: "#d8b24a", short: "wealth" },
  "guild representative": { icon: "🔨", color: "#c98a3a", short: "guild" },
  "scholar/orator": { icon: "📜", color: "#6a8fd8", short: "scholar" },
  "elected by the commons": { icon: "🗳", color: "#7fd0a0", short: "elected" },
  "bribed in": { icon: "🪙", color: "#e08a6a", short: "bribed" },
  "appointed": { icon: "📋", color: "#9fb4cc", short: "appointed" },
};
export const pathMeta = (p: string) => PATH_META[p] ?? { icon: "•", color: T.inkMid, short: p };

/** The four ideology axes, both poles named. */
export const IDEOLOGY_AXES: { name: string; neg: string; pos: string; color: string }[] = [
  { name: "Authority", neg: "strong hand", pos: "the many", color: "#d88a6a" },
  { name: "Tradition", neg: "piety", pos: "inquiry", color: "#8a9ad8" },
  { name: "Openness", neg: "blood & soil", pos: "open gates", color: "#6ac0a0" },
  { name: "Economy", neg: "just price", pos: "free trade", color: "#d8b24a" },
];

/** A tiny four-bar ideology signature (each bar centred at 0, ±5). */
export function IdeologyBars({ v, width = 44, height = 14 }: { v: number[]; width?: number; height?: number }) {
  const bw = width / 4;
  const mid = height / 2;
  return (
    <svg width={width} height={height} style={{ display: "block" }}
      aria-label="ideology">
      <title>{IDEOLOGY_AXES.map((a, i) => `${a.name} ${v[i] >= 0 ? "+" : ""}${(v[i] ?? 0).toFixed(1)} (${(v[i] ?? 0) >= 0 ? a.pos : a.neg})`).join("\n")}</title>
      <line x1={0} x2={width} y1={mid} y2={mid} stroke={T.line} strokeWidth={1} />
      {IDEOLOGY_AXES.map((a, i) => {
        const val = Math.max(-5, Math.min(5, v[i] ?? 0));
        const h = Math.abs(val) / 5 * (mid - 1);
        return <rect key={a.name} x={i * bw + 1.5} width={bw - 3} y={val >= 0 ? mid - h : mid} height={Math.max(h, 0.8)} fill={a.color} rx={1} />;
      })}
    </svg>
  );
}
