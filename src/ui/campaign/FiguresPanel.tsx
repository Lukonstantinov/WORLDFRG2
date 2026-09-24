import { useEffect, useMemo, useRef, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetFigures } from "@bridge";
import type { FigureBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, SERIF, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote, Donut } from "@ui/kit";
import { drawBust, deriveKit, kitForCulture, type DressKit, type Occasion } from "@ui/campaign/cultureDress";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";

/** Notable Figures — the campaign's great lives as a gallery of portraits.
 *  Each card carries the person's likeness (their city's dress, varied per
 *  person), role, house, the years they were prominent, and the one real mark
 *  they left on the world (`legacy`, the effect `raise_notable_figures` applied
 *  when they rose). Clicking a card focuses their city. */

type RoleSpec = { emoji: string; color: string; plural: string; occasion: Occasion; verb: string };
const ROLE: Record<string, RoleSpec> = {
  "Admiral": { emoji: "⚓", color: "#5fa8e8", plural: "Admirals", occasion: "ceremonial", verb: "Commands at sea" },
  "Demagogue": { emoji: "📢", color: "#e0735a", plural: "Demagogues", occasion: "everyday", verb: "Rouses the street" },
  "Master Craftsman": { emoji: "⚒", color: "#d8b24a", plural: "Masters", occasion: "everyday", verb: "Masters a craft" },
  "Great Banker": { emoji: "🏦", color: "#5cc08a", plural: "Bankers", occasion: "ceremonial", verb: "Moves capital" },
  "Explorer": { emoji: "🧭", color: "#b48ae0", plural: "Explorers", occasion: "national", verb: "Charts the unknown" },
};
const ROLE_KEYS = Object.keys(ROLE);
const FALLBACK: RoleSpec = { emoji: "•", color: T.inkMid, plural: "Figures", occasion: "national", verb: "" };
const roleOf = (r: string) => ROLE[r] ?? FALLBACK;

type Sort = "recent" | "tenure" | "name";

export function FiguresPanel() {
  const open = useUIStore((s) => s.showFigures);
  const close = () => useUIStore.getState().setShowFigures(false);
  const setSelectedHub = useUIStore((s) => s.setSelectedHub);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const showMarks = useUIStore((s) => s.overlayVisibility.figureMarks);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const year = snapshot?.clock?.year ?? 0;
  const active = !!snapshot?.active;

  const [rows, setRows] = useState<FigureBrief[]>([]);
  const [role, setRole] = useState<string>("All");
  const [livingOnly, setLivingOnly] = useState(false);
  const [sort, setSort] = useState<Sort>("recent");
  const [picked, setPicked] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetFigures().then(setRows).catch(() => setRows([]));
  }, [open, active, tick]);

  const tenure = (f: FigureBrief) => (f.alive ? year : f.died_year) - f.born_year;

  const shown = useMemo(() => {
    let r = rows;
    if (role !== "All") r = r.filter((f) => f.role === role);
    if (livingOnly) r = r.filter((f) => f.alive);
    r = [...r].sort((a, b) => {
      if (a.alive !== b.alive) return a.alive ? -1 : 1;
      if (sort === "tenure") return tenure(b) - tenure(a);
      if (sort === "name") return a.name.localeCompare(b.name);
      return b.born_year - a.born_year;
    });
    return r.slice(0, 200);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, role, livingOnly, sort, year]);

  const counts = useMemo(() => {
    const c: Record<string, { all: number; living: number }> = {};
    for (const k of ROLE_KEYS) c[k] = { all: 0, living: 0 };
    for (const f of rows) {
      const e = c[f.role] ?? (c[f.role] = { all: 0, living: 0 });
      e.all++; if (f.alive) e.living++;
    }
    return c;
  }, [rows]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.figures);
  if (!open) return null;

  const living = rows.filter((f) => f.alive).length;
  const houses = new Set(rows.filter((f) => f.alive && f.house).map((f) => f.house)).size;
  const focus = (f: FigureBrief) => {
    setPicked(f.name + f.born_year);
    const id = snapshot?.hubs?.[f.hub]?.id;
    if (id != null) setSelectedHub(id);
  };
  const slices = ROLE_KEYS.map((k) => ({ label: ROLE[k].plural, value: counts[k]?.all ?? 0, color: ROLE[k].color }));

  return (
    <Panel onPointerDown={onPointerDown} width={400} maxHeight="80%" style={{ top: 70, right: 12, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="⚜️" title="Notable Figures" onDragStart={onPointerDown} onClose={close}
        right={
          <label data-no-drag style={{ display: "flex", alignItems: "center", gap: 4, cursor: "pointer", fontSize: FZ.tiny, color: T.inkMid, marginRight: 6 }}>
            <input type="checkbox" checked={showMarks} onChange={(e) => setOverlayVisible("figureMarks", e.target.checked)}
              style={{ accentColor: T.gold, width: 11, height: 11 }} />
            on map
          </label>
        } />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        {!active && <EmptyNote>Start the campaign to see notable figures.</EmptyNote>}

        {active && rows.length > 0 && (
          <div style={{
            display: "flex", gap: SPACE.lg, alignItems: "center", marginBottom: SPACE.md,
            padding: "8px 10px", borderRadius: RADIUS.md, border: `1px solid ${T.lineSoft}`,
            background: "linear-gradient(135deg, rgba(216,178,74,0.07), rgba(10,16,24,0.6))",
          }}>
            <Donut slices={slices} size={82} thickness={11} center={String(living)} sub="LIVING" />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontFamily: SERIF, color: T.parchment, fontSize: FZ.base, marginBottom: 4 }}>
                {rows.length} great lives recorded
                {houses > 0 && <span style={{ color: T.inkDim, fontSize: FZ.tiny }}> · {houses} house{houses === 1 ? "" : "s"} served</span>}
              </div>
              {ROLE_KEYS.map((k) => {
                const c = counts[k]; const spec = ROLE[k];
                const on = role === k;
                return (
                  <div key={k} data-no-drag onClick={() => setRole(on ? "All" : k)}
                    style={{
                      display: "grid", gridTemplateColumns: "14px 1fr auto", gap: 6, alignItems: "center",
                      fontSize: FZ.tiny, cursor: "pointer", padding: "1px 3px", borderRadius: RADIUS.sm,
                      background: on ? "rgba(255,255,255,0.06)" : "transparent",
                      opacity: c.all === 0 ? 0.4 : 1,
                    }}>
                    <span style={{ width: 8, height: 8, borderRadius: 2, background: spec.color, justifySelf: "center" }} />
                    <span style={{ color: on ? T.ink : T.inkMid }}>{spec.emoji} {spec.plural}</span>
                    <span style={{ color: T.inkDim, fontVariantNumeric: "tabular-nums" }}>
                      <span style={{ color: c.living ? T.ink : T.inkFaint }}>{c.living}</span> / {c.all}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {active && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: SPACE.md, alignItems: "center" }} data-no-drag>
            <Chip on={role === "All"} onClick={() => setRole("All")}>All</Chip>
            {ROLE_KEYS.map((k) => (
              <Chip key={k} on={role === k} onClick={() => setRole(role === k ? "All" : k)}>{ROLE[k].emoji}</Chip>
            ))}
            <Chip on={livingOnly} onClick={() => setLivingOnly((v) => !v)}>Living</Chip>
            <span style={{ flex: 1 }} />
            <select value={sort} onChange={(e) => setSort(e.target.value as Sort)}
              style={{ background: T.card, color: T.inkMid, border: `1px solid ${T.line}`, borderRadius: RADIUS.sm, fontSize: FZ.tiny, padding: "1px 3px" }}>
              <option value="recent">Newest</option>
              <option value="tenure">Longest career</option>
              <option value="name">Name</option>
            </select>
          </div>
        )}

        <div style={{ overflowY: "auto", flex: 1, display: "flex", flexDirection: "column", gap: SPACE.sm }}>
          {active && shown.length === 0 && <EmptyNote>No figures yet — advance time.</EmptyNote>}
          {shown.map((f) => (
            <FigureCard key={f.name + f.born_year} f={f} year={year}
              selected={picked === f.name + f.born_year} onClick={() => focus(f)} />
          ))}
        </div>
      </PanelBody>
    </Panel>
  );
}

function FigureCard({ f, year, selected, onClick }: {
  f: FigureBrief; year: number; selected: boolean; onClick: () => void;
}) {
  const spec = roleOf(f.role);
  const span = (f.alive ? year : f.died_year) - f.born_year;
  return (
    <div data-no-drag onClick={onClick}
      style={{
        display: "flex", gap: SPACE.md, padding: "8px 9px", cursor: "pointer", flexShrink: 0,
        borderRadius: RADIUS.md, border: `1px solid ${selected ? spec.color : T.lineSoft}`,
        borderLeft: `3px solid ${f.alive ? spec.color : T.inkFaint}`,
        background: f.alive
          ? `linear-gradient(90deg, ${hexA(spec.color, 0.10)}, ${T.card} 55%)`
          : T.card,
      }}>
      <FigurePortrait f={f} spec={spec} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
          <span style={{
            fontFamily: SERIF, fontSize: FZ.head, fontWeight: 700, lineHeight: 1.2,
            color: f.alive ? T.parchment : T.inkMid, flex: 1, minWidth: 0,
          }}>
            {f.name}{!f.alive && <span style={{ color: T.inkDim, fontWeight: 400 }}> †</span>}
          </span>
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 5, marginTop: 3 }}>
          <span style={{
            fontSize: FZ.micro, fontWeight: 700, letterSpacing: 0.5, textTransform: "uppercase",
            color: spec.color, background: hexA(spec.color, 0.14), border: `1px solid ${hexA(spec.color, 0.4)}`,
            borderRadius: 999, padding: "0 6px", lineHeight: 1.6,
          }}>{spec.emoji} {f.role}</span>
          <span style={{ fontSize: FZ.tiny, color: T.inkMid }}>of {f.city}</span>
          {f.culture && <span style={{ fontSize: FZ.tiny, color: T.inkFaint }}>· {f.culture}</span>}
        </div>
        {f.house && (
          <div style={{ display: "flex", alignItems: "center", gap: 5, marginTop: 4, fontSize: FZ.tiny, color: T.inkMid }}>
            <CoatOfArms name={f.house} size={14} />
            <span>in service of <span style={{ color: T.ink }}>House {f.house}</span></span>
          </div>
        )}
        {f.legacy && (
          <div style={{
            marginTop: 5, fontSize: FZ.small, lineHeight: 1.35, fontStyle: "italic",
            color: f.alive ? T.inkMid : T.inkDim,
            borderLeft: `2px solid ${hexA(spec.color, 0.45)}`, paddingLeft: 6,
          }}>
            {f.legacy}
          </div>
        )}
      </div>
      <div style={{ textAlign: "right", flex: "0 0 auto", minWidth: 58 }}>
        <div style={{ fontFamily: SERIF, color: f.alive ? T.gold : T.inkDim, fontSize: FZ.base, fontWeight: 700 }}>
          {f.alive ? `${f.born_year}–` : `${f.born_year}–${f.died_year}`}
        </div>
        <div style={{ fontSize: FZ.micro, color: T.inkDim, marginTop: 1 }}>
          {f.alive ? "rose to note" : "career"}
        </div>
        <div style={{ fontSize: FZ.tiny, color: f.alive ? T.goodInk : T.inkFaint, marginTop: 4 }}>
          {f.alive ? "● living" : "† departed"}
        </div>
        <div style={{ fontSize: FZ.micro, color: T.inkDim }}>{Math.max(0, span)} yr{span === 1 ? "" : "s"}</div>
      </div>
    </div>
  );
}

/** The great lives of ONE city, for the settlement panel's Life tab — who rose
 *  here and what they did to the place. Renders nothing when the city has none,
 *  so an ordinary town stays quiet. `hubIdx` is the sim hub index. */
export function CityNotables({ hubIdx, cityName }: { hubIdx: number; cityName: string }) {
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const year = snapshot?.clock?.year ?? 0;
  const active = !!snapshot?.active;
  const [rows, setRows] = useState<FigureBrief[]>([]);
  useEffect(() => {
    if (!active) return;
    let alive = true;
    campaignGetFigures().then((r) => { if (alive) setRows(r.filter((f) => f.hub === hubIdx)); }).catch(() => {});
    return () => { alive = false; };
  }, [active, tick, hubIdx]);
  if (rows.length === 0) return null;
  const living = rows.filter((f) => f.alive);
  return (
    <div style={{ marginBottom: SPACE.md }}>
      <div style={{ color: T.inkDim, fontSize: FZ.small, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.6, marginBottom: 4 }}>
        Great lives of {cityName}
        <span style={{ color: T.inkFaint, fontWeight: 400, textTransform: "none", letterSpacing: 0 }}>
          {" "}· {living.length} living, {rows.length - living.length} remembered
        </span>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {rows.slice(0, 6).map((f) => {
          const spec = roleOf(f.role);
          return (
            <div key={f.name + f.born_year} style={{
              display: "flex", gap: SPACE.md, alignItems: "center", padding: "5px 7px", borderRadius: RADIUS.md,
              border: `1px solid ${T.lineSoft}`, borderLeft: `3px solid ${f.alive ? spec.color : T.inkFaint}`, background: T.card,
            }}>
              <FigurePortrait f={f} spec={spec} size={36} />
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontFamily: SERIF, color: f.alive ? T.parchment : T.inkMid, fontSize: FZ.body, fontWeight: 700 }}>
                  {f.name}{!f.alive && " †"}
                </div>
                <div style={{ fontSize: FZ.micro, color: spec.color, textTransform: "uppercase", letterSpacing: 0.4 }}>
                  {spec.emoji} {f.role}{f.house ? ` · House ${f.house}` : ""} · {f.alive ? `since ${f.born_year} (${Math.max(0, year - f.born_year)} yrs)` : `${f.born_year}–${f.died_year}`}
                </div>
                {f.legacy && <div style={{ fontSize: FZ.tiny, color: T.inkMid, fontStyle: "italic", marginTop: 1 }}>{f.legacy}</div>}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/** A person's likeness: their city's dress plate, re-dyed and re-cut per person
 *  so two figures of one people read as two people, framed in the role colour. */
function FigurePortrait({ f, spec, size = 54 }: { f: FigureBrief; spec: RoleSpec; size?: number }) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const S = 2;
    el.width = size * S; el.height = size * S;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    drawBust(ctx, 0, 0, size * S, personalKit(f.culture || f.city, f.name), { occasion: spec.occasion });
  }, [f.culture, f.city, f.name, spec.occasion, size]);

  return (
    <div style={{ position: "relative", flex: "0 0 auto", width: size + 8, height: size + 8 }}>
      <div style={{
        width: size + 8, height: size + 8, borderRadius: "50%", padding: 2,
        background: f.alive
          ? `conic-gradient(from 210deg, ${spec.color}, ${hexA(spec.color, 0.25)}, ${spec.color})`
          : T.line,
        boxShadow: f.alive ? `0 0 10px ${hexA(spec.color, 0.3)}` : "none",
      }}>
        <div style={{
          width: size + 4, height: size + 4, borderRadius: "50%", overflow: "hidden",
          background: `radial-gradient(circle at 50% 35%, ${hexA(spec.color, 0.25)}, ${T.card} 70%)`,
          display: "grid", placeItems: "center",
          filter: f.alive ? "none" : "grayscale(0.9) brightness(0.8)",
        }}>
          <canvas ref={ref} style={{ width: size, height: size, imageRendering: "pixelated", display: "block" }} />
        </div>
      </div>
      <span style={{
        position: "absolute", right: -2, bottom: -2, width: size >= 48 ? 20 : 15, height: size >= 48 ? 20 : 15, borderRadius: "50%",
        display: "grid", placeItems: "center", fontSize: size >= 48 ? 11 : 8,
        background: T.panel, border: `1.5px solid ${f.alive ? spec.color : T.inkFaint}`,
      }}>{spec.emoji}</span>
    </div>
  );
}

/** The culture's own costume, with the personal traits (hair, beard, dyes)
 *  drawn from the person's name — the cut, skin and headwear stay the people's. */
function personalKit(culture: string, person: string): DressKit {
  const base = kitForCulture(culture);
  const v = deriveKit(person, { seed: 7 });
  return { ...base, hair: v.hair, beard: v.beard, trim: v.trim, cloth2: v.cloth2 };
}

function hexA(hex: string, a: number): string {
  const s = hex.replace("#", "");
  if (s.length !== 6) return hex;
  const n = parseInt(s, 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
}
