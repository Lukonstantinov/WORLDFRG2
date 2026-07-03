import { useEffect, useState } from "react";
import { useCampaignStore, SPEED_TICKS, type CampaignSpeed } from "../state/campaignStore";
import { useUIStore } from "../state/uiStore";
import { T, SERIF } from "./chronicleTheme";

/** The Chronicle HUD — a Paradox-style command bar docked to the top of the map.
 *
 *  Left → right: the DATE PLAQUE (year/day/season + season progress), the PLAY
 *  cluster (pause/play + speed + single-step), the WORLD PULSE (population,
 *  price index, caravans, starving alert), then the LEDGER MENUS — the ~20
 *  campaign windows grouped into four categories (Economy / Society / Finance /
 *  World) instead of a flat strip of chips. Space bar toggles pause.
 *
 *  Replaces WindowBar in Chronicle mode (Forge keeps WindowBar). */
export function CampaignTopBar() {
  const snapshot = useCampaignStore((s) => s.snapshot);
  const playing = useCampaignStore((s) => s.playing);
  const speed = useCampaignStore((s) => s.speed);
  const busy = useCampaignStore((s) => s.busy);
  const error = useCampaignStore((s) => s.error);
  const play = useCampaignStore((s) => s.play);
  const pause = useCampaignStore((s) => s.pause);
  const setSpeed = useCampaignStore((s) => s.setSpeed);
  const advance = useCampaignStore((s) => s.advance);
  const ui = useUIStore();
  const [openMenu, setOpenMenu] = useState<string | null>(null);

  const active = snapshot?.active === true;
  const clock = snapshot?.clock;

  // Space = pause/play, the strategy-game reflex. Ignored while typing in a field.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.code !== "Space") return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable)) return;
      const st = useCampaignStore.getState();
      if (st.snapshot?.active !== true) return;
      e.preventDefault();
      st.playing ? st.pause() : st.play();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Leaving Chronicle stops the clock (the store loop then flushes to disk).
  useEffect(() => () => useCampaignStore.getState().pause(), []);

  type Item = { label: string; on: boolean; set: (v: boolean) => void };
  const groups: { key: string; label: string; items: Item[] }[] = [
    {
      key: "economy", label: "📦 Economy",
      items: [
        { label: "📦 Goods & Quality", on: ui.showGoodsWindow, set: ui.setShowGoodsWindow },
        { label: "📊 Economy Dashboard", on: ui.showEconomyDashboard, set: ui.setShowEconomyDashboard },
        { label: "📖 Goods Codex", on: ui.showGoodsCodex, set: ui.setShowGoodsCodex },
        { label: "🔀 Trade Matrix", on: ui.showTradeMatrix, set: ui.setShowTradeMatrix },
        { label: "🏬 Warehouses & Estates", on: ui.showWarehouses, set: ui.setShowWarehouses },
        { label: "📜 Futures Contracts", on: ui.showFutures, set: ui.setShowFutures },
        { label: "🧭 Itinerary", on: ui.showItinerary, set: ui.setShowItinerary },
      ],
    },
    {
      key: "society", label: "⚜️ Society",
      items: [
        { label: "⚜️ Merchant Houses", on: ui.showHouses, set: ui.setShowHouses },
        { label: "⚭ Dynasties & Alliances", on: ui.showDynasties, set: ui.setShowDynasties },
        { label: "🏛 Guilds & Crafts", on: ui.showGuilds, set: ui.setShowGuilds },
        { label: "👤 Notable Figures", on: ui.showFigures, set: ui.setShowFigures },
        { label: "🗿 Landmarks & Sites", on: ui.showLandmarks, set: ui.setShowLandmarks },
        { label: "☠ Plagues & Epidemics", on: ui.showPlagues, set: ui.setShowPlagues },
      ],
    },
    {
      key: "finance", label: "🪙 Finance",
      items: [
        { label: "🪙 Coin, Credit & Crashes", on: ui.showCoinCredit, set: ui.setShowCoinCredit },
        { label: "🏦 Banks", on: ui.showBank, set: ui.setShowBank },
        { label: "🫧 Speculation & Poleis", on: ui.showSpeculation, set: ui.setShowSpeculation },
      ],
    },
    {
      key: "world", label: "🌍 World",
      items: [
        { label: "🏆 City Ranking", on: ui.showCityRanking, set: ui.setShowCityRanking },
        { label: "🏛 Colonial Office", on: ui.showColonial, set: ui.setShowColonial },
        { label: "🗞 World News", on: ui.showNews, set: ui.setShowNews },
      ],
    },
  ];

  const hideAll = () => {
    groups.forEach((g) => g.items.forEach((it) => it.set(false)));
    ui.setShowWorkflow(false);
    ui.setShowToolbar(false);
    setOpenMenu(null);
  };

  const dayFrac = clock ? clock.day / 365 : 0;
  const starving = snapshot?.hubs.filter((h) => h.starving > 0.5).length ?? 0;

  return (
    <div style={bar}>
      {/* ── Date plaque ─────────────────────────────────────────────── */}
      <div style={{ minWidth: 148 }}>
        {active && clock ? (
          <>
            <div style={{ display: "flex", alignItems: "baseline", gap: 7 }}>
              <span style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: 16, letterSpacing: 0.5 }}>
                Year {clock.year}
              </span>
              <span style={{ color: T.inkDim, fontSize: 11 }}>
                Day {clock.day} · {clock.season}
              </span>
            </div>
            <div style={{ height: 3, background: "#0a1018", borderRadius: 2, marginTop: 4, overflow: "hidden", position: "relative" }}
              title={`Day ${clock.day} of 365 · sim tick ${clock.last_tick_ms.toFixed(1)} ms`}>
              <div style={{ width: `${dayFrac * 100}%`, height: "100%", background: `linear-gradient(90deg, ${T.goldDim}, ${T.gold})` }} />
              {[0.25, 0.5, 0.75].map((q) => (
                <span key={q} style={{ position: "absolute", left: `${q * 100}%`, top: 0, bottom: 0, width: 1, background: "#1c2a3c" }} />
              ))}
            </div>
          </>
        ) : (
          <div style={{ color: T.inkDim, fontSize: 11, paddingTop: 4 }}>
            <span style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: 13 }}>Chronicle</span>
            {" "}— begin a campaign in the ledger rail
          </div>
        )}
      </div>

      {/* ── Play cluster ────────────────────────────────────────────── */}
      {active && (
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <button
            onClick={() => (playing ? pause() : play())}
            title={playing ? "Pause (Space)" : "Play (Space) — auto-advance"}
            style={{
              width: 34, height: 30, borderRadius: 6, cursor: "pointer", fontSize: 13, fontWeight: 700,
              border: `1px solid ${playing ? "#6a3040" : "#2a6040"}`,
              background: playing ? "#3a1820" : "#12331e",
              color: playing ? T.badInk : T.goodInk,
            }}>
            {playing ? "⏸" : "▶"}
          </button>
          <div style={{ display: "flex", borderRadius: 6, overflow: "hidden", border: `1px solid ${T.line}` }}>
            {(["week", "month", "year"] as CampaignSpeed[]).map((s) => (
              <button key={s} onClick={() => setSpeed(s)}
                title={`Each step advances one ${s}`}
                style={{
                  padding: "6px 10px", border: "none", cursor: "pointer", fontSize: 11,
                  textTransform: "capitalize", fontWeight: speed === s ? 700 : 400,
                  background: speed === s ? T.accentSoft : "transparent",
                  color: speed === s ? T.ink : T.inkDim,
                }}>
                {s}
              </button>
            ))}
          </div>
          <button
            onClick={() => advance(SPEED_TICKS[speed])}
            disabled={busy || playing}
            title={`Advance one ${speed}`}
            style={{
              height: 30, padding: "0 10px", borderRadius: 6, cursor: busy || playing ? "default" : "pointer",
              border: `1px solid ${T.line}`, background: T.raised, color: T.inkMid, fontSize: 12,
              opacity: busy || playing ? 0.45 : 1,
            }}>
            ⏭
          </button>
        </div>
      )}

      {/* ── World pulse ─────────────────────────────────────────────── */}
      {active && (
        <div style={{ display: "flex", alignItems: "stretch", gap: 6 }}>
          <Pulse label="Population" value={fmtNum(snapshot!.total_population)}
            sub={deltaStr(snapshot!.population_delta, true)}
            subColor={snapshot!.population_delta >= 0 ? T.goodInk : T.badInk} />
          <Pulse label="Price idx" value={snapshot!.price_index.toFixed(2) + "×"}
            sub={deltaStr(snapshot!.price_index_delta, false, 2)}
            subColor={snapshot!.price_index_delta <= 0 ? T.goodInk : T.warn} />
          <Pulse label="Caravans" value={String(snapshot!.in_transit)} />
          {starving > 0 && (
            <div style={{ ...pulseTile, border: "1px solid #6a3040", background: "#2a1218" }}
              title={`${starving} cities cannot feed themselves`}>
              <div style={{ ...pulseLabel, color: "#c08090" }}>Starving</div>
              <div style={{ color: T.badInk, fontSize: 13, fontWeight: 700 }}>⚠ {starving}</div>
            </div>
          )}
        </div>
      )}

      <div style={{ flex: 1 }} />

      {/* ── Ledger menus (grouped window launchers) ─────────────────── */}
      {openMenu && (
        <div style={{ position: "fixed", inset: 0, zIndex: 140 }} onClick={() => setOpenMenu(null)} />
      )}
      <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
        {/* The Atlas — the headline "what happens in the world and why" screen. */}
        <button onClick={() => ui.setShowAtlas(!ui.showAtlas)}
          title="World Atlas — world graphs, city census, lifecycle timeline"
          style={{
            padding: "5px 10px", borderRadius: 6, fontSize: 11, cursor: "pointer", whiteSpace: "nowrap",
            border: `1px solid ${ui.showAtlas ? T.goldDim : T.line}`,
            background: ui.showAtlas ? "#241d0c" : "rgba(12,18,26,0.6)",
            color: ui.showAtlas ? T.gold : T.inkDim, fontWeight: ui.showAtlas ? 700 : 400,
          }}>🗺 Atlas</button>
        {/* Map lenses: quick toggles for the trade overlays. */}
        <button onClick={() => ui.toggleOverlay("tradeHeat")} title="Trade Heat — where trade concentrates"
          style={lensChip(ui.overlayVisibility.tradeHeat ?? false, "#d9a441")}>🔥</button>
        <button onClick={() => ui.toggleOverlay("dynamicFlow")} title="Dynamic Trade Flow — the live trade arteries"
          style={lensChip(ui.overlayVisibility.dynamicFlow ?? false, "#4fd0c0")}>🌊</button>
        <button onClick={() => ui.toggleOverlay("tradeBasins")} title="Named trade basins — the regions trade binds together"
          style={lensChip(ui.overlayVisibility.tradeBasins ?? false, "#7fd08a")}>🏞</button>
        <span style={{ width: 1, height: 20, background: T.line, margin: "0 3px" }} />
        {groups.map((g) => {
          const openCount = g.items.filter((i) => i.on).length;
          const isOpen = openMenu === g.key;
          return (
            <div key={g.key} style={{ position: "relative", zIndex: isOpen ? 150 : undefined }}>
              <button
                onClick={() => setOpenMenu(isOpen ? null : g.key)}
                style={{
                  padding: "5px 10px", borderRadius: 6, fontSize: 11, cursor: "pointer", whiteSpace: "nowrap",
                  border: `1px solid ${openCount > 0 || isOpen ? T.accent : T.line}`,
                  background: isOpen ? T.accentSoft : openCount > 0 ? "#132638" : "rgba(12,18,26,0.6)",
                  color: openCount > 0 || isOpen ? T.ink : T.inkDim,
                  fontWeight: openCount > 0 ? 600 : 400,
                }}>
                {g.label}
                {openCount > 0 && (
                  <span style={{
                    marginLeft: 5, padding: "0 5px", borderRadius: 8, fontSize: 9, fontWeight: 700,
                    background: T.accent, color: "#08141f", verticalAlign: 1,
                  }}>{openCount}</span>
                )}
                <span style={{ marginLeft: 4, fontSize: 8, color: T.inkFaint }}>▼</span>
              </button>
              {isOpen && (
                <div style={menu}>
                  {g.items.map((it) => (
                    <button key={it.label} onClick={() => it.set(!it.on)} style={menuItem}>
                      <span style={{
                        width: 12, textAlign: "center", fontSize: 10,
                        color: it.on ? T.gold : T.inkFaint,
                      }}>{it.on ? "◉" : "○"}</span>
                      <span style={{ color: it.on ? T.ink : T.inkMid }}>{it.label}</span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        })}

        <span style={{ width: 1, height: 20, background: T.line, margin: "0 3px" }} />
        <button onClick={() => ui.setShowWorkflow(!ui.showWorkflow)} title="Toggle the Chronicle ledger rail (left)"
          style={chromeChip(ui.showWorkflow)}>◧ Ledger</button>
        <button onClick={() => ui.setShowToolbar(!ui.showToolbar)} title="Toggle the map toolbar (right)"
          style={chromeChip(ui.showToolbar)}>Tools ◨</button>
        <button onClick={hideAll} title="Hide every window, the ledger rail and the toolbar — a clean map"
          style={{
            padding: "5px 10px", borderRadius: 6, fontSize: 11, cursor: "pointer", whiteSpace: "nowrap",
            border: "1px solid #5a2f3a", background: "#3a1820", color: "#e0a0a0", fontWeight: 600,
          }}>✕ Clear</button>
      </div>

      {error && (
        <div style={{
          flexBasis: "100%", color: T.badInk, fontSize: 11, paddingTop: 2,
          borderTop: `1px solid ${T.lineSoft}`,
        }}>{error}</div>
      )}
    </div>
  );
}

function Pulse({ label, value, sub, subColor }: {
  label: string; value: string; sub?: string; subColor?: string;
}) {
  return (
    <div style={pulseTile}>
      <div style={pulseLabel}>{label}</div>
      <div style={{ color: T.ink, fontSize: 13, fontWeight: 700, lineHeight: 1.1 }}>
        {value}
        {sub && <span style={{ color: subColor ?? T.inkDim, fontSize: 9, fontWeight: 400, marginLeft: 4 }}>{sub}</span>}
      </div>
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

const bar: React.CSSProperties = {
  position: "absolute", top: 0, left: 0, right: 0, zIndex: 130,
  display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap",
  padding: "6px 12px",
  background: "linear-gradient(180deg, rgba(11,17,26,0.97), rgba(9,13,20,0.93))",
  borderBottom: `1px solid ${T.line}`,
  borderTop: `2px solid ${T.lineGold}`,
  boxShadow: "0 6px 22px rgba(0,0,0,0.45)",
};

const pulseTile: React.CSSProperties = {
  background: "#0c1420", border: `1px solid ${T.lineSoft}`, borderRadius: 6,
  padding: "3px 9px", minWidth: 64,
};
const pulseLabel: React.CSSProperties = {
  color: T.inkDim, fontSize: 8.5, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.5,
};

const menu: React.CSSProperties = {
  position: "absolute", top: "calc(100% + 4px)", right: 0, zIndex: 150,
  display: "flex", flexDirection: "column", minWidth: 210, padding: 4,
  background: "rgba(10,15,23,0.98)", border: `1px solid ${T.line}`, borderRadius: 8,
  boxShadow: "0 10px 30px rgba(0,0,0,0.6)",
};
const menuItem: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 7, textAlign: "left",
  padding: "6px 8px", borderRadius: 5, border: "none", background: "transparent",
  cursor: "pointer", fontSize: 11.5, whiteSpace: "nowrap",
};
/** A compact map-lens toggle (overlay on/off) tinted in the overlay's colour. */
const lensChip = (on: boolean, hue: string): React.CSSProperties => ({
  width: 28, height: 26, borderRadius: 6, fontSize: 12, cursor: "pointer",
  border: `1px solid ${on ? hue : T.line}`,
  background: on ? "rgba(255,255,255,0.05)" : "rgba(12,18,26,0.6)",
  opacity: on ? 1 : 0.6,
});

const chromeChip = (on: boolean): React.CSSProperties => ({
  padding: "5px 10px", borderRadius: 6, fontSize: 11, cursor: "pointer", whiteSpace: "nowrap",
  border: `1px solid ${on ? T.accent : T.line}`,
  background: on ? T.accentSoft : "rgba(12,18,26,0.6)",
  color: on ? T.ink : T.inkDim, fontWeight: on ? 600 : 400,
});
