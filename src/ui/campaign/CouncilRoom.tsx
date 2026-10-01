import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { GovernmentBrief, IndividualBrief, SeatBrief } from "@types";
import { T, SERIF } from "@ui/campaign/chronicleTheme";
import { houseColor } from "@ui/heraldry/CoatOfArms";
import { individualPerson, makePerson, type Person } from "@ui/campaign/cultureDress";
import { drawChamber, drawTable, type HallStyle, type SceneSeat } from "@canvas/councilArt";
import { PixelBust } from "@ui/campaign/PixelBust";

/** The council's ROOM (design handoff 7a/7b/7c): the same seats and the same
 *  motion, seated three ways — the chamber as a pixel scene, the table seen from
 *  above, a tiered hemicycle by bloc. Each room is authored on a fixed stage and
 *  scaled to the panel's width. Everything shown is the sim's own seat data; a
 *  seat whose person has not loaded yet is drawn from its name and age alone. */

export type RoomKind = "chamber" | "table" | "hemicycle";
export const ROOM_LABEL: Record<RoomKind, string> = { chamber: "Chamber", table: "Table", hemicycle: "Hemicycle" };

const KIN = "#7fd0a0", COMMONS = "#8aa0bc";
export const leanColor = (v: number) => (v > 0.15 ? "#5fc08a" : v < -0.15 ? "#e0735a" : "#9fb4cc");
const ringOf = (s: SeatBrief) => (s.allegiance === 0 && s.house_name ? houseColor(s.house_name) : s.allegiance === 1 ? KIN : COMMONS);

/** Which hall a people sits in — from the head's own culture kit. */
function hallFor(kit: number): HallStyle {
  if (kit === 11 || kit === 15) return "steppe";
  if (kit === 4 || kit === 5 || kit === 9) return "hansa";
  return "med";
}

/** Fixed-size stage scaled to its container's width. */
function Stage({ w, h, children, onScale }: { w: number; h: number; children: (scale: number) => ReactNode; onScale?: (s: number) => void }) {
  const box = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(0.4);
  useLayoutEffect(() => {
    const el = box.current; if (!el) return;
    const upd = () => { const s = Math.min(1.25, el.clientWidth / w); setScale(s); onScale?.(s); };
    upd();
    const ro = new ResizeObserver(upd); ro.observe(el);
    return () => ro.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [w]);
  return (
    <div ref={box} style={{ width: "100%", height: h * scale, position: "relative", overflow: "hidden", borderRadius: 8, border: `1px solid ${T.line}`, background: "#0a1018" }}>
      <div style={{ position: "absolute", left: "50%", top: 0, width: w, height: h, transform: `translateX(-50%) scale(${scale})`, transformOrigin: "50% 0" }}>
        {children(scale)}
      </div>
    </div>
  );
}

export function CouncilRoom({ brief, people, kind, openSeat, onSeat }: {
  brief: GovernmentBrief; people: Record<number, IndividualBrief>; kind: RoomKind; openSeat: number | null; onSeat: (i: number | null) => void;
}) {
  // Seats in brief order; the head first, as `CouncilChamber` indexes them.
  const headIdx = Math.max(0, brief.seats.findIndex((s) => s.role === 0));
  const order = useMemo(() => [headIdx, ...brief.seats.map((_, i) => i).filter((i) => i !== headIdx)], [brief.seats, headIdx]);
  const persons: Person[] = useMemo(() => {
    const headP = people[brief.seats[headIdx]?.individual_id];
    const homeKit = headP ? individualPerson(headP).kit : 0;
    return brief.seats.map((s) => {
      const p = people[s.individual_id];
      return p ? individualPerson(p) : makePerson({ seed: s.name + s.office_title, kit: homeKit, female: s.female, age: s.age });
    });
  }, [brief.seats, people, headIdx]);
  const seats: SceneSeat[] = useMemo(() => brief.seats.map((s, i) => ({ head: i === headIdx, person: persons[i], ring: ringOf(s), lean: s.vote_lean ?? 0 })), [brief.seats, persons, headIdx]);
  const debate = !!brief.debate;
  const houses = useMemo(() => Array.from(new Set(brief.seats.filter((s) => s.house_name).map((s) => houseColor(s.house_name)))), [brief.seats]);
  const style = hallFor(persons[headIdx]?.kit ?? 0);
  const toggle = (i: number) => onSeat(openSeat === order.indexOf(i) ? null : order.indexOf(i));

  if (kind === "chamber") return <Chamber brief={brief} seats={seats} headIdx={headIdx} style={style} houses={houses} debate={debate} toggle={toggle} />;
  if (kind === "table") return <TableRoom brief={brief} seats={seats} headIdx={headIdx} debate={debate} toggle={toggle} />;
  return <Hemicycle brief={brief} seats={seats} headIdx={headIdx} debate={debate} toggle={toggle} />;
}

type RoomProps = { brief: GovernmentBrief; seats: SceneSeat[]; headIdx: number; debate: boolean; toggle: (i: number) => void };

function Label({ name, office, ring, big }: { name: string; office: string; ring: string; big?: boolean }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", pointerEvents: "none" }}>
      <span style={{ background: "rgba(9,14,20,.86)", border: `1px solid ${ring}88`, borderRadius: 4, padding: "1px 7px", font: `600 ${big ? 13 : 10.5}px/1.35 system-ui,sans-serif`, color: "#e8d9b0", whiteSpace: "nowrap" }}>{name}</span>
      <span style={{ font: `600 ${big ? 10.5 : 8.5}px/1.4 system-ui,sans-serif`, letterSpacing: 0.5, textTransform: "uppercase", color: "#c9b88a", textShadow: "0 1px 2px #000" }}>{office}</span>
    </div>
  );
}

function TallyChips({ seats, debate, hall }: { seats: SceneSeat[]; debate: boolean; hall?: string }) {
  const n = (f: (l: number) => boolean) => seats.filter((s) => f(s.lean)).length;
  const chip = (t: ReactNode) => <span style={{ background: "rgba(9,14,20,.86)", color: "#cfe2f6", border: "1px solid #1e2e42", borderRadius: 4, padding: "3px 9px", font: "600 10.5px/1.3 system-ui,sans-serif", whiteSpace: "nowrap" }}>{t}</span>;
  const dot = (c: string) => <span style={{ display: "inline-block", width: 7, height: 7, borderRadius: "50%", background: c, marginRight: 4 }} />;
  return (
    <div style={{ position: "absolute", left: 12, top: 12, display: "flex", gap: 6 }}>
      {hall && chip(`${debate ? "In session" : "Sitting"} · ${hall}`)}
      {debate && chip(<>{dot("#5fc08a")}aye {n((l) => l > 0.15)}  {dot("#e0735a")}nay {n((l) => l < -0.15)}  {dot("#9fb4cc")}wavering {n((l) => Math.abs(l) <= 0.15)}</>)}
    </div>
  );
}

// ── A · the chamber as a pixel scene ──────────────────────────────────────
function Chamber({ brief, seats, headIdx, style, houses, debate, toggle }: RoomProps & { style: HallStyle; houses: string[]; debate: boolean }) {
  const SC = 3;
  const sc = useMemo(() => drawChamber(seats, { style, houses, debate, leanColor }, SC), [seats, style, houses, debate]);
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => { const h = host.current; if (!h) return; h.replaceChildren(sc.out); sc.out.style.cssText = `display:block;width:${sc.LW * SC}px;height:${sc.LH * SC}px;image-rendering:pixelated`; }, [sc]);
  const W = sc.LW * SC, H = sc.LH * SC;
  return (
    <Stage w={W} h={H}>
      {(scale) => (
        <>
          <div ref={host} style={{ position: "absolute", left: 0, top: 0 }} />
          {sc.spots.map((sp) => {
            const s = brief.seats[sp.idx], showLabel = scale >= 0.6 || sp.idx === headIdx;
            return (
              <div key={sp.idx}>
                <div title={`${s.office_title} ${s.name}`} onClick={() => toggle(sp.idx)}
                  style={{ position: "absolute", left: (sp.x - sp.w / 2) * SC, top: sp.top * SC, width: sp.w * SC, height: sp.h * SC, cursor: "pointer", borderRadius: 10 }} />
                {showLabel && (
                  <div style={{ position: "absolute", left: sp.x * SC, top: sp.label * SC + 4, transform: `translateX(-50%) scale(${scale < 0.6 ? Math.min(2.4, 0.62 / scale) : 1})`, transformOrigin: "50% 0" }}>
                    <Label name={s.name} office={s.office_title} ring={seats[sp.idx].ring} />
                  </div>
                )}
              </div>
            );
          })}
          {scale >= 0.6 && <TallyChips seats={seats} debate={debate} hall={`the ${brief.form}`} />}
        </>
      )}
    </Stage>
  );
}

// ── B · the council table from above ──────────────────────────────────────
function TableRoom({ brief, seats, headIdx, debate, toggle }: RoomProps) {
  const SC = 3, ts = useMemo(() => drawTable(seats, SC), [seats]);
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => { const h = host.current; if (!h) return; h.replaceChildren(ts.out); ts.out.style.cssText = `display:block;width:${ts.LW * SC}px;height:${ts.LH * SC}px;image-rendering:pixelated`; }, [ts]);
  const { tx, ty, tw } = ts.geom, d = brief.debate;
  const token = (idx: number, X: number, Y: number, S: number, below: boolean) => {
    const s = brief.seats[idx], ring = seats[idx].ring;
    return (
      <div key={idx} onClick={() => toggle(idx)} title={`${s.office_title} ${s.name}`}
        style={{ position: "absolute", left: X - S / 2, top: below ? Y - S / 2 : Y - S / 2 - 36, display: "flex", flexDirection: below ? "column" : "column-reverse", alignItems: "center", gap: 4, cursor: "pointer" }}>
        <div style={{ position: "relative", width: S, height: S, borderRadius: "50%", overflow: "hidden", boxShadow: `0 0 0 3px ${ring}`, background: "#0a1018" }}>
          <PixelBust person={seats[idx].person} size={S} />
          {debate && <span style={{ position: "absolute", right: 2, bottom: 2, width: 12, height: 12, borderRadius: "50%", background: leanColor(seats[idx].lean), boxShadow: "0 0 0 2px #0d1521" }} />}
        </div>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", background: "rgba(9,14,20,.9)", border: `1px solid ${ring}`, borderRadius: 999, padding: "2px 10px", boxShadow: "0 2px 6px rgba(0,0,0,.18)" }}>
          <span style={{ font: "600 11px/1.25 system-ui,sans-serif", color: T.ink, whiteSpace: "nowrap" }}>{s.name}</span>
          <span style={{ font: "600 8.5px/1.3 system-ui,sans-serif", letterSpacing: 0.5, textTransform: "uppercase", color: T.inkDim, whiteSpace: "nowrap" }}>{s.office_title}</span>
        </div>
      </div>
    );
  };
  return (
    <Stage w={ts.LW * SC} h={ts.LH * SC}>
      {() => (
        <>
          <div ref={host} style={{ position: "absolute", left: 0, top: 0 }} />
          {ts.chairs.map((p) => token(p.idx, p.cx * SC, p.cy * SC + (p.top ? -10 : 10), 62, !p.top))}
          {token(headIdx, ts.headAt.x * SC, ts.headAt.y * SC, 80, true)}
          {d && (
            <div style={{ position: "absolute", left: (tx + tw / 2 - 50) * SC, top: (ty + 14) * SC, width: 100 * SC, height: 18 * SC, boxSizing: "border-box", padding: "8px 14px", display: "flex", flexDirection: "column", justifyContent: "center", gap: 5, color: "#f5ead8" }}>
              <div style={{ font: "600 9px system-ui,sans-serif", letterSpacing: 0.6, textTransform: "uppercase", color: "#e1eecc" }}>Motion · round {d.round} of {d.round_cap}</div>
              <div style={{ fontFamily: SERIF, fontSize: 15, lineHeight: 1.15, color: "#fff8f0" }}>To {d.what || d.family}</div>
              <div style={{ position: "relative", height: 6, borderRadius: 999, background: "linear-gradient(90deg,#c0573a,#2a4a32 50%,#7fd0a0)" }}>
                <span style={{ position: "absolute", top: -3, left: `calc(${((d.tally + 1) / 2) * 100}% - 5px)`, width: 10, height: 12, borderRadius: 3, background: "#fff8f0" }} />
              </div>
            </div>
          )}
        </>
      )}
    </Stage>
  );
}

// ── C · tiered hemicycle by bloc ──────────────────────────────────────────
function Hemicycle({ brief, seats, headIdx, debate, toggle }: RoomProps) {
  const Wd = 1178, Hh = 420, cx = Wd / 2, cy = 384;
  const others = brief.seats.map((s, i) => ({ s, i })).filter((e) => e.i !== headIdx);
  const blocKey = (s: SeatBrief) => (s.allegiance === 0 && s.house_name ? "h:" + s.house_name : s.allegiance === 1 ? "kin" : "commons");
  const { sorted, ord } = useMemo(() => {
    const order: string[] = [];
    for (const e of others) if (!order.includes(blocKey(e.s))) order.push(blocKey(e.s));
    order.sort((a, b) => Number(a === "commons") - Number(b === "commons") || Number(a === "kin") - Number(b === "kin"));
    const [first, ...rest] = order;
    const o = first == null ? [] : [first, ...rest.filter((k) => k === "kin"), ...rest.filter((k) => k === "commons"), ...rest.filter((k) => k !== "kin" && k !== "commons")];
    return { ord: o, sorted: o.flatMap((k) => others.filter((e) => blocKey(e.s) === k)) };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [brief.seats, headIdx]);
  const N = sorted.length;
  const inner = Math.ceil(N * 0.56);
  const rows = [{ r: 262, n: inner }, { r: 176, n: N - inner }];
  const slots: { r: number; a: number }[] = [];
  for (const row of rows) for (let i = 0; i < row.n; i++) slots.push({ r: row.r, a: Math.PI * (0.08 + (0.84 * (i + 0.5)) / row.n) });
  slots.sort((a, b) => a.a - b.a);
  const pt = (r: number, a: number) => `${cx - Math.cos(a) * r},${cy - Math.sin(a) * r}`;
  const arc = (r0: number, r1: number, a0: number, a1: number) => `M${pt(r0, a0)} A${r0} ${r0} 0 0 1 ${pt(r0, a1)} L${pt(r1, a1)} A${r1} ${r1} 0 0 0 ${pt(r1, a0)} Z`;
  const blocs: { k: string; members: number[]; col: string; a0: number; a1: number }[] = [];
  let si = 0;
  for (const k of ord) {
    const members = sorted.filter((e) => blocKey(e.s) === k).map((e) => e.i);
    if (!members.length) continue;
    blocs.push({ k, members, col: seats[members[0]].ring, a0: slots[si].a - 0.045, a1: slots[si + members.length - 1].a + 0.045 });
    si += members.length;
  }
  const name = (k: string) => (k.startsWith("h:") ? "House " + k.slice(2) : k === "kin" ? "Ruler's kin" : "Commons");
  const d = brief.debate;
  const n = (f: (l: number) => boolean) => seats.filter((s) => f(s.lean)).length;
  return (
    <Stage w={Wd} h={Hh}>
      {(scale) => (
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(ellipse at 50% 100%,#2a2418,#0a1018 70%)" }}>
          <svg width={Wd} height={Hh} style={{ position: "absolute", inset: 0 }}>
            <path d={arc(306, 132, 0, Math.PI)} fill="#1a1a14" />
            {[306, 220, 132].map((r) => <path key={r} d={`M${cx - r},${cy} A${r} ${r} 0 0 1 ${cx + r},${cy}`} fill="none" stroke="#5a4a28" strokeDasharray={r === 220 ? "3 5" : undefined} />)}
            {blocs.map((b) => <g key={b.k}><path d={arc(312, 306, b.a0, b.a1)} fill={b.col} /><path d={arc(306, 132, b.a0, b.a1)} fill={b.col} fillOpacity={0.1} /></g>)}
          </svg>
          {sorted.map((e, i) => {
            const { r, a } = slots[i], X = cx - Math.cos(a) * r, Y = cy - Math.sin(a) * r, S = r > 220 ? 56 : 52, ring = seats[e.i].ring;
            return (
              <div key={e.i} onClick={() => toggle(e.i)} title={`${e.s.office_title} ${e.s.name}`} style={{ position: "absolute", left: X - S / 2, top: Y - S / 2, cursor: "pointer" }}>
                <div style={{ width: S, height: S, borderRadius: "50%", overflow: "hidden", boxShadow: `0 0 0 3px ${ring}`, background: "#0a1018" }}><PixelBust person={seats[e.i].person} size={S} /></div>
                {debate && <span style={{ position: "absolute", left: "50%", bottom: -6, transform: "translateX(-50%)", width: 12, height: 12, borderRadius: "50%", background: leanColor(seats[e.i].lean), boxShadow: "0 0 0 2px #0d1521" }} />}
                {scale >= 0.5 && <span style={{ position: "absolute", left: "50%", top: S + 8, transform: "translateX(-50%)", font: "600 10px/1.2 system-ui,sans-serif", color: T.ink, whiteSpace: "nowrap", textShadow: "0 1px 2px #000" }}>{e.s.name.split(" ")[0]}</span>}
              </div>
            );
          })}
          {blocs.map((b) => {
            const a = (b.a0 + b.a1) / 2, X = cx - Math.cos(a) * 334, Y = cy - Math.sin(a) * 334;
            return (
              <div key={b.k} style={{ position: "absolute", left: X, top: Y - 10, transform: `translateX(-50%) scale(${scale < 0.6 ? Math.min(2, 0.62 / scale) : 1})`, display: "flex", gap: 6, alignItems: "center", whiteSpace: "nowrap", font: "600 11px system-ui,sans-serif", color: T.ink }}>
                <span style={{ width: 9, height: 9, borderRadius: "50%", background: b.col }} />{name(b.k)}<span style={{ color: T.inkDim, fontWeight: 400 }}>{b.members.length}</span>
              </div>
            );
          })}
          {/* rostrum + the head */}
          <div style={{ position: "absolute", left: cx - 70, top: cy - 64, width: 140, height: 110, borderRadius: "70px 70px 0 0", background: "#2a2214", boxShadow: "0 0 0 2px #6a5a30" }} />
          <div onClick={() => toggle(headIdx)} style={{ position: "absolute", left: cx - 40, top: cy - 92, cursor: "pointer", width: 80, height: 80, borderRadius: "50%", overflow: "hidden", boxShadow: `0 0 0 4px ${seats[headIdx].ring}, 0 0 12px ${seats[headIdx].ring}55`, background: "#0a1018" }}>
            <PixelBust person={seats[headIdx].person} size={80} occasion="ceremonial" />
          </div>
          <div style={{ position: "absolute", left: cx, top: cy - 4, transform: "translateX(-50%)", textAlign: "center", whiteSpace: "nowrap" }}>
            <div style={{ fontFamily: SERIF, fontSize: 14, color: T.ink }}>{brief.seats[headIdx].name}</div>
            <div style={{ font: "600 9px system-ui,sans-serif", letterSpacing: 0.6, textTransform: "uppercase", color: T.gold }}>{brief.head_title || brief.seats[headIdx].office_title} · presides</div>
          </div>
          {d && scale >= 0.6 && (
            <div style={{ position: "absolute", left: 24, top: 18, width: 300, display: "flex", flexDirection: "column", gap: 6 }}>
              <div style={{ font: "600 9.5px system-ui,sans-serif", letterSpacing: 0.6, textTransform: "uppercase", color: T.inkDim }}>Motion · round {d.round} of {d.round_cap}</div>
              <div style={{ fontFamily: SERIF, fontSize: 16, lineHeight: 1.2, color: T.ink }}>To {d.what || d.family}</div>
              <div style={{ display: "flex", height: 10, borderRadius: 999, overflow: "hidden", background: T.lineSoft }}>
                {[[n((l) => l > 0.15), "#5fc08a"], [n((l) => Math.abs(l) <= 0.15), "#9fb4cc"], [n((l) => l < -0.15), "#e0735a"]].map(([c, col], k) => <span key={k} style={{ flex: Number(c), background: String(col) }} />)}
              </div>
            </div>
          )}
          {scale >= 0.6 && <div style={{ position: "absolute", right: 24, top: 18, width: 260, display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ font: "600 9.5px system-ui,sans-serif", letterSpacing: 0.6, textTransform: "uppercase", color: T.inkDim }}>Power by bloc</div>
            {blocs.map((b) => (
              <div key={b.k} style={{ display: "grid", gridTemplateColumns: "96px minmax(0,1fr) 28px", gap: 8, alignItems: "center" }}>
                <span style={{ font: "400 11px system-ui,sans-serif", color: T.inkMid, whiteSpace: "nowrap" }}>{name(b.k)}</span>
                <div style={{ height: 6, borderRadius: 999, background: T.lineSoft }}><div style={{ width: `${(b.members.length / Math.max(1, others.length)) * 100}%`, height: "100%", borderRadius: 999, background: b.col }} /></div>
                <span style={{ font: "600 11px system-ui,sans-serif", color: T.ink, textAlign: "right" }}>{Math.round((b.members.length / Math.max(1, others.length)) * 100)}%</span>
              </div>
            ))}
          </div>}
        </div>
      )}
    </Stage>
  );
}
