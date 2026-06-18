import { useEffect, useRef, useState } from "react";
import { useCampaignStore } from "../../state/campaignStore";
import { useUIStore } from "../../state/uiStore";
import { campaignPersist } from "../../bridge/tauri";

type Speed = "week" | "month" | "year";
const SPEED_TICKS: Record<Speed, number> = { week: 7, month: 30, year: 365 };

/** Step 11 — DLC 1 "Living Trade": the Campaign Clock. Seed the tick simulation
 *  from the static economy, then advance it — manually or with the ▶ Play button
 *  (auto-advances one Week/Month/Year per step). Cities produce, consume, trade,
 *  grow and starve; prices drift; merchant houses rise and fall. */
export function StepCampaign({ seed }: { seed: number; plateCount: number; invalidateTiles: () => void }) {
  const snapshot = useCampaignStore((s) => s.snapshot);
  const worldEconomy = useCampaignStore((s) => s.worldEconomy);
  const busy = useCampaignStore((s) => s.busy);
  const error = useCampaignStore((s) => s.error);
  const refresh = useCampaignStore((s) => s.refresh);
  const start = useCampaignStore((s) => s.start);
  const advance = useCampaignStore((s) => s.advance);
  const setStatus = useUIStore((s) => s.setStatus);
  const setShowHouses = useUIStore((s) => s.setShowHouses);

  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState<Speed>("month");
  const playingRef = useRef(playing);
  playingRef.current = playing;

  useEffect(() => {
    refresh();
  }, [refresh]);

  const active = snapshot?.active === true;
  const clock = snapshot?.clock;

  // Auto-advance loop: one step of the chosen size, pause between steps.
  useEffect(() => {
    if (!playing || !active) return;
    let cancelled = false;
    const run = async () => {
      let i = 0;
      while (!cancelled && playingRef.current) {
        i++;
        // Heavy panel refresh (economy / houses / diagnostics) only every 6th step;
        // the other steps update just the snapshot (clock + live map markers), so
        // fast Play doesn't fire the costlier panel queries every single tick.
        await advance(SPEED_TICKS[speed], i % 6 === 0);
        if (useCampaignStore.getState().error) { setPlaying(false); break; }
        await new Promise((r) => setTimeout(r, 450));
      }
      if (!cancelled) {
        // Paused by the user: bring the panels current and flush the resident sim.
        await refresh();
        campaignPersist().catch(() => {});
      }
    };
    run();
    return () => { cancelled = true; };
  }, [playing, speed, active, advance, refresh]);

  // Stop playing if the step is left / campaign restarts — and flush to disk so no
  // in-memory ticks are lost when leaving the campaign.
  useEffect(() => () => { playingRef.current = false; campaignPersist().catch(() => {}); }, []);

  const doAdvance = async (ticks: number, label: string) => {
    setStatus(`Advancing ${label}…`);
    await advance(ticks);
    setStatus(`Advanced ${label}.`);
  };

  // Starving / prosperous tallies for an at-a-glance world pulse.
  const starving = snapshot?.hubs.filter((h) => h.starving > 0.5).length ?? 0;
  const estates = snapshot?.hubs.filter((h) => h.is_estate).length ?? 0;
  const dayFrac = clock ? clock.day / 365 : 0;

  // Which cities grow / shrink — month-over-month, biggest movers first.
  const movers = [...(snapshot?.hubs ?? [])].filter((h) => Math.abs(h.growth) > 0.0005);
  const growers = movers.filter((h) => h.growth > 0).sort((a, b) => b.growth - a.growth).slice(0, 4);
  const shrinkers = movers.filter((h) => h.growth < 0).sort((a, b) => a.growth - b.growth).slice(0, 4);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {!active && (
        <>
          <div style={{ color: "#506080", fontSize: 11 }}>
            Start a living economy from the finished trade economy. Merchants will
            move goods each day; prices, wealth and population evolve.
          </div>
          <button
            onClick={() => start(seed)}
            disabled={busy}
            style={{ ...bigBtn, opacity: busy ? 0.5 : 1 }}
          >
            {busy ? "Starting…" : "▶ Begin Campaign"}
          </button>
        </>
      )}

      {active && clock && (
        <>
          {/* ── Campaign Clock ── */}
          <div style={clockCard}>
            <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
              <span style={{ color: "#cfe0f4", fontWeight: 700, fontSize: 15 }}>
                Year {clock.year}
              </span>
              <span style={{ color: "#7a98b8", fontSize: 12 }}>· Day {clock.day}</span>
              <span style={{ flex: 1 }} />
              <span style={{ color: "#6aa0d8", fontSize: 12 }}>{clock.season}</span>
            </div>
            <div style={{ height: 4, background: "#0a1018", borderRadius: 3, marginTop: 5, overflow: "hidden" }}>
              <div style={{ width: `${dayFrac * 100}%`, height: "100%", background: "#2a6a9a" }} />
            </div>
            {/* Play control: auto-advance one step of the chosen size, repeatedly. */}
            <div style={{ display: "flex", gap: 4, marginTop: 7, alignItems: "stretch" }}>
              <button
                onClick={() => setPlaying((p) => !p)}
                style={{ ...advBtn, flex: "0 0 auto", padding: "5px 12px",
                  background: playing ? "#5a2030" : "#1a4a2a",
                  color: playing ? "#e0a0a0" : "#a0e0b0", fontWeight: 700 }}
              >
                {playing ? "⏸ Pause" : "▶ Play"}
              </button>
              <div style={{ display: "flex", flex: 1, gap: 2 }}>
                {(["week", "month", "year"] as Speed[]).map((s) => (
                  <button key={s} onClick={() => setSpeed(s)}
                    style={{ ...advBtn, flex: 1, textTransform: "capitalize",
                      borderColor: speed === s ? "#3a80c0" : "#234058",
                      color: speed === s ? "#cfe2f6" : "#9cc0e0",
                      background: speed === s ? "#13283e" : "#13202e" }}>
                    {s}
                  </button>
                ))}
              </div>
            </div>
            <div style={{ color: "#5a7390", fontSize: 9, marginTop: 4 }}>
              {playing ? `Playing — +1 ${speed} per step` : `Each step advances one ${speed}`}
            </div>
            {/* Manual single steps. */}
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 4, marginTop: 5 }}>
              <button onClick={() => doAdvance(7, "a week")} disabled={busy || playing} style={advBtn}>+ Week</button>
              <button onClick={() => doAdvance(30, "a month")} disabled={busy || playing} style={advBtn}>+ Month</button>
              <button onClick={() => doAdvance(365, "a year")} disabled={busy || playing} style={advBtn}>+ Year</button>
            </div>
            <div style={{ color: "#506080", fontSize: 10, marginTop: 5, display: "flex", gap: 8 }}>
              <span>tick: {clock.last_tick_ms.toFixed(1)} ms</span>
              <span>in transit: {snapshot?.in_transit ?? 0}</span>
            </div>
          </div>

          <button onClick={() => setShowHouses(true)} style={housesBtn}>
            ⚜️ Merchant Houses
          </button>
          <button onClick={() => useUIStore.getState().setShowCityRanking(true)} style={housesBtn}>
            🏆 Richest Cities
          </button>
          <button onClick={() => useUIStore.getState().setShowFutures(true)} style={housesBtn}>
            📜 Futures Contracts
          </button>
          <button onClick={() => useUIStore.getState().setShowWarehouses(true)} style={housesBtn}>
            🏬 Warehouses & Estates
          </button>
          <button onClick={() => useUIStore.getState().setShowSpeculation(true)} style={housesBtn}>
            🫧 Finance &amp; Speculation
          </button>
          <button onClick={() => useUIStore.getState().setShowCoinCredit(true)} style={housesBtn}>
            🪙 Coin, Credit &amp; Crashes
          </button>

          {/* ── World pulse: population + prices in NUMBERS, with monthly Δ ── */}
          <div style={statRow}>
            <Stat
              label="Population"
              value={fmtNum(snapshot?.total_population ?? 0)}
              sub={deltaStr(snapshot?.population_delta ?? 0, true)}
              subColor={(snapshot?.population_delta ?? 0) >= 0 ? "#80c890" : "#e08080"}
            />
            <Stat
              label="Price idx"
              value={(snapshot?.price_index ?? 1).toFixed(2) + "×"}
              sub={deltaStr(snapshot?.price_index_delta ?? 0, false, 2)}
              subColor={(snapshot?.price_index_delta ?? 0) <= 0 ? "#80c890" : "#e0b080"}
            />
          </div>
          <div style={statRow}>
            <Stat label="Hubs" value={String(snapshot?.hubs.length ?? 0)} />
            <Stat label="Estates" value={String(estates)} />
            <Stat label="Starving" value={String(starving)} color={starving > 0 ? "#e08080" : undefined} />
          </div>

          {/* ── Growing / shrinking cities (month-over-month) ── */}
          {(growers.length > 0 || shrinkers.length > 0) && (
            <div style={card}>
              <div style={hdr}>Cities growing / shrinking</div>
              {growers.map((h) => (
                <div key={`g${h.id}`} style={lineRow}>
                  <span style={{ color: "#80c890" }}>▲</span>
                  <span style={{ flex: 1, color: "#9fb4cc", marginLeft: 5 }}>{h.name}</span>
                  <span style={{ color: "#9fb4cc", fontSize: 10, marginRight: 6 }}>{fmtNum(h.population)}</span>
                  <span style={{ color: "#80c890" }}>{(h.growth * 100).toFixed(1)}%</span>
                </div>
              ))}
              {shrinkers.map((h) => (
                <div key={`s${h.id}`} style={lineRow}>
                  <span style={{ color: "#e08080" }}>▼</span>
                  <span style={{ flex: 1, color: "#9fb4cc", marginLeft: 5 }}>{h.name}</span>
                  <span style={{ color: "#9fb4cc", fontSize: 10, marginRight: 6 }}>{fmtNum(h.population)}</span>
                  <span style={{ color: "#e08080" }}>{(h.growth * 100).toFixed(1)}%</span>
                </div>
              ))}
              {growers.length === 0 && shrinkers.length === 0 && (
                <div style={{ color: "#506080", fontSize: 11 }}>Advance a month to see movement.</div>
              )}
            </div>
          )}

          {/* ── World price-index sparkline ── */}
          {worldEconomy && worldEconomy.index_series.length > 1 && (
            <div style={card}>
              <div style={hdr}>World price index</div>
              <Sparkline series={worldEconomy.index_series} />
            </div>
          )}

          {/* ── World shortage + merchant population (rollup) ── */}
          {worldEconomy && (worldEconomy.lack_series?.length ?? 0) > 1 && (
            <div style={card}>
              <div style={hdr}>People lacking goods (world)</div>
              <WorldLines
                rows={worldEconomy.lack_series!} min={0} max={1}
                fmt={(v) => `${Math.round(v * 100)}%`}
                legend={[
                  { label: "Basic", color: "#ff8a6a" },
                  { label: "Comfort", color: "#e0c060" },
                  { label: "Luxury", color: "#9ab0c8" },
                ]}
              />
              <div style={{ ...hdr, marginTop: 8 }}>Merchant population (world)</div>
              <WorldLines
                rows={worldEconomy.merchant_series ?? []} min={0}
                fmt={(v) => Math.round(v).toLocaleString()}
                legend={[
                  { label: "🧺 Local", color: "#7fd0a0" },
                  { label: "🏛 Houses", color: "#e0c060" },
                  { label: "⚖ Guilds", color: "#8aa0c0" },
                ]}
              />
            </div>
          )}

          {/* ── Costliest goods worldwide ── */}
          {worldEconomy && worldEconomy.goods.length > 0 && (
            <div style={card}>
              <div style={hdr}>Dearest goods (× world standard)</div>
              {worldEconomy.goods.slice(0, 6).map((g) => (
                <div key={g.good} style={lineRow}>
                  <span style={{ flex: 1, color: "#9fb4cc" }}>{g.name}</span>
                  <span style={{ color: "#d8b070" }}>{g.world_price.toFixed(2)}×</span>
                  <span style={{ color: "#506080", fontSize: 10, marginLeft: 6 }}>
                    {g.producers} src
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* ── Event log ── */}
          <div style={card}>
            <div style={hdr}>Chronicle</div>
            {(snapshot?.recent_events ?? []).length === 0 ? (
              <div style={{ color: "#506080", fontSize: 11 }}>No events yet — advance time.</div>
            ) : (
              [...(snapshot?.recent_events ?? [])].reverse().map((e, i) => (
                <div key={i} style={{ fontSize: 11, color: eventColor(e.kind), marginBottom: 2 }}>
                  <span style={{ color: "#46586e" }}>Y{Math.floor(e.tick / 365)} </span>
                  {e.text}
                </div>
              ))
            )}
          </div>

          <button onClick={() => start(seed)} disabled={busy} style={{ ...smallReset, opacity: busy ? 0.5 : 1 }}>
            ⟲ Restart campaign from current economy
          </button>
        </>
      )}

      {error && <div style={{ color: "#e08080", fontSize: 11 }}>{error}</div>}
    </div>
  );
}

function Stat({ label, value, color, sub, subColor }: {
  label: string; value: string; color?: string; sub?: string; subColor?: string;
}) {
  return (
    <div style={{ flex: 1, background: "#0a1018", borderRadius: 4, padding: "4px 6px" }}>
      <div style={{ color: "#506080", fontSize: 9 }}>{label}</div>
      <div style={{ color: color ?? "#b8cce4", fontSize: 13, fontWeight: 600 }}>{value}</div>
      {sub && <div style={{ color: subColor ?? "#506080", fontSize: 9 }}>{sub}</div>}
    </div>
  );
}

/** Compact number: 12,400 / 1.2M. */
function fmtNum(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(0) + "k";
  return Math.round(n).toLocaleString();
}

/** Signed delta string ("+180 this month" / "−0.03"). */
function deltaStr(d: number, perMonth: boolean, digits = 0): string {
  if (Math.abs(d) < (digits ? 0.005 : 0.5)) return perMonth ? "—" : "±0";
  const sign = d > 0 ? "+" : "−";
  const mag = digits ? Math.abs(d).toFixed(digits) : fmtNum(Math.abs(d));
  return `${sign}${mag}${perMonth ? "/mo" : ""}`;
}

function Sparkline({ series }: { series: [number, number][] }) {
  const w = 200, h = 36;
  const ys = series.map((p) => p[1]);
  const lo = Math.min(...ys), hi = Math.max(...ys);
  const span = hi - lo || 1;
  const pts = series
    .map((p, i) => {
      const x = (i / (series.length - 1)) * w;
      const y = h - ((p[1] - lo) / span) * (h - 4) - 2;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <svg width={w} height={h} style={{ display: "block" }}>
      <polyline points={pts} fill="none" stroke="#4a90c0" strokeWidth={1.5} />
    </svg>
  );
}

/** Three overlaid series (cols 1-3 of each row) + a legend with current values.
 *  Used for the world shortage-by-tier and merchant-population-by-class rollups. */
function WorldLines({ rows, legend, min, max, fmt }: {
  rows: [number, number, number, number][];
  legend: { label: string; color: string }[];
  min?: number; max?: number; fmt: (v: number) => string;
}) {
  const w = 200, h = 40;
  if (rows.length < 2) return <div style={{ color: "#506080", fontSize: 10 }}>—</div>;
  const vals = rows.flatMap((r) => [r[1], r[2], r[3]]);
  const lo = min ?? Math.min(...vals);
  const hi = max ?? Math.max(...vals, lo + 1e-6);
  const span = Math.max(1e-6, hi - lo);
  const line = (col: 1 | 2 | 3) =>
    rows.map((r, i) => {
      const x = (i / (rows.length - 1)) * w;
      const y = h - ((r[col] - lo) / span) * (h - 4) - 2;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(" ");
  const last = rows[rows.length - 1];
  return (
    <>
      <svg width={w} height={h} style={{ display: "block", background: "#0b1622", borderRadius: 3 }}>
        {[1, 2, 3].map((c) => (
          <polyline key={c} points={line(c as 1 | 2 | 3)} fill="none"
            stroke={legend[c - 1]?.color ?? "#888"} strokeWidth={1.5} />
        ))}
      </svg>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 2 }}>
        {legend.map((l, i) => (
          <span key={i} style={{ display: "inline-flex", alignItems: "center", gap: 3, fontSize: 9, color: "#9ab0c8" }}>
            <span style={{ width: 8, height: 8, borderRadius: 2, background: l.color }} />
            {l.label} <span style={{ color: l.color, fontWeight: 700 }}>{fmt(last[(i + 1) as 1 | 2 | 3])}</span>
          </span>
        ))}
      </div>
    </>
  );
}

function eventColor(kind: string): string {
  switch (kind) {
    case "starvation": return "#e08080";
    case "estate": return "#8fd0a0";
    case "succession": return "#c0a0e0";
    case "world": return "#cfe0f4"; // monthly world summary (brighter — the headline)
    default: return "#a8bcd4";
  }
}

const bigBtn: React.CSSProperties = {
  width: "100%", padding: "8px", borderRadius: 5, border: "1px solid #2a6040",
  background: "#1a4a2a", color: "#a0e0b0", fontWeight: 600, cursor: "pointer", fontSize: 12,
};
const clockCard: React.CSSProperties = {
  background: "#0c1420", border: "1px solid #1e3450", borderRadius: 6, padding: "8px",
};
const advBtn: React.CSSProperties = {
  padding: "5px", borderRadius: 4, border: "1px solid #234058",
  background: "#13202e", color: "#9cc0e0", cursor: "pointer", fontSize: 11,
};
const housesBtn: React.CSSProperties = {
  width: "100%", padding: "6px", borderRadius: 5, border: "1px solid #3a3050",
  background: "#181426", color: "#d8c8f0", cursor: "pointer", fontSize: 11, fontWeight: 600,
};
const statRow: React.CSSProperties = { display: "flex", gap: 4 };
const card: React.CSSProperties = {
  background: "#0a1018", border: "1px solid #16202e", borderRadius: 5, padding: "6px 8px",
};
const hdr: React.CSSProperties = { color: "#5a7390", fontSize: 10, fontWeight: 600, marginBottom: 4, textTransform: "uppercase" };
const lineRow: React.CSSProperties = { display: "flex", alignItems: "center", fontSize: 11, marginBottom: 2 };
const smallReset: React.CSSProperties = {
  padding: "4px", borderRadius: 4, border: "1px solid #2a2030",
  background: "#160e16", color: "#a07090", cursor: "pointer", fontSize: 10,
};
