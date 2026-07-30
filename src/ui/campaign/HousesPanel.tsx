import { useEffect, useMemo, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";
import { CoinIcon } from "@ui/heraldry/CoinIcon";
import { YearChronicle } from "@ui/campaign/YearChronicle";
import { FeudsView, HouseStandingView } from "@ui/campaign/HouseDossier";
import { cultureFigureSVG, cultureSeed, type Occasion } from "@ui/campaign/cultureFigure";
import { GOOD_DEFS } from "@goods";
import { campaignGetHouseHistory, campaignMerchantRoutes, campaignHouseLedger, campaignGetBanks, campaignGetExpeditions, campaignGetHouseKin } from "@bridge";
import type { HouseHistory, CampaignDiagnostics, HouseBrief, MerchantRoute, HouseLedger, BankBrief, ExpeditionView, KinBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";

const GOOD_ICON = new Map(GOOD_DEFS.map((g) => [g.name, g.emoji]));
const goodIcon = (name: string) => GOOD_ICON.get(name) ?? "\u{1F4E6}"; // 📦 fallback

/** House tiers (`HOUSE_PEOPLE_AND_TIERS.md` §1) — a rank BAND among live peers, not an
 *  absolute score. Tier 1 may be empty on a young world; that's meaningful, not a bug. */
const TIER_META: Record<number, { glyph: string; name: string; band: string }> = {
  1: { glyph: "⬤", name: "Great Houses", band: "dominate trade" },
  2: { glyph: "◐", name: "Major Houses", band: "a power in their quarter" },
  3: { glyph: "◔", name: "Lesser Houses", band: "trade, matter locally" },
  4: { glyph: "○", name: "Marginal", band: "negligible influence" },
};
/** A house whose tier isn't computed yet (founded since the last monthly pass) reads
 *  as Marginal — it hasn't proven anything yet, which is the honest starting point. */
const tierOf = (h: HouseBrief) => (h.tier && h.tier >= 1 && h.tier <= 4 ? h.tier : 4);

/** Phase 2.2 · holdings authorship — is this holding run by a POSTED kin (family) or
 *  a hired factor? Returns the kin's given name if family-run, "" if hired (silent —
 *  "hired" is the unremarkable default, same discipline as everywhere else here).
 *  A snapshot from the roster's last generation (see `Kin.posted`'s own doc comment),
 *  so it can occasionally lag a holding gained since — cosmetic, not a bug. */
function familyRunAt(kin: KinBrief[], cityName: string): string {
  const k = kin.find((x) => x.role === "factor" && x.posted_name === cityName);
  return k ? k.name.split(" ")[0] : "";
}

/** Desaturate a house colour toward grey — guilds read DULL vs the vivid private
 *  houses, so the civic bodies are visually distinct. */
function dull(hex: string): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex || "");
  if (!m) return "#7a8694";
  const n = parseInt(m[1], 16);
  const mix = (c: number) => Math.round(c * 0.45 + 0x86 * 0.55);
  const r = mix((n >> 16) & 255), g = mix((n >> 8) & 255), b = mix(n & 255);
  return `#${((r << 16) | (g << 8) | b).toString(16).padStart(6, "0")}`;
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

/** Merchant Houses browser — every trading family, its coat of arms, head of
 *  family, home city, wealth, the trades it controls (monopolies) and rivals.
 *  Active houses first; ruined ones greyed at the bottom. */
export function HousesPanel() {
  const open = useUIStore((s) => s.showHouses);
  const houses = useCampaignStore((s) => s.houses);
  const diag = useCampaignStore((s) => s.diagnostics);
  const [history, setHistory] = useState<HouseHistory | null>(null);
  const [tab, setTab] = useState<"houses" | "guilds" | "feuds">("houses");
  const [selected, setSelected] = useState<HouseBrief | null>(null);
  // Tier 3/4 collapse by default (§1 schematic) — that IS the "see who has the power
  // at a glance" the tiers exist for; expand either to browse the long tail.
  const [collapsedTiers, setCollapsedTiers] = useState<Record<number, boolean>>({ 3: true, 4: true });
  const toggleTier = (t: number) => setCollapsedTiers((c) => ({ ...c, [t]: !c[t] }));
  const clockTick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
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
  const inTab = active.filter((h) => (tab === "guilds") === !!h.is_guild);
  const maxWealth = Math.max(1, ...inTab.map((h) => h.wealth));
  const nHouses = active.filter((h) => !h.is_guild).length;
  const nGuilds = active.filter((h) => h.is_guild).length;

  const renderHouseCard = (h: HouseBrief, i: number) => (
    <div key={h.name + i} style={{ ...card, cursor: "pointer" }} onClick={() => selectHouse(h)} title="Open this family's detail">
      <CoatOfArms name={h.name} size={30} guild={h.is_guild} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
          {/* Colour chip — vivid for private houses, dull for civic guilds */}
          <span style={{ width: 9, height: 9, borderRadius: 2, background: h.is_guild ? dull(h.color ?? "") : (h.color ?? "#888"), flex: "0 0 auto", alignSelf: "center" }} />
          {h.owns_bank && <span title="Owns a chartered bank" style={{ fontSize: 10 }}>🏦</span>}
          {!h.is_guild && h.tier ? (
            <span title={`${TIER_META[tierOf(h)].name} · standing ${((h.standing ?? 0) * 100).toFixed(0)}%`}
              style={{ fontSize: 10, color: "#c9a227" }}>{TIER_META[tierOf(h)].glyph}</span>
          ) : null}
          <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 12,
            textDecoration: h.owns_bank ? "underline" : "none", textDecorationColor: "#c9a227" }}>{h.name}</span>
          {h.coin_name && <CoinIcon issuer={h.name} value={h.coin_value} size={14} title={`Mints the ${h.coin_name}`} />}
          {h.is_guild && <span title="A civic Merchant Guild — acts in its home city's interest" style={{ fontSize: 9, color: "#7fd0c0", border: "1px solid #2e5a52", borderRadius: 3, padding: "0 3px" }}>GUILD</span>}
          <span style={{ color: "#6a86a6", fontSize: 9 }}>· {h.home_name}</span>
          <span style={{ flex: 1 }} />
          {h.dominant && <span title="Controls its seat city (>=50% of its trade)" style={{ fontSize: 10 }}>⚖</span>}
          {h.political_power > 0.5 && <span title="A leading political power" style={{ fontSize: 10 }}>👑</span>}
        </div>
        <div style={{ color: "#9ab0c8", fontSize: 10 }}>
          {h.head_name} · gen. {h.generation} · led {h.head_age}y
        </div>
        {/* Trades the house controls — with good icons */}
        {h.specialties.length > 0 && (
          <div style={{ color: "#cdbb88", fontSize: 10, marginTop: 1, display: "flex", flexWrap: "wrap", gap: 4, alignItems: "center" }}>
            {h.specialties.map((g) => (
              <span key={g} title={g} style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
                <span style={{ fontSize: 11 }}>{goodIcon(g)}</span>{g}
              </span>
            ))}
          </div>
        )}
        {h.top_goods && h.top_goods.length > 0 && (
          <div style={{ color: "#8fbf9a", fontSize: 10, marginTop: 1 }} title="Top goods this family is known for exporting (by profit)">
            known for {h.top_goods.map((g) => `${goodIcon(g)} ${g}`).join(" · ")}
          </div>
        )}
        {h.monopolies.length > 0 && (
          <div style={{ color: "#e0b060", fontSize: 10 }}>
            {h.monopolies.map(([g, s]) => `${goodIcon(g)} ${g} ${Math.round(s * 100)}%`).join(" · ")} of the trade
          </div>
        )}
        {/* Cities this house trades with (seat first) */}
        {h.cities && h.cities.length > 0 && (
          <div style={{ color: "#88a8c8", fontSize: 9, marginTop: 1 }}>
            🏙 {h.cities.slice(0, 6).join(", ")}{h.cities.length > 6 ? ` +${h.cities.length - 6}` : ""}
          </div>
        )}
        {/* Fleet — each vessel is one concurrent shipment the house can run */}
        {(() => {
          const sea = h.fleet_sea ?? 0, river = h.fleet_river ?? 0, car = h.fleet_caravan ?? 0;
          if (sea + river + car === 0) return null;
          const parts: string[] = [];
          if (sea) parts.push(`🚢 ${sea} ship${sea > 1 ? "s" : ""}`);
          if (river) parts.push(`🛶 ${river} boat${river > 1 ? "s" : ""}`);
          if (car) parts.push(`🐫 ${car} caravan${car > 1 ? "s" : ""}`);
          return (
            <div style={{ color: "#a0b8c8", fontSize: 9, marginTop: 1 }}
              title="Transport capital — each vessel carries one shipment at a time">
              {parts.join(" · ")}
            </div>
          );
        })()}
        {/* Foreign offices — footholds in other cities (−5% on goods bought there) */}
        {h.offices && h.offices.length > 0 && (
          <div style={{ color: "#c8a8e0", fontSize: 9, marginTop: 1 }}
            title="Offices abroad — each gives −5% on goods bought there and a base to trade from">
            🏢 offices: {h.offices.map(([nm]) => nm).slice(0, 6).join(", ")}
            {h.offices.length > 6 ? ` +${h.offices.length - 6}` : ""}
          </div>
        )}
        {h.rivals.length > 0 && (
          <div style={{ color: "#c98", fontSize: 9 }}>⚔ rivals: {h.rivals.slice(0, 3).join(", ")}</div>
        )}
        {/* Wealth bar */}
        <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 2 }}>
          <div style={{ flex: 1, height: 4, background: "#0a1018", borderRadius: 3, overflow: "hidden" }}>
            <div style={{ width: `${(h.wealth / maxWealth) * 100}%`, height: "100%", background: "#c9a227" }} />
          </div>
          <span style={{ color: "#c9a227", fontSize: 9, minWidth: 40, textAlign: "right" }}>
            {h.wealth >= 1000 ? `${(h.wealth / 1000).toFixed(1)}k` : h.wealth.toFixed(0)}
          </span>
        </div>
      </div>
    </div>
  );

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      {history && <HouseTimeline history={history} onClose={() => setHistory(null)} />}
      {selected && <HouseDetail h={selected} onClose={() => selectHouse(null)} onChronicle={openTimeline} />}
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>⚜️ Trading Families</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>
      {/* Houses vs Guilds tabs */}
      <div style={{ display: "flex", gap: 2, padding: "0 8px", borderBottom: "1px solid #1e2e42" }}>
        {([["houses", `👑 Houses (${nHouses})`], ["guilds", `🏛 Guilds (${nGuilds})`],
           ["feuds", "⚔ Feuds"]] as const).map(([id, lbl]) => (
          <div key={id} onClick={() => setTab(id)}
            style={{ padding: "4px 9px", cursor: "pointer", fontSize: 11, fontWeight: tab === id ? 700 : 400,
              color: tab === id ? "#cfe2f6" : "#6a86a6",
              borderBottom: tab === id ? "2px solid #3a80c0" : "2px solid transparent" }}>
            {lbl}
          </div>
        ))}
      </div>
      {diag && tab !== "feuds" && <TradeDiagnostics diag={diag} />}
      {tab === "feuds" ? (
        // The world's quarrels. A feud is no longer a name in a rival list: it has a
        // cause, a temperature, a stage that decides which weapon comes out, and an
        // ending — arbitrated by a council, sealed by a marriage, ended in ruin, or
        // simply forgotten once the two families stopped getting in each other's way.
        <div style={{ overflowY: "auto", padding: "6px 8px 10px" }}>
          <FeudsView refreshKey={clockTick} />
        </div>
      ) : (
      <div style={{ overflowY: "auto", padding: "4px 8px 10px" }}>
        {houses.length === 0 && (
          <div style={empty}>Begin the campaign (Step 11) — trading families rise as goods start to move.</div>
        )}
        {houses.length > 0 && inTab.length === 0 && (
          <div style={empty}>{tab === "guilds" ? "No civic guilds yet (cities form a guild at 50,000 people)." : "No private houses yet."}</div>
        )}
        {tab === "houses" ? (
          ([1, 2, 3, 4] as const).map((t) => {
            const group = inTab.filter((h) => tierOf(h) === t);
            if (group.length === 0) return null;
            const meta = TIER_META[t];
            const collapsed = collapsedTiers[t];
            return (
              <div key={`tier-${t}`}>
                <div data-no-drag onClick={() => toggleTier(t)} title={meta.band}
                  style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer",
                    margin: "8px 0 3px", padding: "2px 4px", borderBottom: "1px solid #1e2e42" }}>
                  <span style={{ color: "#c9a227", fontSize: 11 }}>{meta.glyph}</span>
                  <span style={{ color: "#9ab0c8", fontSize: 10, fontWeight: 700, letterSpacing: 0.3 }}>
                    TIER {t} · {meta.name.toUpperCase()} ({group.length})
                  </span>
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
            <div style={{ color: "#5a6a7e", fontSize: 9, margin: "8px 0 2px", textTransform: "uppercase" }}>
              Fallen houses ({gone.length})
            </div>
            {gone.map((h, i) => (
              <div key={"d" + i} style={{ ...card, opacity: 0.55, cursor: "pointer" }} onClick={() => openTimeline(h.name)} title="View this family's timeline">
                <CoatOfArms name={h.name} size={22} guild={h.is_guild} />
                <div style={{ flex: 1 }}>
                  <span style={{ color: "#9aa6b4", fontSize: 11, textDecoration: "line-through" }}>{h.name}</span>
                  <span style={{ color: "#566", fontSize: 9 }}> · once of {h.home_name}</span>
                </div>
              </div>
            ))}
          </>
        )}
      </div>
      )}
    </div>
  );
}

/** Click-through detail for one house/guild: where it's active, its offices and
 *  estates, its fleet, and its TOP 5 routes (back & forth) with the goods it moves
 *  each way and the volume. */
function HouseDetail({ h, onClose, onChronicle }:
  { h: HouseBrief; onClose: () => void; onChronicle: (name: string) => void }) {
  const [routes, setRoutes] = useState<MerchantRoute[]>([]);
  const [ledger, setLedger] = useState<HouseLedger | null>(null);
  const [bank, setBank] = useState<BankBrief | null>(null);
  const [chron, setChron] = useState<HouseHistory | null>(null);
  const [expeds, setExpeds] = useState<ExpeditionView[]>([]);
  const [kin, setKin] = useState<KinBrief[]>([]);
  // Chronicle-first (§2.3 of the design): the dossier has nothing for the player to
  // DECIDE, so the primary artefact is the family's record, not its balance sheet.
  const [view, setView] = useState<"chronicle" | "summary" | "kin" | "standing" | "feuds" | "bank" | "ledger" | "expeditions">("chronicle");
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
    }
    // Find this family's bank (if any) so we can show its balance-sheet subtab.
    if (h.owns_bank) {
      campaignGetBanks().then((bs) => {
        if (alive) setBank(bs.find((b) => b.owner_idx === h.idx) ?? null);
      }).catch(() => {});
    } else { setBank(null); }
    return () => { alive = false; };
  }, [h.name, h.idx, h.owns_bank, tick]);
  const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
  const goodsStr = (gs: [string, number][]) => gs.slice(0, 3).map(([g, v]) => `${goodIcon(g)}${fmt(v)}`).join(" ") || "—";
  const Row = ({ label, children }: { label: string; children: React.ReactNode }) => (
    <div style={{ fontSize: 9, marginTop: 3 }}>
      <span style={{ color: "#6a86a6", textTransform: "uppercase", letterSpacing: 0.4 }}>{label}</span>
      <div style={{ color: "#bcd0e4" }}>{children}</div>
    </div>
  );
  // Phase 1.2 · the culture-dress figure. Seeded on the HEAD's own name (so it
  // re-rolls at every succession — a new head is visibly a different person), wearing
  // the seat culture's kit and the head's own sex (the line rule may put a woman at
  // the head of the house — see sim::inheritance). Register follows tier: Tier 1 reads
  // as finery before you read a number, Tier 3/4 as plain working dress.
  const figureSvg = useMemo(() => {
    const t = tierOf(h);
    const occasion: Occasion = !h.tier ? "national" : t === 1 ? "ceremonial" : t >= 3 ? "everyday" : "national";
    return cultureFigureSVG({
      kit: h.kit ?? -1, sex: h.head_female ? "f" : "m",
      seed: cultureSeed(h.head_name || h.name), occasion,
    });
  }, [h.kit, h.head_female, h.head_name, h.name, h.tier]);
  return (
    <div data-draggable style={{ ...detailPanel, ...rootStyle }} onPointerDown={onPointerDown}>
      <div style={{ display: "flex", gap: 8 }}>
        {/* The figure — house identity is added as three marks: a coloured frame (the
            house's colour, standing in for the garment accent), a coat-of-arms badge
            at the shoulder, and the tier glyph. */}
        <div style={{ position: "relative", width: 52, flex: "0 0 auto" }} title={`${TIER_META[tierOf(h)].name} · standing ${((h.standing ?? 0) * 100).toFixed(0)}%`}>
          <div style={{
            width: 52, height: 118, borderRadius: 4, overflow: "hidden", background: "#0a1119",
            border: `2px solid ${h.is_guild ? dull(h.color ?? "") : (h.color ?? "#3a5570")}`,
            boxShadow: h.tier === 1 ? "0 0 8px rgba(201,162,39,0.4)" : "none",
          }} dangerouslySetInnerHTML={{ __html: figureSvg }} />
          <div style={{ position: "absolute", top: -4, right: -4 }}>
            <CoatOfArms name={h.name} size={17} guild={h.is_guild} />
          </div>
          {!h.is_guild && h.tier ? (
            <div style={{ position: "absolute", bottom: -1, left: -1, fontSize: 10, color: "#c9a227",
              background: "#0c141edd", borderRadius: "50%", width: 13, height: 13, lineHeight: "13px", textAlign: "center" }}>
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
        {h.is_guild && <span style={{ fontSize: 8, color: "#7fd0c0" }}>GUILD</span>}
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={onClose} style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>
      <div style={{ color: "#9ab0c8", fontSize: 10 }}>{h.head_name} · of {h.home_name} · gen {h.generation}</div>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ color: "#c9a227", fontSize: 11, fontWeight: 700 }}>wealth {fmt(h.wealth)}</span>
        {h.coin_name && (h.coin_value ?? 0) > 0 && (
          <span style={{ color: "#9ab0c8", fontSize: 9 }} title={`Wealth denominated in the family's coin (grain-equivalent ÷ coin value ${(h.coin_value ?? 0).toFixed(2)}×)`}>
            ≈ {fmt(h.wealth / (h.coin_value ?? 1))} {h.coin_name}
          </span>
        )}
        {h.coin_name ? (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 9, color: "#d8c878" }}
            title={`Mints the ${h.coin_name} · value ${(h.coin_value ?? 0).toFixed(2)}× · trust ${Math.round((h.coin_trust ?? 0) * 100)}%`}>
            <CoinIcon issuer={h.name} value={h.coin_value} size={16} /> mints {h.coin_name} · {(h.coin_value ?? 0).toFixed(2)}×
          </span>
        ) : null}
      </div>
        </div>
      </div>

      {/* Subtabs — Chronicle first (§2.3: the dossier has nothing to DECIDE, so the
          record is the primary artefact). Accountant gets its own roomy view so
          expenses aren't clipped. */}
      <div style={{ display: "flex", gap: 4, margin: "7px 0 5px", borderBottom: "1px solid #1a2a3e", flexWrap: "wrap" }}>
        {(["chronicle", "summary", ...(kin.length > 0 ? ["kin" as const] : []),
           ...(expeds.length > 0 ? ["expeditions" as const] : []),
           "standing", "feuds", ...(bank ? ["bank" as const] : []), "ledger"] as const).map((t) => (
          <div key={t} onClick={() => setView(t)} style={{
            fontSize: 10, padding: "3px 9px", cursor: "pointer",
            color: view === t ? "#e8dcc0" : "#7090b0", fontWeight: view === t ? 700 : 400,
            borderBottom: view === t ? "2px solid #c9a227" : "2px solid transparent",
          }}>{t === "chronicle" ? "📜 Chronicle"
            : t === "summary" ? "Summary"
            : t === "kin" ? `👪 Kin (${kin.length})`
            : t === "expeditions" ? `🧭 Expeditions (${expeds.length})`
            : t === "standing" ? "⚖ Standing"
            : t === "feuds" ? `⚔ Feuds${h.rivals.length > 0 ? ` (${h.rivals.length})` : ""}`
            : t === "bank" ? "🏦 Bank"
            : `📒 Accountant${ledger && ledger.year > 0 ? ` · yr ${ledger.year}` : ""}`}</div>
        ))}
      </div>

      {view === "chronicle" ? (
        <ChronicleTab h={h} chron={chron} onExpand={() => onChronicle(h.name)} fmt={fmt} />
      ) : view === "kin" ? (
        <KinTab kin={kin} />
      ) : view === "expeditions" ? (
        <ExpeditionsTab expeds={expeds} fmt={fmt} />
      ) : view === "standing" ? (
        // The five stability gauges. Everything here was already in the sim — the
        // solvency countdown in particular has always decided whether this family
        // lives, and was the one number the UI never showed.
        <HouseStandingView idx={h.idx ?? 0} refreshKey={tick} />
      ) : view === "feuds" ? (
        <FeudsView house={h.idx ?? 0} refreshKey={tick} />
      ) : view === "bank" && bank ? (
        <BankSheet b={bank} fmt={fmt} />
      ) : view === "summary" ? (
        <>
          <HouseStatGrid h={h} fmt={fmt} />
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
              <div style={{ color: "#cfe0f4" }}>{r.sea ? "🚢" : "🐫"} {r.a_name} ⇄ {r.b_name} <span style={{ color: "#6a86a6" }}>· vol {fmt(r.volume)}</span></div>
              <div style={{ color: "#9ab0c8" }}>→ {goodsStr(r.out_goods)} · ← {goodsStr(r.ret_goods)}</div>
            </div>
          ))}
          <div onClick={() => onChronicle(h.name)} style={{ color: "#88a8c8", fontSize: 9, cursor: "pointer", marginTop: 5, textDecoration: "underline" }}>
            View family chronicle →
          </div>
        </>
      ) : ledger ? (
        <LedgerView l={ledger} fmt={fmt} />
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
              return (
                <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 5, padding: "1px 0", fontSize: 10 }}>
                  <span style={{ width: 12, textAlign: "center" }}>{p.female ? "♀" : "♂"}</span>
                  <span style={{ color: living ? "#e8dcc0" : "#9ab0c8", flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {p.name}{p.epithet ? ` ${p.epithet}` : ""}
                  </span>
                  <span style={{ color: "#6a86a6", fontSize: 9 }}>
                    gen {p.generation} · {p.since_year}{living ? "–present" : `–${p.until_year}`}
                    {" "}({p.age_at_accession}{p.age_at_death > 0 ? `→${p.age_at_death}` : ""}y)
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
    || l.fleet_cost || l.lost_cargo || l.events || l.consumption || l.inflation;
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
  position: "absolute", top: 60, right: 690, width: 290, maxHeight: "78vh", overflowY: "auto",
  background: "#0c141e", border: "1px solid #24364e", borderRadius: 8,
  padding: "9px 11px", boxShadow: "0 8px 28px rgba(0,0,0,0.55)", zIndex: 45,
};

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 320, maxHeight: "78vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const card: React.CSSProperties = {
  display: "flex", gap: 8, alignItems: "flex-start", padding: "6px 4px",
  borderBottom: "1px solid #131e2a", cursor: "default",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "10px 4px" };
const diagBar: React.CSSProperties = {
  padding: "6px 10px", borderBottom: "1px solid #1a2a3e", background: "#0a1119",
};
const diagCell: React.CSSProperties = {
  flex: 1, textAlign: "center", padding: "3px 2px", borderRadius: 4,
  background: "#101c28",
};

const EVENT_ICON: Record<string, string> = {
  founded: "🏛", succession: "👤", monopoly: "💰", monopoly_lost: "💸",
  control_gained: "⚖", control_lost: "💔", branch: "🌿", loss: "⚠️", dissolved: "🪦",
  figure: "🎖", marriage: "💍", piracy: "🏴",
  // Phase 1.1/1.4 · tiers + positive events (§2.2) — the mechanism otherwise only
  // produces decline; these give the chronicle something other than obituaries.
  tier_up: "⬆", golden_age: "☀", dynasty: "👑", inheritance: "📜",
};
const EVENT_COLOR: Record<string, string> = {
  founded: "#cfe0f4", succession: "#9ab0c8", monopoly: "#e0b060", monopoly_lost: "#b08a5a",
  control_gained: "#7fd0a0", control_lost: "#d88", loss: "#e08a5a",
  branch: "#9fe07a", dissolved: "#8a93a0", figure: "#e6c878", marriage: "#e6a6c8", piracy: "#c07070",
  tier_up: "#7fd0a0", golden_age: "#e6c878", dynasty: "#e6c878", inheritance: "#9ab0c8",
};

/** A house's chronicle as a vertical timeline: founding, successions, monopolies,
 *  cities controlled (gained/lost + year), the worst loss — plus its most
 *  profitable trade resources. */
function HouseTimeline({ history, onClose }: { history: HouseHistory; onClose: () => void }) {
  const ev = history.events;
  const maxProfit = Math.max(1e-6, ...history.top_goods.map(([, p]) => p));
  return (
    <div style={timelinePanel}>
      <div style={{ ...header, borderBottom: "1px solid #1a2a3e" }}>
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
                <span style={{ color: "#cdbb88", fontSize: 10, width: 78, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{g}</span>
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
