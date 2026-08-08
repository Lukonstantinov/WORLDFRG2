import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useViewportStore } from "@state/viewportStore";
import { campaignGetWars } from "@bridge";
import type { WarsPayload, WarBrief, WarRecord } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE } from "@ui/campaign/chronicleTheme";
import {
  Panel, PanelHeader, PanelBody, Section, Card, Divider, StatGrid, Stat,
  Badge, Tabs, Chip, EmptyNote, FootNote,
} from "@ui/kit";

/** DLC 3.5 §3.4a-f · the War Council — the ONE window for the whole war picture,
 *  pulled out of Money & Finance's "Shocks" tab (which still keeps its own compact
 *  summary as a jump-off). Three tabs:
 *   • Active — every ongoing war in full: score, chests, levies, round, goal.
 *   • History — the complete concluded-war log, searchable and sortable.
 *   • Records — aggregate stats: causes, costs, and the most warlike poleis. */

const WAR_ROUND_CAP = 12; // §3.4a · 3-year backstop, quarterly rounds (tick/mod.rs)

const CAUSE_LABEL: Record<string, string> = {
  "rival councils": "rivalry between councils",
  "a house's war": "a house's feud, escalated",
  "independence": "a colony's bid for independence",
};
const causeLabel = (cause: string) => CAUSE_LABEL[cause] ?? cause;

const fmtk = (v: number) => {
  const a = Math.abs(v);
  if (a >= 1000) return `${(v / 1000).toFixed(1)}k`;
  return v.toFixed(a < 10 ? 1 : 0);
};

type Tab = "active" | "history" | "records";
const TABS: readonly (readonly [Tab, string])[] = [
  ["active", "⚔ Active"],
  ["history", "📜 History"],
  ["records", "📊 Records"],
];

export function WarPanel() {
  const open = useUIStore((s) => s.showWar);
  const setWarHighlight = useViewportStore((s) => s.setWarHighlight);
  const clearWarHighlight = useViewportStore((s) => s.clearWarHighlight);
  const close = () => { clearWarHighlight(); useUIStore.getState().setShowWar(false); };
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [wars, setWars] = useState<WarsPayload>({ active: [], log: [] });
  const [tab, setTab] = useState<Tab>("active");

  useEffect(() => {
    if (!open || !active) return;
    campaignGetWars().then(setWars).catch(() => setWars({ active: [], log: [] }));
  }, [open, active, tick]);

  // Clear the map's war highlight whenever the panel closes/unmounts.
  useEffect(() => { if (!open) clearWarHighlight(); }, [open, clearWarHighlight]);
  useEffect(() => () => clearWarHighlight(), [clearWarHighlight]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.war);
  if (!open) return null;

  const focusHub = (name: string) => {
    const h = snapshot?.hubs.find((x) => x.name === name);
    if (h) setSelectedHub(h.id);
  };
  // Light both belligerent cities on the map: attacker (a) red, defender (b) blue.
  const showWarOnMap = (w: WarBrief) => {
    if (w.a_x == null) return;
    setWarHighlight(w.a_x, w.a_y ?? 0, w.b_x ?? 0, w.b_y ?? 0);
  };

  return (
    <Panel onPointerDown={onPointerDown} width={392} maxHeight="80vh" style={{ top: 60, right: 360, zIndex: 40, ...rootStyle }}>
      <PanelHeader icon="⚔" title="War Council" onDragStart={onPointerDown} onClose={close}
        right={wars.active.length > 0 ? <Badge tone="bad">{wars.active.length} at war</Badge> : undefined} />
      <Tabs tabs={TABS} active={tab} onSelect={setTab} style={{ padding: "0 10px" }} />
      <PanelBody style={{ flex: 1 }}>
        {!active && <EmptyNote>Begin the campaign and let it run — the first wars flare from year 10 or so.</EmptyNote>}
        {active && tab === "active" && <ActiveTab wars={wars.active} onFocus={focusHub} onShowMap={showWarOnMap} />}
        {active && tab === "history" && <HistoryTab log={wars.log} onFocus={focusHub} />}
        {active && tab === "records" && <RecordsTab wars={wars} />}
      </PanelBody>
    </Panel>
  );
}

// ── Active wars ────────────────────────────────────────────────────────────────
function ActiveTab({ wars, onFocus, onShowMap }: { wars: WarBrief[]; onFocus: (name: string) => void; onShowMap: (w: WarBrief) => void }) {
  if (wars.length === 0) return <EmptyNote>No wars underway — every polis is at peace.</EmptyNote>;
  return (
    <div>
      {wars.map((w, i) => <ActiveWarCard key={i} w={w} onFocus={onFocus} onShowMap={onShowMap} />)}
    </div>
  );
}

function ActiveWarCard({ w, onFocus, onShowMap }: { w: WarBrief; onFocus: (name: string) => void; onShowMap: (w: WarBrief) => void }) {
  const scoreAbs = Math.min(100, Math.abs(w.score));
  const favoursA = w.score >= 0;
  const roundFrac = Math.min(1, w.round / WAR_ROUND_CAP);
  return (
    <Card style={{ marginBottom: SPACE.md }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, flexWrap: "wrap" }}>
        <span onClick={() => onFocus(w.a)} title="Attacker — focus on map"
          style={{ color: "#ff6a5d", fontWeight: 700, fontSize: FZ.head, cursor: "pointer" }}>{w.a}</span>
        <span style={{ color: T.inkDim, fontSize: FZ.body }}>⚔</span>
        <span onClick={() => onFocus(w.b)} title="Defender — focus on map"
          style={{ color: "#5aa0ff", fontWeight: 700, fontSize: FZ.head, cursor: "pointer" }}>{w.b}</span>
        <span style={{ flex: 1 }} />
        <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>since y{w.start_year} ({w.years}y)</span>
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 2 }}>
        <span style={{ fontSize: FZ.micro, color: "#ff6a5d" }}>● attacker</span>
        <span style={{ fontSize: FZ.micro, color: "#5aa0ff" }}>● defender</span>
        <span style={{ flex: 1 }} />
        {w.a_x != null && (
          <button onClick={() => onShowMap(w)} title="Light both cities on the map (attacker red, defender blue)"
            style={{ fontSize: FZ.micro, cursor: "pointer", color: "#e6d2a0", background: "rgba(180,150,90,0.18)",
              border: "1px solid rgba(180,150,90,0.4)", borderRadius: 10, padding: "1px 8px" }}>
            ⚔ Show on map
          </button>
        )}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 3, flexWrap: "wrap" }}>
        <span style={{ color: T.inkMid, fontSize: FZ.small, fontStyle: "italic" }}>{causeLabel(w.cause)}</span>
        {w.backer_house_name && <Badge tone="warn">⚜ backed by {w.backer_house_name}</Badge>}
        <span style={{ flex: 1 }} />
        <Badge tone="gold">{w.goal_label}</Badge>
      </div>

      {/* §3.4a · bidirectional war score, −100..100. Positive favours `a`. */}
      <div style={{ marginTop: SPACE.sm }}>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: FZ.micro, color: T.inkFaint, marginBottom: 2 }}>
          <span>{w.a}</span><span title={`War score ${w.score.toFixed(0)} (±100 ends it outright)`}>score {w.score.toFixed(0)}</span><span>{w.b}</span>
        </div>
        <div style={{ position: "relative", height: 6, background: T.card, borderRadius: 3, overflow: "hidden" }}>
          <div style={{
            position: "absolute", top: 0, bottom: 0,
            left: favoursA ? `${50 - scoreAbs / 2}%` : "50%",
            width: `${scoreAbs / 2}%`,
            background: favoursA ? "#ff6a5d" : "#5aa0ff", // attacker red / defender blue
          }} />
          <div style={{ position: "absolute", left: "50%", top: 0, bottom: 0, width: 1, background: T.line }} />
        </div>
      </div>

      <StatGrid cols={3} style={{ marginTop: SPACE.sm }}>
        <Stat label="War chest" value={`${fmtk(w.chest_a)} / ${fmtk(w.chest_b)}`} hint="spent by each side (attacker / defender)" />
        <Stat label="Levied" value={`${fmtk(w.levies_a ?? 0)} / ${fmtk(w.levies_b ?? 0)}`} hint="forced from each side's houses" tone="warn" />
        <Stat label="Round" value={`${w.round} / ${WAR_ROUND_CAP}`} hint="quarterly — the last-resort cap" />
      </StatGrid>

      {/* §3.4a · the battle history — the quarterly rounds that moved the score. */}
      {w.battles && w.battles.length > 0 && (
        <div style={{ marginTop: SPACE.sm }}>
          <div style={{ fontSize: FZ.micro, color: T.inkFaint, marginBottom: 2 }}>Recent battles</div>
          {[...w.battles].slice(-6).reverse().map((bt, i) => {
            const attackerWon = bt.favored === 0;
            return (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: FZ.micro, padding: "1px 0" }}>
                <span style={{ color: T.inkDim, width: 58 }}>y{bt.year} · r{bt.round}</span>
                <span style={{ color: attackerWon ? "#ff6a5d" : "#5aa0ff", fontWeight: 600 }}>
                  {bt.decisive ? "▲▲" : "▲"} {attackerWon ? w.a : w.b}
                </span>
                <span style={{ color: T.inkFaint }}>{bt.decisive ? "presses hard" : "gains ground"}</span>
                <span style={{ flex: 1 }} />
                <span style={{ color: T.inkDim, fontVariantNumeric: "tabular-nums" }}>score {bt.score_after.toFixed(0)}</span>
              </div>
            );
          })}
        </div>
      )}

      {roundFrac >= 0.75 && (
        <FootNote>Nearing its round cap — if neither side breaks first, the war ends by exhaustion within {WAR_ROUND_CAP - w.round} more round{WAR_ROUND_CAP - w.round === 1 ? "" : "s"}.</FootNote>
      )}
    </Card>
  );
}

// ── History (concluded wars) ─────────────────────────────────────────────────
type Sort = "recent" | "longest" | "costliest";
const SORTS: { id: Sort; label: string }[] = [
  { id: "recent", label: "Recent" },
  { id: "longest", label: "Longest" },
  { id: "costliest", label: "Costliest" },
];

function HistoryTab({ log, onFocus }: { log: WarRecord[]; onFocus: (name: string) => void }) {
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<Sort>("recent");

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    let r = q
      ? log.filter((w) => w.a_name.toLowerCase().includes(q) || w.b_name.toLowerCase().includes(q))
      : log;
    r = [...r].sort((a, b) => {
      switch (sort) {
        case "longest": return (b.end_year - b.start_year) - (a.end_year - a.start_year);
        case "costliest": return b.reparations - a.reparations;
        default: return b.end_year - a.end_year;
      }
    });
    return r;
  }, [log, query, sort]);

  if (log.length === 0) return <EmptyNote>No war has concluded yet — the record builds as they end.</EmptyNote>;

  return (
    <div>
      <div style={{ display: "flex", gap: 4, marginBottom: SPACE.sm, alignItems: "center" }} data-no-drag>
        <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="search a polis…"
          style={{ flex: 1, background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 4,
            color: T.ink, fontSize: FZ.small, padding: "3px 7px" }} />
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md }} data-no-drag>
        {SORTS.map((s) => <Chip key={s.id} on={sort === s.id} onClick={() => setSort(s.id)}>{s.label}</Chip>)}
      </div>
      {shown.length === 0 && <EmptyNote>No war matches "{query}".</EmptyNote>}
      {shown.map((w, i) => (
        <Card key={i} style={{ marginBottom: SPACE.sm }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6, flexWrap: "wrap" }}>
            <span onClick={() => onFocus(w.winner)} title="Show on map" style={{ color: T.goodInk, fontWeight: 700, fontSize: FZ.body, cursor: "pointer" }}>{w.winner}</span>
            <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>beat</span>
            <span onClick={() => onFocus(w.loser)} title="Show on map" style={{ color: T.badInk, fontWeight: 700, fontSize: FZ.body, cursor: "pointer" }}>{w.loser}</span>
            <span style={{ flex: 1 }} />
            <span style={{ color: T.inkDim, fontSize: FZ.tiny }}>
              y{w.start_year}–{w.end_year} ({Math.max(1, w.end_year - w.start_year)}y)
            </span>
          </div>
          <div style={{ color: T.inkMid, fontSize: FZ.small, marginTop: 1, fontStyle: "italic" }}>{causeLabel(w.cause)}</div>
          <div style={{ color: T.inkMid, fontSize: FZ.small, marginTop: 2, lineHeight: 1.4 }}>{w.text}</div>
          <div style={{ display: "flex", gap: 10, fontSize: FZ.tiny, color: T.inkDim, marginTop: 3 }}>
            <span style={{ color: T.gold }}>reparations {fmtk(w.reparations)}</span>
            <span>levied {fmtk(w.levies_total)}</span>
          </div>
        </Card>
      ))}
    </div>
  );
}

// ── Records (aggregate stats) ────────────────────────────────────────────────
function RecordsTab({ wars }: { wars: WarsPayload }) {
  const stats = useMemo(() => {
    const { active, log } = wars;
    const totalWars = active.length + log.length;
    const totalReparations = log.reduce((s, w) => s + w.reparations, 0);
    const totalLeviesLog = log.reduce((s, w) => s + w.levies_total, 0);
    const totalLeviesActive = active.reduce((s, w) => s + w.levies, 0);
    const durations = log.map((w) => Math.max(1, w.end_year - w.start_year));
    const avgDuration = durations.length ? durations.reduce((a, b) => a + b, 0) / durations.length : 0;
    const longest = log.length
      ? log.reduce((best, w) => ((w.end_year - w.start_year) > (best.end_year - best.start_year) ? w : best))
      : undefined;
    const costliest = log.length ? log.reduce((best, w) => (w.reparations > best.reparations ? w : best)) : undefined;

    const causeCounts = new Map<string, number>();
    for (const w of active) causeCounts.set(w.cause, (causeCounts.get(w.cause) ?? 0) + 1);
    for (const w of log) causeCounts.set(w.cause, (causeCounts.get(w.cause) ?? 0) + 1);

    const board = new Map<string, { wars: number; wins: number; losses: number }>();
    const bump = (name: string, k: "wars" | "wins" | "losses") => {
      const e = board.get(name) ?? { wars: 0, wins: 0, losses: 0 };
      e[k] += 1;
      board.set(name, e);
    };
    for (const w of active) { bump(w.a, "wars"); bump(w.b, "wars"); }
    for (const w of log) {
      bump(w.a_name, "wars"); bump(w.b_name, "wars");
      bump(w.winner, "wins"); bump(w.loser, "losses");
    }
    const leaderboard = [...board.entries()]
      .map(([name, e]) => ({ name, ...e }))
      .sort((a, b) => b.wars - a.wars)
      .slice(0, 8);

    return { totalWars, totalReparations, totalLeviesLog, totalLeviesActive, avgDuration, longest, costliest, causeCounts, leaderboard };
  }, [wars]);

  if (stats.totalWars === 0) return <EmptyNote>No war on record — nothing to measure yet.</EmptyNote>;
  const maxCause = Math.max(1, ...stats.causeCounts.values());
  const maxBoard = Math.max(1, ...stats.leaderboard.map((b) => b.wars));

  return (
    <div>
      <StatGrid cols={2}>
        <Stat label="Wars on record" value={stats.totalWars} hint={`${wars.active.length} ongoing · ${wars.log.length} concluded`} />
        <Stat label="Avg. duration" value={stats.avgDuration ? `${stats.avgDuration.toFixed(1)}y` : "—"} />
        <Stat label="Reparations paid" value={fmtk(stats.totalReparations)} tone="gold" />
        <Stat label="Levied (ongoing + past)" value={fmtk(stats.totalLeviesActive + stats.totalLeviesLog)} tone="warn" />
      </StatGrid>

      {(stats.longest || stats.costliest) && (
        <Section title="Notable wars" style={{ marginTop: SPACE.lg }}>
          {stats.longest && (
            <div style={{ fontSize: FZ.small, color: T.inkMid, marginBottom: 2 }}>
              <b style={{ color: T.ink }}>Longest:</b> {stats.longest.a_name} vs {stats.longest.b_name} — {Math.max(1, stats.longest.end_year - stats.longest.start_year)} years (y{stats.longest.start_year}–{stats.longest.end_year})
            </div>
          )}
          {stats.costliest && stats.costliest.reparations > 0 && (
            <div style={{ fontSize: FZ.small, color: T.inkMid }}>
              <b style={{ color: T.ink }}>Costliest:</b> {stats.costliest.winner} extracted {fmtk(stats.costliest.reparations)} from {stats.costliest.loser} (y{stats.costliest.end_year})
            </div>
          )}
        </Section>
      )}

      <Section title="Causes" style={{ marginTop: SPACE.lg }}>
        {[...stats.causeCounts.entries()].sort((a, b) => b[1] - a[1]).map(([cause, n]) => (
          <div key={cause} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
            <span style={{ width: 150, color: T.inkMid, fontSize: FZ.small, flex: "0 0 auto" }}>{causeLabel(cause)}</span>
            <div style={{ flex: 1, height: 6, background: T.card, borderRadius: 3, overflow: "hidden" }}>
              <div style={{ width: `${(n / maxCause) * 100}%`, height: "100%", background: T.accent }} />
            </div>
            <span style={{ color: T.inkDim, fontSize: FZ.tiny, width: 16, textAlign: "right" }}>{n}</span>
          </div>
        ))}
      </Section>

      <Section title="Most warlike poleis" style={{ marginTop: SPACE.lg }}>
        {stats.leaderboard.map((b) => (
          <div key={b.name} style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 3 }}>
            <span style={{ width: 110, color: T.ink, fontSize: FZ.small, flex: "0 0 auto", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{b.name}</span>
            <div style={{ flex: 1, height: 6, background: T.card, borderRadius: 3, overflow: "hidden" }}>
              <div style={{ width: `${(b.wars / maxBoard) * 100}%`, height: "100%", background: T.bad }} />
            </div>
            <span style={{ color: T.inkDim, fontSize: FZ.tiny, width: 66, textAlign: "right" }}>
              {b.wars}w · <span style={{ color: T.goodInk }}>{b.wins}</span>/<span style={{ color: T.badInk }}>{b.losses}</span>
            </span>
          </div>
        ))}
      </Section>
      <Divider />
      <FootNote>Wars are economic (CITY_PROVINCE_WAR_PLAN.md §3.4) — no troops, waged with war chests, forced levies and blockade. A decisive score, exhaustion, or the {WAR_ROUND_CAP}-round cap ends every war.</FootNote>
    </div>
  );
}
