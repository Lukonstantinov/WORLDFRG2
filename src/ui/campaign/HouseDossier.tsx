import { useEffect, useState } from "react";
import { campaignGetFeuds, campaignHouseStability } from "@bridge";
import type { FeudRow, Gauge, HouseStability } from "@types";

/** ⚜️ House Dossier — the two views the Houses panel never had.
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
