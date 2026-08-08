import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useWorldStore } from "@state/worldStore";
import { computeStates, campaignProvinceLandAll } from "@bridge";
import type { StateRegion, ProvinceLand } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Card, StatGrid, Stat, Badge, Chip, EmptyNote, FootNote } from "@ui/kit";

/** REALM_AND_GOVERNMENT_PLAN.md R1 · the Realms panel — the first dedicated window
 *  for a proclaimed realm (drawn on the map as a territory tint but until now never
 *  surfaced anywhere else). Aggregates each realm's held `ProvinceLand` rows
 *  (population, revenue, surplus) — by `st.province_ids`, not just the capital's
 *  own writ, since a realm can hold provinces administered by member cities too —
 *  so a coloured blob on the map becomes an actual polity with numbers behind it. */

const fmtk = (v: number) => {
  const a = Math.abs(v);
  if (a >= 1000) return `${(v / 1000).toFixed(1)}k`;
  return v.toFixed(a < 10 ? 1 : 0);
};

type Sort = "territory" | "population" | "wealth";
const SORTS: { id: Sort; label: string }[] = [
  { id: "territory", label: "Territory" },
  { id: "population", label: "Population" },
  { id: "wealth", label: "Revenue" },
];

/** `Realm.rank` — 0 city-state · 1 kingdom · 2 great power · 3 hegemon. */
const RANK_NAMES = ["City-state", "Kingdom", "Great power", "Hegemon"];

function GaugeBar({ label, v, tone }: { label: string; v: number; tone: "good" | "warn" | "bad" }) {
  const color = tone === "good" ? T.gold : tone === "warn" ? "#d9a441" : "#c4593f";
  return (
    <div style={{ marginBottom: 4 }}>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: FZ.micro, color: T.inkDim }}>
        <span>{label}</span><span>{Math.round(v * 100)}%</span>
      </div>
      <div style={{ height: 4, background: T.lineSoft, borderRadius: 2 }}>
        <div style={{ width: `${Math.round(Math.min(1, Math.max(0, v)) * 100)}%`, height: "100%", background: color, borderRadius: 2 }} />
      </div>
    </div>
  );
}

export function StatesPanel() {
  const open = useUIStore((s) => s.showStates);
  const close = () => useUIStore.getState().setShowStates(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setSelectedProvince = useUIStore((s) => s.setSelectedProvince);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const provinces = useWorldStore((s) => s.provinces);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [states, setStates] = useState<StateRegion[]>([]);
  const [land, setLand] = useState<ProvinceLand[]>([]);
  const [sort, setSort] = useState<Sort>("territory");
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (!open || !active) return;
    computeStates().then(setStates).catch(() => setStates([]));
    campaignProvinceLandAll().then(setLand).catch(() => setLand([]));
  }, [open, active, Math.floor(tick / 365)]);

  const provinceName = useMemo(() => {
    const m = new Map<number, string>();
    for (const p of provinces) m.set(p.id, p.name);
    return m;
  }, [provinces]);

  const rows = useMemo(() => {
    return states.map((st) => {
      const ids = new Set(st.province_ids);
      const held = land.filter((p) => ids.has(p.id));
      const capital = snapshot?.hubs.find((h) => h.id === st.capital_hub);
      // `p.urban` already sums every hub's population in that province (the seat's
      // included), so adding the capital's population again would double-count it —
      // this now covers every member city's province, not just the capital's own.
      const population = held.reduce((s, p) => s + p.rural + p.urban, 0);
      const revenue = held.reduce((s, p) => s + p.revenue, 0);
      const surplus = held.reduce((s, p) => s + p.surplus, 0);
      const unrest = held.length ? held.reduce((s, p) => s + p.unrest, 0) / held.length : 0;
      return { st, held, capital, population, revenue, surplus, unrest };
    }).sort((a, b) => {
      switch (sort) {
        case "population": return b.population - a.population;
        case "wealth": return b.revenue - a.revenue;
        default: return b.st.province_count - a.st.province_count;
      }
    });
  }, [states, land, snapshot, sort]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.province);
  if (!open) return null;

  const totalPop = rows.reduce((s, r) => s + r.population, 0);
  const totalProvinces = rows.reduce((s, r) => s + r.st.province_count, 0);

  const focusCapital = (hubId: number) => setSelectedHub(hubId);
  const focusProvince = (id: number) => setSelectedProvince(id);

  return (
    <Panel onPointerDown={onPointerDown} width={372} maxHeight="80vh" style={{ top: 60, right: 360, zIndex: 40, ...rootStyle }}>
      <PanelHeader icon="👑" title="Realms" onDragStart={onPointerDown} onClose={close}
        right={rows.length > 0 ? <Badge tone="gold">{rows.length}</Badge> : undefined} />
      <PanelBody style={{ flex: 1 }}>
        {!active && <EmptyNote>Begin the campaign — realms can be proclaimed from year 50, by a house that has held a city's government for a decade.</EmptyNote>}
        {active && rows.length === 0 && (
          <EmptyNote>
            No realm has been proclaimed yet. A house needs to hold a city's government (its captor, not merely a
            council seat) continuously for ten years, be tier 2 or better itself in a tier 2-or-better city, hold
            at least one province's writ, and have both the treasury and the prestige to risk it — and none of
            this is possible before year 50. Most cities never proclaim; this is expected, especially on a young world.
          </EmptyNote>
        )}
        {active && rows.length > 0 && (
          <>
            <StatGrid cols={2} style={{ marginBottom: SPACE.md }}>
              <Stat label="Realms proclaimed" value={rows.length} />
              <Stat label="Provinces held" value={totalProvinces} hint={`of the world's total`} />
              <Stat label="Population under a crown" value={fmtk(totalPop)} tone="gold" />
              <Stat label="Largest" value={rows[0]?.st.name ?? "—"} hint={`${rows[0]?.st.province_count ?? 0} provinces`} />
            </StatGrid>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md }} data-no-drag>
              {SORTS.map((s) => <Chip key={s.id} on={sort === s.id} onClick={() => setSort(s.id)}>{s.label}</Chip>)}
            </div>
            {rows.map(({ st, held, capital, population, revenue, surplus, unrest }) => {
              const isOpen = expanded.has(st.capital_hub);
              return (
                <Card key={st.capital_hub} style={{ marginBottom: SPACE.sm }}>
                  <div onClick={() => setExpanded((prev) => {
                    const n = new Set(prev); n.has(st.capital_hub) ? n.delete(st.capital_hub) : n.add(st.capital_hub); return n;
                  })} style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }} data-no-drag>
                    <span style={{ color: T.inkDim, fontSize: FZ.tiny, width: 9 }}>{isOpen ? "▾" : "▸"}</span>
                    <span style={{ width: 9, height: 9, borderRadius: 2, background: `rgb(${st.color.join(",")})`, flex: "0 0 auto" }} />
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ color: T.parchment, fontWeight: 700, fontSize: FZ.body }}>{st.name}</div>
                      <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>
                        {RANK_NAMES[st.rank] ?? "City-state"} · House {st.ruling_house || "?"} · seat{" "}
                        <span onClick={(e) => { e.stopPropagation(); focusCapital(st.capital_hub); }} style={{ color: T.accent, cursor: "pointer" }}>{capital?.name ?? "?"}</span>
                        {" · "}{st.province_count} province{st.province_count === 1 ? "" : "s"}
                      </div>
                    </div>
                    <div style={{ textAlign: "right" }}>
                      <div style={{ color: T.gold, fontSize: FZ.base, fontWeight: 700 }}>{fmtk(population)}</div>
                      <div style={{ color: T.inkDim, fontSize: FZ.micro }}>souls</div>
                    </div>
                  </div>
                  {isOpen && (
                    <div style={{ marginTop: SPACE.sm, paddingTop: SPACE.sm, borderTop: `1px solid ${T.lineSoft}` }}>
                      <StatGrid cols={3} style={{ marginBottom: SPACE.sm }}>
                        <Stat label="Revenue" value={fmtk(revenue)} hint="yearly, from held provinces" tone="gold" />
                        <Stat label="Surplus" value={fmtk(surplus)} hint="food, feeds the seat" />
                        <Stat label="Unrest" value={`${(unrest * 100).toFixed(0)}%`} tone={unrest > 0.5 ? "bad" : unrest > 0.25 ? "warn" : "good"} />
                      </StatGrid>
                      <StatGrid cols={2} style={{ marginBottom: SPACE.sm }}>
                        <Stat label="Crown treasury" value={fmtk(st.treasury)} tone="gold" />
                        <Stat label="Crown debts" value={fmtk(st.debts)} tone={st.debts > 0 ? "warn" : "good"} />
                      </StatGrid>
                      <GaugeBar label="Legitimacy" v={st.legitimacy} tone={st.legitimacy > 0.5 ? "good" : st.legitimacy > 0.25 ? "warn" : "bad"} />
                      <GaugeBar label="Cohesion" v={st.cohesion} tone={st.cohesion > 0.5 ? "good" : st.cohesion > 0.25 ? "warn" : "bad"} />
                      <div style={{ color: T.inkDim, fontSize: FZ.tiny, margin: "6px 0 2px" }}>Held provinces</div>
                      {held.length === 0 && <div style={{ color: T.inkFaint, fontSize: FZ.tiny, fontStyle: "italic" }}>none reporting yet</div>}
                      {held.map((p) => (
                        <div key={p.id} onClick={() => focusProvince(p.id)}
                          style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 0", cursor: "pointer" }}>
                          <span style={{ flex: 1, color: T.inkMid, fontSize: FZ.small, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                            {provinceName.get(p.id) ?? `Province ${p.id}`}
                          </span>
                          <span style={{ color: T.inkFaint, fontSize: FZ.micro }}>{fmtk(p.rural + p.urban)} pop</span>
                          <span style={{ color: T.gold, fontSize: FZ.micro, width: 40, textAlign: "right" }}>{fmtk(p.revenue)}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </Card>
              );
            })}
            <FootNote>
              A realm is proclaimed, not derived (REALM_AND_GOVERNMENT_PLAN.md R1) — once founded it persists
              even if its capital's own standing later slips, exactly like a merchant house's tier does. The
              ruling house is ELEVATED at the coronation, not dissolved: it survives as the dynasty, its wealth
              becomes the crown treasury whole.
            </FootNote>
          </>
        )}
      </PanelBody>
    </Panel>
  );
}
