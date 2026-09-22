import { useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";
import { CoinIcon } from "@ui/heraldry/CoinIcon";
import { HouseDetail, HouseTimeline } from "@ui/campaign/HouseDossier";
import { HouseCompareWindow } from "@ui/campaign/HouseCompare";
import { goodIcon, TIER_META, tierOf, dull } from "@ui/campaign/houseShared";
import { clarifyGemLabel } from "@goods";
import { campaignGetHouseHistory } from "@bridge";
import type { HouseHistory, CampaignDiagnostics, HouseBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";

/** ⚜️ Trading Families — HOUSES_GUILDS_AND_MARKET_PLAN.md S10's own BROWSE window:
 *  rank / filter only, per §5's own table. What used to live here besides the list —
 *  the big per-house dossier and the world Feuds board — are now their own windows
 *  (`HouseDossier.tsx`'s `HouseDetail`, and `FeudsAlliancesPanel.tsx`), because a
 *  feud belongs to two houses, not one, and a 1,455-line file that browsed AND
 *  detailed AND adjudicated the world's quarrels was the asymmetry the plan named.
 *
 *  Merchant Companies (`House.is_guild`) — civic firms, not a different kind of
 *  thing from a private house — stop being a TAB (which is what made the "guild"
 *  naming collision with `CraftGuild` visible to users in the first place) and
 *  become a filter CHIP here instead. */
export function HousesPanel() {
  const open = useUIStore((s) => s.showHouses);
  const houses = useCampaignStore((s) => s.houses);
  const diag = useCampaignStore((s) => s.diagnostics);
  const [history, setHistory] = useState<HouseHistory | null>(null);
  // The one remaining filter: private houses (the default) vs civic companies.
  const [showCompanies, setShowCompanies] = useState(false);
  const [selected, setSelected] = useState<HouseBrief | null>(null);
  const [compareOpen, setCompareOpen] = useState(false);
  // Tier 3/4 collapse by default (§1 schematic) — that IS the "see who has the power
  // at a glance" the tiers exist for; expand either to browse the long tail.
  const [collapsedTiers, setCollapsedTiers] = useState<Record<number, boolean>>({ 3: true, 4: true });
  const toggleTier = (t: number) => setCollapsedTiers((c) => ({ ...c, [t]: !c[t] }));
  const setSelectedHouse = useCampaignStore((s) => s.setSelectedHouse);
  // Focus a house: open its detail AND tell the map to highlight only it.
  const selectHouse = (h: HouseBrief | null) => {
    setSelected(h);
    setSelectedHouse(h?.idx ?? null);
    // Auto-show the House Control map layer so the focused house's sphere is visible.
    if (h) useUIStore.getState().setOverlayVisible("houseControl", true);
  };
  const close = () => useUIStore.getState().setShowHouses(false);
  const openTimeline = (name: string) => {
    campaignGetHouseHistory(name).then((h) => setHistory(h)).catch(() => setHistory(null));
  };
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.houses);
  if (!open) return null;

  const active = houses.filter((h) => !h.defunct);
  const gone = houses.filter((h) => h.defunct);
  const inTab = active.filter((h) => showCompanies === !!h.is_guild);
  const maxWealth = Math.max(1, ...inTab.map((h) => h.wealth));
  const nHouses = active.filter((h) => !h.is_guild).length;
  const nGuilds = active.filter((h) => h.is_guild).length;

  const fmtWealth = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));

  const renderHouseCard = (h: HouseBrief, i: number) => {
    const accent = h.is_guild ? dull(h.color ?? "") : (h.color ?? "#888");
    const sea = h.fleet_sea ?? 0, river = h.fleet_river ?? 0, car = h.fleet_caravan ?? 0;
    const fleetTotal = sea + river + car;
    return (
    <div key={h.name + i} style={{ ...card, cursor: "pointer" }} onClick={() => selectHouse(h)}
      onMouseEnter={(e) => { e.currentTarget.style.background = "#152234"; e.currentTarget.style.borderColor = "#2a3f5a"; }}
      onMouseLeave={(e) => { e.currentTarget.style.background = "#101a26"; e.currentTarget.style.borderColor = "#1c2c3f"; }}
      title="Open this family's detail">
      {/* A left accent bar carries the house's own colour the instant the eye lands
          on the card — identity before the reader even reaches the coat of arms. */}
      <span style={{ position: "absolute", left: 0, top: 6, bottom: 6, width: 3, borderRadius: 2, background: accent }} />
      <CoatOfArms name={h.name} size={32} guild={h.is_guild} />
      <div style={{ flex: 1, minWidth: 0 }}>
        {/* ── IDENTITY ROW: name is the loudest thing on the card ── */}
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {!h.is_guild && h.tier ? (
            <span title={`${TIER_META[tierOf(h)].name} · standing ${((h.standing ?? 0) * 100).toFixed(0)}%`}
              style={{ fontSize: 11, color: "#c9a227", flex: "0 0 auto" }}>{TIER_META[tierOf(h)].glyph}</span>
          ) : null}
          <span style={{ color: "#f0e4c8", fontWeight: 700, fontSize: 13, lineHeight: 1.2,
            overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
            textDecoration: h.owns_bank ? "underline" : "none", textDecorationColor: "#c9a227", textUnderlineOffset: 2 }}>
            {h.name}
          </span>
          {h.owns_bank && <span title="Owns a chartered bank" style={{ fontSize: 10.5, flex: "0 0 auto" }}>🏦</span>}
          {h.coin_name && <CoinIcon issuer={h.name} value={h.coin_value} size={13} title={`Mints the ${h.coin_name}`} />}
          {h.is_guild && <span title="A civic Merchant Guild — acts in its home city's interest"
            style={{ fontSize: 8.5, color: "#7fd0c0", border: "1px solid #2e5a52", borderRadius: 3, padding: "0 3px", flex: "0 0 auto" }}>COMPANY</span>}
          <span style={{ flex: 1 }} />
          {h.dominant && <span title="Controls its seat city (>=50% of its trade)" style={{ fontSize: 10.5, flex: "0 0 auto" }}>⚖</span>}
          {h.political_power > 0.5 && <span title="A leading political power" style={{ fontSize: 10.5, flex: "0 0 auto" }}>👑</span>}
        </div>
        {/* ── META ROW: who leads it, where, wealth — the second-loudest facts ── */}
        <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginTop: 2, fontSize: 10.5 }}>
          <span style={{ color: "#8fa6be" }}>{h.head_name}</span>
          <span style={{ color: "#465870" }}>gen.{h.generation} · led {h.head_age}y</span>
          <span style={{ color: "#3e5068" }}>·</span>
          <span style={{ color: "#7a90a8" }}>{h.home_name}</span>
          <span style={{ flex: 1 }} />
          <span style={{ color: "#e0c060", fontWeight: 700, fontVariantNumeric: "tabular-nums" }}>{fmtWealth(h.wealth)}</span>
        </div>
        {/* Wealth bar — a background-weight cue under the meta row, not the very
            last, easiest-to-miss line of a six-line stack. */}
        <div style={{ height: 3, background: "#0a1018", borderRadius: 2, overflow: "hidden", marginTop: 3 }}>
          <div style={{ width: `${(h.wealth / maxWealth) * 100}%`, height: "100%",
            background: `linear-gradient(90deg, ${accent}99, #c9a227)` }} />
        </div>
        {/* ── FACTS: every "known for / carries / holds" line collapsed into one
            wrapping row of chips, instead of five separate stacked sentences ── */}
        {(h.top_goods?.length || h.specialties.length || h.monopolies.length || fleetTotal > 0
          || h.offices?.length || h.rivals.length) ? (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}>
            {h.top_goods && h.top_goods.length > 0 && (
              <span style={{ ...chip, color: "#a8dcb4", borderColor: "#1e3a2a" }}
                title="Top goods this family is known for exporting (by profit)">
                {h.top_goods.slice(0, 3).map((g) => `${goodIcon(g)} ${clarifyGemLabel(g, h.gem_variety)}`).join("  ")}
              </span>
            )}
            {h.specialties.filter((g) => !h.top_goods?.includes(g)).slice(0, 3).map((g) => (
              <span key={g} style={{ ...chip, color: "#d8c896" }} title={`Specialises in ${g}`}>
                {goodIcon(g)} {clarifyGemLabel(g, h.gem_variety)}
              </span>
            ))}
            {h.monopolies.map(([g, s]) => (
              <span key={g} style={{ ...chip, color: "#f0c878", borderColor: "#3a2e14", background: "#180f06" }}
                title={`Holds ${Math.round(s * 100)}% of the world's ${g} trade`}>
                {goodIcon(g)} {Math.round(s * 100)}% {g}
              </span>
            ))}
            {fleetTotal > 0 && (
              <span style={chip} title="Transport capital — each vessel carries one shipment at a time">
                {sea > 0 && `🚢${sea} `}{river > 0 && `🛶${river} `}{car > 0 && `🐫${car}`}
              </span>
            )}
            {h.offices && h.offices.length > 0 && (
              <span style={{ ...chip, color: "#c8a8e0", borderColor: "#2e2244" }}
                title={`Offices abroad: ${h.offices.map(([nm]) => nm).join(", ")}`}>
                🏢 {h.offices.length} office{h.offices.length > 1 ? "s" : ""}
              </span>
            )}
            {h.rivals.length > 0 && (
              <span style={{ ...chip, color: "#e0a89a", borderColor: "#3a2420" }}
                title={`Feuding: ${h.rivals.join(", ")}`}>
                ⚔ {h.rivals.length} rival{h.rivals.length > 1 ? "s" : ""}
              </span>
            )}
          </div>
        ) : null}
        {/* Cities traded with — kept as its own quiet line (a list of place names
            reads better unwrapped than squeezed into a chip). */}
        {h.cities && h.cities.length > 0 && (
          <div style={{ color: "#5e7692", fontSize: 9, marginTop: 4, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
            title={h.cities.join(", ")}>
            🏙 {h.cities.slice(0, 6).join(", ")}{h.cities.length > 6 ? ` +${h.cities.length - 6}` : ""}
          </div>
        )}
      </div>
    </div>
    );
  };

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      {history && <HouseTimeline history={history} onClose={() => setHistory(null)} />}
      {selected && <HouseDetail h={selected} onClose={() => selectHouse(null)} onChronicle={openTimeline} onSelectHouse={selectHouse} />}
      {compareOpen && <HouseCompareWindow houses={houses} onClose={() => setCompareOpen(false)} />}
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>⚜️ Trading Families</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      {/* The one remaining filter (private houses vs civic companies), plus quick
          entry points to the two windows this list used to embed as tabs. */}
      <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 10px", borderBottom: "1px solid #1e2e42", flexWrap: "wrap" }}>
        <span style={{ color: "#e8dcc0", fontSize: 11.5, fontWeight: 700 }}>👑 Houses ({nHouses})</span>
        <div data-no-drag onClick={() => setShowCompanies((v) => !v)}
          title="Merchant Companies (House.is_guild) — civic firms, not a different kind of thing from a private house"
          style={{
            padding: "3px 9px", cursor: "pointer", fontSize: 10.5, borderRadius: 12,
            color: showCompanies ? "#0c141e" : "#7fd0c0",
            background: showCompanies ? "#7fd0c0" : "#0e2420",
            border: "1px solid #2e5a52", fontWeight: showCompanies ? 700 : 400,
          }}>
          🏛 Companies ({nGuilds})
        </div>
        <span style={{ flex: 1 }} />
        <div data-no-drag onClick={() => useUIStore.getState().setShowFeuds(true)} title="Open the world Feuds & Alliances board"
          style={{ padding: "3px 9px", cursor: "pointer", fontSize: 10.5, color: "#e0a89a",
            border: "1px solid #3a2420", borderRadius: 5, background: "#160f0e" }}>
          ⚔ Feuds
        </div>
        <div data-no-drag onClick={() => setCompareOpen(true)} title="Compare two houses side by side"
          style={{ padding: "3px 9px", cursor: "pointer", fontSize: 10.5, color: "#cfe2f6",
            border: "1px solid #2e4864", borderRadius: 5, background: "#101c28" }}>
          ⚖ Compare
        </div>
      </div>
      {diag && <TradeDiagnostics diag={diag} />}
      <div style={{ overflowY: "auto", padding: "4px 8px 10px" }}>
        {houses.length === 0 && (
          <div style={empty}>Begin the campaign (Step 11) — trading families rise as goods start to move.</div>
        )}
        {houses.length > 0 && inTab.length === 0 && (
          <div style={empty}>{showCompanies ? "No civic companies yet (cities form one at 50,000 people)." : "No private houses yet."}</div>
        )}
        {!showCompanies ? (
          ([1, 2, 3, 4] as const).map((t) => {
            const group = inTab.filter((h) => tierOf(h) === t);
            if (group.length === 0) return null;
            const meta = TIER_META[t];
            const collapsed = collapsedTiers[t];
            return (
              <div key={`tier-${t}`}>
                {/* A filled bar, not a thin bottom-rule — the section that answers
                    "who has the power at a glance" deserves more weight than the
                    cards it groups, not less. */}
                <div data-no-drag onClick={() => toggleTier(t)} title={meta.band}
                  style={{ display: "flex", alignItems: "center", gap: 7, cursor: "pointer",
                    margin: "10px 0 6px", padding: "5px 9px", borderRadius: 6,
                    background: t === 1 ? "linear-gradient(90deg, #1c1608, #120e06)" : "#0e1622",
                    border: `1px solid ${t === 1 ? "#3a2e10" : "#1a2838"}` }}>
                  <span style={{ color: "#c9a227", fontSize: 12 }}>{meta.glyph}</span>
                  <span style={{ color: t === 1 ? "#e8d090" : "#9ab0c8", fontSize: 10.5, fontWeight: 700, letterSpacing: 0.4 }}>
                    {meta.name.toUpperCase()}
                  </span>
                  <span style={{ color: "#4e6480", fontSize: 9.5, fontWeight: 400 }}>({group.length})</span>
                  <span style={{ flex: 1 }} />
                  <span style={{ color: "#5a6a7e", fontSize: 9 }}>{meta.band}</span>
                  <span style={{ color: "#5a6a7e", fontSize: 9 }}>{collapsed ? "▸" : "▾"}</span>
                </div>
                {!collapsed && group.map((h, i) => renderHouseCard(h, i))}
              </div>
            );
          })
        ) : (
          inTab.map((h, i) => renderHouseCard(h, i))
        )}
        {gone.length > 0 && (
          <>
            <div style={{ color: "#5a6a7e", fontSize: 9.5, fontWeight: 700, letterSpacing: 0.4,
              margin: "12px 0 6px", padding: "0 2px", textTransform: "uppercase" }}>
              🪦 Fallen houses ({gone.length})
            </div>
            {gone.map((h, i) => (
              <div key={"d" + i} style={{ ...card, padding: "6px 10px", opacity: 0.6, cursor: "pointer" }} onClick={() => openTimeline(h.name)} title="View this family's timeline">
                <CoatOfArms name={h.name} size={22} guild={h.is_guild} />
                <div style={{ flex: 1, display: "flex", alignItems: "baseline", gap: 6 }}>
                  <span style={{ color: "#9aa6b4", fontSize: 11.5, textDecoration: "line-through" }}>{h.name}</span>
                  <span style={{ color: "#4a5c72", fontSize: 9.5 }}>once of {h.home_name}</span>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}

/** "Is trade actually moving?" — a compact health strip above the houses list.
 *  Answers the core merchant-house question: are shipments flowing, how many are
 *  financed by houses vs. local guilds, are voyages being lost, and how much of
 *  the world do houses actually control. */
function TradeDiagnostics({ diag }: { diag: CampaignDiagnostics }) {
  const fleet = diag.fleet_sea + diag.fleet_river + diag.fleet_caravan;
  const moving = diag.shipments_last > 0;
  const housePct = diag.shipments_last > 0
    ? Math.round((diag.by_house / diag.shipments_last) * 100) : 0;
  const stat = (label: string, value: string, color = "#cfe0f4", title?: string) => (
    <div style={diagCell} title={title}>
      <div style={{ color, fontWeight: 700, fontSize: 12 }}>{value}</div>
      <div style={{ color: "#6a86a6", fontSize: 9 }}>{label}</div>
    </div>
  );
  return (
    <div style={diagBar}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
        <span style={{ width: 7, height: 7, borderRadius: "50%", background: moving ? "#5fd08a" : "#d06a5f" }} />
        <span style={{ color: moving ? "#9fe0b8" : "#e0a09a", fontSize: 10, fontWeight: 600 }}>
          {moving ? "Trade is flowing" : "No shipments last advance"}
        </span>
        <span style={{ flex: 1 }} />
        <span style={{ color: "#6a86a6", fontSize: 9 }}>year {diag.year}</span>
      </div>
      <div style={{ display: "flex", gap: 4 }}>
        {stat("shipped", String(diag.shipments_last), "#cfe0f4", "Shipments dispatched last advance")}
        {stat("by houses", `${housePct}%`, housePct > 0 ? "#e0c060" : "#7a90a8", `${diag.by_house} financed by a house, ${diag.by_guild} by local guilds`)}
        {stat("lost", String(diag.lost_last), diag.lost_last > 0 ? "#e0a09a" : "#7a90a8", "Voyages lost to storm/ambush last advance")}
        {stat("in transit", String(diag.in_transit), "#9ab0c8", "Shipments currently in flight")}
      </div>
      <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
        {stat("controls", String(diag.controlled_settlements), diag.controlled_settlements > 0 ? "#9fe0b8" : "#d06a5f", "Settlements a house controls (>=50% of trade throughput)")}
        {stat("ships", String(fleet), "#9ab0c8", `${diag.fleet_sea} sea · ${diag.fleet_river} river · ${diag.fleet_caravan} caravan`)}
        {stat("houses", `${diag.houses_active}`, "#cfe0f4", `${diag.houses_active} active · ${diag.houses_defunct} ruined`)}
        {stat("wealth", diag.total_house_wealth >= 100 ? `${Math.round(diag.total_house_wealth)}` : diag.total_house_wealth.toFixed(1), "#e0c060", "Combined wealth of all active houses")}
      </div>
    </div>
  );
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 396, maxHeight: "80vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #24364e", borderRadius: 10,
  boxShadow: "0 12px 36px rgba(0,0,0,0.55)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "10px 12px", borderBottom: "1px solid #1a2a3e",
  color: "#e8dcc0", fontWeight: 700, fontSize: 13, letterSpacing: 0.2,
};
/** One house's card: a raised tile (not just a bottom-rule row) so a dense list of
 *  36 families still reads as distinct entries rather than one long scroll of text.
 *  A left accent bar in the house's own colour carries identity even before the
 *  reader reaches the coat of arms — the same trick a kanban/ticket card uses. */
const card: React.CSSProperties = {
  display: "flex", gap: 10, alignItems: "flex-start", padding: "9px 10px 9px 12px",
  margin: "0 0 6px", borderRadius: 7, background: "#101a26",
  border: "1px solid #1c2c3f", position: "relative", cursor: "default",
  transition: "background 0.12s, border-color 0.12s",
};
/** A small rounded label — the unit every "fact" (fleet count, office count, a
 *  monopoly share) renders as, instead of a bare run of icon+text on its own line.
 *  Consistent chip styling is what turns six stacked sentences into one scannable row. */
const chip: React.CSSProperties = {
  display: "inline-flex", alignItems: "center", gap: 3, fontSize: 9.5,
  padding: "1.5px 6px", borderRadius: 9, background: "#0a1420",
  border: "1px solid #1e3048", color: "#9ab0c8", lineHeight: 1.5, whiteSpace: "nowrap",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "14px 6px", textAlign: "center" };
const diagBar: React.CSSProperties = {
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e", background: "#0a1119",
};
const diagCell: React.CSSProperties = {
  flex: 1, textAlign: "center", padding: "4px 2px", borderRadius: 5,
  background: "#101c28", border: "1px solid #16222e",
};
