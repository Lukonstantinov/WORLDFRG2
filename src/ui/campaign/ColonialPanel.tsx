import { useEffect, useMemo, useState } from "react";
import { useCampaignStore } from "@state/campaignStore";
import { useUIStore } from "@state/uiStore";
import { campaignGetColonies, campaignColonyGates, campaignGetColony, campaignGetExpeditions, campaignGetCorridors } from "@bridge";
import type { ColonySummary, ColonyGateStatus, ColonyDetail, ExpeditionView, TradeCorridor } from "@types";
import { useFloatingWindow } from "@ui/world/useFloatingWindow";

// Colours mirror OverlayManager.ts (settlementColony / houseOutpost) + the app theme.
const COLONY_VIOLET = "#c08cff";
const OUTPOST_TAN = "#c9a96a";
const OK = "#4fc06a";
const WARN = "#e0503a";
const GOLD = "#e0c060";
const COLONIAL_TINT = "rgba(22,16,30,0.97)"; // plum-violet, distinct from Finance

const STAGE = ["", "Outpost", "Colony", "Town", "City"];

function reserveFrac(c: ColonySummary): number {
  return c.reserve_cap > 0 ? Math.min(1, c.reserve_food / c.reserve_cap) : 1;
}
function reserveColor(f: number): string {
  return f >= 0.5 ? OK : f >= 0.25 ? GOLD : WARN;
}
function fmtPop(p: number): string {
  if (p >= 1000) return `${(p / 1000).toFixed(p >= 10000 ? 0 : 1)}k`;
  return `${Math.round(p)}`;
}

/** Colonial Office — an empire-wide roster of settlement colonies + house trade
 *  outposts grouped by metropolis, with a per-colony detail (joint-stock backers +
 *  supply roster) and, when none exist yet, a read-only diagnosis of which founding
 *  gate is blocking. Reads straight from the live campaign sim; changes nothing. */
export function ColonialPanel() {
  const open = useUIStore((s) => s.showColonial);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const selectedHub = useUIStore((s) => s.selectedHub);
  const setColonyHighlight = useUIStore((s) => s.setColonyHighlight);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [colonies, setColonies] = useState<ColonySummary[]>([]);
  const [gates, setGates] = useState<ColonyGateStatus | null>(null);
  const [sel, setSel] = useState<number | null>(null);
  const [detail, setDetail] = useState<ColonyDetail | null>(null);
  const [expeditions, setExpeditions] = useState<ExpeditionView[]>([]);
  const [failedCount, setFailedCount] = useState(0);
  const [corridors, setCorridors] = useState<TradeCorridor[]>([]);

  // Closing the panel clears the map's colony↔metropolis highlight line.
  useEffect(() => { if (!open) setColonyHighlight(null); }, [open, setColonyHighlight]);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetColonies().then(setColonies).catch(() => setColonies([]));
    campaignColonyGates().then(setGates).catch(() => setGates(null));
    campaignGetExpeditions().then((p) => { setExpeditions(p.active); setFailedCount(p.failed.length); })
      .catch(() => { setExpeditions([]); setFailedCount(0); });
    // Established corridors (event-driven); permissive reach for the roster list.
    campaignGetCorridors([], 0, 1.0).then(setCorridors).catch(() => setCorridors([]));
  }, [open, active, tick]);

  // A click on a colony ON THE MAP selects that hub globally — mirror it into the panel
  // so its detail opens (only when the selected hub is actually a colony we list).
  useEffect(() => {
    if (selectedHub != null && colonies.some((c) => c.id === selectedHub)) setSel(selectedHub);
  }, [selectedHub, colonies]);

  useEffect(() => {
    if (sel == null) { setDetail(null); return; }
    campaignGetColony(sel).then(setDetail).catch(() => setDetail(null));
  }, [sel, tick]);

  const groups = useMemo(() => {
    const m = new Map<number, { founder: string; rows: ColonySummary[] }>();
    for (const c of colonies) {
      if (!m.has(c.founder_hub)) m.set(c.founder_hub, { founder: c.founder_name || "—", rows: [] });
      m.get(c.founder_hub)!.rows.push(c);
    }
    return [...m.values()].sort((a, b) => b.rows.length - a.rows.length);
  }, [colonies]);

  const stats = useMemo(() => {
    const settle = colonies.filter((c) => c.colony_kind === 1);
    const outposts = colonies.filter((c) => c.colony_kind === 2);
    const autonomous = settle.filter((c) => c.autonomous).length;
    const stressed = settle.filter((c) => reserveFrac(c) < 0.4 && !c.autonomous).length;
    const metros = new Set(settle.map((c) => c.founder_hub)).size;
    return { settle: settle.length, outposts: outposts.length, autonomous, stressed, metros };
  }, [colonies]);

  const { rootStyle, onPointerDown } = useFloatingWindow(COLONIAL_TINT);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowColonial(false);

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }} onPointerDown={onPointerDown}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span>🏛 Colonial Office & Expeditions</span>
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8" }} onClick={close}>✕</span>
      </div>

      {!active && (
        <div style={empty}>Begin the campaign (Step 11) and let it run — colonies are founded once
          the world matures (from year 30) and a bank backs the venture.</div>
      )}

      {active && (
        <>
          {/* stat strip */}
          <div style={statRow}>
            <Stat n={stats.settle} l="colonies" />
            <Stat n={stats.outposts} l="outposts" />
            <Stat n={stats.autonomous} l="autonomous" />
            <Stat n={stats.stressed} l="⚠ food-short" warn={stats.stressed > 0} />
            <Stat n={stats.metros} l="metropoleis" />
          </div>

          <div style={scroll}>
            <ExpeditionsSection expeditions={expeditions} failedCount={failedCount} corridors={corridors}
              onFocus={(x, y) => { setColonyHighlight({ ax: x, ay: y, bx: x, by: y }); }} />

            {colonies.length === 0 && <GateCard gates={gates} />}

            {groups.map((g, gi) => (
              <div key={gi} style={{ marginBottom: 10 }}>
                <div style={metroHead}>
                  <span style={{ width: 8, height: 8, borderRadius: "50%", background: "#ffd700", display: "inline-block" }} />
                  {g.founder} <span style={{ color: "#5a7290" }}>— {g.rows.length}</span>
                </div>
                {g.rows.map((c) => (
                  <ColonyRow key={c.id} c={c} selected={sel === c.id}
                    onClick={() => {
                      setSel(c.id);
                      setSelectedHub(c.id);
                      // Shine the metropolis, this colony/satellite, and the tie between.
                      setColonyHighlight(c.founder_x != null && c.founder_x >= 0
                        ? { ax: c.founder_x, ay: c.founder_y ?? 0, bx: c.x, by: c.y } : null);
                    }} />
                ))}
              </div>
            ))}

            {sel != null && detail && <DetailCard detail={detail} colony={colonies.find((c) => c.id === sel)} />}
          </div>
        </>
      )}
    </div>
  );
}

const HAZARD_LABEL = ["fever", "the climate", "a native raid", "a storm", "shipwreck", "starvation", "bandits"];

/** Expeditions & Corridors — the ventures crawling toward distant lands (financed,
 *  hazard-culled, some failing), and the corridors they've proven. Merged into the
 *  Colonial Office because a corridor is how a remote settlement network is born. */
function ExpeditionsSection({ expeditions, failedCount, corridors, onFocus }: {
  expeditions: ExpeditionView[]; failedCount: number; corridors: TradeCorridor[];
  onFocus: (x: number, y: number) => void;
}) {
  if (expeditions.length === 0 && corridors.length === 0 && failedCount === 0) {
    return (
      <div style={{ ...card, marginTop: 0 }}>
        <div style={{ fontWeight: 700, color: "#dce8f6", marginBottom: 2 }}>⛵ Expeditions & Corridors</div>
        <div style={{ fontSize: 10.5, color: "#7a90a8", lineHeight: 1.5 }}>
          From year 15, wealthy houses fund ventures toward distant, unconnected cities. Most early
          attempts are lost to fever, storms or native raids; only after several proven round-trips does
          a lasting corridor open — founding port &amp; caravanserai villages along it. Turn on the
          <b style={{ color: "#cfe0f4" }}> ⛵ Expeditions</b> overlay to watch them travel.
        </div>
      </div>
    );
  }
  return (
    <div style={{ ...card, marginTop: 0 }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
        <span style={{ fontWeight: 700, color: "#dce8f6" }}>⛵ Expeditions &amp; Corridors</span>
        <span style={{ fontSize: 9.5, color: "#7a90a8" }}>
          {expeditions.length} en route · {corridors.length} corridors · {failedCount} lost
        </span>
      </div>

      {expeditions.map((e) => {
        const col = e.outbound ? "#e0b24a" : "#5fc8a8";
        const survived = Math.round(e.survived * 100);
        const fleet = e.ships > e.caravans ? `${e.ships} ships` : `${e.caravans} caravans`;
        const last = e.hazards[0];
        return (
          <div key={e.id} style={{ ...expRow }} onClick={() => onFocus(e.x, e.y)}>
            <span style={{ width: 9, height: 9, borderRadius: e.ships > e.caravans ? 2 : "50%", background: col, flex: "0 0 auto", marginTop: 3 }} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: 11, color: "#dce8f6", fontWeight: 600, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                {e.origin} → {e.dest}
              </div>
              <div style={{ fontSize: 9.5, color: "#8aa6c0" }}>
                {e.leader} · {fleet} · {e.good || "goods"} · {e.outbound ? "outbound" : "returning"}
              </div>
              <div style={{ ...bar, marginTop: 3 }}>
                <i style={{ width: `${Math.round(e.progress * 100)}%`, background: col, display: "block", height: "100%", borderRadius: 5 }} />
              </div>
              <div style={{ fontSize: 9, color: survived < 50 ? WARN : "#6a86a6", marginTop: 1 }}>
                {survived}% of the fleet survives{last ? ` · last struck by ${HAZARD_LABEL[last[2]] ?? "peril"}` : ""}
              </div>
            </div>
          </div>
        );
      })}

      {corridors.length > 0 && (
        <>
          <div style={{ ...sec, marginTop: 8 }}>Established corridors</div>
          {corridors.map((c, i) => {
            const ports = c.waystations.filter((w) => w.kind === 3).length;
            const cara = c.waystations.filter((w) => w.kind === 2).length;
            return (
              <div key={i} style={{ display: "flex", gap: 7, padding: "4px 4px", alignItems: "flex-start" }}>
                <span style={{ width: 8, height: 8, borderRadius: 2, background: c.color, flex: "0 0 auto", marginTop: 3 }} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 10.5, color: "#cfe0f4", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                    {c.origin} ↔ {c.dest}
                  </div>
                  <div style={{ fontSize: 9.5, color: "#7a90a8" }}>
                    {c.owner} · {c.good || "trade"} · {Math.round(c.km).toLocaleString()} km
                    {ports > 0 && ` · ⚓${ports}`}{cara > 0 && ` · 🐫${cara}`}
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

function Stat({ n, l, warn }: { n: number; l: string; warn?: boolean }) {
  return (
    <div style={statBox}>
      <div style={{ fontSize: 16, fontWeight: 700, color: warn && n > 0 ? WARN : "#eaf3ff" }}>{n}</div>
      <div style={{ fontSize: 9.5, color: "#6a86a6" }}>{l}</div>
    </div>
  );
}

function ColonyRow({ c, selected, onClick }: { c: ColonySummary; selected: boolean; onClick: () => void }) {
  const outpost = c.colony_kind === 2;
  const satellite = c.colony_kind === 3;
  const settlementColony = c.colony_kind === 1;
  const frac = reserveFrac(c);
  const stage = c.autonomous ? "Autonomous" : outpost ? "Outpost" : satellite ? "Satellite" : (STAGE[c.colony_stage] || "Colony");
  return (
    <div onClick={onClick} style={{ ...rowS, ...(selected ? rowSel : {}) }}>
      <span style={{
        width: 10, height: 10, borderRadius: outpost ? 2 : "50%", flex: "0 0 auto", marginTop: 3,
        background: outpost ? "#0a0a0a" : satellite ? "#e0503a" : COLONY_VIOLET,
        border: outpost ? `2px solid ${c.owner_color || OUTPOST_TAN}` : "none",
      }} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontWeight: 600, color: "#dce8f6", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{c.name}</span>
          <span style={badge(c.autonomous)}>{stage}</span>
        </div>
        <div style={{ fontSize: 10, color: "#7a90a8" }}>
          {outpost
            ? `${c.owner_house_name} · pop ${fmtPop(c.population)}`
            : `pop ${fmtPop(c.population)}${c.coin_name ? ` · ${c.coin_name}` : ""}${c.charter_open ? " · charter ✔" : ""}`}
        </div>
        {settlementColony && !c.autonomous && (
          <>
            <div style={{ ...bar, marginTop: 4 }}><i style={{ width: `${frac * 100}%`, background: reserveColor(frac), display: "block", height: "100%", borderRadius: 5 }} /></div>
            <div style={{ fontSize: 9.5, color: frac < 0.25 ? WARN : "#6a86a6", marginTop: 1 }}>
              food {c.reserve_food.toFixed(1)}/{c.reserve_cap.toFixed(0)} mo · supplied {c.supply_years.toFixed(0)}y
              {c.supply_ships > 0 && <span style={{ color: "#7fb8ff" }}> · 🚢{c.supply_ships}</span>}
            </div>
          </>
        )}
      </div>
      {(settlementColony || satellite) && !c.autonomous && (
        <div style={{ textAlign: "right", flex: "0 0 auto" }}>
          <div style={{ fontSize: 9, color: "#6a86a6" }}>independence</div>
          <div style={{ fontSize: 10.5, color: c.indep_in_years <= 0 ? OK : GOLD }}>
            {c.indep_in_years <= 0 ? "eligible" : `in ${c.indep_in_years}y`}
          </div>
        </div>
      )}
    </div>
  );
}

function DetailCard({ detail, colony }: { detail: ColonyDetail; colony?: ColonySummary }) {
  return (
    <div style={card}>
      <div style={{ fontWeight: 700, color: "#dce8f6", marginBottom: 4 }}>{colony?.name ?? "Colony"} — detail</div>
      <KV k="Metropolis" v={detail.founder_name || "—"} />
      <KV k="Stage" v={STAGE[detail.stage] || "Colony"} />
      <KV k="Bank / mint" v={`${detail.main_bank_name || "—"}${detail.coin_name ? ` · ${detail.coin_name}` : ""}`} />
      <KV k="Charter" v={detail.charter_open ? "Closed market (backers only)" : "Open"} />
      <KV k="Food reserve" v={`${detail.reserve_food.toFixed(1)} / ${detail.reserve_cap.toFixed(0)} mo · ${detail.supply_years.toFixed(0)}y unbroken`} />
      <KV k="Independence" v={detail.indep_in_years <= 0 ? "eligible now" : `in ${detail.indep_in_years}y`} />

      {(detail.supply_ships > 0 || detail.supply_source) && (
        <>
          <div style={sec}>⚓ Grain lifeline</div>
          <div style={{ display: "flex", alignItems: "center", gap: 8, margin: "3px 0", fontSize: 11, color: "#cfe0f4" }}>
            <span style={{ fontSize: 14 }}>🚢</span>
            <span style={{ flex: 1 }}>
              <b>{detail.supply_ships}</b> dedicated supply ship{detail.supply_ships === 1 ? "" : "s"}
              <span style={{ color: "#7a90a8" }}> · {Math.round(detail.supply_capacity).toLocaleString()}/mo carriage</span>
            </span>
          </div>
          <KV k="Food source" v={detail.supply_source || "— none sufficient —"} />
          <div style={{ ...bar, marginTop: 2 }}>
            <i style={{
              width: `${detail.supply_capacity > 0 ? Math.min(100, (detail.supply_delivered / detail.supply_capacity) * 100) : 0}%`,
              background: detail.supply_delivered > 0 ? OK : WARN, display: "block", height: "100%", borderRadius: 5 }} />
          </div>
          <div style={{ fontSize: 9.5, color: detail.supply_delivered <= 0 ? WARN : "#6a86a6", marginTop: 2 }}>
            delivered {Math.round(detail.supply_delivered).toLocaleString()}/mo of {Math.round(detail.supply_capacity).toLocaleString()} capacity
          </div>
        </>
      )}

      {detail.backers.length > 0 && (
        <>
          <div style={sec}>Joint-stock backers</div>
          {detail.backers.map((b, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 6, margin: "2px 0" }}>
              <span style={{ width: 9, height: 9, borderRadius: 2, background: b.color, flex: "0 0 auto" }} />
              <span style={{ flex: 1, fontSize: 11, color: "#cfe0f4", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{b.name}</span>
              <span style={{ fontSize: 11, color: "#8aa6c0" }}>{(b.share * 100).toFixed(0)}%</span>
            </div>
          ))}
        </>
      )}

      {detail.supply.length > 0 && (
        <>
          <div style={sec}>Supply roster</div>
          {detail.supply.map((s, i) => (
            <div key={i} style={{ display: "flex", justifyContent: "space-between", fontSize: 10.5, color: "#9fb4cc", margin: "1px 0" }}>
              <span>{s.supplier} → {s.good}</span>
              <span style={{ color: "#7a90a8" }}>{s.qty.toFixed(0)}/yr</span>
            </div>
          ))}
        </>
      )}
    </div>
  );
}

function GateCard({ gates }: { gates: ColonyGateStatus | null }) {
  if (!gates) return <div style={empty}>No colonies founded yet.</div>;
  const block = gates.blocking_gate;
  const rows: { id: string; label: string; ok: boolean; detail: string }[] = [
    { id: "cap", label: `Under the colony cap (${gates.max_settlement_colonies})`, ok: !gates.at_colony_cap, detail: `${gates.settlement_colonies} live` },
    { id: "year", label: `Campaign age ≥ ${gates.start_year} years`, ok: gates.year_ok, detail: `year ${gates.year}` },
    { id: "founder", label: `A large, prosperous founder city (pop ≥ ${(gates.min_pop / 1000).toFixed(0)}k)`, ok: gates.founder_ok, detail: gates.qualifying_founder || "none qualifies" },
    { id: "bank", label: "A chartered bank on the continent", ok: gates.bank_on_continent, detail: gates.bank_on_continent ? "present" : "none on this landmass" },
    { id: "site", label: "A reachable unsettled colony site", ok: gates.site_ok, detail: `${gates.colonizable_sites_in_range} in range` },
  ];
  return (
    <div style={card}>
      <div style={{ fontWeight: 700, color: "#dce8f6", marginBottom: 2 }}>No colonies founded yet</div>
      <div style={{ fontSize: 10.5, color: "#7a90a8", marginBottom: 8 }}>
        Colonisation opens once the world matures and a venture can be backed. Outstanding gates:
      </div>
      {rows.map((r) => {
        const blocking = block === r.id;
        return (
          <div key={r.id} style={{
            display: "flex", alignItems: "center", gap: 8, padding: "3px 6px", borderRadius: 5, margin: "2px 0",
            background: blocking ? "rgba(224,80,58,0.12)" : "transparent",
            border: blocking ? `1px solid ${WARN}` : "1px solid transparent",
          }}>
            <span style={{ color: r.ok ? OK : WARN, fontWeight: 700, width: 12 }}>{r.ok ? "✔" : "✗"}</span>
            <span style={{ flex: 1, fontSize: 10.5, color: r.ok ? "#9fb4cc" : "#cfe0f4" }}>{r.label}</span>
            <span style={{ fontSize: 10, color: r.ok ? "#6a86a6" : WARN }}>{r.detail}</span>
          </div>
        );
      })}
      {block === "bank" && (
        <div style={{ fontSize: 10, color: "#7a90a8", marginTop: 8, lineHeight: 1.5 }}>
          The <b style={{ color: "#cfe0f4" }}>bank gate</b> is the usual blocker: a settlement colony needs a
          bank to stake the venture and mint its coin — and banks are themselves slow to charter.
        </div>
      )}
    </div>
  );
}

function KV({ k, v }: { k: string; v: string }) {
  return (
    <div style={{ display: "flex", gap: 8, fontSize: 11, margin: "2px 0" }}>
      <span style={{ color: "#6a86a6", minWidth: 92 }}>{k}</span>
      <span style={{ color: "#cfe0f4" }}>{v}</span>
    </div>
  );
}

function badge(autonomous: boolean): React.CSSProperties {
  return {
    fontSize: 9.5, padding: "0 6px", borderRadius: 10, whiteSpace: "nowrap",
    color: autonomous ? "#9fe0b0" : "#d8c0ff",
    border: `1px solid ${autonomous ? "#2a5a3a" : "#3a2a5a"}`,
  };
}

const panel: React.CSSProperties = {
  position: "absolute", top: 60, right: 360, width: 360, maxHeight: "80vh",
  display: "flex", flexDirection: "column",
  background: "#0c141e", border: "1px solid #1e3450", borderRadius: 8,
  boxShadow: "0 8px 28px rgba(0,0,0,0.5)", zIndex: 40,
};
const header: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  padding: "8px 10px", borderBottom: "1px solid #1a2a3e",
  color: "#cfe0f4", fontWeight: 700, fontSize: 12,
};
const statRow: React.CSSProperties = {
  display: "flex", gap: 5, padding: "8px 10px", borderBottom: "1px solid #1a2a3e", flexWrap: "wrap",
};
const statBox: React.CSSProperties = {
  background: "rgba(12,18,26,0.6)", border: "1px solid #1e2e42", borderRadius: 6,
  padding: "4px 9px", minWidth: 56, textAlign: "center",
};
const scroll: React.CSSProperties = { overflowY: "auto", padding: "6px 8px 10px" };
const metroHead: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 7, fontWeight: 650, fontSize: 11.5,
  color: "#eaf3ff", margin: "4px 0 4px",
};
const rowS: React.CSSProperties = {
  display: "flex", gap: 9, padding: "7px 8px", margin: "4px 0 4px 12px", cursor: "pointer",
  background: "rgba(12,18,26,0.55)", border: "1px solid #16242f", borderRadius: 8,
};
const rowSel: React.CSSProperties = { borderColor: COLONY_VIOLET, boxShadow: `0 0 0 1px ${COLONY_VIOLET} inset` };
const expRow: React.CSSProperties = {
  display: "flex", gap: 8, padding: "6px 6px", margin: "3px 0", cursor: "pointer",
  background: "rgba(12,18,26,0.5)", border: "1px solid #16242f", borderRadius: 7,
};
const bar: React.CSSProperties = { height: 6, borderRadius: 5, background: "#16242f", overflow: "hidden" };
const card: React.CSSProperties = {
  marginTop: 6, padding: "8px 10px", background: "rgba(12,18,26,0.6)",
  border: "1px solid #1e2e42", borderRadius: 8,
};
const sec: React.CSSProperties = {
  color: "#7a90a8", fontSize: 9, textTransform: "uppercase", letterSpacing: 0.6, margin: "8px 0 3px",
};
const empty: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "12px 10px", lineHeight: 1.5 };
