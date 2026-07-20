import { useEffect, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { T, SERIF, sectionHdr, cardStyle } from "@ui/campaign/chronicleTheme";

/** The Chronicle's left rail — the WORLD LEDGER. Pure reading matter: the
 *  world's pulse, movers, price history, shortages and the running chronicle.
 *  The campaign CLOCK (play/pause/speed) lives in the CampaignTopBar HUD;
 *  window launchers live in the HUD's grouped ledger menus. This rail only
 *  starts a campaign and reads the world. */
export function ChroniclePanel() {
  const snapshot = useCampaignStore((s) => s.snapshot);
  const worldEconomy = useCampaignStore((s) => s.worldEconomy);
  const busy = useCampaignStore((s) => s.busy);
  const error = useCampaignStore((s) => s.error);
  const refresh = useCampaignStore((s) => s.refresh);
  const start = useCampaignStore((s) => s.start);
  const coldStart = useCampaignStore((s) => s.coldStart);
  const newGame = useCampaignStore((s) => s.newGame);
  const pause = useCampaignStore((s) => s.pause);
  const setStatus = useUIStore((s) => s.setStatus);

  // The campaign RNG seed. Random per session start; the campaign, once begun,
  // carries its own state, so this only seeds a fresh "Begin Campaign".
  const [seed] = useState(() => Math.floor(Math.random() * 999999));

  useEffect(() => { refresh(); }, [refresh]);

  const active = snapshot?.active === true;
  const clock = snapshot?.clock;

  const starving = snapshot?.hubs.filter((h) => h.starving > 0.5).length ?? 0;
  const estates = snapshot?.hubs.filter((h) => h.is_estate).length ?? 0;

  // Which cities grow / shrink — month-over-month, biggest movers first.
  const movers = [...(snapshot?.hubs ?? [])].filter((h) => Math.abs(h.growth) > 0.0005);
  const growers = movers.filter((h) => h.growth > 0).sort((a, b) => b.growth - a.growth).slice(0, 4);
  const shrinkers = movers.filter((h) => h.growth < 0).sort((a, b) => a.growth - b.growth).slice(0, 4);

  return (
    <div style={rail}>
      {/* ── Masthead ── */}
      <div style={{ paddingBottom: 6, borderBottom: `1px solid ${T.lineGold}` }}>
        <div style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: 16, letterSpacing: 0.5 }}>
          📜 Chronicle
        </div>
        <div style={{ color: T.inkDim, fontSize: 10, lineHeight: 1.5, marginTop: 2 }}>
          The world is finalized. Prices, wealth and trade are simulated <b style={{ color: T.inkMid }}>live</b> —
          this ledger reads the world; the clock rides the top bar.
        </div>
      </div>

      {!active && (
        <div style={{ ...cardStyle, border: `1px solid ${T.goldDim}`, padding: "10px" }}>
          <div style={{ fontFamily: SERIF, color: T.parchment, fontSize: 13, fontWeight: 700, marginBottom: 4 }}>
            Begin the Chronicle
          </div>
          <div style={{ color: T.inkDim, fontSize: 11, lineHeight: 1.5, marginBottom: 8 }}>
            Seed a living economy from the finished trade economy. Merchants move
            goods each day; prices, wealth and population evolve; houses rise and fall.
          </div>
          <button onClick={() => start(seed)} disabled={busy}
            style={{ ...beginBtn, opacity: busy ? 0.5 : 1 }}>
            {busy ? "Starting…" : "▶ Begin Campaign"}
          </button>
        </div>
      )}

      {active && clock && clock.tick === 0 && (
        <div style={{ ...cardStyle, border: `1px solid ${T.goldDim}`, padding: "10px" }}>
          <div style={{ fontFamily: SERIF, color: T.parchment, fontSize: 13, fontWeight: 700, marginBottom: 4 }}>
            ❄ Cold Start (optional)
          </div>
          <div style={{ color: T.inkDim, fontSize: 11, lineHeight: 1.5, marginBottom: 8 }}>
            Zero <b>everything</b> — merchant houses, guilds, banks, coinage, warehouses and
            wealth — and reset every city to a small seed. On unpause the world rebuilds its
            trade network, finance and cities from nothing. Only before the clock starts.
          </div>
          <button onClick={() => coldStart()} disabled={busy}
            style={{ ...beginBtn, opacity: busy ? 0.5 : 1 }}>
            {busy ? "Zeroing…" : "❄ Cold Start — build from nothing"}
          </button>
        </div>
      )}

      {active && clock && (
        <>
          {/* ── World pulse ── */}
          <div style={statRow}>
            <Stat
              label="Population"
              value={fmtNum(snapshot?.total_population ?? 0)}
              sub={deltaStr(snapshot?.population_delta ?? 0, true)}
              subColor={(snapshot?.population_delta ?? 0) >= 0 ? T.goodInk : T.badInk}
            />
            <Stat
              label="Price idx"
              value={(snapshot?.price_index ?? 1).toFixed(2) + "×"}
              sub={deltaStr(snapshot?.price_index_delta ?? 0, false, 2)}
              subColor={(snapshot?.price_index_delta ?? 0) <= 0 ? T.goodInk : T.warn}
            />
          </div>
          <div style={statRow}>
            <Stat label="Hubs" value={String(snapshot?.hubs.length ?? 0)} />
            <Stat label="Estates" value={String(estates)} />
            <Stat label="Starving" value={String(starving)} color={starving > 0 ? T.badInk : undefined} />
          </div>
          <div style={{ color: T.inkFaint, fontSize: 9, display: "flex", gap: 10 }}>
            <span>sim tick {clock.last_tick_ms.toFixed(1)} ms</span>
            <span>{snapshot?.in_transit ?? 0} caravans in transit</span>
          </div>

          {/* ── Growing / shrinking cities (month-over-month) ── */}
          {(growers.length > 0 || shrinkers.length > 0) && (
            <div style={cardStyle}>
              <div style={sectionHdr}>Cities growing / shrinking</div>
              {growers.map((h) => (
                <div key={`g${h.id}`} style={lineRow}>
                  <span style={{ color: T.goodInk }}>▲</span>
                  <span style={{ flex: 1, color: T.inkMid, marginLeft: 5 }}>{h.name}</span>
                  <span style={{ color: T.inkMid, fontSize: 10, marginRight: 6 }}>{fmtNum(h.population)}</span>
                  <span style={{ color: T.goodInk }}>{(h.growth * 100).toFixed(1)}%</span>
                </div>
              ))}
              {shrinkers.map((h) => (
                <div key={`s${h.id}`} style={lineRow}>
                  <span style={{ color: T.badInk }}>▼</span>
                  <span style={{ flex: 1, color: T.inkMid, marginLeft: 5 }}>{h.name}</span>
                  <span style={{ color: T.inkMid, fontSize: 10, marginRight: 6 }}>{fmtNum(h.population)}</span>
                  <span style={{ color: T.badInk }}>{(h.growth * 100).toFixed(1)}%</span>
                </div>
              ))}
            </div>
          )}

          {/* ── World price-index sparkline ── */}
          {worldEconomy && worldEconomy.index_series.length > 1 && (
            <div style={cardStyle}>
              <div style={sectionHdr}>World price index</div>
              <Sparkline series={worldEconomy.index_series} />
            </div>
          )}

          {/* ── World shortage + merchant population (rollup) ── */}
          {worldEconomy && (worldEconomy.lack_series?.length ?? 0) > 1 && (
            <div style={cardStyle}>
              <div style={sectionHdr}>People lacking goods (world)</div>
              <WorldLines
                rows={worldEconomy.lack_series!} min={0} max={1}
                fmt={(v) => `${Math.round(v * 100)}%`}
                legend={[
                  { label: "Basic", color: "#ff8a6a" },
                  { label: "Comfort", color: "#e0c060" },
                  { label: "Luxury", color: "#9ab0c8" },
                ]}
              />
              <div style={{ ...sectionHdr, marginTop: 8 }}>Merchant population (world)</div>
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
            <div style={cardStyle}>
              <div style={sectionHdr}>Dearest goods (× world standard)</div>
              {worldEconomy.goods.slice(0, 6).map((g) => (
                <div key={g.good} style={lineRow}>
                  <span style={{ flex: 1, color: T.inkMid }}>{g.name}</span>
                  <span style={{ color: "#d8b070" }}>{g.world_price.toFixed(2)}×</span>
                  <span style={{ color: T.inkDim, fontSize: 10, marginLeft: 6 }}>{g.producers} src</span>
                </div>
              ))}
            </div>
          )}

          {/* ── Event log ── */}
          <div style={cardStyle}>
            <div style={sectionHdr}>Chronicle</div>
            {(snapshot?.recent_events ?? []).length === 0 ? (
              <div style={{ color: T.inkDim, fontSize: 11 }}>No events yet — advance time.</div>
            ) : (
              [...(snapshot?.recent_events ?? [])].reverse().map((e, i) => (
                <div key={i} style={{ fontSize: 11, color: eventColor(e.kind), marginBottom: 2 }}>
                  <span style={{ color: T.inkFaint }}>Y{Math.floor(e.tick / 365)} </span>
                  {e.text}
                </div>
              ))
            )}
          </div>

          {/* A running campaign is NEVER restarted in place. Starting another game
              first SAVES this one to its own .campaign file, then seeds a fresh,
              dynamic campaign on the same world (a new random seed). */}
          <button
            onClick={async () => {
              pause();
              const ok = await newGame(Math.floor(Math.random() * 1_000_000_000));
              if (ok) { setStatus("New campaign started (previous one saved)."); await refresh(); }
            }}
            disabled={busy}
            style={{ ...smallReset, opacity: busy ? 0.5 : 1 }}
            title="Saves the current campaign to its own file, then begins a fresh one"
          >
            ➕ New campaign (saves the current one first)
          </button>
        </>
      )}

      {error && <div style={{ color: T.badInk, fontSize: 11 }}>{error}</div>}
    </div>
  );
}

function Stat({ label, value, color, sub, subColor }: {
  label: string; value: string; color?: string; sub?: string; subColor?: string;
}) {
  return (
    <div style={{ flex: 1, background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 5, padding: "4px 7px" }}>
      <div style={{ color: T.inkDim, fontSize: 8.5, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.5 }}>{label}</div>
      <div style={{ color: color ?? T.ink, fontSize: 13, fontWeight: 700 }}>{value}</div>
      {sub && <div style={{ color: subColor ?? T.inkDim, fontSize: 9 }}>{sub}</div>}
    </div>
  );
}

/** Compact number: 12,400 / 1.2M. */
function fmtNum(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(0) + "k";
  return Math.round(n).toLocaleString();
}

/** Signed delta string ("+180/mo" / "−0.03"). */
function deltaStr(d: number, perMonth: boolean, digits = 0): string {
  if (Math.abs(d) < (digits ? 0.005 : 0.5)) return perMonth ? "—" : "±0";
  const sign = d > 0 ? "+" : "−";
  const mag = digits ? Math.abs(d).toFixed(digits) : fmtNum(Math.abs(d));
  return `${sign}${mag}${perMonth ? "/mo" : ""}`;
}

function Sparkline({ series }: { series: [number, number][] }) {
  const w = 216, h = 36;
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
  const w = 216, h = 40;
  if (rows.length < 2) return <div style={{ color: T.inkDim, fontSize: 10 }}>—</div>;
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
    case "figure": return "#e6c878";     // notable figures (Great Lives)
    case "fair": return "#6fceb0";       // trade fairs
    case "pilgrimage": return "#c6a6e6"; // pilgrimage seasons
    case "temple": return "#e0cf9a";     // holy cities / temples
    case "contagion": return "#c07070";  // route-borne epidemics
    case "marriage": return "#e6a6c8";   // dynastic marriages
    case "feud": return "#d88a6a";       // broken alliances
    case "guildhall": return "#cdbb88";  // craft guildhalls
    case "guild_strike": return "#d0a060";
    case "fashion": return "#e0a0d0";    // fashion cycles
    case "wonder": return "#b8c8a0";     // civic wonders
    case "piracy": return "#c07070";     // corsair raids
    case "diaspora": return "#8ac0c0";   // diaspora quarters
    default: return "#a8bcd4";
  }
}

const rail: React.CSSProperties = {
  width: 252, background: T.panel, borderRight: `1px solid ${T.line}`,
  padding: "10px", overflowY: "auto", display: "flex", flexDirection: "column",
  gap: 8, fontSize: 12, flexShrink: 0,
};
const beginBtn: React.CSSProperties = {
  width: "100%", padding: "9px", borderRadius: 6, border: "1px solid #2a6040",
  background: "#12331e", color: T.goodInk, fontWeight: 700, cursor: "pointer", fontSize: 12,
};
const statRow: React.CSSProperties = { display: "flex", gap: 5 };
const lineRow: React.CSSProperties = { display: "flex", alignItems: "center", fontSize: 11, marginBottom: 2 };
const smallReset: React.CSSProperties = {
  padding: "5px", borderRadius: 5, border: "1px solid #2a2030",
  background: "#160e16", color: "#a07090", cursor: "pointer", fontSize: 10,
};
