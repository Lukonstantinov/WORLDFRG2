import { useEffect, useMemo, useRef, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useWorldStore } from "../state/worldStore";
import { useGoodsStore } from "../state/goodsStore";
import { useCampaignStore } from "../state/campaignStore";
import { campaignGetHub } from "../bridge/tauri";
import type {
  EconHub, HubDetail, HubGoodDetail, HouseBrief, SocietyBrief, CultureMood,
  Settlement, CityFinance,
} from "../types";
import { settlementStory } from "../settlementStory";
import { GOOD_DEFS } from "../goods";
import { climatePhrase } from "./climate";
import { CoatOfArms, houseColor } from "./CoatOfArms";
import { CoinIcon } from "./CoinIcon";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";

const HP_GOOD_EMOJI: Record<string, string> = Object.fromEntries(GOOD_DEFS.map((g) => [g.name, g.emoji]));

/* ══════════════════════════════════════════════════════════════════════════
   ANTIQUE LEDGER — the settlement dossier rendered as a clerk's bound folio.
   A single high-density scroll (no tabs): parchment ground, oxblood/ink-green
   double-entry, monospace figures, geometric line-art rules. The Bureaucrat's
   Dossier collects everything the old tabbed window held into one reading.
   ══════════════════════════════════════════════════════════════════════════ */

// ── Parchment palette ──────────────────────────────────────────────────────
const PARCH = "#E5E0D0";       // page ground
const PARCH_HI = "#EFEAD9";    // raised inset (lighter leaf)
const PARCH_LO = "#D8D2BE";    // sunken inset / stripe
const INK = "#202830";         // primary ink
const INK_SOFT = "#4b5560";    // secondary ink
const INK_FAINT = "#6f7680";   // labels / captions
const RULE = "#202830";        // strong rule
const RULE_SOFT = "#B3AA92";   // faint rule
const OXBLOOD = "#8a2b20";     // debit / deficit / war
const INKGREEN = "#3f5e34";    // credit / surplus
const GOLD = "#8a6a2c";        // coin / accent
const MONO = "'SFMono-Regular', ui-monospace, 'DejaVu Sans Mono', Menlo, Consolas, monospace";

const mono: React.CSSProperties = { fontFamily: MONO, fontVariantNumeric: "tabular-nums" };

/** Deterministic 32-bit PRNG so a settlement's townscape is stable per id. */
function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const fmt = (v: number) => (v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(0));
const fmtN = fmt;

/** A geometric line-art rule. `double` draws the ledger's twin hairlines. */
function Rule({ double, soft, mt = 0, mb = 0 }: { double?: boolean; soft?: boolean; mt?: number; mb?: number }) {
  const c = soft ? RULE_SOFT : RULE;
  if (double) return (
    <div style={{ margin: `${mt}px 0 ${mb}px`, borderTop: `1px solid ${c}`, borderBottom: `1px solid ${c}`, height: 3 }} />
  );
  return <div style={{ margin: `${mt}px 0 ${mb}px`, borderTop: `1px solid ${c}` }} />;
}

/** A ruled section caption in the ledger hand: small caps under a hairline, with
 *  a leading register mark. */
function Head({ children, sub }: { children: React.ReactNode; sub?: React.ReactNode }) {
  return (
    <div style={{ margin: "12px 0 5px" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ color: INK, fontSize: 8 }}>▐</span>
        <span style={{ color: INK, fontSize: 11, fontWeight: 800, letterSpacing: 1.3, textTransform: "uppercase" }}>
          {children}
        </span>
        {sub != null && <span style={{ color: INK_FAINT, fontSize: 9, fontStyle: "italic", ...mono }}>{sub}</span>}
        <span style={{ flex: 1 }} />
      </div>
      <Rule mt={2} />
    </div>
  );
}

// ── Icons/colours for the chronicle event kinds (ink-toned for parchment). ──
const HUB_EVENT_ICON: Record<string, string> = {
  estate: "🏡", starvation: "💀", succession: "👤", structure: "🏗", coinage: "🪙",
  bank: "🏦", crash: "📉", war: "⚔", event: "⚡", figure: "🎖", fair: "🎪",
  pilgrimage: "🕊", temple: "⛪", contagion: "☠", marriage: "💍", feud: "🗡",
  guild_strike: "✊", guildhall: "🏛", fashion: "👗", wonder: "🗿", piracy: "🏴", diaspora: "🧭",
};

const LOCAL_COLOR = "#7a6a4a";  // unaffiliated local merchants
const GUILD_COLOR = "#3a5570";  // organised merchant guilds
const ESTATE_EMOJI: Record<number, string> = { 1: "🌾", 2: "⛏️", 3: "🌿", 4: "🎣", 5: "🍇", 6: "🏭" };
const ESTATE_LABEL: Record<number, string> = { 1: "Farm", 2: "Mine", 3: "Plantation", 4: "Fishery", 5: "Vineyard", 6: "Manufactory" };
const STRUCT_EMOJI: Record<string, string> = { Granary: "🌾", Warehouse: "📦", Shipyard: "⚓", Guildhall: "🏛", Workshop: "🔨" };

/** Hub inspector, re-cut as an antique ledger. One scrolling dossier: the crest &
 *  townscape head the folio, then the register of accounts, the double-entry books,
 *  the scales ledger of the market, holdings, government, the populace and the
 *  bound chronicle — all on parchment, figures set in the clerk's monospace hand. */
export function HubPanel() {
  const selectedHub = useUIStore((s) => s.selectedHub);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setShowColonial = useUIStore((s) => s.setShowColonial);
  const economy = useWorldStore((s) => s.economy);
  const goodMeta = useGoodsStore((s) => s.meta);
  const campActive = useCampaignStore((s) => s.snapshot?.active ?? false);
  const campTick = useCampaignStore((s) => s.snapshot?.clock.tick ?? 0);
  const isConstruction = useCampaignStore((s) => {
    if (selectedHub == null || !s.snapshot?.active) return false;
    const h = s.snapshot.hubs.find((x) => x.id === selectedHub);
    return !!h && (h.build_stage ?? 0) > 0;
  });
  const selKind = useCampaignStore((s) => {
    if (selectedHub == null || !s.snapshot?.active) return -1;
    const h = s.snapshot.hubs.find((x) => x.id === selectedHub);
    return h ? h.colony_kind : -1;
  });

  const [detail, setDetail] = useState<HubDetail | null>(null);

  // Pull live per-hub detail while a campaign runs, refreshed each tick.
  useEffect(() => {
    let alive = true;
    if (selectedHub === null || !campActive) { setDetail(null); return; }
    campaignGetHub(selectedHub).then((d) => { if (alive) setDetail(d); }).catch(() => { if (alive) setDetail(null); });
    return () => { alive = false; };
  }, [selectedHub, campActive, campTick]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.settlement);
  if (selectedHub === null || !economy || isConstruction) return null;
  const econHub = economy.hubs.find((h) => h.id === selectedHub);
  const starsForPop = (pop: number): number =>
    pop >= 35_000 ? 5 : pop >= 8_000 ? 4 : pop >= 1_500 ? 3 : pop >= 350 ? 2 : 1;
  const hub: EconHub | undefined = econHub ?? ((detail && selKind !== 1 && selKind !== 2) ? {
    id: detail.id, x: detail.x, y: detail.y, name: detail.name, power: 0,
    stars: starsForPop(detail.population),
    wealth: detail.trade_wealth + detail.grain_wealth, population: detail.population,
    coastal: detail.coastal, koppen: detail.koppen, sea_access: detail.coastal,
    produces: [], receives: [],
  } : undefined);
  if (!hub) return null;
  const inEconomy = !!econHub;

  const iconFor = (id: string) => goodMeta(id).icon;
  const labelFor = (id: string) => goodMeta(id).name;
  const hubName = (id: number) => economy.hubs.find((h) => h.id === id)?.name ?? `Hub ${id}`;
  const stars = Math.max(1, Math.min(5, hub.stars));

  const topHub = economy.hubs.reduce((a, b) => (b.throughput ?? 0) > (a.throughput ?? 0) ? b : a, economy.hubs[0]);
  const isTop = !!topHub && topHub.id === hub.id;
  const cmp = isTop
    ? `≈${Math.round(hub.ref_pct ?? 100)}% of ${hub.nearest_ref ?? "Venice"}`
    : `≈${Math.round(((hub.throughput ?? 0) / ((topHub?.throughput) || 1)) * 100)}% of ${topHub?.name ?? "the capital"}`;
  const wealthSorted = [...economy.hubs].sort((a, b) => b.wealth - a.wealth);
  const wealthRank = wealthSorted.findIndex((h) => h.id === hub.id) + 1;

  const shortageReason = (r: string): string => ({
    no_supplier: "produced nowhere reachable",
    unreachable: "no trade route reaches a producer",
    no_port: "landlocked — cannot reach a sea producer",
    deficit: "local demand outstrips supply",
  } as Record<string, string>)[r] ?? "scarce";

  // Is the city's craft-industry running? (drives townscape chimney smoke.)
  const manufacturing = !!detail && (
    (detail.estates_here ?? []).some((e) => e.kind === 6) ||
    (detail.structures ?? []).some(([n]) => n === "Workshop" || n === "Guildhall") ||
    ((detail.finance?.prev?.tax_manufacture ?? detail.finance?.tax_manufacture ?? 0) > 0.01)
  );
  const atWar = !!detail?.war_with;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle, background: PARCH }}>
      {/* ════════ MASTHEAD — crest, name, rating (drag handle) ════════ */}
      <div style={{ display: "flex", gap: 10, alignItems: "flex-start", cursor: "move" }} onPointerDown={onPointerDown}>
        <DossierCrest name={hub.name} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
            <span style={{ color: INK, fontSize: 17, fontWeight: 800, letterSpacing: 0.3, fontFamily: "Georgia, 'Times New Roman', serif" }}>
              {hub.name}
            </span>
            <span style={{ flex: 1 }} />
            <span data-no-drag onClick={() => setSelectedHub(null)}
              style={{ color: INK_SOFT, cursor: "pointer", fontSize: 18, lineHeight: 1, border: `1px solid ${RULE}`, borderRadius: 2, padding: "0 4px" }}
              title="Close the folio">×</span>
          </div>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginTop: 1 }}>
            <span style={{ color: GOLD, fontSize: 12, letterSpacing: 2 }}>
              {"★".repeat(stars)}<span style={{ color: RULE_SOFT }}>{"★".repeat(5 - stars)}</span>
            </span>
            <span style={{ ...mono, color: INK_SOFT, fontSize: 9 }}>{cmp}</span>
          </div>
          <div style={{ color: INK_FAINT, fontSize: 9.5, fontStyle: "italic", marginTop: 2 }}>
            {isTop ? "Sovereign Emporium of the trade world"
              : hub.emporium ? "A chartered Emporium of the realm"
              : `Registered market seat${inEconomy ? ` · wealth rank № ${wealthRank} of ${economy.hubs.length}` : ""}`}
            {hub.sea_access === false && " · inland / lakeside"}
          </div>
          {detail?.patron && (
            <div style={{ color: INKGREEN, fontSize: 9.5, marginTop: 2 }}
              title="A merchant house develops this city as a trade base.">
              ⚓ Trade base of {detail.patron}
            </div>
          )}
        </div>
      </div>
      <Rule double mt={8} />

      {/* ════════ TOWNSCAPE — pixel woodcut skyline of the seat ════════ */}
      <Townscape hub={hub} detail={detail} stars={stars} manufacturing={manufacturing} atWar={atWar} />
      <div style={{ display: "flex", justifyContent: "space-between", ...mono, fontSize: 8.5, color: INK_FAINT, marginTop: 2 }}>
        <span>{hub.coastal ? "port seat" : "landward seat"}{atWar ? " · UNDER SIEGE" : ""}</span>
        <span>{manufacturing ? "manufactories at work ✎" : "no smoke — mercantile only"}</span>
      </div>

      {/* ════════ REGISTER OF ACCOUNTS — headline figures ════════ */}
      <Head sub="as entered by the clerk">Register of accounts</Head>
      {campActive && detail ? (
        <>
          <div style={statGrid}>
            <StatCell label="Sold →" value={fmt(detail.sold ?? 0)} />
            <StatCell label="← Bought" value={fmt(detail.bought ?? 0)} />
            <StatCell label="By sea" value={fmt(detail.in_by_sea ?? 0)} />
            <StatCell label="By land" value={fmt(detail.in_by_land ?? 0)} />
            <StatCell label="Coffer" value={fmt((detail.trade_wealth ?? 0) + (detail.grain_wealth ?? 0))} />
            <StatCell label="Souls" value={detail.population.toLocaleString()} />
          </div>
          <div style={{ ...mono, fontSize: 8, color: INK_FAINT, margin: "3px 1px 0", letterSpacing: 0.3 }}>
            ✦ live entries · trade recorded this year
          </div>
        </>
      ) : (
        <div style={statGrid}>
          <StatCell label="Throughput" value={fmt(hub.throughput ?? 0)} />
          <StatCell label="Exports →" value={fmt(hub.exports ?? 0)} />
          <StatCell label="← Imports" value={fmt(hub.imports ?? 0)} />
          <StatCell label="Partners" value={String(hub.partners ?? 0)} />
          <StatCell label="Wealth" value={`${Math.round(hub.wealth * 100)}%`} />
          <StatCell label="Souls" value={hub.population.toLocaleString()} />
        </div>
      )}

      {/* Estate charter note */}
      {detail && (detail.estate_kind ?? 0) > 0 && (
        <div style={estateBox}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
            <span style={{ fontSize: 13 }}>{ESTATE_EMOJI[detail.estate_kind ?? 0] ?? "🏡"}</span>
            <span style={{ color: INK, fontWeight: 800, fontSize: 12, letterSpacing: 0.5 }}>
              {ESTATE_LABEL[detail.estate_kind ?? 0] ?? "Estate"}
            </span>
            <span style={{ flex: 1 }} />
            <span style={{ ...mono, color: INKGREEN, fontSize: 9 }}>income → owner</span>
          </div>
          <div style={{ color: INK_SOFT, fontSize: 10, marginTop: 2 }}>
            Held by <span style={{ color: INK, fontWeight: 700 }}>{detail.estate_owner || "—"}</span>
            {detail.estate_good && <> · works {iconFor(detail.estate_good)} {labelFor(detail.estate_good)}</>}
          </div>
        </div>
      )}

      {/* Richest trade + monopolies line */}
      {hub.top_export && (
        <div style={{ color: INK, fontSize: 11, margin: "6px 1px 2px", display: "flex", gap: 6, alignItems: "baseline" }}>
          <span style={{ color: INK_FAINT, fontSize: 10 }}>Chief ware:</span>
          <span style={{ fontWeight: 700 }}>{iconFor(hub.top_export)} {labelFor(hub.top_export)}</span>
          {hub.top_export_share !== undefined && hub.top_export_share > 0 && (
            <span style={{ ...mono, color: INK_SOFT, fontSize: 10 }}>{Math.round(hub.top_export_share * 100)}% of export value</span>
          )}
        </div>
      )}
      {hub.monopolies && hub.monopolies.length > 0 && (
        <div style={{ color: INK_SOFT, fontSize: 10, margin: "3px 1px 2px" }}>
          <span style={{ color: INK_FAINT }}>Monopolies held: </span>
          {hub.monopolies.map((m) => `${iconFor(m)} ${labelFor(m)}`).join(", ")}
        </div>
      )}

      {/* ════════ DOUBLE-ENTRY BOOKS — the city treasury ════════ */}
      {detail && detail.treasury !== undefined && (
        <TAccount detail={detail} />
      )}

      {/* ════════ THE SCALES LEDGER — market by balance of supply & demand ════════ */}
      <Head sub="supply weighed against demand">The scales ledger</Head>
      <ScalesLedger detail={detail} hub={hub} iconFor={iconFor} labelFor={labelFor} />

      {/* ════════ REGISTERS — exports & imports ════════ */}
      {(hub.produces.length > 0 || hub.receives.length > 0) && (
        <>
          <Head>Registers of the road</Head>
          <div style={{ display: "flex", gap: 10 }}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={colHdr}>→ Exports ({hub.produces.length})</div>
              {hub.produces.length === 0 && <div style={emptyTxt}>nothing of note</div>}
              {hub.produces.slice(0, 12).map((p) => (
                <LedgerLine key={`p${p.good}`} icon={iconFor(p.good_name)} label={labelFor(p.good_name)}
                  right={`${p.price.toFixed(1)}×`} note={p.grade} rightColor={INKGREEN} />
              ))}
            </div>
            <div style={{ borderLeft: `1px solid ${RULE}` }} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={colHdr}>← Imports ({hub.receives.length})</div>
              {hub.receives.length === 0 && <div style={emptyTxt}>self-sufficient</div>}
              {hub.receives.slice(0, 14).map((r) => (
                <LedgerLine key={`r${r.chain}-${r.good}`} icon={iconFor(r.good_name)} label={labelFor(r.good_name)}
                  right={`${r.price.toFixed(1)}×`} note={hubName(r.from_hub).slice(0, 6)} rightColor={OXBLOOD} />
              ))}
            </div>
          </div>
        </>
      )}

      {/* ════════ HOLDINGS — estates, manufactories & buildings ════════ */}
      {detail && ((detail.estates_here?.length ?? 0) > 0 || (detail.structures?.length ?? 0) > 0) && (
        <>
          <Head sub="all routed through this seat">Schedule of holdings</Head>
          {[...(detail.estates_here ?? [])].sort((a, b) => b.output - a.output).map((e, i) => {
            const isManu = e.kind === 6;
            const ownerColor = e.owner_is_civic ? INKGREEN : INK;
            const ownerLabel = e.owner_is_civic ? "city-owned" : e.owner;
            return (
              <div key={i} style={{ display: "flex", alignItems: "baseline", gap: 6, fontSize: 10, padding: "2px 1px", borderBottom: `1px solid ${RULE_SOFT}` }}>
                <span style={{ alignSelf: "stretch", width: 3, background: isManu ? INKGREEN : GOLD }} />
                <span style={{ fontSize: 13 }}>{ESTATE_EMOJI[e.kind] ?? "🏡"}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: INK, fontWeight: 700 }}>
                    {ESTATE_LABEL[e.kind] ?? "Estate"} · {iconFor(e.good)} {labelFor(e.good)}
                    <span style={{ ...mono, color: GOLD, fontSize: 9, marginLeft: 5 }}>
                      {"★".repeat(e.tier ?? 1)}<span style={{ color: RULE_SOFT }}>{"★".repeat(Math.max(0, 5 - (e.tier ?? 1)))}</span>
                    </span>
                  </div>
                  <div style={{ color: INK_FAINT, fontSize: 9 }}>
                    owner: <span style={{ color: ownerColor }}>{ownerLabel}</span> · {isManu ? "MANUFACTORY" : "ESTATE"}
                    {(e.damage ?? 0) > 0.01 && (
                      <span style={{ color: OXBLOOD, fontWeight: 700, marginLeft: 5 }}>🔥 {Math.round((e.damage ?? 0) * 100)}% ruined</span>
                    )}
                  </div>
                </div>
                <span style={{ ...mono, color: INKGREEN, fontSize: 10 }}>▲ {fmt(e.output)}/d</span>
              </div>
            );
          })}
          {(detail.structures ?? []).map(([nm, eff], i) => (
            <div key={`s${i}`} style={{ display: "flex", gap: 6, alignItems: "baseline", fontSize: 10, padding: "2px 1px" }}>
              <span style={{ fontSize: 12 }}>{STRUCT_EMOJI[nm] ?? "🏗"}</span>
              <span style={{ color: INK, fontWeight: 700, minWidth: 72 }}>{nm}</span>
              <span style={{ flex: 1 }} />
              <span style={{ color: INKGREEN }}>{eff}</span>
            </div>
          ))}
        </>
      )}

      {/* ════════ GOVERNMENT (DLC 3 polis) ════════ */}
      {detail?.government && <GovernmentBlock g={detail.government} />}

      {/* ════════ THE POPULACE — mood, sentiment, society ════════ */}
      <Head>The populace</Head>
      {detail ? <MoodBlock detail={detail} /> : (
        <div style={emptyTxt}>Begin the campaign (Step 11) to hear how the people feel.</div>
      )}
      {detail?.society && (
        <>
          <div style={{ ...colHdr, marginTop: 8 }}>Estates of society</div>
          <SocietyBlock society={detail.society} />
        </>
      )}

      {/* Peoples & contentment */}
      {detail?.culture && (
        <>
          <Head sub="quarters & what they crave">Peoples of the seat</Head>
          <CultureRoster majority={detail.culture} minorities={(detail.minorities ?? []) as [string, number][]} moods={detail.culture_moods ?? []} />
        </>
      )}

      {/* ════════ WHO HOLDS THE TRADE — houses, guilds, locals ════════ */}
      {(() => {
        const hs = detail?.houses ?? [];
        const mlev = hub.merchant_level ?? 0.3;
        const houseVol = hs.reduce((s, h) => s + Math.max(0, h.volume ?? h.wealth), 0);
        const independent = houseVol * (0.25 + 0.7 * mlev) + 0.5;
        const guildVolume = independent * mlev;
        const localVolume = independent * (1 - mlev);
        const total = Math.max(1e-6, hs.reduce((s, h) => s + Math.max(0, h.wealth), 0));
        return (
          <>
            <Head sub="houses · guilds · free traders">Who holds the trade</Head>
            <HouseControl houses={hs} localVolume={localVolume} guildVolume={guildVolume} merchants={hub.merchants ?? 0} />
            {hs.filter((h) => (Math.max(0, h.wealth) / total) > 0.01).map((h, i) => (
              <div key={h.name + i} style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 4 }}>
                <CoatOfArms name={h.name} size={20} guild={h.is_guild} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
                    <span style={{ color: INK, fontSize: 11, fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.name}</span>
                    <span style={{ flex: 1 }} />
                    <span style={{ ...mono, color: GOLD, fontSize: 10 }}>{Math.round((Math.max(0, h.wealth) / total) * 100)}%</span>
                  </div>
                  <div style={{ height: 3, background: PARCH_LO, border: `1px solid ${RULE_SOFT}` }}>
                    <div style={{ width: `${(Math.max(0, h.wealth) / total) * 100}%`, height: "100%", background: GOLD }} />
                  </div>
                  {h.specialties.length > 0 && (
                    <div style={{ color: INK_FAINT, fontSize: 9 }}>{h.head_name} · {h.specialties.join(", ")}</div>
                  )}
                </div>
              </div>
            ))}
          </>
        );
      })()}

      {/* Shortages — why goods don't arrive */}
      {hub.shortages && hub.shortages.length > 0 && (
        <>
          <Head sub="why wares fail to arrive">Recorded shortages</Head>
          {hub.shortages.map((s) => (
            <div key={s.good} style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 10, padding: "1px 1px" }}>
              <span style={{ minWidth: 14 }}>{iconFor(s.good_name)}</span>
              <span style={{ color: OXBLOOD, minWidth: 64, fontWeight: 600 }}>{labelFor(s.good_name)}</span>
              <span style={{ flex: 1, color: INK_SOFT, fontStyle: "italic" }}>{shortageReason(s.reason)}</span>
              <span style={{ ...mono, color: OXBLOOD, fontSize: 9 }}>{Math.round(s.severity * 100)}% short</span>
            </div>
          ))}
        </>
      )}

      {/* Colonies of this city */}
      {detail?.related_colonies && detail.related_colonies.length > 0 && (
        <>
          <Head>Colonies of this seat ({detail.related_colonies.length})</Head>
          {detail.related_colonies.map((c) => (
            <div key={c.id} data-no-drag onClick={() => { setSelectedHub(c.id); setShowColonial(true); }}
              style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 1px", cursor: "pointer", borderBottom: `1px solid ${RULE_SOFT}` }}>
              <span style={{ width: 8, height: 8, background: c.colony_kind === 2 ? GOLD : "#6a4a86", flex: "0 0 auto" }} />
              <span style={{ flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", color: INK, fontSize: 11 }}>{c.name}</span>
              <span style={{ ...mono, color: INK_FAINT, fontSize: 9 }}>
                {c.colony_kind === 2 ? "outpost" : (["", "outpost", "colony", "town", "city"][c.colony_stage] || "colony")}
                {" · "}{c.population >= 1000 ? `${(c.population / 1000).toFixed(0)}k` : Math.round(c.population)}
              </span>
            </div>
          ))}
        </>
      )}

      {/* ════════ CHARACTER & SITE ════════ */}
      <div style={blurbBox}>{peopleSummary(hub, labelFor, topHub?.name, isTop)}</div>
      <SettlementStoryBox x={hub.x} y={hub.y} name={hub.name} stars={stars} />

      {/* ════════ THE BOUND CHRONICLE ════════ */}
      {detail && (
        <>
          <Head sub="click a year to unbind it">The bound chronicle</Head>
          <LedgerChronicle entries={detail.events.map((e) => ({ year: Math.floor(e.tick / 365), kind: e.kind, text: e.text }))} />
        </>
      )}

      {/* ════════ WEALTHIEST SEATS — quick navigation ════════ */}
      <Head>Roll of the wealthiest seats</Head>
      {wealthSorted.slice(0, 6).map((h, i) => (
        <div key={h.id} data-no-drag onClick={() => setSelectedHub(h.id)}
          style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, padding: "2px 1px", cursor: "pointer",
            background: h.id === hub.id ? PARCH_LO : "transparent", borderBottom: `1px solid ${RULE_SOFT}` }}>
          <span style={{ ...mono, color: INK_FAINT, minWidth: 22 }}>№{i + 1}</span>
          <span style={{ flex: 1, color: INK, fontWeight: h.id === hub.id ? 800 : 400, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {h.id === topHub?.id ? "◆ " : h.emporium ? "▲ " : ""}{h.name}
          </span>
          <span style={{ ...mono, color: INK_FAINT, fontSize: 9 }}>{h.population.toLocaleString()}</span>
          <span style={{ ...mono, color: INKGREEN, fontSize: 10, minWidth: 32, textAlign: "right" }}>{Math.round(h.wealth * 100)}%</span>
        </div>
      ))}
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   DYNAMIC CREST — the settlement's deterministic arms in a pixel-art frame.
   ══════════════════════════════════════════════════════════════════════════ */
function DossierCrest({ name }: { name: string }) {
  const S = 6; // pixel unit
  return (
    <div style={{ position: "relative", flex: "0 0 auto", padding: S, background: PARCH_HI }}>
      {/* Chunky stepped pixel frame drawn with layered box-shadows. */}
      <div style={{
        position: "relative", width: 48, height: 57, display: "flex", alignItems: "center", justifyContent: "center",
        boxShadow: `0 0 0 3px ${INK}, 0 0 0 6px ${PARCH}, 0 0 0 8px ${INK}`,
        background: PARCH_LO,
      }}>
        <CoatOfArms name={name} size={40} />
        {/* Corner pixels — the blocky studs of a stamped seal. */}
        {[[-8, -8], [-8, 0], [0, -8]].map(([dx, dy], i) => (
          <span key={`tl${i}`} style={{ position: "absolute", left: dx - 1, top: dy - 1, width: S, height: S, background: INK }} />
        ))}
        {[[8, -8], [8, 0], [0, -8]].map(([dx, dy], i) => (
          <span key={`tr${i}`} style={{ position: "absolute", right: dx - 9, top: dy - 1, width: S, height: S, background: INK }} />
        ))}
        {[[-8, 8], [-8, 0], [0, 8]].map(([dx, dy], i) => (
          <span key={`bl${i}`} style={{ position: "absolute", left: dx - 1, bottom: dy - 9, width: S, height: S, background: INK }} />
        ))}
        {[[8, 8], [8, 0], [0, 8]].map(([dx, dy], i) => (
          <span key={`br${i}`} style={{ position: "absolute", right: dx - 9, bottom: dy - 9, width: S, height: S, background: INK }} />
        ))}
      </div>
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   DYNAMIC TOWNSCAPE — a pixel-woodcut horizontal skyline. Its scale grows with
   the seat's rating; chimneys smoke when manufactories run; oxblood cracks split
   the walls when the city is besieged / at war.
   ══════════════════════════════════════════════════════════════════════════ */
function Townscape({ hub, detail, stars, manufacturing, atWar }: {
  hub: EconHub; detail: HubDetail | null; stars: number; manufacturing: boolean; atWar: boolean;
}) {
  const W = 168, H = 46, ground = 38;
  const coastal = hub.coastal || hub.sea_access !== false;

  const buildings = useMemo(() => {
    const r = mulberry32((hub.id + 1) * 40503);
    const n = 3 + stars * 2;                 // 5 … 13 structures
    const maxH = 10 + stars * 4;             // taller for greater seats
    const x0 = coastal ? 34 : 6;
    const span = W - x0 - 4;
    type B = { x: number; w: number; h: number; roof: number; chimney: boolean; wins: [number, number][] };
    const bs: B[] = [];
    let x = x0;
    for (let i = 0; i < n && x < W - 8; i++) {
      const w = 8 + Math.floor(r() * 8);
      const h = Math.round((0.35 + 0.65 * r()) * maxH);
      const roof = r() < 0.34 ? (r() < 0.5 ? 1 : 2) : 0; // 0 flat · 1 gable · 2 tower/spire
      const chimney = manufacturing && r() < 0.5;
      // Precompute pin-prick windows so they stay put across re-renders (each tick).
      const wins: [number, number][] = [];
      const top = ground - h;
      for (let wy = top + 3; wy < ground - 2; wy += 4)
        for (let wx = x + 2; wx < x + w - 1; wx += 4)
          if (r() < 0.55) wins.push([wx, wy]);
      bs.push({ x, w, h, roof, chimney, wins });
      x += w + 1 + Math.floor(r() * 3);
    }
    // Ensure the span is used; recentre if short.
    const used = x - x0;
    const shift = Math.max(0, (span - used) / 2);
    return bs.map((b) => ({ ...b, x: b.x + shift, wins: b.wins.map(([wx, wy]) => [wx + shift, wy] as [number, number]) }));
  }, [hub.id, stars, coastal, manufacturing]);

  const smokeCols = buildings.filter((b) => b.chimney).slice(0, 3);
  const crackTargets = atWar ? buildings.filter((_, i) => i % 2 === 0).slice(0, 3) : [];

  return (
    <svg width="100%" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="xMidYMid meet"
      style={{ display: "block", background: PARCH_HI, border: `1px solid ${RULE}`, shapeRendering: "crispEdges", imageRendering: "pixelated" }}>
      {/* Sky hatch — faint horizontal register lines. */}
      {[8, 14, 20].map((y) => <line key={y} x1={0} y1={y} x2={W} y2={y} stroke={RULE_SOFT} strokeWidth={0.4} opacity={0.5} />)}

      {/* Sea for a port seat. */}
      {coastal && (
        <g>
          <rect x={0} y={ground - 4} width={32} height={H - (ground - 4)} fill={PARCH_LO} />
          {[0, 3, 6].map((dy) => (
            <line key={dy} x1={2} y1={ground + 1 + dy} x2={30} y2={ground + 1 + dy} stroke={INK} strokeWidth={0.5} opacity={0.5} strokeDasharray="2 3" />
          ))}
          {/* a small hull + mast */}
          <rect x={10} y={ground - 4} width={12} height={3} fill={INK} />
          <rect x={15} y={ground - 11} width={1} height={7} fill={INK} />
          <path d={`M16 ${ground - 11} L22 ${ground - 6} L16 ${ground - 6} Z`} fill={INK} />
        </g>
      )}

      {/* Ground line. */}
      <line x1={0} y1={ground} x2={W} y2={ground} stroke={INK} strokeWidth={1} />

      {/* Buildings — ink-outlined woodcut blocks with pin-prick windows. */}
      {buildings.map((b, i) => {
        const top = ground - b.h;
        return (
          <g key={i}>
            <rect x={b.x} y={top} width={b.w} height={b.h} fill={PARCH_LO} stroke={INK} strokeWidth={0.7} />
            {b.roof === 1 && <path d={`M${b.x - 1} ${top} L${b.x + b.w / 2} ${top - 4} L${b.x + b.w + 1} ${top} Z`} fill={INK} />}
            {b.roof === 2 && (
              <g fill={INK}>
                <rect x={b.x + b.w / 2 - 2} y={top - 6} width={4} height={6} />
                <path d={`M${b.x + b.w / 2 - 3} ${top - 6} L${b.x + b.w / 2} ${top - 11} L${b.x + b.w / 2 + 3} ${top - 6} Z`} />
              </g>
            )}
            {b.wins.map(([wx, wy], k) => <rect key={k} x={wx} y={wy} width={1.4} height={1.6} fill={INK} opacity={0.75} />)}
          </g>
        );
      })}

      {/* Chimney smoke — stacked pixel puffs rising when manufactories run. */}
      {smokeCols.map((b, i) => {
        const cx = b.x + b.w - 3, ct = ground - b.h;
        return (
          <g key={`sm${i}`} fill={INK_SOFT}>
            <rect x={cx} y={ct - 3} width={2} height={4} fill={INK} />
            {[0, 1, 2, 3].map((k) => (
              <rect key={k} x={cx - 1 + (k % 2)} y={ct - 6 - k * 3} width={2} height={2} opacity={0.7 - k * 0.14} />
            ))}
          </g>
        );
      })}

      {/* Siege cracks — jagged oxblood fractures across the walls. */}
      {crackTargets.map((b, i) => {
        const cx = b.x + b.w * 0.5, top = ground - b.h;
        const pts = `${cx},${top} ${cx - 2},${top + b.h * 0.3} ${cx + 2},${top + b.h * 0.55} ${cx - 1},${ground}`;
        return <polyline key={`cr${i}`} points={pts} fill="none" stroke={OXBLOOD} strokeWidth={0.9} />;
      })}
      {atWar && (
        <text x={W - 3} y={9} textAnchor="end" fill={OXBLOOD} fontSize={6} style={{ ...mono, fontWeight: 700 }}>⚔ SIEGE</text>
      )}
      {detail && (
        <text x={3} y={H - 2} fill={INK_FAINT} fontSize={5.5} style={mono}>{detail.population.toLocaleString()} souls</text>
      )}
    </svg>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   T-ACCOUNT — the city treasury as a traditional double-entry ledger, split
   evenly down the middle: Expenditure (Dr.) | Revenue (Cr.).
   ══════════════════════════════════════════════════════════════════════════ */
function TAccount({ detail }: { detail: HubDetail }) {
  const f: CityFinance | null = detail.finance?.prev ?? detail.finance ?? null;
  const revenues: [string, number][] = f ? [
    ["Trade tariffs", f.tax_trade],
    ["Estate tax", f.tax_estate],
    ["Manufacturing tax", f.tax_manufacture],
    ["Wealth tax", f.tax_wealth],
    ["Seigniorage", f.seigniorage],
    ["War levy", f.war_levy],
    ["Reparations", f.reparations_in],
  ] : [];
  const expenses: [string, number][] = f ? [
    ["To the commons", f.spent_civic],
    ["War effort", f.spent_war],
    ["Public works", f.spent_works],
    ["Hospices", f.spent_health ?? 0],
    ["Reparations paid", f.reparations_out],
  ] : [];
  const revTot = revenues.reduce((s, [, v]) => s + Math.max(0, v), 0);
  const expTot = expenses.reduce((s, [, v]) => s + Math.max(0, v), 0);

  const Side = ({ rows, color, sign }: { rows: [string, number][]; color: string; sign: string }) => {
    const shown = rows.filter(([, v]) => v > 0.01);
    return (
      <div style={{ flex: 1, minWidth: 0, padding: "0 6px" }}>
        {shown.length === 0 && <div style={emptyTxt}>— nil —</div>}
        {shown.map(([label, v]) => (
          <div key={label} style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", fontSize: 9.5, padding: "1px 0" }}>
            <span style={{ color: INK_SOFT, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{label}</span>
            <span style={{ ...mono, color, marginLeft: 6 }}>{sign}{fmtN(v)}</span>
          </div>
        ))}
      </div>
    );
  };

  return (
    <>
      <Head sub={f ? `year ${f.year}` : undefined}>The treasury books</Head>
      {/* Balances line */}
      <div style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 10, flexWrap: "wrap", marginBottom: 4 }}>
        <span style={{ ...mono, color: INK, fontWeight: 800 }}>🏛 Treasury {fmtN(detail.treasury ?? 0)}</span>
        {detail.coin_name && (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 3, ...mono, color: GOLD }}
            title={`${detail.coin_name} · value ${(detail.coin_value ?? 0).toFixed(2)}×`}>
            <CoinIcon issuer={detail.name} value={detail.coin_value} size={14} />
            {detail.coin_name} {(detail.coin_value ?? 0).toFixed(2)}×
          </span>
        )}
        {detail.war_with && <span style={{ ...mono, color: OXBLOOD, fontWeight: 700 }}>⚔ war: {detail.war_with}</span>}
      </div>

      {f && (f.year > 0 || (detail.treasury ?? 0) > 0) ? (
        <div style={{ border: `1px solid ${RULE}`, background: PARCH_HI }}>
          {/* Column titles */}
          <div style={{ display: "flex", borderBottom: `1px solid ${RULE}` }}>
            <div style={{ ...tHead }}>Expenditure <span style={{ color: INK_FAINT }}>(Dr.)</span></div>
            <div style={{ width: 1, background: RULE }} />
            <div style={{ ...tHead }}>Revenue <span style={{ color: INK_FAINT }}>(Cr.)</span></div>
          </div>
          {/* Bodies straddling the central rule */}
          <div style={{ display: "flex", minHeight: 40 }}>
            <Side rows={expenses} color={OXBLOOD} sign="−" />
            <div style={{ width: 1, background: RULE }} />
            <Side rows={revenues} color={INKGREEN} sign="+" />
          </div>
          {/* Totals with the accountant's double rule */}
          <div style={{ display: "flex", borderTop: `1px solid ${RULE}` }}>
            <div style={{ ...tTot }}>
              <span style={{ color: INK_FAINT, textTransform: "uppercase", fontSize: 8, letterSpacing: 0.5 }}>Total</span>
              <span style={{ ...mono, color: OXBLOOD, fontWeight: 800 }}>−{fmtN(expTot)}</span>
            </div>
            <div style={{ width: 1, background: RULE }} />
            <div style={{ ...tTot }}>
              <span style={{ color: INK_FAINT, textTransform: "uppercase", fontSize: 8, letterSpacing: 0.5 }}>Total</span>
              <span style={{ ...mono, color: INKGREEN, fontWeight: 800 }}>+{fmtN(revTot)}</span>
            </div>
          </div>
          <div style={{ borderTop: `2px double ${RULE}`, textAlign: "center", ...mono, fontSize: 9, padding: "2px 0", color: revTot >= expTot ? INKGREEN : OXBLOOD }}>
            Balance carried {revTot >= expTot ? "+" : "−"}{fmtN(Math.abs(revTot - expTot))}
          </div>
        </div>
      ) : (
        <div style={emptyTxt}>No year has been posted to the books yet.</div>
      )}

      {(detail.public_health ?? 0) > 0.02 && (
        <div style={{ marginTop: 5, fontSize: 9.5, color: INKGREEN, display: "flex", alignItems: "center", gap: 6 }}>
          <span title="Hospices & quarantine spending — fewer die in a plague.">⚕ Public health</span>
          <div style={{ flex: 1, height: 5, background: PARCH_LO, border: `1px solid ${RULE_SOFT}`, maxWidth: 120 }}>
            <div style={{ width: `${Math.round(((detail.public_health ?? 0) / 0.6) * 100)}%`, height: "100%", background: INKGREEN }} />
          </div>
          <span style={{ ...mono, color: INK_SOFT }}>−{Math.round((detail.public_health ?? 0) * 100)}% deaths</span>
        </div>
      )}
    </>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   THE SCALES LEDGER — each ware's price is drawn as a tilting balance-scale:
   the beam dips toward Demand when a good is dear (short), toward Supply when
   it is a glut. Raw numbers give way to the illustrated weighing.
   ══════════════════════════════════════════════════════════════════════════ */
function ScalesLedger({ detail, hub, iconFor, labelFor }: {
  detail: HubDetail | null; hub: EconHub;
  iconFor: (id: string) => string; labelFor: (id: string) => string;
}) {
  // Build a uniform row model from live detail, else the worldgen market snapshot.
  type Row = { good: string; name: string; ratio: number; supply: number; demand: number; production: number };
  let rows: Row[] = [];
  if (detail) {
    rows = [...detail.goods]
      .filter((g: HubGoodDetail) => g.production > 0.01 || g.stock > 0.01 || g.need > 0.01)
      .map((g) => ({
        good: g.name, name: g.name,
        ratio: g.price / Math.max(1e-6, g.base_value),
        supply: Math.max(g.stock, g.production), demand: g.need, production: g.production,
      }))
      .sort((a, b) => (b.production + b.demand) - (a.production + a.demand))
      .slice(0, 14);
  } else if (hub.market) {
    rows = hub.market.prices.slice(0, 14).map((p) => ({
      good: p.good_name, name: p.good_name,
      ratio: p.price / Math.max(1e-6, p.base_value),
      supply: 1, demand: p.price / Math.max(1e-6, p.base_value), production: 0,
    }));
  }
  if (rows.length === 0) return <div style={emptyTxt}>No market is held at this seat.</div>;

  return (
    <div>
      <div style={{ display: "flex", ...mono, fontSize: 8, color: INK_FAINT, padding: "0 1px 2px", letterSpacing: 0.3 }}>
        <span style={{ flex: 1 }}>ware</span>
        <span style={{ width: 46, textAlign: "center" }}>supply ⇕ demand</span>
        <span style={{ width: 40, textAlign: "right" }}>× std</span>
      </div>
      {rows.map((r) => {
        const dear = r.ratio > 1.3, cheap = r.ratio < 0.77;
        const col = dear ? OXBLOOD : cheap ? INKGREEN : INK;
        const tag = dear ? "short" : cheap ? "glut" : "even";
        return (
          <div key={r.good} style={{ display: "flex", alignItems: "center", gap: 4, padding: "1px 1px", borderBottom: `1px solid ${RULE_SOFT}` }}>
            <span style={{ flex: 1, minWidth: 0, fontSize: 10, color: INK, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {iconFor(r.name)} {labelFor(r.name)}
              <span style={{ color: INK_FAINT, fontSize: 8 }}> · {tag}</span>
            </span>
            <BalanceScale ratio={r.ratio} color={col} />
            <span style={{ ...mono, width: 40, textAlign: "right", fontSize: 10, fontWeight: 700, color: col }}>
              {r.ratio.toFixed(2)}×
            </span>
          </div>
        );
      })}
      <div style={{ ...mono, fontSize: 8, color: INK_FAINT, marginTop: 3 }}>
        beam dips toward the heavier pan · <span style={{ color: OXBLOOD }}>demand</span> ⇢ dear · <span style={{ color: INKGREEN }}>supply</span> ⇢ glut
      </div>
    </div>
  );
}

/** A little balance-scale that tilts by the price ratio: >1 dear (demand pan down,
 *  right), <1 glut (supply pan down, left). Pure line-art on parchment. */
function BalanceScale({ ratio, color }: { ratio: number; color: string }) {
  const t = Math.max(-1, Math.min(1, Math.log(Math.max(1e-3, ratio)) / Math.log(3)));
  const ang = t * 20 * (Math.PI / 180);          // radians, +ve (dear) = demand pan sinks
  const W = 46, H = 22, cx = W / 2, beamY = 6, arm = 15;
  // Heavier pan sinks: dear ⇒ demand (right) drops; glut ⇒ supply (left) drops.
  const lx = cx - arm * Math.cos(ang), ly = beamY - arm * Math.sin(ang);
  const rx = cx + arm * Math.cos(ang), ry = beamY + arm * Math.sin(ang);
  const pan = (px: number, py: number) => (
    <g stroke={INK} strokeWidth={0.6} fill="none">
      <line x1={px} y1={py} x2={px} y2={py + 4} />
      <path d={`M${px - 4} ${py + 4} Q${px} ${py + 8} ${px + 4} ${py + 4}`} fill={PARCH_LO} />
    </g>
  );
  return (
    <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} style={{ flex: "0 0 auto", shapeRendering: "geometricPrecision" }}>
      {/* stand */}
      <line x1={cx} y1={beamY} x2={cx} y2={H - 3} stroke={INK} strokeWidth={0.8} />
      <path d={`M${cx - 4} ${H - 3} L${cx + 4} ${H - 3} L${cx + 2} ${H - 1} L${cx - 2} ${H - 1} Z`} fill={INK} />
      {/* beam */}
      <line x1={lx} y1={ly} x2={rx} y2={ry} stroke={color} strokeWidth={1.3} />
      <circle cx={cx} cy={beamY} r={1.3} fill={INK} />
      {pan(lx, ly)}
      {pan(rx, ry)}
    </svg>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   GOVERNMENT — condensed polis council + fiscal policy + speculation.
   ══════════════════════════════════════════════════════════════════════════ */
function GovernmentBlock({ g }: { g: NonNullable<HubDetail["government"]> }) {
  const pct = (x: number) => `${(x * 100).toFixed(1)}%`;
  const tierColor = g.spec_tier === "HIGH" ? OXBLOOD : g.spec_tier === "MED" ? GOLD : INKGREEN;
  const govRow = (label: string, val: string, warn = false) => (
    <div style={{ display: "flex", justifyContent: "space-between", padding: "1px 0", fontSize: 10.5 }}>
      <span style={{ color: INK_SOFT }}>{label}</span>
      <span style={{ ...mono, color: warn ? OXBLOOD : INK, fontWeight: 700 }}>{val}</span>
    </div>
  );
  return (
    <>
      <Head sub="the polis in council">Charter of government</Head>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 5 }}>
        {g.council !== "—" ? <CoatOfArms name={g.council} size={30} guild={g.council_is_guild} /> : <span style={{ fontSize: 22 }}>🏛</span>}
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ color: INK, fontWeight: 800 }}>{g.council}</div>
          <div style={{ color: INK_FAINT, fontSize: 9.5 }}>
            {g.council_archetype || (g.council === "—" ? "no governing house" : "")}{g.council_is_guild ? " · civic guild" : ""}
          </div>
        </div>
        {g.captor && <span style={{ ...mono, color: OXBLOOD, fontSize: 9, fontWeight: 700 }}>🔴 held by {g.captor}</span>}
      </div>
      {govRow("Regime", g.govt_type || "—")}
      {govRow("Next turnover", g.next_election_years <= 0 ? "imminent" : `in ${g.next_election_years}y`)}
      {govRow("Export tariff", pct(g.tariff_export))}
      {govRow("Import tariff", pct(g.tariff_import))}
      {govRow("Mint fineness", g.mint_fineness.toFixed(2), g.mint_fineness < 0.97)}
      {g.mint_fineness < 0.97 && <div style={{ color: OXBLOOD, fontSize: 9, marginTop: 1 }}>⚠ debased coin — "cheap money"</div>}
      {g.laws.length > 0 && (
        <>
          <div style={{ ...colHdr, marginTop: 6 }}>Laws &amp; decrees</div>
          {g.laws.slice(0, 6).map((l, i) => (
            <div key={i} style={{ fontSize: 10, color: INK_SOFT, margin: "1px 0" }}>
              <span style={{ ...mono, color: INK_FAINT }}>Y{l.year} </span>{l.text}
            </div>
          ))}
        </>
      )}
      <div style={{ ...colHdr, marginTop: 6 }}>Speculation</div>
      {g.spec_tier ? (
        <>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6, flexWrap: "wrap" }}>
            <span style={{ color: tierColor, fontWeight: 800 }}>{g.spec_tier}</span>
            <span style={{ ...mono, color: GOLD }}>{"●".repeat(g.spec_stars)}<span style={{ color: RULE_SOFT }}>{"○".repeat(Math.max(0, 5 - g.spec_stars))}</span></span>
            <span style={{ ...mono, color: INK_SOFT }}>({g.spec_risk.toFixed(2)})</span>
            <span style={{ color: INK_FAINT, fontStyle: "italic" }}>{g.spec_pattern}</span>
          </div>
          {g.spec_drivers.slice(0, 3).map((d, i) => <div key={i} style={{ color: INK_SOFT, fontSize: 9.5, marginLeft: 4 }}>• {d}</div>)}
        </>
      ) : <div style={{ color: INKGREEN, fontSize: 10 }}>Calm — no speculative pressure this year.</div>}
    </>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   THE POPULACE — mood card + society strata, ledger-toned.
   ══════════════════════════════════════════════════════════════════════════ */
function MoodBlock({ detail }: { detail: HubDetail }) {
  const mood = detail.mood;
  const face = mood > 0.75 ? "😄 Joyful" : mood > 0.58 ? "🙂 Content" : mood > 0.42 ? "😐 Uneasy"
    : mood > 0.25 ? "😟 Discontent" : "😠 Rebellious";
  const moodColor = mood > 0.58 ? INKGREEN : mood > 0.42 ? GOLD : OXBLOOD;
  const stab = detail.sent_stability;
  const stabNote = stab < 0.5 ? " (recent disasters)" : "";
  return (
    <div style={{ margin: "2px 0 4px" }}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 3 }}>
        <span style={{ color: moodColor, fontSize: 12, fontWeight: 800 }}>{face}</span>
        <span style={{ ...mono, color: INK_FAINT, fontSize: 10 }}>{Math.round(mood * 100)}%</span>
        {detail.starving > 0.4 && <span style={{ color: OXBLOOD, fontSize: 9 }}>· starving</span>}
      </div>
      <DriverBar label="Food" frac={detail.sent_food} color={INKGREEN} />
      <DriverBar label="Prosperity" frac={detail.sent_prosperity} color={GOLD} />
      <DriverBar label={`Stability${stabNote}`} frac={detail.sent_stability} color={INK_SOFT} />
    </div>
  );
}

const STRATA = [
  { key: "patrician", label: "Patrician", color: GOLD },
  { key: "burgher", label: "Burgher", color: "#a5622a" },
  { key: "commoner", label: "Commoner", color: INK_SOFT },
  { key: "underclass", label: "Underclass", color: "#8f8264" },
] as const;

function SocietyBlock({ society }: { society: SocietyBrief }) {
  const ineqLabel = society.inequality > 0.66 ? "extreme" : society.inequality > 0.4 ? "marked" : "moderate";
  const ineqColor = society.inequality > 0.66 ? OXBLOOD : society.inequality > 0.4 ? GOLD : INKGREEN;
  const welColor = society.welfare > 0.5 ? INKGREEN : society.welfare > 0.28 ? GOLD : OXBLOOD;
  const unrest = society.unrest ?? 0;
  const unrestLabel = unrest >= 0.82 ? "revolt" : unrest >= 0.6 ? "rioting" : unrest >= 0.35 ? "restless" : "calm";
  const unrestColor = unrest >= 0.6 ? OXBLOOD : unrest >= 0.35 ? GOLD : INKGREEN;
  return (
    <div style={{ margin: "2px 0 4px" }}>
      <div style={{ display: "flex", height: 12, border: `1px solid ${RULE}`, overflow: "hidden" }}>
        {STRATA.map((s) => {
          const v = (society as unknown as Record<string, number>)[s.key];
          return v > 0.001 ? <div key={s.key} title={`${s.label} ${Math.round(v * 100)}%`} style={{ width: `${v * 100}%`, background: s.color }} /> : null;
        })}
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: "1px 10px", marginTop: 3 }}>
        {STRATA.map((s) => {
          const v = (society as unknown as Record<string, number>)[s.key];
          return (
            <span key={s.key} style={{ ...mono, fontSize: 8.5, color: INK_SOFT, display: "flex", alignItems: "center", gap: 3 }}>
              <span style={{ width: 8, height: 8, background: s.color, display: "inline-block" }} />
              {s.label} {Math.round(v * 100)}%
            </span>
          );
        })}
      </div>
      <div style={{ marginTop: 4 }}>
        <DriverBar label={`Inequality (${ineqLabel})`} frac={society.inequality} color={ineqColor} />
        <DriverBar label="Commoner welfare" frac={society.welfare} color={welColor} />
        <DriverBar label={`Unrest (${unrestLabel})`} frac={unrest} color={unrestColor} />
      </div>
    </div>
  );
}

function DriverBar({ label, frac, color }: { label: string; frac: number; color: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6, margin: "1px 0" }}>
      <span style={{ color: INK_FAINT, fontSize: 9, minWidth: 108 }}>{label}</span>
      <div style={{ flex: 1, height: 5, background: PARCH_LO, border: `1px solid ${RULE_SOFT}` }}>
        <div style={{ height: "100%", width: `${Math.max(2, Math.min(100, frac * 100))}%`, background: color }} />
      </div>
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   PEOPLES — majority + minority quarters, and per-people contentment.
   ══════════════════════════════════════════════════════════════════════════ */
function CultureRoster({ majority, minorities, moods }: {
  majority: string; minorities: [string, number][]; moods: CultureMood[];
}) {
  const mins = minorities.filter(([, s]) => s > 0.005);
  const majShare = Math.max(0, 1 - mins.reduce((s, [, v]) => s + v, 0));
  return (
    <div>
      <div style={{ display: "flex", height: 12, border: `1px solid ${RULE}`, overflow: "hidden" }}>
        <div title={`${majority} ${Math.round(majShare * 100)}%`} style={{ width: `${majShare * 100}%`, background: INK_SOFT }} />
        {mins.map(([nm, sh], i) => (
          <div key={nm} title={`${nm} ${Math.round(sh * 100)}%`} style={{ width: `${sh * 100}%`, background: i % 2 ? GOLD : "#7a6a4a" }} />
        ))}
      </div>
      <div style={{ ...mono, fontSize: 9, color: INK_SOFT, marginTop: 2 }}>
        {majority} <span style={{ color: INK_FAINT }}>{Math.round(majShare * 100)}%</span>
        {mins.slice(0, 4).map(([nm, sh]) => <span key={nm}> · {nm} <span style={{ color: INK_FAINT }}>{Math.round(sh * 100)}%</span></span>)}
      </div>
      {moods.length > 0 && (
        <>
          <div style={{ ...colHdr, marginTop: 6 }}>Contentment — is what they crave supplied?</div>
          {moods.map((m) => {
            const s = m.satisfaction;
            const face = s >= 0.66 ? "😀" : s >= 0.5 ? "🙂" : s >= 0.38 ? "😐" : "☹️";
            const fc = s >= 0.5 ? INKGREEN : s >= 0.38 ? GOLD : OXBLOOD;
            return (
              <div key={m.name} style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 1px", fontSize: 10.5 }}>
                <span style={{ width: 8, height: 8, background: `rgb(${m.color[0]},${m.color[1]},${m.color[2]})`, flex: "0 0 auto", border: `1px solid ${RULE}` }} />
                <span style={{ width: 80, color: INK, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{m.name}</span>
                <span style={{ fontSize: 12 }} title={`${Math.round(s * 100)}% supplied`}>{face}</span>
                <span style={{ flex: 1, minWidth: 0, textAlign: "right", color: fc, fontSize: 9.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {m.unmet.length > 0
                    ? <>craves {m.unmet.map((g) => HP_GOOD_EMOJI[g] ?? g).join(" ")}</>
                    : <>content {m.met.slice(0, 3).map((g) => HP_GOOD_EMOJI[g] ?? "").join(" ")}</>}
                </span>
              </div>
            );
          })}
        </>
      )}
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   WHO HOLDS THE TRADE — a ledger pie of house vs guild vs local shares.
   ══════════════════════════════════════════════════════════════════════════ */
function HouseControl({ houses, localVolume, guildVolume, merchants }:
  { houses: HouseBrief[]; localVolume: number; guildVolume: number; merchants: number }) {
  const R = 28, r = 15, cx = 32, cy = 32;
  const volTotal = houses.reduce((s, h) => s + Math.max(0, h.volume ?? 0), 0);
  const useVol = volTotal > 1e-4;
  const houseVal = (h: HouseBrief) => useVol ? Math.max(0, h.volume ?? 0) : Math.max(0, h.wealth);
  const raw: { name: string; value: number; color: string }[] = houses
    .map((h) => ({ name: h.name, value: houseVal(h), color: h.color ?? houseColor(h.name) }));
  raw.push({ name: "Merchant guilds", value: Math.max(0, guildVolume), color: GUILD_COLOR });
  raw.push({ name: "Free traders", value: Math.max(0, localVolume), color: LOCAL_COLOR });
  const total = Math.max(1e-6, raw.reduce((s, x) => s + x.value, 0));
  const slices = raw.map((x) => ({ ...x, frac: x.value / total })).filter((s) => s.frac > 0.004).sort((a, b) => b.frac - a.frac);
  let a0 = -Math.PI / 2;
  const arc = (frac: number) => {
    const a1 = a0 + Math.min(frac, 0.9999) * Math.PI * 2;
    const large = frac > 0.5 ? 1 : 0;
    const p = (rad: number, ang: number) => `${cx + rad * Math.cos(ang)},${cy + rad * Math.sin(ang)}`;
    const d = `M ${p(R, a0)} A ${R} ${R} 0 ${large} 1 ${p(R, a1)} L ${p(r, a1)} A ${r} ${r} 0 ${large} 0 ${p(r, a0)} Z`;
    a0 = a1;
    return d;
  };
  const top = slices[0];
  const nHouses = houses.length;
  return (
    <div style={{ display: "flex", gap: 10, alignItems: "center", margin: "2px 0 6px" }}>
      <svg width={64} height={64} viewBox="0 0 64 64" style={{ flex: "0 0 auto" }}>
        {slices.map((s) => (
          <path key={s.name} d={arc(s.frac)} fill={s.color} stroke={PARCH} strokeWidth={0.8}>
            <title>{`${s.name}: ${Math.round(s.frac * 100)}%`}</title>
          </path>
        ))}
        {slices.length === 0 && <circle cx={cx} cy={cy} r={R} fill={PARCH_LO} />}
      </svg>
      <div style={{ fontSize: 10, color: INK_SOFT, lineHeight: 1.5 }}>
        <div style={{ ...mono, color: INK, fontWeight: 700 }}>{nHouses} {nHouses === 1 ? "house" : "houses"} · {Math.round(merchants)} merchants</div>
        {top && <div>Leads: <span style={{ color: top.color, fontWeight: 700 }}>{top.name}</span> ({Math.round(top.frac * 100)}%)</div>}
        <div style={{ color: INK_FAINT }}>by trade volume moved</div>
      </div>
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   THE BOUND CHRONICLE — a parchment year-grouped chronicle (self-contained so
   the shared dark YearChronicle stays untouched).
   ══════════════════════════════════════════════════════════════════════════ */
function LedgerChronicle({ entries }: { entries: { year: number; kind: string; text: string }[] }) {
  const byYear = new Map<number, { kind: string; text: string }[]>();
  for (const e of entries) { const a = byYear.get(e.year); if (a) a.push(e); else byYear.set(e.year, [e]); }
  const years = [...byYear.keys()].sort((a, b) => b - a);
  const [open, setOpen] = useState<Set<number>>(() => new Set(years.slice(0, 1)));
  if (years.length === 0) return <div style={emptyTxt}>No notable events yet.</div>;
  const toggle = (y: number) => setOpen((prev) => { const n = new Set(prev); n.has(y) ? n.delete(y) : n.add(y); return n; });
  return (
    <div>
      {years.map((y) => {
        const evs = byYear.get(y)!;
        const isOpen = open.has(y);
        const glance = [...new Set(evs.map((e) => HUB_EVENT_ICON[e.kind] ?? "•"))].slice(0, 6).join(" ");
        return (
          <div key={y} style={{ borderBottom: `1px solid ${RULE_SOFT}` }}>
            <div onClick={() => toggle(y)} style={{ display: "flex", alignItems: "center", gap: 7, cursor: "pointer", padding: "3px 1px", userSelect: "none" }}>
              <span style={{ color: INK_FAINT, fontSize: 9, width: 9, textAlign: "center" }}>{isOpen ? "▾" : "▸"}</span>
              <span style={{ ...mono, color: INK, fontWeight: 800, fontSize: 11, minWidth: 56 }}>Anno {y}</span>
              <span style={{ color: INK_FAINT, fontSize: 9 }}>{evs.length} entr{evs.length === 1 ? "y" : "ies"}</span>
              <span style={{ flex: 1 }} />
              {!isOpen && <span style={{ fontSize: 11, letterSpacing: 1 }}>{glance}</span>}
            </div>
            {isOpen && (
              <div style={{ position: "relative", paddingLeft: 16, paddingBottom: 5 }}>
                <div style={{ position: "absolute", left: 5, top: 2, bottom: 4, width: 1, background: RULE }} />
                {evs.map((e, i) => (
                  <div key={i} style={{ position: "relative", marginBottom: 4 }}>
                    <span style={{ position: "absolute", left: -13, top: 0, fontSize: 11 }}>{HUB_EVENT_ICON[e.kind] ?? "•"}</span>
                    <div style={{ color: INK_SOFT, fontSize: 10.5, lineHeight: 1.3 }}>{e.text}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/* ══════════════════════════════════════════════════════════════════════════
   SITE & STORY — the seat's account of the river/reach/lake it stands on.
   ══════════════════════════════════════════════════════════════════════════ */
function SettlementStoryBox({ x, y, name, stars }: { x: number; y: number; name: string; stars: number }) {
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const lakes = useWorldStore((s) => s.lakes);
  const toponyms = useWorldStore((s) => s.toponyms);
  const meta = useWorldStore((s) => s.meta);
  const worldW = meta?.grid_width ?? 0;
  const story = useMemo(() => {
    const size: Settlement["size"] = stars >= 5 ? "capital" : stars >= 4 ? "city" : stars >= 3 ? "town" : stars >= 2 ? "village" : "outpost";
    const found = settlements.find((s) => s.x === x && s.y === y);
    const st: Settlement = found ?? { id: "", x, y, name, size, population: 0, score: 0 };
    return settlementStory(st, rivers, toponyms, lakes, worldW);
  }, [x, y, name, stars, settlements, rivers, toponyms, lakes, worldW]);
  if (!story.text) return null;
  return (
    <>
      <Head>Site &amp; story</Head>
      <div style={{ ...blurbBox, fontStyle: "italic" }}>{story.text}</div>
    </>
  );
}

function peopleSummary(hub: EconHub, labelFor: (id: string) => string, topName?: string, isTop?: boolean): string {
  const known = hub.monopolies && hub.monopolies.length > 0
    ? hub.monopolies.map(labelFor)
    : hub.produces.slice(0, 2).map((p) => labelFor(p.good_name));
  const wants = hub.receives.slice(0, 2).map((r) => labelFor(r.good_name));
  const { clim, analogue } = climatePhrase(hub.koppen ?? 0, hub.elevation ?? 0, !!hub.coastal);
  const society = hub.emporium ? "a cosmopolitan merchant republic, its quays thronged with foreign tongues"
    : hub.coastal && hub.wealth > 0.4 ? "a proud guild port of shipwrights and factors"
    : hub.wealth > 0.6 ? "a prosperous burgher city"
    : (hub.koppen === 21 || hub.koppen === 16 || hub.koppen === 17) ? "a hardy frontier community"
    : hub.wealth > 0.3 ? "an industrious market town" : "a modest farming community";
  const founders = [
    "founded where an old road forded the river", "grown from a sheltered anchorage",
    "raised around a hill-fort and its market", "settled by traders drawn to its ore and springs",
    "begun as a temple town and pilgrim halt", "planted as a colony at the edge of the known world",
  ];
  const founding = founders[hub.id % founders.length];
  const eliteLvl = hub.elite_level ?? 0, merchLvl = hub.merchant_level ?? 0;
  const eliteWord = eliteLvl > 0.6 ? "a broad and gilded patrician class" : eliteLvl > 0.3 ? "a comfortable upper class" : "few of great wealth";
  const merchWord = merchLvl > 0.6 ? "a teeming merchant quarter" : merchLvl > 0.3 ? "an active body of traders" : "only a handful of traders";
  let s = `${clim.charAt(0).toUpperCase()}${clim.slice(1)} of roughly ${hub.population.toLocaleString()} souls — ${society}, ${founding}.`;
  s += ` Its climate recalls ${analogue}.`;
  s += ` Home to ${eliteWord} and ${merchWord}`;
  if (hub.top_export) s += `, who grow richest on ${labelFor(hub.top_export)}`;
  s += ".";
  if (known.length > 0) s += ` Renowned for its ${known.join(" and ")}.`;
  if (wants.length > 0) s += ` Its merchants hunger for ${wants.join(" and ")} from afar.`;
  if (isTop && hub.nearest_ref) s += ` The world's pre-eminent entrepôt — a rival to ${hub.nearest_ref} of old.`;
  else if (topName) s += ` In trade it looks up to ${topName}, the realm's greatest market.`;
  return s;
}

/* ── Small pieces ──────────────────────────────────────────────────────────*/
function StatCell({ label, value }: { label: string; value: string }) {
  return (
    <div style={statTile}>
      <div style={{ ...mono, color: INK, fontSize: 13, fontWeight: 800 }}>{value}</div>
      <div style={{ color: INK_FAINT, fontSize: 8, textTransform: "uppercase", letterSpacing: 0.4 }}>{label}</div>
    </div>
  );
}

function LedgerLine({ icon, label, right, note, rightColor }: {
  icon: string; label: string; right: string; note?: string; rightColor: string;
}) {
  return (
    <div style={{ display: "flex", alignItems: "baseline", gap: 4, fontSize: 10, padding: "1px 1px", borderBottom: `1px dotted ${RULE_SOFT}` }}>
      <span style={{ minWidth: 14 }}>{icon}</span>
      <span style={{ flex: 1, color: INK, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{label}</span>
      {note && <span style={{ ...mono, color: INK_FAINT, fontSize: 8 }}>{note}</span>}
      <span style={{ ...mono, color: rightColor, fontSize: 10, minWidth: 30, textAlign: "right" }}>{right}</span>
    </div>
  );
}

/* ── Styles ────────────────────────────────────────────────────────────────*/
const panel: React.CSSProperties = {
  position: "absolute", top: 12, right: 12, width: 384, maxHeight: "90vh", overflowY: "auto",
  color: INK, border: `2px solid ${INK}`, borderRadius: 2,
  padding: "12px 14px", zIndex: 110, boxShadow: "0 10px 34px rgba(20,24,32,0.45)",
  fontFamily: "Georgia, 'Times New Roman', serif",
};
const statGrid: React.CSSProperties = { display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 4, marginTop: 2 };
const statTile: React.CSSProperties = {
  background: PARCH_HI, border: `1px solid ${RULE}`, padding: "3px 5px", textAlign: "center",
};
const blurbBox: React.CSSProperties = {
  marginTop: 8, padding: "6px 9px", background: PARCH_HI, borderLeft: `3px solid ${INK}`,
  color: INK_SOFT, fontSize: 10.5, fontStyle: "italic", lineHeight: 1.45,
};
const colHdr: React.CSSProperties = {
  color: INK_FAINT, fontSize: 9, fontWeight: 700, textTransform: "uppercase",
  letterSpacing: 0.6, borderBottom: `1px solid ${RULE_SOFT}`, paddingBottom: 2, marginBottom: 3,
};
const estateBox: React.CSSProperties = {
  margin: "6px 0 3px", padding: "5px 8px", background: PARCH_HI, border: `1px solid ${GOLD}`,
};
const emptyTxt: React.CSSProperties = { color: INK_FAINT, fontSize: 10, fontStyle: "italic", padding: "2px 3px" };
const tHead: React.CSSProperties = {
  flex: 1, textAlign: "center", fontSize: 9, fontWeight: 800, letterSpacing: 0.5,
  textTransform: "uppercase", color: INK, padding: "2px 0", background: PARCH_LO,
};
const tTot: React.CSSProperties = {
  flex: 1, display: "flex", justifyContent: "space-between", alignItems: "baseline",
  padding: "2px 6px", background: PARCH_LO,
};
