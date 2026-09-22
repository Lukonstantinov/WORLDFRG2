import { useEffect, useMemo, useRef, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";
import { CoinIcon } from "@ui/heraldry/CoinIcon";
import { YearChronicle } from "@ui/campaign/YearChronicle";
import { drawFigure, resolveKit, type Occasion } from "@ui/campaign/cultureDress";
import { clarifyGemLabel } from "@goods";
import { goodIcon, TIER_META, tierOf, dull, familyRunAt } from "@ui/campaign/houseShared";
import {
  campaignGetFeuds, campaignHouseStability, campaignGetHouseHistory, campaignMerchantRoutes,
  campaignHouseLedger, campaignGetBanks, campaignGetExpeditions, campaignGetHouseKin,
  campaignGetHouseGoals, campaignGetHouseCrisis, campaignGetHouseLineage,
} from "@bridge";
import type {
  FeudRow, Gauge, HouseStability, HouseHistory, HouseBrief, MerchantRoute, HouseLedger,
  BankBrief, ExpeditionView, KinBrief, GoalsBrief, CrisisBrief, HouseLineage, LineageNode, HeadBrief,
} from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";

/** ⚜️ House Dossier — HOUSES_GUILDS_AND_MARKET_PLAN.md S10's own window: one
 *  house, big, floating, the tabs. Split out of the former monolithic
 *  HousesPanel.tsx (which kept the browse/list half — see that file), so the
 *  detail view (`HouseDetail` below, plus its ten subtabs) and this file's
 *  original two views (Standing, Feuds) live together as "everything about
 *  ONE house" rather than being spread across the file that browses ALL of
 *  them. A pure move: no rendering logic changed in the split itself.
 *
 *  **Standing** answers "is this family about to die, and of what?". The simulation
 *  already knew: `debt_since` is a literal twelve-month countdown to bankruptcy in
 *  `update_solvency`, and none of it reached the screen, so a house vanished between
 *  two advances. Four of the five gauges are pure derivations of state that already
 *  existed — nothing about the simulation changed to draw them.
 *
 *  **Feuds** shows the quarrels: their cause, their temperature, the stage they have
 *  escalated to, and how each one ended. A feud used to be a name in a rival list. */

// ── Standing ────────────────────────────────────────────────────────────────

const GAUGE_ICON: Record<string, string> = {
  solvency: "⚖", liquidity: "💧", exposure: "🎯", succession: "⏳", cohesion: "🔗",
};

/** Five pips and a phrase. Deliberately NOT a percentage: a raw 0..1 tells a player
 *  nothing, and a gauge that is fine stays quiet — five permanently-amber dials teach
 *  a player to ignore all five. */
function GaugeCard({ g }: { g: Gauge }) {
  const color = g.warn ? "#e0a09a" : g.pips >= 4 ? "#9fe0b8" : "#cfe0f4";
  return (
    <div style={{
      // Five gauges across a 460px panel gave each ~88px, which CLIPPED the phrase —
      // and the phrase is the product ("in the red 7 of 12 months" became
      // "in the red 7 of 12 months —…"). They now wrap to a 3+2 grid with a floor
      // wide enough for the longest phrase the backend can emit.
      flex: "1 1 132px", minWidth: 132, background: "#0d1622",
      border: `1px solid ${g.warn ? "#5a3630" : "#1e2e42"}`, borderRadius: 5,
      padding: "5px 6px",
    }} title={g.phrase}>
      <div style={{ display: "flex", alignItems: "center", gap: 3 }}>
        <span style={{ fontSize: 9 }}>{GAUGE_ICON[g.key] ?? "•"}</span>
        <span style={{
          color: "#6a86a6", fontSize: 8, textTransform: "uppercase", letterSpacing: 0.5,
        }}>{g.label}</span>
      </div>
      <div style={{ display: "flex", gap: 2, margin: "3px 0 2px" }}>
        {[1, 2, 3, 4, 5].map((p) => (
          <span key={p} style={{
            width: 6, height: 6, borderRadius: "50%",
            background: p <= g.pips ? color : "#1b2736",
          }} />
        ))}
      </div>
      {/* No clamp. A gauge that cannot say what is wrong is worse than no gauge, and
          the card now wraps rather than truncating. */}
      <div style={{ color, fontSize: 9, lineHeight: 1.25 }}>{g.phrase}</div>
    </div>
  );
}

const fmt = (v: number) => (Math.abs(v) >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(1));

export function HouseStandingView({ idx, refreshKey }: { idx: number; refreshKey?: number }) {
  const [st, setSt] = useState<HouseStability | null>(null);
  useEffect(() => {
    let alive = true;
    campaignHouseStability(idx)
      .then((s) => { if (alive) setSt(s); })
      .catch(() => { if (alive) setSt(null); });
    return () => { alive = false; };
  }, [idx, refreshKey]);

  if (!st) return <div style={dim}>No standing figures for this family.</div>;
  return (
    <div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 6 }}>
        {st.gauges.map((g) => <GaugeCard key={g.key} g={g} />)}
      </div>

      {/* The countdown. This is the single highest-value number in the panel: the sim
          has always known it, and the player could never see it. */}
      {st.debt_months > 0 && (
        <div style={{
          background: "#2a1512", border: "1px solid #5a3630", borderRadius: 5,
          padding: "5px 7px", marginBottom: 6,
        }}>
          <div style={{ color: "#e0a09a", fontSize: 10, fontWeight: 700 }}>
            In the red {st.debt_months} of {st.debt_limit} months
          </div>
          <div style={{ height: 4, background: "#170c0a", borderRadius: 2, margin: "3px 0", overflow: "hidden" }}>
            <div style={{
              width: `${(st.debt_months / Math.max(1, st.debt_limit)) * 100}%`,
              height: "100%", background: "#c9503f",
            }} />
          </div>
          <div style={{ color: "#a8836f", fontSize: 9 }}>
            A private house insolvent for a full year is declared bankrupt.
          </div>
        </div>
      )}

      <SubHead>Books</SubHead>
      <KV k="Liquid wealth" v={fmt(st.liquid)} />
      <KV k="Monthly outgoings" v={fmt(st.monthly_burn)} />
      <KV k="Committed liabilities" v={fmt(st.liabilities_total)} />
      {st.liabilities.length === 0 ? (
        <div style={dim}>Nothing committed — the family owes no-one.</div>
      ) : st.liabilities.map((l, i) => (
        <div key={i} style={{ display: "flex", gap: 6, fontSize: 9, padding: "1px 0" }}>
          <span style={{ color: "#9ab0c8", flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {l.label}
          </span>
          <span style={{ color: "#6a86a6", flex: "0 1 auto", fontSize: 8, maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {l.note}
          </span>
          <span style={{ color: "#e0c060", minWidth: 44, textAlign: "right" }}>{fmt(l.amount)}</span>
        </div>
      ))}

      <SubHead>Concentration</SubHead>
      {st.top_good ? (
        <Bar label={`${Math.round(st.top_good_share * 100)}% of its living from ${st.top_good}`}
          frac={st.top_good_share} warn={st.top_good_share > 0.6} />
      ) : <div style={dim}>No single trade dominates its earnings.</div>}
      {st.top_city && (
        <Bar label={`${Math.round(st.top_city_share * 100)}% of its standing in ${st.top_city}`}
          frac={st.top_city_share} warn={st.top_city_share > 0.7} />
      )}

      <SubHead>Head of family</SubHead>
      <KV k="Years in office" v={`${st.head_years} of about ${Math.max(1, st.head_span_years)}`} />
      {st.feuds_live > 0 && (
        <KV k="Quarrels running" v={`${st.feuds_live}${st.feuds_hot > 0 ? ` · ${st.feuds_hot} at trade war or worse` : ""}`} />
      )}
    </div>
  );
}

// ── Feuds ───────────────────────────────────────────────────────────────────

/** Stage colours, cold → hot. The stage is what a feud DOES, so it carries the colour. */
const STAGE_COLOR = ["#7a90a8", "#d8b45c", "#d98040", "#c9503f"];
const STAGE_ICON = ["·", "⚔", "🚫", "🗡"];

/** How a feud ended, as a chip. The old model had no endings at all — a quarrel ran
 *  until one side died — so this column is the visible part of the elaboration. */
function OutcomeChip({ f }: { f: FeudRow }) {
  if (f.running) return null;
  const tint: Record<string, string> = {
    arbitrated: "#7fd0c0", "sealed by marriage": "#d8a0c8",
    "ended in ruin": "#c9503f", cooled: "#7a90a8",
  };
  const c = tint[f.outcome] ?? "#7a90a8";
  return (
    <span style={{
      fontSize: 8, color: c, border: `1px solid ${c}55`, borderRadius: 3,
      padding: "0 3px", whiteSpace: "nowrap",
    }}>{f.outcome} · yr {f.ended_year}</span>
  );
}

function FeudCard({ f, focus }: { f: FeudRow; focus?: number }) {
  const [open, setOpen] = useState(false);
  const stage = Math.min(3, Math.max(0, f.stage_idx));
  // When the dossier is focused on one house, name the OTHER family first — the
  // player is reading this family's quarrels, not an abstract pair.
  const weAreA = focus !== undefined && f.a === focus;
  const them = focus === undefined ? null : weAreA ? f.b_name : f.a_name;
  const ourLoss = weAreA ? f.damage_a : f.damage_b;
  const theirLoss = weAreA ? f.damage_b : f.damage_a;
  return (
    <div style={{
      background: "#0d1622", border: "1px solid #1e2e42", borderRadius: 5,
      padding: "5px 7px", marginBottom: 4, opacity: f.running ? 1 : 0.7,
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
        <span style={{ fontSize: 10 }}>{STAGE_ICON[stage]}</span>
        <span style={{ width: 8, height: 8, borderRadius: 2, background: f.a_color, flex: "0 0 auto" }} />
        <span style={{ width: 8, height: 8, borderRadius: 2, background: f.b_color, flex: "0 0 auto" }} />
        <span style={{
          color: "#e8dcc0", fontSize: 11, fontWeight: 600, flex: 1, minWidth: 0,
          overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
        }}>
          {them ?? `${f.a_name} vs ${f.b_name}`}
        </span>
        <OutcomeChip f={f} />
      </div>

      <div style={{ color: "#9ab0c8", fontSize: 9, marginTop: 1 }}>
        over {f.cause}
        {f.good ? ` · ${f.good}` : ""}
        {f.city ? ` · at ${f.city}` : ""}
      </div>

      {/* Temperature. A feud is not on/off — this is the axis the flat rival list
          could not express, and it is what decides which weapon comes out. */}
      <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 3 }}>
        <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
          <div style={{
            width: `${Math.min(100, f.intensity * 100)}%`, height: "100%",
            background: STAGE_COLOR[stage],
          }} />
        </div>
        <span style={{ color: STAGE_COLOR[stage], fontSize: 9, minWidth: 66, textAlign: "right" }}>
          {f.stage}
        </span>
      </div>

      <div style={{ color: "#6a86a6", fontSize: 8, marginTop: 2 }}>
        {f.running ? `running ${f.years}y` : `ran ${f.years}y from ${f.started_year}`}
        {" · "}{f.flares} flare{f.flares === 1 ? "" : "s"}
        {focus !== undefined
          ? ` · cost us ${fmt(ourLoss)}, them ${fmt(theirLoss)}`
          : ` · ${fmt(f.damage_a)} / ${fmt(f.damage_b)} lost`}
      </div>

      {f.log.length > 0 && (
        <>
          <div onClick={() => setOpen((o) => !o)} style={{
            color: "#7090b0", fontSize: 9, cursor: "pointer", marginTop: 3,
          }}>
            {open ? "▾" : "▸"} {f.log.length} recorded episode{f.log.length === 1 ? "" : "s"}
          </div>
          {open && f.log.slice().reverse().map((l, i) => (
            <div key={i} style={{ fontSize: 9, color: "#bcd0e4", padding: "1px 0 1px 10px" }}>
              <span style={{ color: "#6a86a6" }}>yr {l.year}</span>{" "}
              {l.text}
              {l.cost > 0.05 && <span style={{ color: "#c98" }}> (−{fmt(l.cost)})</span>}
            </div>
          ))}
        </>
      )}
    </div>
  );
}

/** The feud board. `house` < 0 shows every quarrel in the world. */
export function FeudsView({ house = -1, refreshKey }: { house?: number; refreshKey?: number }) {
  const [feuds, setFeuds] = useState<FeudRow[] | null>(null);
  const [showSettled, setShowSettled] = useState(false);
  useEffect(() => {
    let alive = true;
    campaignGetFeuds(house)
      .then((f) => { if (alive) setFeuds(f); })
      .catch(() => { if (alive) setFeuds([]); });
    return () => { alive = false; };
  }, [house, refreshKey]);

  if (!feuds) return <div style={dim}>Reading the quarrels…</div>;
  const live = feuds.filter((f) => f.running);
  const done = feuds.filter((f) => !f.running);
  if (feuds.length === 0) {
    return (
      <div style={dim}>
        No quarrels. Houses fall out when they live off the same trade in the same
        market, court the same council, or a match between them sours.
      </div>
    );
  }
  return (
    <div>
      {live.length === 0
        ? <div style={dim}>Nothing running — every quarrel has been settled.</div>
        : live.map((f, i) => <FeudCard key={`l${i}`} f={f} focus={house >= 0 ? house : undefined} />)}
      {done.length > 0 && (
        <>
          <div onClick={() => setShowSettled((s) => !s)} style={{
            color: "#7090b0", fontSize: 9, cursor: "pointer", margin: "6px 0 3px",
            textTransform: "uppercase", letterSpacing: 0.4,
          }}>
            {showSettled ? "▾" : "▸"} Settled ({done.length})
          </div>
          {showSettled && done.map((f, i) =>
            <FeudCard key={`d${i}`} f={f} focus={house >= 0 ? house : undefined} />)}
        </>
      )}
    </div>
  );
}

// ── shared bits ─────────────────────────────────────────────────────────────

function SubHead({ children }: { children: React.ReactNode }) {
  return (
    <div style={{
      color: "#6a86a6", fontSize: 8, textTransform: "uppercase", letterSpacing: 0.5,
      margin: "7px 0 2px", borderBottom: "1px solid #1a2a3e", paddingBottom: 1,
    }}>{children}</div>
  );
}

function KV({ k, v }: { k: string; v: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9, padding: "1px 0" }}>
      <span style={{ color: "#6a86a6" }}>{k}</span>
      <span style={{ color: "#bcd0e4" }}>{v}</span>
    </div>
  );
}

function Bar({ label, frac, warn }: { label: string; frac: number; warn?: boolean }) {
  return (
    <div style={{ margin: "2px 0 4px" }}>
      <div style={{ height: 4, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
        <div style={{
          width: `${Math.min(100, frac * 100)}%`, height: "100%",
          background: warn ? "#c9503f" : "#5a9bd4",
        }} />
      </div>
      <div style={{ color: warn ? "#e0a09a" : "#9ab0c8", fontSize: 9 }}>{label}</div>
    </div>
  );
}

const dim: React.CSSProperties = { color: "#6a86a6", fontSize: 9, padding: "4px 0" };

// ═════════════════════════════════════════════════════════════════════════════════
//  THE DETAIL WINDOW — moved verbatim from HousesPanel.tsx (S10 split)
// ═════════════════════════════════════════════════════════════════════════════════

/** Trait valence for the ruler's character words (from the sim's CHAR_ADJ table).
 *  A merchant's disposition reads as an asset, a liability, or a neutral bent —
 *  coloured green / red / grey so the head's character is legible at a glance. Words
 *  match `character_phrase` (`sim::tick`); an unlisted word falls back to neutral. */
const TRAIT_VALENCE: Record<string, 1 | 0 | -1> = {
  // caution ↔ boldness
  hoarding: -1, cautious: 0, bold: 1, reckless: -1,
  // honour ↔ greed
  scrupulous: 1, honourable: 1, grasping: -1, ruthless: -1,
  // private ↔ civic
  "close-fisted": -1, private: 0, "civic-minded": 1, openhanded: 1,
  // rooted ↔ expansive
  insular: -1, rooted: 0, expansive: 1, "far-reaching": 1,
};
const TRAIT_COLOR: Record<number, string> = { 1: "#7fd0a0", 0: "#9aa6b4", [-1]: "#e0857f" };
const TRAIT_BG: Record<number, string> = { 1: "rgba(127,208,160,0.12)", 0: "rgba(154,166,180,0.10)", [-1]: "rgba(224,133,127,0.12)" };
/** Split a `character_phrase` ("Bold, grasping, expansive.") into coloured chips. */
function traitChips(phrase: string): { word: string; v: number }[] {
  return phrase.replace(/\.\s*$/, "").split(",").map((w) => w.trim()).filter(Boolean)
    .map((w) => ({ word: w, v: TRAIT_VALENCE[w.toLowerCase()] ?? 0 }));
}

/** A small header, duplicated from HousesPanel.tsx's own `header` style rather than
 *  cross-imported — the two files must not depend on each other's internals. */
const dossierHeader: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "10px 12px", borderBottom: "1px solid #1a2a3e",
  color: "#e8dcc0", fontWeight: 700, fontSize: 13, letterSpacing: 0.2,
};

/** Click-through detail for one house/guild: where it's active, its offices and
 *  estates, its fleet, and its TOP 5 routes (back & forth) with the goods it moves
 *  each way and the volume. */
export function HouseDetail({ h, onClose, onChronicle, onSelectHouse }:
  { h: HouseBrief; onClose: () => void; onChronicle: (name: string) => void; onSelectHouse: (h: HouseBrief) => void }) {
  const allHouses = useCampaignStore((s) => s.houses);
  const jumpTo = (idx: number) => {
    const target = allHouses.find((x) => x.idx === idx);
    if (target) onSelectHouse(target);
  };
  const [routes, setRoutes] = useState<MerchantRoute[]>([]);
  const [ledger, setLedger] = useState<HouseLedger | null>(null);
  const [bank, setBank] = useState<BankBrief | null>(null);
  const [chron, setChron] = useState<HouseHistory | null>(null);
  const [expeds, setExpeds] = useState<ExpeditionView[]>([]);
  const [kin, setKin] = useState<KinBrief[]>([]);
  const [goals, setGoals] = useState<GoalsBrief | null>(null);
  const [crisis, setCrisis] = useState<CrisisBrief | null>(null);
  const [lineage, setLineage] = useState<HouseLineage | null>(null);
  // Chronicle-first (§2.3 of the design): the dossier has nothing for the player to
  // DECIDE, so the primary artefact is the family's record, not its balance sheet.
  const [view, setView] = useState<"chronicle" | "summary" | "kin" | "goals" | "crisis" | "lineage" | "standing" | "feuds" | "bank" | "ledger" | "expeditions">("chronicle");
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.house);
  const tick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
  useEffect(() => {
    let alive = true;
    campaignMerchantRoutes().then((rs) => {
      if (alive) setRoutes(rs.filter((r) => r.holder === h.name).sort((a, b) => b.volume - a.volume).slice(0, 5));
    }).catch(() => {});
    if (h.idx !== undefined) {
      campaignHouseLedger(h.idx).then((l) => { if (alive) setLedger(l); }).catch(() => {});
    }
    campaignGetHouseHistory(h.name).then((hh) => { if (alive) setChron(hh); }).catch(() => {});
    // Phase 1.3 · this house's own live expeditions (Expedition.house already existed;
    // dest_province is the one new field, so the tab can highlight where they're
    // actually reaching for on the province plate).
    if (h.idx !== undefined) {
      campaignGetExpeditions().then((p) => {
        if (alive) setExpeds(p.active.filter((e) => e.house === h.idx));
      }).catch(() => {});
      // Phase 2.1 · the kin roster (empty for a guild or an older save).
      campaignGetHouseKin(h.idx).then((k) => { if (alive) setKin(k); }).catch(() => {});
      // Phase 3.1 · this house's ambitions, active and historical.
      campaignGetHouseGoals(h.idx).then((g) => { if (alive) setGoals(g); }).catch(() => {});
      // Phase 3.2-3.6 · the live succession struggle (if any) + past risings.
      campaignGetHouseCrisis(h.idx).then((c) => { if (alive) setCrisis(c); }).catch(() => {});
      // Lineage: the chain this house descends from + what split off it directly.
      campaignGetHouseLineage(h.idx).then((l) => { if (alive) setLineage(l); }).catch(() => {});
    }
    // Find this family's bank (if any) so we can show its balance-sheet subtab.
    if (h.owns_bank) {
      campaignGetBanks().then((bs) => {
        if (alive) setBank(bs.find((b) => b.owner_idx === h.idx) ?? null);
      }).catch(() => {});
    } else { setBank(null); }
    return () => { alive = false; };
  }, [h.name, h.idx, h.owns_bank, tick]);
  const fmtW = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  const goodsStr = (gs: [string, number][]) => gs.slice(0, 3).map(([g, v]) => `${goodIcon(g)}${fmtW(v)}`).join(" ") || "—";
  const Row = ({ label, children }: { label: string; children: React.ReactNode }) => (
    <div style={{ fontSize: 9, marginTop: 3 }}>
      <span style={{ color: "#6a86a6", textTransform: "uppercase", letterSpacing: 0.4 }}>{label}</span>
      <div style={{ color: "#bcd0e4" }}>{children}</div>
    </div>
  );
  // The house portrait now wears the SAME pixel-treated dress-plate art the
  // Peoples panel's culture figures use (`cultureDress.ts`, via `drawFigure` —
  // the identical call `CultureFigures.tsx` makes), rather than the older
  // SVG figure system. That system draws a PEOPLE, not a named individual —
  // it carries no sex axis (see that file's own doc comment) — so a house's
  // portrait is now its SEAT CULTURE's costume plate, not a likeness of the
  // current head; the head's own name/sex/character still read from the text
  // beside it. Register follows tier: Tier 1 reads as finery before you read
  // a number, Tier 3/4 as plain working dress — the same mapping the old
  // figure used.
  const occasion: Occasion = (() => {
    const t = tierOf(h);
    return !h.tier ? "national" : t === 1 ? "ceremonial" : t >= 3 ? "everyday" : "national";
  })();
  const FIG_W = 118, FIG_SCALE = 2;
  const figureRef = useRef<HTMLCanvasElement | null>(null);
  useEffect(() => {
    const el = figureRef.current;
    if (!el) return;
    const K = resolveKit(h.kit != null && h.kit >= 0 ? h.kit : h.name, { region: "" });
    const figH = Math.round(FIG_W * 2.1);
    el.width = FIG_W * FIG_SCALE; el.height = figH * FIG_SCALE;
    el.style.width = FIG_W + "px"; el.style.height = figH + "px";
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    drawFigure(ctx, 0, 0, FIG_W * FIG_SCALE, K, { occasion });
  }, [h.kit, h.name, occasion]);
  // The ruler card that fills the header beside the figure: who leads, for how long,
  // their character read as coloured traits, and the family's two most recent deeds.
  const head = kin.find((k) => k.role === "head") ?? kin[0];
  const traits = head?.character_phrase ? traitChips(head.character_phrase) : [];
  const recentEvents = useMemo(() => {
    const evs = chron?.events ?? [];
    return [...evs].sort((x, y) => y.year - x.year).slice(0, 3);
  }, [chron]);
  return (
    <div data-draggable style={{ ...detailPanel, ...rootStyle }} onPointerDown={onPointerDown}>
      <div style={{ display: "flex", gap: 8 }}>
        {/* The figure — house identity is added as three marks: a coloured frame (the
            house's colour, standing in for the garment accent), a coat-of-arms badge
            at the shoulder, and the tier glyph. */}
        <div style={{ position: "relative", width: 130, flex: "0 0 auto" }} title={`${TIER_META[tierOf(h)].name} · standing ${((h.standing ?? 0) * 100).toFixed(0)}%`}>
          <div style={{
            width: 130, height: 295, borderRadius: 6, overflow: "hidden", background: "#0a1119",
            border: `3px solid ${h.is_guild ? dull(h.color ?? "") : (h.color ?? "#3a5570")}`,
            boxShadow: h.tier === 1 ? "0 0 16px rgba(201,162,39,0.4)" : "none",
            display: "flex", alignItems: "flex-end", justifyContent: "center", paddingBottom: 6,
          }}>
            <canvas ref={figureRef} style={{ display: "block" }} />
          </div>
          <div style={{ position: "absolute", top: -8, right: -8 }}>
            <CoatOfArms name={h.name} size={34} guild={h.is_guild} />
          </div>
          {!h.is_guild && h.tier ? (
            <div style={{ position: "absolute", bottom: -2, left: -2, fontSize: 18, color: "#c9a227",
              background: "#0c141edd", borderRadius: "50%", width: 26, height: 26, lineHeight: "26px", textAlign: "center" }}>
              {TIER_META[tierOf(h)].glyph}
            </div>
          ) : null}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 3, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ width: 10, height: 10, borderRadius: 2, background: h.is_guild ? dull(h.color ?? "") : (h.color ?? "#888"), alignSelf: "center" }} />
        {h.owns_bank && <span title="This family owns a chartered bank" style={{ fontSize: 11 }}>🏦</span>}
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13,
          textDecoration: h.owns_bank ? "underline" : "none", textDecorationColor: "#c9a227" }}>{h.name}</span>
        {h.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>COMPANY</span>}
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={onClose} style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>
      <div style={{ color: "#9ab0c8", fontSize: 10 }}>{h.head_name} · of {h.home_name} · gen {h.generation}</div>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ color: "#c9a227", fontSize: 11, fontWeight: 700 }}>wealth {fmtW(h.wealth)}</span>
        {h.coin_name && (h.coin_value ?? 0) > 0 && (
          <span style={{ color: "#9ab0c8", fontSize: 9 }} title={`Wealth denominated in the family's coin (grain-equivalent ÷ coin value ${(h.coin_value ?? 0).toFixed(2)}×)`}>
            ≈ {fmtW(h.wealth / (h.coin_value ?? 1))} {h.coin_name}
          </span>
        )}
        {h.coin_name ? (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 9, color: "#d8c878" }}
            title={`Mints the ${h.coin_name} · value ${(h.coin_value ?? 0).toFixed(2)}× · trust ${Math.round((h.coin_trust ?? 0) * 100)}%`}>
            <CoinIcon issuer={h.name} value={h.coin_value} size={16} /> mints {h.coin_name} · {(h.coin_value ?? 0).toFixed(2)}×
          </span>
        ) : null}
      </div>

      {/* The ruler card — fills the header beside the figure (previously blank): who
          rules, how long, their character as coloured traits, and the latest deeds. */}
      <div style={{ marginTop: 7, borderTop: "1px solid #1a2a3e", paddingTop: 6 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
          <span style={{ fontSize: 11 }}>👑</span>
          <span style={{ color: "#e8dcc0", fontSize: 11, fontWeight: 600 }}>{h.head_name}</span>
          <span style={{ color: "#8fa6be", fontSize: 9 }}>{h.head_female ? "♀" : "♂"}</span>
          <span style={{ flex: 1 }} />
          <span style={{ color: "#9ab0c8", fontSize: 9.5 }} title="Years the current head has led the house">
            {h.head_age > 0 ? `has ruled ${h.head_age}y` : "newly acceded"}
          </span>
        </div>
        {traits.length > 0 ? (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 5 }} title="The ruler's character — green favours the house, red is a liability, grey neutral">
            {traits.map((t, i) => (
              <span key={i} style={{
                fontSize: 9, color: TRAIT_COLOR[t.v], background: TRAIT_BG[t.v],
                border: `1px solid ${TRAIT_COLOR[t.v]}44`, borderRadius: 8, padding: "1px 7px",
                textTransform: "capitalize",
              }}>{t.word}</span>
            ))}
          </div>
        ) : (
          <div style={{ color: "#6a7a8a", fontSize: 9, marginTop: 4, fontStyle: "italic" }}>
            {h.is_guild ? "A civic guild — no single ruler's temperament." : "An even-tempered head — nothing marked."}
          </div>
        )}
        {recentEvents.length > 0 && (
          <div style={{ marginTop: 6 }}>
            <div style={{ color: "#6a86a6", fontSize: 8.5, textTransform: "uppercase", letterSpacing: 0.4 }}>Latest</div>
            {recentEvents.map((e, i) => (
              <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 5, fontSize: 9.5, marginTop: 2 }}>
                <span style={{ flex: "0 0 auto" }}>{EVENT_ICON[e.kind] ?? "·"}</span>
                <span style={{ color: EVENT_COLOR[e.kind] ?? "#9ab0c8", flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={e.text}>
                  {e.text}
                </span>
                <span style={{ color: "#56708e", flex: "0 0 auto" }}>yr {e.year}</span>
              </div>
            ))}
          </div>
        )}
      </div>
        </div>
      </div>

      {/* Subtabs — Chronicle first (§2.3: the dossier has nothing to DECIDE, so the
          record is the primary artefact). Accountant gets its own roomy view so
          expenses aren't clipped. */}
      <div style={{ display: "flex", gap: 4, margin: "7px 0 5px", borderBottom: "1px solid #1a2a3e", flexWrap: "wrap" }}>
        {(["chronicle", "summary", ...(kin.length > 0 ? ["kin" as const] : []),
           ...(goals && (goals.active.length > 0 || goals.history.length > 0) ? ["goals" as const] : []),
           ...(crisis && (crisis.active || crisis.history.length > 0) ? ["crisis" as const] : []),
           ...((lineage && (lineage.ancestors.length > 0 || lineage.offshoots.length > 0)) || (chron?.line?.length ?? 0) > 0 ? ["lineage" as const] : []),
           ...(expeds.length > 0 ? ["expeditions" as const] : []),
           "standing", "feuds", ...(bank ? ["bank" as const] : []), "ledger"] as const).map((t) => (
          <div key={t} onClick={() => setView(t)} style={{
            fontSize: 10, padding: "3px 9px", cursor: "pointer",
            color: view === t ? "#e8dcc0" : "#7090b0", fontWeight: view === t ? 700 : 400,
            borderBottom: view === t ? "2px solid #c9a227" : "2px solid transparent",
          }}>{t === "chronicle" ? "📜 Chronicle"
            : t === "summary" ? "Summary"
            : t === "kin" ? `👪 Kin (${kin.length})`
            : t === "goals" ? `🎯 Ambitions${goals && goals.active.length > 0 ? ` (${goals.active.length})` : ""}`
            : t === "crisis" ? `⚠ Crisis${crisis?.active ? ` r${crisis.active.round}/${crisis.active.round_cap}` : ""}`
            : t === "lineage" ? "🌳 Lineage"
            : t === "expeditions" ? `🧭 Expeditions (${expeds.length})`
            : t === "standing" ? "⚖ Standing"
            : t === "feuds" ? `⚔ Feuds${h.rivals.length > 0 ? ` (${h.rivals.length})` : ""}`
            : t === "bank" ? "🏦 Bank"
            : `📒 Accountant${ledger && ledger.year > 0 ? ` · yr ${ledger.year}` : ""}`}</div>
        ))}
      </div>

      {view === "chronicle" ? (
        <ChronicleTab h={h} chron={chron} onExpand={() => onChronicle(h.name)} fmt={fmtW} />
      ) : view === "kin" ? (
        <KinTab kin={kin} />
      ) : view === "goals" ? (
        <GoalsTab goals={goals} />
      ) : view === "crisis" ? (
        <CrisisTab crisis={crisis} />
      ) : view === "lineage" ? (
        <LineageTab lineage={lineage} current={h} onJump={jumpTo} line={chron?.line ?? []} />
      ) : view === "expeditions" ? (
        <ExpeditionsTab expeds={expeds} fmt={fmtW} />
      ) : view === "standing" ? (
        // The five stability gauges. Everything here was already in the sim — the
        // solvency countdown in particular has always decided whether this family
        // lives, and was the one number the UI never showed.
        <HouseStandingView idx={h.idx ?? 0} refreshKey={tick} />
      ) : view === "feuds" ? (
        <FeudsView house={h.idx ?? 0} refreshKey={tick} />
      ) : view === "bank" && bank ? (
        <BankSheet b={bank} fmt={fmtW} />
      ) : view === "summary" ? (
        <>
          <HouseStatGrid h={h} fmt={fmtW} />
          {h.active && h.active.length > 0 ? (
            <div style={{ margin: "2px 0 5px" }}>
              <div style={{ color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4 }}>
                Active in ({h.active.length}) — most influential first
              </div>
              {h.active.slice(0, 12).map((c, i) => {
                const mark = c.role === "seat" ? "👑" : c.role === "bailo" ? "🏛️"
                  : c.role === "owner" ? "🏭" : c.role === "dominant" ? "◆" : c.role === "office" ? "◇" : "·";
                const roleColor = c.role === "seat" ? "#f4c430" : c.role === "bailo" ? "#e0863a"
                  : c.role === "owner" ? "#d8a85a" : c.role === "dominant" ? "#cfe2f6" : "#9fb4cc";
                return (
                  <div key={i} style={{ display: "flex", alignItems: "center", gap: 5, padding: "1px 0" }}>
                    <span style={{ width: 16, textAlign: "center" }}>{mark}</span>
                    <span style={{ flex: 1, minWidth: 0, color: roleColor, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 11 }}>
                      {c.name}{c.role === "bailo" ? " · BAILO" : c.role === "dominant" ? " · dominates" : c.role === "owner" ? " · owns works here" : c.role === "office" ? " · office" : ""}
                      {c.contested && <span style={{ color: "#e08a8a" }}> ⚔</span>}
                    </span>
                    <div style={{ width: 54, height: 6, background: "#16202c", borderRadius: 3, overflow: "hidden" }}>
                      <div style={{ width: `${Math.round(Math.min(1, c.influence) * 100)}%`, height: "100%", background: roleColor }} />
                    </div>
                    <span style={{ width: 30, textAlign: "right", color: "#7a90a8", fontSize: 9 }}>{c.influence.toFixed(2)}</span>
                  </div>
                );
              })}
            </div>
          ) : (
            h.cities && h.cities.length > 0 && <Row label="Active in">{h.cities.slice(0, 10).join(" · ")}</Row>
          )}
          {h.offices && h.offices.length > 0 && (
            <Row label="Offices">
              {h.offices.map(([nm]) => `${nm}${familyRunAt(kin, nm) ? ` 👪${familyRunAt(kin, nm)}` : ""}`).join(" · ")}
            </Row>
          )}
          {h.estates && h.estates.length > 0 && (
            <Row label="Estates">
              {h.estates.map(([g, c]) => `${goodIcon(g)} ${g} (${c}${familyRunAt(kin, c) ? ` · 👪${familyRunAt(kin, c)}` : ""})`).join(" · ")}
            </Row>
          )}
          <Row label="Fleet">🚢 {h.fleet_sea ?? 0} · 🛶 {h.fleet_river ?? 0} · 🐫 {h.fleet_caravan ?? 0}</Row>
          {h.barred && h.barred.length > 0 && (
            <div style={{ fontSize: 9, marginTop: 3, color: "#e08a8a" }}>⚔ Barred from (trade war): {h.barred.join(" · ")}</div>
          )}
          <div style={{ color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, marginTop: 6 }}>Top routes (back &amp; forth)</div>
          {routes.length === 0 && <div style={{ color: "#56708e", fontSize: 9 }}>no active routes right now</div>}
          {routes.map((r, i) => (
            <div key={i} style={{ fontSize: 9, marginBottom: 3, borderBottom: "1px solid #131f2c", paddingBottom: 2 }}>
              <div style={{ color: "#cfe0f4" }}>{r.sea ? "🚢" : "🐫"} {r.a_name} ⇄ {r.b_name} <span style={{ color: "#6a86a6" }}>· vol {fmtW(r.volume)}</span></div>
              <div style={{ color: "#9ab0c8" }}>→ {goodsStr(r.out_goods)} · ← {goodsStr(r.ret_goods)}</div>
            </div>
          ))}
          <div onClick={() => onChronicle(h.name)} style={{ color: "#88a8c8", fontSize: 9, cursor: "pointer", marginTop: 5, textDecoration: "underline" }}>
            View family chronicle →
          </div>
        </>
      ) : ledger ? (
        <LedgerView l={ledger} fmt={fmtW} />
      ) : (
        <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>No completed year yet — the first year's books appear after a full year passes.</div>
      )}
    </div>
  );
}

/** Phase 1.4 · the dossier's DEFAULT tab (§2.3): the family's record, not its balance
 *  sheet — the succession line (who held it, at what age, how they came in, how the
 *  family fared, and the by-name they earned) and the year-grouped chronicle, with the
 *  positive events (§2.2) reading alongside the obituaries rather than being buried. */
function ChronicleTab({ h, chron, onExpand, fmt }:
  { h: HouseBrief; chron: HouseHistory | null; onExpand: () => void; fmt: (v: number) => string }) {
  const nowYear = useCampaignStore((s) => Math.floor((s.snapshot?.clock.tick ?? 0) / 365));
  if (!chron) return <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>Loading…</div>;
  const line = chron.line ?? [];
  const peakYear = Math.floor((h.peak_wealth_tick ?? 0) / 365);
  return (
    <div>
      <div style={{ color: "#9ab0c8", fontSize: 10, marginBottom: 6 }}>
        {chron.founder || `Founded in year ${chron.founded_year}`}
        {chron.defunct && <span style={{ color: "#d88" }}> · fallen</span>}
      </div>

      {/* Positive events (2.2) as a small highlight strip, not buried in the log. */}
      {(h.peak_wealth ?? 0) > 0 && (
        <div style={{ color: "#e6c878", fontSize: 9, marginBottom: 5 }} title="All-time peak wealth">
          🏆 Finest hour: {fmt(h.peak_wealth ?? 0)}{peakYear > 0 ? ` in year ${peakYear}` : ""}
        </div>
      )}

      {/* The succession line — Phase 0.4's record, first shown here. */}
      {line.length > 0 && (
        <>
          <div style={timelineHdr}>Succession ({line.length} head{line.length > 1 ? "s" : ""})</div>
          <div style={{ marginBottom: 8 }}>
            {line.map((p, i) => {
              const living = p.until_year === 0;
              const grew = p.wealth_end > p.wealth_start;
              const ruled = Math.max(0, (living ? nowYear : p.until_year) - p.since_year);
              return (
                <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 5, padding: "1px 0", fontSize: 10 }}>
                  <span style={{ width: 12, textAlign: "center" }}>{p.female ? "♀" : "♂"}</span>
                  <span style={{ color: living ? "#e8dcc0" : "#9ab0c8", flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {p.name}{p.epithet ? ` ${p.epithet}` : ""}
                  </span>
                  <span style={{ color: "#6a86a6", fontSize: 9 }}>
                    gen {p.generation} · {p.since_year}{living ? "–present" : `–${p.until_year}`}
                    {" "}· ruled {ruled}y
                  </span>
                  {!living && (
                    <span style={{ color: grew ? "#9fe0a8" : "#e0a09a", fontSize: 9 }}>{grew ? "▲" : "▼"}</span>
                  )}
                </div>
              );
            })}
          </div>
        </>
      )}

      <div style={timelineHdr}>Chronicle <span style={{ color: "#56708e", fontWeight: 400 }}>(click a year)</span></div>
      <YearChronicle entries={chron.events} icons={EVENT_ICON} colors={EVENT_COLOR} />
      <div onClick={onExpand} style={{ color: "#88a8c8", fontSize: 9, cursor: "pointer", marginTop: 6, textDecoration: "underline" }}>
        Expand chronicle (goods, colonies) →
      </div>
    </div>
  );
}

/** Phase 1.3 · this house's own live expeditions — leader, destination, progress,
 *  fleet, and (clicking a row) the destination's PROVINCE highlighted on the map, so
 *  a viewer can see what the venture is actually reaching for. Returned/failed
 *  ventures are not shown here yet — they aren't recorded per-house, only in the
 *  world journal and the map's ✕ overlay (`ColonialPanel`). */
function ExpeditionsTab({ expeds, fmt }: { expeds: ExpeditionView[]; fmt: (v: number) => string }) {
  const setSelectedProvince = useUIStore((s) => s.setSelectedProvince);
  if (expeds.length === 0) {
    return <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>No live expeditions right now.</div>;
  }
  return (
    <div>
      {expeds.map((e) => {
        const kind = e.ships > 0 ? "⛵" : "🐫";
        const leg = e.outbound ? "outbound" : "returning";
        return (
          <div key={e.id}
            onClick={() => e.dest_province >= 0 && setSelectedProvince(e.dest_province)}
            style={{ fontSize: 10, marginBottom: 6, borderBottom: "1px solid #131f2c", paddingBottom: 4,
              cursor: e.dest_province >= 0 ? "pointer" : "default" }}
            title={e.dest_province >= 0 ? "Show the destination province on the map" : undefined}>
            <div style={{ color: "#cfe0f4" }}>
              {kind} {e.leader} <span style={{ color: "#6a86a6" }}>→ {e.dest}</span>
              {e.dest_province >= 0 && <span style={{ color: "#88a8c8" }}> 🗺</span>}
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 2 }}>
              <span style={{ color: "#9ab0c8", fontSize: 9, width: 62 }}>{leg}</span>
              <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 3, overflow: "hidden" }}>
                <div style={{ width: `${Math.round(e.progress * 100)}%`, height: "100%",
                  background: e.outbound ? "#7fb0e0" : "#7fd0a0" }} />
              </div>
              <span style={{ color: "#7a90a8", fontSize: 9, width: 30, textAlign: "right" }}>
                {Math.round(e.progress * 100)}%
              </span>
            </div>
            <div style={{ color: "#9ab0c8", fontSize: 9, marginTop: 1 }}>
              {e.ships > 0 && `${e.ships} ships `}{e.caravans > 0 && `${e.caravans} caravans `}
              {goodIcon(e.good)} {e.good} · cost {fmt(e.cost)}
              {e.survived < 1 && <span style={{ color: "#e0a09a" }}> · {Math.round(e.survived * 100)}% survived</span>}
            </div>
            {e.hazards.length > 0 && (
              <div style={{ color: "#e0a09a", fontSize: 9 }}>⚠ {e.hazards.length} struggle{e.hazards.length > 1 ? "s" : ""} along the way</div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/** Phase 2.1/2.3/2.6 · the family roster. The head is always kin[0] and reads bold;
 *  everyone else shows their role, a character phrase (never four raw numbers — §3's
 *  own discipline), loyalty/skill bars, and their share of the house's internal power
 *  (§2.6, sums to 100 across the roster). A posted "factor" names the holding they
 *  run. Nothing here is wired to any decision yet — Phase 2.4/2.5, unbuilt. */
function KinTab({ kin }: { kin: KinBrief[] }) {
  if (kin.length === 0) {
    return <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>No roster on record.</div>;
  }
  const roleMark: Record<string, string> = {
    head: "★", heir: "◆", factor: "◇", idle: "·", "married out": "✕", dead: "†",
  };
  const roleColor: Record<string, string> = {
    head: "#e8dcc0", heir: "#cfe2f6", factor: "#9fb4cc", idle: "#7a90a8",
    "married out": "#6a7a8a", dead: "#566",
  };
  return (
    <div>
      {kin.map((k, i) => (
        <div key={i} style={{ padding: "4px 2px", borderBottom: "1px solid #131f2c" }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
            <span style={{ width: 12, textAlign: "center", color: roleColor[k.role] ?? "#9ab0c8" }}>
              {roleMark[k.role] ?? "·"}
            </span>
            <span style={{ color: roleColor[k.role] ?? "#9ab0c8", fontSize: 11, fontWeight: k.role === "head" ? 700 : 400 }}>
              {k.name}
            </span>
            <span style={{ color: "#6a86a6", fontSize: 9 }}>{k.female ? "♀" : "♂"} {k.age}y</span>
            <span style={{ flex: 1 }} />
            <span style={{ color: "#7a90a8", fontSize: 9 }}>{k.role}{k.posted_name ? ` · ${k.posted_name}` : ""}</span>
          </div>
          {k.character_phrase && (
            <div style={{ color: "#9ab0c8", fontSize: 9, marginTop: 1, fontStyle: "italic" }}>{k.character_phrase}</div>
          )}
          <div style={{ display: "flex", gap: 10, marginTop: 2 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 3 }}>
              <span style={{ color: "#6a86a6", fontSize: 8 }}>loyal</span>
              <div style={{ width: 36, height: 3, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                <div style={{ width: `${Math.round(k.loyalty * 100)}%`, height: "100%", background: "#7fd0a0" }} />
              </div>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 3 }}>
              <span style={{ color: "#6a86a6", fontSize: 8 }}>skill</span>
              <div style={{ width: 36, height: 3, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                <div style={{ width: `${Math.round(k.skill * 100)}%`, height: "100%", background: "#7fb0e0" }} />
              </div>
            </div>
            <span style={{ color: "#c9a227", fontSize: 8 }}>{k.power_share.toFixed(0)}% power</span>
          </div>
        </div>
      ))}
    </div>
  );
}

/** Phase 3.1 · a house's ambitions — active first (with a progress bar where the
 *  kind has an honest fraction, else just a phrase and a deadline), then achieved/
 *  failed outcomes as ✓/✗ (§2.1's own rule: one headline per row, everything else a
 *  phrase). A goal biases weights on decisions the house already makes; it's shown
 *  here as a fact about the family, not something the player sets. */
function GoalsTab({ goals }: { goals: GoalsBrief | null }) {
  const year = useCampaignStore((s) => Math.floor((s.snapshot?.clock.tick ?? 0) / 365));
  if (!goals || (goals.active.length === 0 && goals.history.length === 0)) {
    return <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>No ambitions on record.</div>;
  }
  return (
    <div>
      {goals.active.length > 0 && (
        <>
          <div style={timelineHdr}>Pursuing</div>
          {goals.active.map((g, i) => (
            <div key={i} style={{ marginBottom: 6 }}>
              <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                <span style={{ fontSize: 11 }}>🎯</span>
                <span style={{ color: "#e8dcc0", fontSize: 11 }}>{g.what}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#7a90a8", fontSize: 9 }}>by {g.deadline_year}</span>
              </div>
              {g.progress_frac >= 0 ? (
                <div style={{ height: 4, background: "#0a1018", borderRadius: 3, overflow: "hidden", marginTop: 2 }}>
                  <div style={{ width: `${Math.round(g.progress_frac * 100)}%`, height: "100%", background: "#c9a227" }} />
                </div>
              ) : (
                <div style={{ color: "#6a86a6", fontSize: 9, marginTop: 1 }}>
                  {g.deadline_year > year ? `${g.deadline_year - year} years left` : "due"}
                </div>
              )}
            </div>
          ))}
        </>
      )}
      {goals.history.length > 0 && (
        <>
          <div style={{ ...timelineHdr, marginTop: goals.active.length > 0 ? 8 : 0 }}>Outcomes</div>
          {goals.history.map((g, i) => (
            <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 5, padding: "1px 0" }}>
              <span style={{ color: g.state === 1 ? "#7fd0a0" : "#c98", fontSize: 10 }}>
                {g.state === 1 ? "✓" : "✗"}
              </span>
              <span style={{ color: "#9ab0c8", fontSize: 10 }}>{g.what}</span>
              <span style={{ flex: 1 }} />
              <span style={{ color: "#6a86a6", fontSize: 9 }}>{g.set_year}–{g.deadline_year}</span>
            </div>
          ))}
        </>
      )}
    </div>
  );
}

const CRISIS_OUTCOME_LABEL: Record<number, string> = { 1: "the ruler prevailed", 2: "DEPOSED", 3: "DISSOLVED" };
const CRISIS_ACTION_LABEL: Record<number, string> = {
  0: "conceded a holding", 1: "bought off the plot", 2: "launched a venture", 3: "stood firm",
};

/** Phase 3.2-3.6 · the succession struggle — a compact reading of
 *  `HOUSE_POWER_STRUGGLE_VIEW.md`'s window (two named factions in their own
 *  tinctures, a round log, the heir's choice), plus the permanent record of past
 *  risings. There is nothing here for the player to decide (observation only, per
 *  `HOUSE_SUCCESSION_CRISIS.md` decision 2) — every choice is the AI's. */
function CrisisTab({ crisis }: { crisis: CrisisBrief | null }) {
  if (!crisis || (!crisis.active && crisis.history.length === 0)) {
    return <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>No rising on record.</div>;
  }
  const c = crisis.active;
  return (
    <div>
      {crisis.secure_until_year > 0 && (
        <div style={{ color: "#7fd0a0", fontSize: 10, marginBottom: 6 }}>
          🛡 secure until {crisis.secure_until_year} — the house will not rise again so soon
        </div>
      )}
      {c && (
        <div style={{ marginBottom: 8 }}>
          <div style={{ color: "#e08a8a", fontSize: 10, marginBottom: 2 }}>
            ⚠ SUCCESSION CRISIS · opened {c.opened_year} · round {c.round} of {c.round_cap}
          </div>
          <div style={{ color: "#9ab0c8", fontSize: 9, marginBottom: 4 }}>cause: {c.cause}</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ width: 8, height: 8, borderRadius: "50%", background: c.loyalist_tint }} />
                <span style={{ color: "#e8dcc0", fontSize: 10 }}>{c.loyalist_name}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#cfe0f4", fontSize: 10 }}>{Math.round(c.head_support * 100)}%</span>
              </div>
              <div style={{ height: 5, background: "#0a1018", borderRadius: 3, marginTop: 2, overflow: "hidden" }}>
                <div style={{ width: `${Math.round(c.head_support * 100)}%`, height: "100%", background: c.loyalist_tint }} />
              </div>
              {c.head_motive && (
                <div style={{ color: "#7a90a8", fontSize: 9, marginTop: 3 }}>▸ {c.head_motive}</div>
              )}
            </div>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ width: 8, height: 8, borderRadius: "50%", background: c.plot_tint }} />
                <span style={{ color: "#e8dcc0", fontSize: 10 }}>{c.plot_name}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: "#cfe0f4", fontSize: 10 }}>{Math.round(c.plot_support * 100)}%</span>
              </div>
              <div style={{ height: 5, background: "#0a1018", borderRadius: 3, marginTop: 2, overflow: "hidden" }}>
                <div style={{ width: `${Math.round(c.plot_support * 100)}%`, height: "100%", background: c.plot_tint }} />
              </div>
              {c.plot_leader_name && (
                <div style={{ color: "#c99", fontSize: 9, marginTop: 3 }}>
                  ◆ {c.plot_leader_name} leads the plot{c.plot_leader_motive ? ` — ${c.plot_leader_motive}` : ""}
                </div>
              )}
            </div>
          </div>
          {c.heir_choice !== 2 && (
            <div style={{ color: "#7a90a8", fontSize: 9, marginTop: 1 }}>
              the heir {c.heir_choice === 0 ? "stood with the ruler" : "turned to the plot"}
            </div>
          )}
          {c.rounds.length > 0 && (
            <div style={{ marginTop: 6 }}>
              {c.rounds.map((r, i) => (
                <div key={i} style={{ fontSize: 9, padding: "2px 0", borderTop: "1px solid #16202c" }}>
                  <span style={{ color: r.result > 0 ? "#7fd0a0" : r.result < 0 ? "#e08a8a" : "#7a90a8" }}>
                    {r.result > 0 ? "✓" : r.result < 0 ? "✗" : "○"}
                  </span>{" "}
                  <span style={{ color: "#9ab0c8" }}>{CRISIS_ACTION_LABEL[r.action] ?? "acted"} — {r.text}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      {crisis.history.length > 0 && (
        <>
          <div style={timelineHdr}>Past risings ({crisis.history.length})</div>
          {crisis.history.map((r, i) => (
            <div key={i} style={{ marginBottom: 5, fontSize: 9 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ color: "#9ab0c8" }}>{r.opened_year}–{r.closed_year}</span>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: r.loyalist_tint }} />
                <span style={{ color: "#cfe0f4" }}>{r.loyalist_name}</span>
                <span style={{ color: "#6a86a6" }}>vs</span>
                <span style={{ width: 7, height: 7, borderRadius: "50%", background: r.plot_tint }} />
                <span style={{ color: "#cfe0f4" }}>{r.plot_name}</span>
              </div>
              <div style={{ color: "#7a90a8", marginTop: 1 }}>
                opened over {r.cause} · {r.rounds} rounds · peak plot {Math.round(r.peak_plot * 100)}%
              </div>
              <div style={{ color: r.outcome === 1 ? "#7fd0a0" : "#e08a8a", marginTop: 1 }}>
                {r.outcome === 1 ? "✓" : "✕"} {CRISIS_OUTCOME_LABEL[r.outcome] ?? ""}
                {r.outcome !== 1 && r.successor ? ` — ${r.successor} takes the seat` : ""}
              </div>
            </div>
          ))}
        </>
      )}
    </div>
  );
}

const ORIGIN_LABEL: Record<number, string> = {
  0: "founded",
  1: "chartered from a guild's own capital",
  2: "a wealthy cadet branch",
  3: "a co-heir's share, under Partible inheritance",
  4: "departed in a schism",
  5: "rose to rule a free city",
};
const ORIGIN_TAG: Record<number, { label: string; color: string }> = {
  1: { label: "GUILD-SEEDED", color: "#c9a227" },
  2: { label: "BRANCH", color: "#7fd0a0" },
  3: { label: "DIVISION", color: "#7fb0d0" },
  4: { label: "DEPARTED", color: "#e08a8a" },
  5: { label: "INDEPENDENCE", color: "#c9a227" },
};

/** Phase 5-adjacent · a house's LINEAGE — the chain it descends from (root first)
 *  and what split off it directly (branches, divisions, departures). Each row is a
 *  fact already recorded on `House.origin_house`/`origin_kind`, read here rather
 *  than reconstructed from chronicle text. Click a name to re-center the dossier on
 *  that house — deep multi-generation trees are navigated one hop at a time rather
 *  than fetched all at once. */
function LineageTab({ lineage, current, onJump, line }:
  { lineage: HouseLineage | null; current: HouseBrief; onJump: (idx: number) => void; line: HeadBrief[] }) {
  const nowYear = useCampaignStore((s) => Math.floor((s.snapshot?.clock.tick ?? 0) / 365));
  const hasTree = !!lineage && (lineage.ancestors.length > 0 || lineage.offshoots.length > 0);
  if (!hasTree && line.length === 0) {
    return <div style={{ color: "#56708e", fontSize: 10, padding: "8px 2px" }}>An original founding — no recorded ancestry or offshoots.</div>;
  }
  const Node = ({ n, self: isSelf }: { n: LineageNode; self?: boolean }) => (
    <div
      onClick={() => !isSelf && onJump(n.idx)}
      style={{
        display: "flex", alignItems: "flex-start", gap: 8, padding: "6px 8px", borderRadius: 4,
        cursor: isSelf ? "default" : "pointer",
        background: isSelf ? "rgba(201,162,39,0.10)" : "transparent",
        border: isSelf ? "1px solid #4a3d1e" : "1px solid transparent",
      }}
    >
      <span style={{ width: 8, height: 8, borderRadius: 2, background: n.color, marginTop: 4, flex: "0 0 auto" }} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ color: n.alive ? "#e8dcc0" : "#7a6a5a", fontWeight: isSelf ? 700 : 600, fontSize: 11 }}>
            {n.name}{!n.alive ? " (defunct)" : ""}
          </span>
          {ORIGIN_TAG[n.origin_kind] && (
            <span style={{ fontSize: 8.5, fontWeight: 700, color: ORIGIN_TAG[n.origin_kind].color,
              border: `1px solid ${ORIGIN_TAG[n.origin_kind].color}55`, borderRadius: 8, padding: "0 5px" }}>
              {ORIGIN_TAG[n.origin_kind].label}
            </span>
          )}
          <span style={{ color: "#6a86a6", fontSize: 9 }}>{n.origin_year}</span>
        </div>
        <div style={{ color: "#9ab0c8", fontSize: 10, marginTop: 1, lineHeight: 1.4 }}>{n.origin_text}</div>
      </div>
    </div>
  );
  return (
    <div>
      {hasTree && lineage && (
        <>
          {lineage.ancestors.length > 0 && (
            <>
              <div style={timelineHdr}>Descent ({lineage.ancestors.length} generation{lineage.ancestors.length === 1 ? "" : "s"})</div>
              {lineage.ancestors.map((n, i) => (
                <div key={n.idx} style={{ marginLeft: i * 14, borderLeft: i > 0 ? "1px dashed #2a3d52" : "none", paddingLeft: i > 0 ? 10 : 0 }}>
                  <Node n={n} />
                </div>
              ))}
            </>
          )}
          <div style={{ marginLeft: lineage.ancestors.length * 14, borderLeft: lineage.ancestors.length > 0 ? "1px dashed #2a3d52" : "none", paddingLeft: lineage.ancestors.length > 0 ? 10 : 0, marginTop: 2 }}>
            <Node n={{
              idx: current.idx ?? -1, name: current.name, alive: !current.defunct,
              tier: current.tier ?? 0, origin_kind: 0, origin_year: current.founded_year ?? 0,
              origin_text: ORIGIN_LABEL[0], color: current.color ?? "#888",
            }} self />
          </div>
          {lineage.offshoots.length > 0 && (
            <>
              <div style={{ ...timelineHdr, marginTop: 10 }}>Split off this house ({lineage.offshoots.length})</div>
              {lineage.offshoots.map((n) => (
                <div key={n.idx} style={{ marginLeft: 14, borderLeft: "1px dashed #2a3d52", paddingLeft: 10 }}>
                  <Node n={n} />
                </div>
              ))}
            </>
          )}
        </>
      )}

      {/* The house's own succession — every ruler it has had, with the years each
          held the seat. The inter-house tree above answers "where did this house come
          from"; this answers "who has led it". */}
      {line.length > 0 && (
        <>
          <div style={{ ...timelineHdr, marginTop: hasTree ? 12 : 0 }}>
            Rulers of this house ({line.length})
          </div>
          {line.map((p, i) => {
            const living = p.until_year === 0;
            const ruled = Math.max(0, (living ? nowYear : p.until_year) - p.since_year);
            const grew = p.wealth_end > p.wealth_start;
            return (
              <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 6, padding: "2px 4px", fontSize: 10,
                background: living ? "rgba(201,162,39,0.08)" : "transparent", borderRadius: 3 }}>
                <span style={{ width: 14, textAlign: "center", color: "#8fa6be" }}>{p.female ? "♀" : "♂"}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: living ? "#e8dcc0" : "#c3d2e2", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {p.name}{p.epithet ? ` ${p.epithet}` : ""}
                    {living && <span style={{ color: "#c9a227", fontSize: 8.5 }}> · reigning</span>}
                  </div>
                  <div style={{ color: "#6a86a6", fontSize: 8.5 }}>
                    gen {p.generation} · {p.accession} · acceded age {p.age_at_accession}
                  </div>
                </div>
                <div style={{ textAlign: "right", flex: "0 0 auto" }}>
                  <div style={{ color: "#c9a227", fontSize: 10, fontWeight: 600 }}>ruled {ruled}y</div>
                  <div style={{ color: "#56708e", fontSize: 8.5 }}>
                    {p.since_year}{living ? "–now" : `–${p.until_year}`}
                    {!living && <span style={{ color: grew ? "#7fd0a0" : "#e0857f" }}> {grew ? "▲" : "▼"}</span>}
                  </div>
                </div>
              </div>
            );
          })}
        </>
      )}
    </div>
  );
}

/** DLC 3.5 · a compact grid of a family's individual stats (top of the Summary). */
function HouseStatGrid({ h, fmt }: { h: HouseBrief; fmt: (v: number) => string }) {
  const year = useCampaignStore((s) => Math.floor((s.snapshot?.clock.tick ?? 0) / 365));
  const age = h.founded_year !== undefined ? Math.max(0, year - h.founded_year) : undefined;
  const Cell = ({ label, value, bar }: { label: string; value: React.ReactNode; bar?: number }) => (
    <div style={{ background: "#0a1119", border: "1px solid #1a2a3c", borderRadius: 4, padding: "3px 5px" }}>
      <div style={{ color: "#6a86a6", fontSize: 8, textTransform: "uppercase", letterSpacing: 0.3 }}>{label}</div>
      <div style={{ color: "#cfe0f4", fontSize: 10, fontWeight: 700 }}>{value}</div>
      {bar !== undefined && (
        <div style={{ height: 3, background: "#0a1018", borderRadius: 2, marginTop: 1, overflow: "hidden" }}>
          <div style={{ width: `${Math.min(100, bar * 100)}%`, height: "100%", background: "#c9a227" }} />
        </div>
      )}
    </div>
  );
  return (
    <>
      {h.archetype_perk ? (
        <div style={{ color: "#9fd0c0", fontSize: 9, marginBottom: 4 }}>{h.archetype_label} · {h.archetype_perk}</div>
      ) : null}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 4 }}>
        <Cell label="Prestige" value={(h.prestige ?? 0).toFixed(2)} bar={h.prestige} />
        <Cell label="Political power" value={(h.political_power ?? 0).toFixed(2)} bar={h.political_power} />
        <Cell label="Trade volume" value={fmt(h.volume ?? 0)} />
        <Cell label="Founded" value={age !== undefined ? `yr ${h.founded_year} · ${age}y` : "—"} />
        <Cell label="Controls" value={String(h.controls?.length ?? 0)} />
        <Cell label="Estates" value={String(h.estates?.length ?? 0)} />
        <Cell label="Offices" value={String(h.offices?.length ?? 0)} />
        <Cell label="Monopolies (all-time)" value={String(h.mono_ever_count ?? 0)} />
        {(h.worst_loss ?? 0) > 0.01 && <Cell label="Worst loss" value={`−${fmt(h.worst_loss ?? 0)}`} />}
        {h.coin_name ? <Cell label="Mints coin" value={`${(h.coin_value ?? 0).toFixed(2)}×`} /> : null}
      </div>
    </>
  );
}

/** DLC 3.5 · a bank's T-account balance sheet (the Bank subtab in a house detail). */
function BankSheet({ b, fmt }: { b: BankBrief; fmt: (v: number) => string }) {
  const assets = b.reserves + b.loans_out + b.real_estate;
  const liab = b.deposits + b.notes_issued;
  const fragile = b.reserve_ratio < 0.22;
  const Side = ({ title, rows, total, color }: { title: string; rows: [string, number][]; total: number; color: string }) => (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ color: "#8aa8c8", fontSize: 9, fontWeight: 700, borderBottom: "1px solid #1c2c40", paddingBottom: 2, marginBottom: 2 }}>{title}</div>
      {rows.map(([k, v]) => (
        <div key={k} style={{ display: "flex", justifyContent: "space-between", fontSize: 9.5, color: "#b8c8da" }}>
          <span>{k}</span><span>{fmt(v)}</span>
        </div>
      ))}
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 9.5, fontWeight: 700, color, borderTop: "1px solid #1c2c40", marginTop: 2, paddingTop: 1 }}>
        <span>Σ</span><span>{fmt(total)}</span>
      </div>
    </div>
  );
  return (
    <div style={{ padding: "2px 4px 6px", border: "1px solid #1b2a3c", borderRadius: 4, background: "#0a1119" }}>
      <div style={{ color: "#e8dcc0", fontSize: 11, fontWeight: 700 }}>{b.name}{b.defunct ? " · FAILED" : ""}</div>
      <div style={{ color: "#9ab0c8", fontSize: 9, marginBottom: 4 }}>{b.seat} · est. {b.founded_year}</div>
      <div style={{ display: "flex", gap: 12 }}>
        <Side title="Assets" color="#80c890" total={assets}
          rows={[["Specie reserves", b.reserves], ["Loans out", b.loans_out], ["Real estate", b.real_estate]]} />
        <Side title="Liabilities" color="#e0a880" total={liab}
          rows={[["Deposits", b.deposits], ["Notes issued", b.notes_issued], ["Equity", b.equity]]} />
      </div>
      <div style={{ display: "flex", gap: 10, fontSize: 9, color: "#8aa8c8", marginTop: 4, flexWrap: "wrap" }}>
        <span style={{ color: fragile ? "#e6303a" : "#8aa8c8" }} title="Reserves ÷ liabilities — below 22% = fragile">
          reserve ratio {Number.isFinite(b.reserve_ratio) ? `${Math.round(b.reserve_ratio * 100)}%` : "—"}{fragile ? " ⚠" : ""}
        </span>
        <span>{b.n_loans} loans</span>
        <span style={{ color: "#80c890" }}>+{fmt(b.interest_earned)} earned</span>
        {b.losses > 0.01 && <span style={{ color: "#e08080" }}>−{fmt(b.losses)} lost</span>}
      </div>
      {b.branches.length > 0 && (
        <div style={{ fontSize: 9, color: "#7fa0c0", marginTop: 2 }}>Counting-houses: {b.branches.join(", ")}</div>
      )}
      {b.events.length > 0 && (
        <div style={{ fontSize: 9, color: "#6a86a6", marginTop: 2, fontStyle: "italic" }}>{b.events[0]}</div>
      )}
    </div>
  );
}

/** The yearly ledger (Accountant view): Income then Expenditure stacked FULL-WIDTH
 *  (so nothing is clipped in the narrow panel) + NET + warehouse stock. Per-city
 *  tax/profit lines arrive sorted largest → lowest. */
function LedgerView({ l, fmt }: { l: HouseLedger; fmt: (v: number) => string }) {
  const head: React.CSSProperties = { color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, marginTop: 7, marginBottom: 2 };
  const Line = ({ label, amt, neg }: { label: string; amt: number; neg?: boolean }) => (
    <div style={{ display: "flex", justifyContent: "space-between", gap: 8, fontSize: 10, padding: "1px 0" }}>
      <span style={{ color: "#9ab0c8", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{label}</span>
      <span style={{ color: neg ? "#e0a0a0" : "#7fcf8f", flexShrink: 0 }}>{neg ? "−" : "+"}{fmt(Math.abs(amt))}</span>
    </div>
  );
  const Sub = ({ label, amt, color }: { label: string; amt: number; color: string }) => (
    <div style={{ display: "flex", justifyContent: "space-between", borderTop: "1px solid #1b2a3c", marginTop: 2, paddingTop: 2, fontSize: 10, fontWeight: 700 }}>
      <span style={{ color: "#8ca4bc" }}>{label}</span>
      <span style={{ color }}>{amt >= 0 ? "+" : "−"}{fmt(Math.abs(amt))}</span>
    </div>
  );
  const anyExp = l.import_tax.length || l.export_tax.length || l.estate_tax || l.upkeep
    || l.fleet_cost || l.lost_cargo || l.events || l.consumption || l.inflation
    || l.war_levy || l.war_damage;
  return (
    <div style={{ padding: "2px 4px 6px", border: "1px solid #1b2a3c", borderRadius: 4, background: "#0a1119" }}>
      <div style={head}>Income</div>
      {l.trade_profit.length === 0 && l.office_income === 0 && l.estate_income === 0 &&
        <div style={{ color: "#56708e", fontSize: 9 }}>none recorded yet</div>}
      {l.trade_profit.map((c, i) => <Line key={i} label={`Trade · ${c.label}`} amt={c.amount} />)}
      {l.office_income > 0 && <Line label="Office income" amt={l.office_income} />}
      {l.estate_income > 0 && <Line label="Estate income" amt={l.estate_income} />}
      <Sub label="Total income" amt={l.income_total} color="#7fcf8f" />

      <div style={head}>Expenditure</div>
      {!anyExp && <div style={{ color: "#56708e", fontSize: 9 }}>none recorded yet</div>}
      {l.import_tax.map((c, i) => <Line key={`i${i}`} label={`Import tax · ${c.label}`} amt={c.amount} neg />)}
      {l.export_tax.map((c, i) => <Line key={`e${i}`} label={`Export tax · ${c.label}`} amt={c.amount} neg />)}
      {l.estate_tax > 0 && <Line label="Estate tax" amt={l.estate_tax} neg />}
      {l.upkeep > 0 && <Line label="Upkeep & retainers" amt={l.upkeep} neg />}
      {l.fleet_cost > 0 && <Line label="Fleet upkeep & decay" amt={l.fleet_cost} neg />}
      {l.lost_cargo > 0 && <Line label="Lost cargo" amt={l.lost_cargo} neg />}
      {l.events > 0 && <Line label="Misfortune & fees" amt={l.events} neg />}
      {l.consumption > 0 && <Line label="Feasts & consumption" amt={l.consumption} neg />}
      {l.inflation > 0 && <Line label="Inflation" amt={l.inflation} neg />}
      {l.war_levy > 0 && <Line label="⚔ War levy" amt={l.war_levy} neg />}
      {l.war_damage > 0 && <Line label="⚔ War damage" amt={l.war_damage} neg />}
      <Sub label="Total spent" amt={-l.expense_total} color="#e0a0a0" />

      <div style={{ display: "flex", justifyContent: "space-between", borderTop: "1px solid #24364e", marginTop: 5, paddingTop: 4 }}>
        <span style={{ color: "#cfe0f4", fontWeight: 700, fontSize: 11 }}>NET</span>
        <span style={{ color: l.net >= 0 ? "#9fe0a8" : "#e88", fontWeight: 700, fontSize: 11 }}>{l.net >= 0 ? "+" : "−"}{fmt(Math.abs(l.net))}</span>
      </div>
      {((l.wealth_years?.length ?? 0) >= 2 || l.wealth_graph.length >= 2) && (
        <WealthGraph
          data={(l.wealth_years?.length ?? 0) >= 2 ? l.wealth_years : l.wealth_graph}
          startYear={l.wealth_start_year ?? 0}
          yearly={(l.wealth_years?.length ?? 0) >= 2}
          fmt={fmt} />
      )}
      {l.warehouse.length > 0 && (
        <div>
          <div style={head}>Warehouse · {l.warehouse_city}</div>
          <div style={{ fontSize: 11, color: "#bcd0e4", lineHeight: 1.6 }}>{l.warehouse.map((w) => `${goodIcon(w.label)}${fmt(w.amount)}`).join("  ")}</div>
        </div>
      )}
    </div>
  );
}

/** Wealth graph for the Accountant. With `yearly` it plots the family's wealth over
 *  the past ~10 YEARS (X = campaign year, Y = wealth) with real axis labels; else it
 *  falls back to the within-year monthly samples. */
function WealthGraph({ data, startYear, yearly, fmt }:
  { data: number[]; startYear: number; yearly: boolean; fmt: (v: number) => string }) {
  const W = 252, H = 80, padL = 42, padR = 6, padT = 10, padB = 16;
  const plotW = W - padL - padR, plotH = H - padT - padB;
  const max = Math.max(...data), min = Math.min(...data);
  const span = max - min || Math.abs(max) || 1;
  const X = (i: number) => padL + (data.length <= 1 ? 0 : (i / (data.length - 1)) * plotW);
  const Y = (v: number) => padT + (1 - (v - min) / span) * plotH;
  const pts = data.map((v, i) => `${X(i).toFixed(1)},${Y(v).toFixed(1)}`).join(" ");
  const up = data[data.length - 1] >= data[0];
  const stroke = up ? "#9fe0a8" : "#e8a0a0";
  const growth = data[0] !== 0 ? ((data[data.length - 1] - data[0]) / Math.abs(data[0])) * 100 : 0;
  const mid = (max + min) / 2;
  return (
    <div style={{ marginTop: 7 }}>
      <div style={{ color: "#6a86a6", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 2 }}>
        Wealth — {yearly ? `past ${data.length} year${data.length > 1 ? "s" : ""}` : "through the year"}
        <span style={{ color: stroke, marginLeft: 6, fontWeight: 700 }}>{up ? "▲" : "▼"} {growth >= 0 ? "+" : ""}{growth.toFixed(0)}%</span>
      </div>
      <svg width={W} height={H} style={{ display: "block", background: "#0a1119", border: "1px solid #1b2a3c", borderRadius: 3 }}>
        {/* Axes */}
        <line x1={padL} y1={padT} x2={padL} y2={padT + plotH} stroke="#26384c" strokeWidth={1} />
        <line x1={padL} y1={padT + plotH} x2={W - padR} y2={padT + plotH} stroke="#26384c" strokeWidth={1} />
        {/* Y grid + labels (max / mid / min) */}
        {[max, mid, min].map((v, k) => (
          <g key={k}>
            <line x1={padL} y1={Y(v)} x2={W - padR} y2={Y(v)} stroke="#13202c" strokeWidth={1} />
            <text x={padL - 3} y={Y(v) + 3} textAnchor="end" fill="#6a86a6" fontSize={8}>{fmt(v)}</text>
          </g>
        ))}
        <polyline points={pts} fill="none" stroke={stroke} strokeWidth={1.6} strokeLinejoin="round" />
        <circle cx={X(data.length - 1)} cy={Y(data[data.length - 1])} r={2.2} fill={stroke} />
        {/* X labels (first / last) */}
        <text x={padL} y={H - 4} fill="#7a8aa0" fontSize={8}>{yearly ? `yr ${startYear}` : "start"}</text>
        <text x={W - padR} y={H - 4} textAnchor="end" fill="#7a8aa0" fontSize={8}>
          {yearly ? `yr ${startYear + data.length - 1}` : "now"}
        </text>
      </svg>
    </div>
  );
}

const detailPanel: React.CSSProperties = {
  position: "absolute", top: 40, right: 690, width: 720, maxHeight: "88vh", overflowY: "auto",
  background: "#0c141e", border: "1px solid #24364e", borderRadius: 8,
  padding: "12px 16px", boxShadow: "0 8px 28px rgba(0,0,0,0.55)", zIndex: 45,
};

const EVENT_ICON: Record<string, string> = {
  founded: "🏛", succession: "👤", monopoly: "💰", monopoly_lost: "💸",
  control_gained: "⚖", control_lost: "💔", branch: "🌿", loss: "⚠️", dissolved: "🪦",
  figure: "🎖", marriage: "💍", piracy: "🏴",
  // Phase 1.1/1.4 · tiers + positive events (§2.2) — the mechanism otherwise only
  // produces decline; these give the chronicle something other than obituaries.
  tier_up: "⬆", golden_age: "☀", dynasty: "👑", inheritance: "📜",
  // Phase 3.1 · goals.
  goal_set: "🎯", goal_achieved: "✓", goal_failed: "✗",
};
const EVENT_COLOR: Record<string, string> = {
  founded: "#cfe0f4", succession: "#9ab0c8", monopoly: "#e0b060", monopoly_lost: "#b08a5a",
  control_gained: "#7fd0a0", control_lost: "#d88", loss: "#e08a5a",
  branch: "#9fe07a", dissolved: "#8a93a0", figure: "#e6c878", marriage: "#e6a6c8", piracy: "#c07070",
  tier_up: "#7fd0a0", golden_age: "#e6c878", dynasty: "#e6c878", inheritance: "#9ab0c8",
  goal_set: "#9ab0c8", goal_achieved: "#7fd0a0", goal_failed: "#c98",
};

/** A house's chronicle as a vertical timeline: founding, successions, monopolies,
 *  cities controlled (gained/lost + year), the worst loss — plus its most
 *  profitable trade resources. Exported so the Houses browser (HousesPanel.tsx)
 *  can open it from the "fallen houses" list without duplicating it. */
export function HouseTimeline({ history, onClose }: { history: HouseHistory; onClose: () => void }) {
  const ev = history.events;
  const maxProfit = Math.max(1e-6, ...history.top_goods.map(([, p]) => p));
  return (
    <div style={timelinePanel}>
      <div style={{ ...dossierHeader, borderBottom: "1px solid #1a2a3e" }}>
        <span style={{ display: "flex", alignItems: "center", gap: 7 }}>
          <span style={{ width: 11, height: 11, borderRadius: 2, background: history.color }} />
          <CoatOfArms name={history.name} size={22} />
          <span>{history.name}</span>
        </span>
        <span style={{ cursor: "pointer", color: "#7a90a8" }} onClick={onClose}>✕</span>
      </div>
      <div style={{ overflowY: "auto", padding: "8px 12px 12px" }}>
        <div style={{ color: "#9ab0c8", fontSize: 10, marginBottom: 8 }}>
          {history.founder || `Founded in year ${history.founded_year}`}
          {history.defunct && <span style={{ color: "#d88" }}> · fallen</span>}
        </div>

        {/* Most profitable resources */}
        {history.top_goods.length > 0 && (
          <>
            <div style={timelineHdr}>Most profitable trade resources</div>
            {history.top_goods.map(([g, p]) => (
              <div key={g} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
                <span style={{ fontSize: 12, width: 16 }}>{goodIcon(g)}</span>
                <span style={{ color: "#cdbb88", fontSize: 10, width: 78, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{clarifyGemLabel(g, history.gem_variety)}</span>
                <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 2, overflow: "hidden" }}>
                  <div style={{ width: `${(p / maxProfit) * 100}%`, height: "100%", background: "#c9a227" }} />
                </div>
              </div>
            ))}
          </>
        )}

        {/* Colonies this house owns (outposts) or backed (joint-stock). */}
        {history.colonies && history.colonies.length > 0 && (
          <>
            <div style={{ ...timelineHdr, marginTop: 10 }}>🏛 Colonies &amp; outposts ({history.colonies.length})</div>
            {history.colonies.map((c) => (
              <div key={c.id} data-no-drag
                onClick={() => { useUIStore.getState().setSelectedHub(c.id); useUIStore.getState().setShowColonial(true); }}
                style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 2px", cursor: "pointer" }}>
                <span style={{ width: 9, height: 9, borderRadius: c.colony_kind === 2 ? 2 : "50%",
                  background: c.colony_kind === 2 ? "#c9a96a" : "#c08cff", flex: "0 0 auto" }} />
                <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: "#d8c8f4", fontSize: 10 }}>{c.name}</span>
                <span style={{ color: "#7a90a8", fontSize: 9 }}>{c.colony_kind === 2 ? "outpost" : "colony"}</span>
              </div>
            ))}
          </>
        )}

        {/* Timeline — grouped by year; click a year to expand what happened. */}
        <div style={{ ...timelineHdr, marginTop: 10 }}>Chronicle <span style={{ color: "#56708e", fontWeight: 400 }}>(click a year)</span></div>
        <YearChronicle entries={ev} icons={EVENT_ICON} colors={EVENT_COLOR} />
      </div>
    </div>
  );
}

const timelinePanel: React.CSSProperties = {
  position: "absolute", top: 0, right: 326, width: 300, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0a121c", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.6)", zIndex: 41,
};
const timelineHdr: React.CSSProperties = {
  color: "#7a90a8", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.4,
  margin: "4px 0 3px", borderBottom: "1px solid #16222e", paddingBottom: 2,
};
