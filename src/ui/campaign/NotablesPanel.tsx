import { useEffect, useMemo, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetNotableIndividuals, campaignGetHallOfDead } from "@bridge";
import type { IndividualBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, SERIF, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, EmptyNote } from "@ui/kit";
import { individualPerson } from "@ui/campaign/cultureDress";
import { PixelBust } from "@ui/campaign/PixelBust";
import { CareerLadder, roleColor } from "@ui/campaign/personShared";

/** Notable people — the roster (design handoff "Council & People", 7d).
 *  Every famous person the chronicle follows as a card: portrait ringed in their
 *  role colour, where they are, the latest thing the record says of them, a
 *  career ladder, a fame bar. Filter by role, search, sort; the strip on top
 *  lists those currently AWAY from home (their last recorded stop is a visit
 *  within the last year). A card opens the full Character window.
 *  Everything shown is what `campaign_get_notables` / `_hall_of_dead` serve —
 *  the design's richer "Now / Next" and journey progress are not served yet, so
 *  the card says "Latest" and reads the record's own last line instead. */

const roleOf = (p: IndividualBrief) => p.roles[p.roles.length - 1] ?? "Notable";
type Sort = "fame" | "career" | "name" | "age";

export function NotablesPanel() {
  const open = useUIStore((s) => s.showNotables);
  const close = () => useUIStore.getState().setShowNotables(false);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const year = Math.floor(tick / 365);
  const active = !!snapshot?.active;

  const [tab, setTab] = useState<"living" | "dead">("living");
  const [living, setLiving] = useState<IndividualBrief[]>([]);
  const [dead, setDead] = useState<IndividualBrief[]>([]);
  const [role, setRole] = useState("All");
  const [q, setQ] = useState("");
  const [sort, setSort] = useState<Sort>("fame");

  useEffect(() => {
    if (!open || !active) return;
    campaignGetNotableIndividuals().then(setLiving).catch(() => setLiving([]));
    campaignGetHallOfDead().then(setDead).catch(() => setDead([]));
  }, [open, active, tick]);

  const all = useMemo(() => [...living, ...dead], [living, dead]);
  const roles = useMemo(() => ["All", ...Array.from(new Set(all.map(roleOf)))], [all]);
  const away = useMemo(() => living.filter((p) => {
    const last = p.places?.[p.places.length - 1];
    return !!last && last.visit && last.year >= year - 1;
  }), [living, year]);
  const houses = useMemo(() => new Set(living.filter((p) => p.house).map((p) => p.house)).size, [living]);

  const rows = useMemo(() => {
    let r = tab === "living" ? living : dead;
    if (role !== "All") r = r.filter((p) => roleOf(p) === role);
    const s = q.trim().toLowerCase();
    if (s) r = r.filter((p) => `${p.name} ${p.city} ${p.house} ${p.culture}`.toLowerCase().includes(s));
    const key: Record<Sort, (a: IndividualBrief, b: IndividualBrief) => number> = {
      fame: (a, b) => b.fame - a.fame,
      career: (a, b) => a.debut_year - b.debut_year,
      name: (a, b) => a.name.localeCompare(b.name),
      age: (a, b) => (a.age ?? 0) - (b.age ?? 0),
    };
    return [...r].sort(key[sort]);
  }, [tab, living, dead, role, q, sort]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.notables);
  if (!open) return null;
  const openPerson = (id: number) => useUIStore.getState().setPersonWindow(id);

  return (
    <Panel onPointerDown={onPointerDown} width={980} maxHeight="88vh" style={{ top: 56, left: 330, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="👥" title="Notable people" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: 0 }}>
        {!active && <div style={{ padding: SPACE.lg }}><EmptyNote>Start the campaign to see notable people.</EmptyNote></div>}
        {active && (
          <>
            <div style={{ display: "flex", alignItems: "baseline", gap: 14, padding: "10px 18px", borderBottom: `1px solid ${T.line}`, background: T.raised }}>
              <span style={{ fontFamily: SERIF, fontSize: 18, color: T.gold }}>{all.length} great lives</span>
              <span style={{ color: T.inkDim, fontSize: FZ.small }}>where they are, what the record says of them, how they rose</span>
              <span style={{ flex: 1 }} />
              <Stat label="Living" value={living.length} />
              <Stat label="Away" value={away.length} color={T.gold} />
              <Stat label="Houses served" value={houses} />
            </div>

            {away.length > 0 && (
              <div data-no-drag style={{ display: "flex", gap: 10, alignItems: "center", padding: "8px 18px", background: T.card, borderBottom: `1px solid ${T.line}`, overflowX: "auto" }}>
                <span style={{ fontSize: FZ.micro, letterSpacing: 0.6, textTransform: "uppercase", color: T.inkDim, width: 64, flex: "none" }}>Away from home</span>
                {away.map((p) => {
                  const last = p.places![p.places!.length - 1];
                  return (
                    <div key={p.id} onClick={() => openPerson(p.id)} style={{ display: "flex", gap: 8, alignItems: "center", cursor: "pointer", padding: "4px 10px", borderRadius: RADIUS.md, border: `1px solid ${T.line}`, background: T.panel, flex: "none" }}>
                      <Ring p={p} size={34} />
                      <div style={{ minWidth: 0 }}>
                        <div style={{ fontSize: FZ.small, color: T.ink, fontWeight: 600 }}>{p.name}</div>
                        <div style={{ fontSize: FZ.micro, color: T.inkDim }}>{last.from_city ? `${last.from_city} → ` : "to "}<b style={{ color: T.ink }}>{last.city}</b>{last.km ? ` · ${Math.round(last.km)} km` : ""}</div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            <div data-no-drag style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", padding: "10px 18px", borderBottom: `1px solid ${T.line}` }}>
              <div style={{ display: "flex", border: `1px solid ${T.line}`, borderRadius: 999, padding: 2, gap: 2 }}>
                {([["living", `Living · ${living.length}`], ["dead", `Hall of the Dead · ${dead.length}`]] as const).map(([k, l]) => (
                  <span key={k} onClick={() => setTab(k)} style={{ cursor: "pointer", padding: "3px 12px", borderRadius: 999, fontSize: FZ.small, fontWeight: 600, background: tab === k ? T.gold : "transparent", color: tab === k ? T.panel : T.inkMid }}>{l}</span>
                ))}
              </div>
              <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Find a person, city, house…"
                style={{ width: 190, padding: "4px 12px", borderRadius: 999, border: `1px solid ${T.line}`, background: T.card, color: T.ink, fontSize: FZ.small, outline: "none" }} />
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap", flex: 1, minWidth: 0 }}>
                {roles.map((r) => (
                  <span key={r} onClick={() => setRole(r)} style={{
                    cursor: "pointer", display: "inline-flex", alignItems: "center", gap: 5, padding: "1px 9px", borderRadius: 999, fontSize: FZ.micro, fontWeight: 600,
                    border: `1px solid ${role === r ? T.lineGold : T.line}`, background: role === r ? "rgba(216,178,74,.14)" : "transparent", color: role === r ? T.ink : T.inkDim,
                  }}>
                    {r !== "All" && <span style={{ width: 7, height: 7, borderRadius: "50%", background: roleColor(r) }} />}{r}
                  </span>
                ))}
              </div>
              <select value={sort} onChange={(e) => setSort(e.target.value as Sort)}
                style={{ padding: "3px 10px", borderRadius: 999, border: `1px solid ${T.line}`, background: T.card, color: T.inkMid, fontSize: FZ.small }}>
                <option value="fame">Sort · fame</option><option value="career">Sort · longest career</option>
                <option value="name">Sort · name</option><option value="age">Sort · youngest</option>
              </select>
            </div>

            <div style={{ overflowY: "auto", flex: 1, padding: "12px 16px 16px", display: "grid", gridTemplateColumns: "repeat(3,minmax(0,1fr))", gap: 10, alignContent: "start", minHeight: 260 }}>
              {rows.length === 0 && (
                <div style={{ gridColumn: "1/-1" }}>
                  <EmptyNote>{all.length === 0 ? "No one has become notable yet." : "No one matches."}</EmptyNote>
                </div>
              )}
              {rows.map((p) => <Card key={p.id} p={p} onOpen={() => openPerson(p.id)} />)}
            </div>
          </>
        )}
      </PanelBody>
    </Panel>
  );
}

function Stat({ label, value, color }: { label: string; value: number; color?: string }) {
  return <span style={{ fontSize: FZ.tiny, color: T.inkDim }}>{label} <b style={{ fontFamily: SERIF, fontSize: 15, color: color ?? T.ink }}>{value}</b></span>;
}

function Ring({ p, size }: { p: IndividualBrief; size: number }) {
  const rc = roleColor(roleOf(p));
  return (
    <div style={{ width: size, height: size, borderRadius: "50%", overflow: "hidden", flex: "none", boxShadow: `0 0 0 2px ${rc}${p.famous && p.alive ? ", 0 0 10px " + rc + "55" : ""}`, background: T.card, filter: p.alive ? undefined : "grayscale(.85) brightness(.9)" }}>
      <PixelBust person={individualPerson(p)} size={size} occasion={p.famous ? "ceremonial" : "national"} />
    </div>
  );
}

function Card({ p, onOpen }: { p: IndividualBrief; onOpen: () => void }) {
  const role = roleOf(p), rc = roleColor(role);
  const latest = p.life_log.length ? p.life_log[p.life_log.length - 1] : "";
  const last = p.places?.[p.places.length - 1];
  return (
    <div data-no-drag onClick={onOpen}
      onMouseEnter={(e) => (e.currentTarget.style.borderColor = rc)} onMouseLeave={(e) => (e.currentTarget.style.borderColor = "")}
      style={{ display: "flex", flexDirection: "column", gap: 8, padding: "10px 12px", borderRadius: RADIUS.md, background: T.card, border: `1px solid ${T.line}`, cursor: "pointer", minWidth: 0, opacity: p.alive ? 1 : 0.82 }}>
      <div style={{ display: "flex", gap: 10, alignItems: "flex-start", minWidth: 0 }}>
        <Ring p={p} size={60} />
        <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 3 }}>
          <div style={{ fontFamily: SERIF, fontSize: 15, lineHeight: 1.15, color: p.alive ? T.parchment : T.inkMid }}>{p.name}{p.alive ? "" : " †"}</div>
          <div style={{ display: "flex", gap: 5, flexWrap: "wrap", alignItems: "center" }}>
            <span style={{ fontSize: 10, fontWeight: 700, color: rc, border: `1px solid ${rc}66`, background: `${rc}22`, borderRadius: 999, padding: "0 7px", textTransform: "uppercase" }}>{role}</span>
            <span style={{ fontSize: FZ.micro, color: T.inkMid }}>of {p.city || "—"}{p.culture ? ` · ${p.culture}` : ""}</span>
          </div>
          {p.house && <div style={{ fontSize: FZ.micro, color: T.inkDim }}>in service of <span style={{ color: T.ink }}>{p.house}</span></div>}
          <div style={{ fontSize: FZ.micro, color: T.inkDim }}>{p.alive ? `age ${p.age ?? "?"} · debuted ${p.debut_year}` : `${p.debut_year}–${p.death_year} · ${p.death_cause || "old age"}`}</div>
        </div>
      </div>
      {(latest || last) && (
        <div style={{ padding: "5px 9px", borderRadius: RADIUS.md, background: "rgba(0,0,0,.2)", border: `1px solid ${T.lineSoft}` }}>
          <div style={{ fontSize: 9, fontWeight: 600, letterSpacing: 0.6, textTransform: "uppercase", color: T.inkDim }}>{p.alive ? "Latest" : "Remembered"}</div>
          <div style={{ fontSize: FZ.small, lineHeight: 1.35, color: T.ink, display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden" }}>{latest || `at ${last?.city}`}</div>
        </div>
      )}
      {p.roles.length > 0 && <CareerLadder roles={p.roles} max={4} />}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: "auto" }}>
        <span style={{ fontSize: 10, color: T.inkDim }}>fame</span>
        <div style={{ flex: 1, height: 4, borderRadius: 999, background: T.lineSoft, overflow: "hidden" }}><div style={{ width: `${Math.min(100, p.fame * 100)}%`, height: "100%", background: rc }} /></div>
        <span style={{ fontSize: FZ.micro, fontWeight: 600, color: T.gold }}>Open ›</span>
      </div>
    </div>
  );
}
