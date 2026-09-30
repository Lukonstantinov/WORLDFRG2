import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useUIStore, type PersonRoad } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useViewportStore } from "@state/viewportStore";
import { campaignGetIndividual, renderWorldThumbnail } from "@bridge";
import type { IndividualBrief, LifeEntryBrief, DecisionBrief, PlaceBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, SERIF, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Tabs, Chip, EmptyNote } from "@ui/kit";
import { Bust } from "@ui/campaign/personShared";
import { TraitChip, TRAIT_META, traitColor, IDEOLOGY_AXES } from "@ui/campaign/traitIcons";
import { LifeStory } from "@ui/campaign/LifeStory";

/** 2026-09-30b · THE CHARACTER WINDOW — one person's whole life, in four
 *  leaves: the LIFE (a written biography over the structured timeline), every
 *  DECISION they faced in full (the situation, each option with the odds
 *  `decide()` gave it and what it would have meant, the reasons on both sides,
 *  what it did to them), their TRAVELS (a map of every city they lived in or
 *  visited, the itinerary, and a button that draws the road on the world map),
 *  and their CHARACTER (each trait with how they came by it, the arc of their
 *  nature over the years, their ideology pole by pole, what they are going
 *  through now, and who they are tied to). Opened app-wide by
 *  `uiStore.personWindow` from every place a person appears. */

type Leaf = "life" | "decisions" | "travels" | "character";
const GOLD = "#e8b86a";
const VISIT = "#b8a0e8";

export function PersonWindowHost() {
  const id = useUIStore((s) => s.personWindow);
  if (id == null) return null;
  return <PersonWindow key={id} id={id} />;
}

function PersonWindow({ id }: { id: number }) {
  const close = () => useUIStore.getState().setPersonWindow(null);
  const tick = useCampaignStore((s) => s.snapshot?.clock?.tick ?? 0);
  const [p, setP] = useState<IndividualBrief | null>(null);
  const [missing, setMissing] = useState(false);
  const [leaf, setLeaf] = useState<Leaf>("life");
  // Refresh on the year so a living person's story keeps up.
  const year = Math.floor(tick / 365);
  useEffect(() => {
    let alive = true;
    campaignGetIndividual(id).then((r) => { if (!alive) return; setP(r); setMissing(!r); }).catch(() => setMissing(true));
    return () => { alive = false; };
  }, [id, year]);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.notables);

  const decisions = useMemo(() => (p?.life ?? []).filter((e) => e.decision), [p]);
  return (
    <Panel onPointerDown={onPointerDown} width={780} maxHeight="88vh" style={{ top: 56, left: 360, zIndex: 140, ...rootStyle }}>
      <PanelHeader icon="📖" title={p ? `The life of ${p.name}` : "A life"} onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        {missing && <EmptyNote>This person has been forgotten — only the famous are remembered after death.</EmptyNote>}
        {p && (
          <>
            <Header p={p} />
            <Tabs<Leaf> style={{ marginTop: SPACE.md }} active={leaf} onSelect={setLeaf} tabs={[
              ["life", "📖 Life"],
              ["decisions", `⚖ Decisions (${decisions.length})`],
              ["travels", `🧭 Travels (${(p.places ?? []).length})`],
              ["character", "🎭 Character"],
            ]} />
            <div data-no-drag style={{ overflowY: "auto", flex: 1, paddingTop: SPACE.md, cursor: "auto" }}>
              {leaf === "life" && <LifeLeaf p={p} decisions={decisions} />}
              {leaf === "decisions" && <DecisionsLeaf p={p} decisions={decisions} />}
              {leaf === "travels" && <TravelsLeaf p={p} />}
              {leaf === "character" && <CharacterLeaf p={p} />}
            </div>
          </>
        )}
      </PanelBody>
    </Panel>
  );
}

// ── Header ────────────────────────────────────────────────────────────────

function Header({ p }: { p: IndividualBrief }) {
  const roles = p.roles.length ? p.roles : ["Citizen"];
  const life = p.alive
    ? `born c. ${p.birth_year ?? "?"} · aged ${p.age ?? "?"} · now in ${p.city || "parts unknown"}`
    : `${p.birth_year ?? "?"}–${p.death_year} · died of ${p.death_cause || "old age"} aged ${p.age ?? "?"}`;
  return (
    <div style={{ display: "flex", gap: SPACE.lg, alignItems: "flex-start" }}>
      <div style={{ border: `2px solid ${p.famous ? T.gold : T.line}`, borderRadius: 10, padding: 2, background: T.card }}>
        <Bust person={p} size={92} />
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8, flexWrap: "wrap" }}>
          <span style={{ fontFamily: SERIF, fontSize: 22, fontWeight: 700, color: T.parchment }}>{p.name}</span>
          {p.famous && <Tag color={T.gold}>{p.alive ? "★ Notable" : "✝ Hall of the Dead"}</Tag>}
          {!p.alive && !p.famous && <Tag color={T.inkDim}>✝ dead</Tag>}
        </div>
        <div style={{ color: T.gold, fontSize: FZ.base, marginTop: 2 }}>
          {roles.join(" → ")}{p.scholar_stage ? ` · ${p.scholar_stage}` : ""}
        </div>
        <div style={{ color: T.inkMid, fontSize: FZ.small, marginTop: 2 }}>
          {life}{p.culture ? ` · of the ${p.culture}` : ""}{p.home_city ? ` · home: ${p.home_city}` : ""}
        </div>
        {p.house && (
          <div style={{ fontSize: FZ.small, marginTop: 2 }}>
            <span style={{ color: T.inkDim }}>of </span>
            <span data-no-drag style={{ color: T.accent, cursor: "pointer", textDecoration: "underline dotted" }}
              onClick={() => { if ((p.house_idx ?? -1) >= 0) useUIStore.getState().setDossierHouse(p.house_idx!); }}>{p.house}</span>
          </div>
        )}
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 6 }}>
          <span style={{ fontSize: FZ.micro, color: T.inkDim, textTransform: "uppercase", letterSpacing: 0.5 }}>Fame</span>
          <div style={{ width: 120, height: 6, borderRadius: 3, background: T.lineSoft, overflow: "hidden" }}>
            <div style={{ width: `${Math.round(p.fame * 100)}%`, height: "100%", background: T.gold }} />
          </div>
          <span style={{ fontSize: FZ.micro, color: T.inkMid }}>{Math.round(p.fame * 100)}</span>
          {p.ideology_name && <Tag color="#8a9ad8">⚑ {p.ideology_name}</Tag>}
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}>
          {p.traits.map((t, i) => <TraitChip key={t} name={t} strength={p.trait_strength?.[i] ?? 1} />)}
        </div>
      </div>
    </div>
  );
}

function Tag({ color, children }: { color: string; children: ReactNode }) {
  return (
    <span style={{ fontSize: FZ.micro, color, border: `1px solid ${color}66`, background: `${color}14`, borderRadius: RADIUS.pill, padding: "0 7px", lineHeight: 1.7, whiteSpace: "nowrap" }}>
      {children}
    </span>
  );
}

function SectionTitle({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div style={{ display: "flex", alignItems: "baseline", gap: 8, margin: `${SPACE.md}px 0 ${SPACE.sm}px` }}>
      <span style={{ fontFamily: SERIF, color: T.gold, fontSize: FZ.head, fontWeight: 700 }}>{children}</span>
      <span style={{ flex: 1, borderBottom: `1px solid ${T.lineSoft}` }} />
      {right}
    </div>
  );
}

// ── Life ─────────────────────────────────────────────────────────────────

function pronouns(p: IndividualBrief) {
  return p.female ? { they: "she", them: "her", their: "her", They: "She" } : { they: "he", them: "him", their: "his", They: "He" };
}

/** A written biography, built only from what the record holds — nothing
 *  here is invented; each sentence is dropped when its data is absent. */
function biography(p: IndividualBrief, decisions: LifeEntryBrief[]): string[] {
  const pr = pronouns(p);
  const out: string[] = [];
  const places = p.places ?? [];
  const role = (p.roles[0] ?? "citizen").toLowerCase();
  const art = /^[aeiou]/.test(role) ? "an" : "a";
  out.push(`${p.name} was born around the year ${p.birth_year ?? "?"}${p.home_city ? ` in ${p.home_city}` : ""}${p.culture ? `, among the ${p.culture}` : ""}. ${pr.They} entered public life in ${p.debut_year} as ${art} ${role}${p.roles.length > 1 ? `, and in time became ${p.roles.slice(1).map((r) => r.toLowerCase()).join(", then ")}` : ""}.`);
  const links = p.links ?? [];
  const teacher = links.find((l) => l.kind === "teacher");
  const students = links.filter((l) => l.kind === "student");
  if (teacher || students.length) {
    out.push([teacher ? `${pr.They} studied under ${teacher.name}` : "", students.length ? `${teacher ? " and" : pr.They} taught ${students.length} student${students.length === 1 ? "" : "s"} of ${pr.their} own, among them ${students.slice(0, 2).map((s) => s.name).join(" and ")}` : ""].join("") + ".");
  }
  const homes = places.filter((x) => !x.visit);
  const journeys = places.filter((x) => x.visit);
  const km = places.reduce((a, x) => a + (x.km ?? 0) * (x.visit ? 2 : 1), 0);
  const cities = new Set(places.map((x) => x.hub)).size;
  if (places.length > 1) {
    out.push(`${pr.They} lived in ${homes.length} place${homes.length === 1 ? "" : "s"}${journeys.length ? ` and made ${journeys.length} journey${journeys.length === 1 ? "" : "s"}` : ""}, seeing ${cities} cit${cities === 1 ? "y" : "ies"} and covering some ${Math.round(km).toLocaleString()} km.`);
  } else {
    out.push(`${pr.They} never left ${p.home_city || "home"}.`);
  }
  if (decisions.length) {
    const against = decisions.filter((e) => {
      const d = e.decision!; const o = d.options[d.pick]; return o && o.odds >= 0 && o.odds < 0.5;
    }).length;
    const certain = decisions.filter((e) => e.decision!.certain).length;
    out.push(`${pr.They} faced ${decisions.length} decision${decisions.length === 1 ? "" : "s"} worth recording${against ? `; ${against} of them went against the odds` : ""}${certain ? `, and ${certain} ${certain === 1 ? "was" : "were"} never in doubt` : ""}.`);
  }
  const seenFlip = new Set<string>();
  const flips = (p.life ?? []).filter((e) => {
    if (e.change !== "overturned" || !e.lost) return false;
    const k = `${e.year}|${e.lost}|${e.gained}`;
    if (seenFlip.has(k)) return false;
    seenFlip.add(k); return true;
  });
  if (flips.length) {
    out.push(`Life changed ${pr.them}: ${flips.map((e) => `once ${(e.lost ?? "").toLowerCase()}, ${pr.they} became ${e.gained.toLowerCase()} (age ${e.age})`).join("; ")}.`);
  }
  out.push(p.alive
    ? `${pr.They} ${p.famous ? "is one of the notable people of the age and " : ""}still lives, in ${p.city || "parts unknown"}.`
    : `${pr.They} died of ${p.death_cause || "old age"} in ${p.death_year}, aged ${p.age}${p.famous ? ", and is remembered in the Hall of the Dead" : ""}.`);
  return out;
}

function LifeLeaf({ p, decisions }: { p: IndividualBrief; decisions: LifeEntryBrief[] }) {
  const bio = useMemo(() => biography(p, decisions), [p, decisions]);
  return (
    <div>
      <div style={{ fontFamily: SERIF, fontSize: FZ.base, lineHeight: 1.6, color: T.parchment, background: T.card, border: `1px solid ${T.lineGold}`, borderRadius: RADIUS.md, padding: `${SPACE.md}px ${SPACE.lg}px` }}>
        {bio.map((s, i) => <span key={i}>{s} </span>)}
      </div>
      <SectionTitle>The story, year by year</SectionTitle>
      <LifeStory life={p.life} places={p.places} fallback={p.life_log} />
    </div>
  );
}

// ── Decisions ────────────────────────────────────────────────────────────

const KIND_META: Record<string, { icon: string; label: string; color: string }> = {
  dilemma: { icon: "⚖", label: "A dilemma", color: "#d8b24a" },
  journey: { icon: "🧭", label: "A journey's end", color: VISIT },
  study: { icon: "📚", label: "Study abroad?", color: "#6a8fd8" },
  career: { icon: "🎓", label: "A scholar's road", color: "#7ab0e8" },
  commission: { icon: "🎨", label: "The call of a greater city", color: "#e0a0c0" },
};

type DecFilter = "all" | "dilemma" | "journey" | "scholar" | "odds";

function DecisionsLeaf({ p, decisions }: { p: IndividualBrief; decisions: LifeEntryBrief[] }) {
  const [f, setF] = useState<DecFilter>("all");
  const [newestFirst, setNewestFirst] = useState(true);
  const againstOdds = (e: LifeEntryBrief) => { const d = e.decision!; const o = d.options[d.pick]; return !!o && o.odds >= 0 && o.odds < 0.5; };
  const shown = useMemo(() => {
    const xs = decisions.filter((e) => {
      const k = e.decision!.kind;
      if (f === "dilemma") return k === "dilemma";
      if (f === "journey") return k === "journey";
      if (f === "scholar") return k === "study" || k === "career" || k === "commission";
      if (f === "odds") return againstOdds(e);
      return true;
    });
    return newestFirst ? [...xs].reverse() : xs;
  }, [decisions, f, newestFirst]);
  if (decisions.length === 0) {
    return <EmptyNote>{p.name} has not yet faced a decision worth recording. Dilemmas come with life events; journeys and a scholar's choices are recorded as they happen.</EmptyNote>;
  }
  // What most often decided things for them.
  const reasonCount = new Map<string, number>();
  decisions.forEach((e) => e.decision!.why.forEach((w) => reasonCount.set(w, (reasonCount.get(w) ?? 0) + 1)));
  const topReasons = [...reasonCount.entries()].sort((a, b) => b[1] - a[1]).slice(0, 3);
  const favourite = decisions.filter((e) => { const d = e.decision!; const best = Math.max(...d.options.map((o) => o.odds)); return d.options[d.pick]?.odds === best && best >= 0; }).length;
  const count = (k: DecFilter) => decisions.filter((e) => {
    const kk = e.decision!.kind;
    return k === "dilemma" ? kk === "dilemma" : k === "journey" ? kk === "journey" : k === "scholar" ? (kk === "study" || kk === "career" || kk === "commission") : againstOdds(e);
  }).length;
  return (
    <div>
      <div style={{ display: "flex", gap: SPACE.lg, background: T.card, border: `1px solid ${T.line}`, borderRadius: RADIUS.md, padding: SPACE.md, marginBottom: SPACE.md, fontSize: FZ.small, color: T.inkMid, flexWrap: "wrap" }}>
        <span><b style={{ color: T.ink, fontSize: FZ.head }}>{decisions.length}</b> decisions</span>
        <span><b style={{ color: T.ink, fontSize: FZ.head }}>{favourite}</b> took the likelier road</span>
        <span><b style={{ color: "#e08a6a", fontSize: FZ.head }}>{count("odds")}</b> went against the odds</span>
        {topReasons.length > 0 && <span>most often moved by <b style={{ color: T.gold }}>{topReasons.map(([r]) => r).join(", ")}</b></span>}
      </div>
      <div style={{ display: "flex", gap: 5, marginBottom: SPACE.md, flexWrap: "wrap", alignItems: "center" }}>
        <Chip on={f === "all"} onClick={() => setF("all")}>All ({decisions.length})</Chip>
        {count("dilemma") > 0 && <Chip on={f === "dilemma"} onClick={() => setF("dilemma")}>⚖ Dilemmas ({count("dilemma")})</Chip>}
        {count("journey") > 0 && <Chip on={f === "journey"} onClick={() => setF("journey")}>🧭 Journeys ({count("journey")})</Chip>}
        {count("scholar") > 0 && <Chip on={f === "scholar"} onClick={() => setF("scholar")}>🎓 Career ({count("scholar")})</Chip>}
        {count("odds") > 0 && <Chip on={f === "odds"} onClick={() => setF("odds")}>🎲 Against the odds ({count("odds")})</Chip>}
        <span style={{ flex: 1 }} />
        <Chip on={false} onClick={() => setNewestFirst(!newestFirst)}>{newestFirst ? "newest first ↓" : "oldest first ↑"}</Chip>
      </div>
      {shown.map((e, i) => <DecisionCard key={i} e={e} d={e.decision!} p={p} />)}
    </div>
  );
}

function DecisionCard({ e, d, p }: { e: LifeEntryBrief; d: DecisionBrief; p: IndividualBrief }) {
  const km = KIND_META[d.kind] ?? KIND_META.dilemma;
  const chosen = d.options[d.pick];
  const known = d.options.every((o) => o.odds >= 0);
  const against = !!chosen && chosen.odds >= 0 && chosen.odds < 0.5;
  return (
    <div style={{ border: `1px solid ${against ? "#e08a6a55" : T.line}`, borderLeft: `3px solid ${km.color}`, borderRadius: RADIUS.md, background: T.card, padding: `${SPACE.md}px ${SPACE.lg}px`, marginBottom: SPACE.md }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4, flexWrap: "wrap" }}>
        <span style={{ color: km.color, fontSize: FZ.small, fontWeight: 700 }}>{km.icon} {km.label}</span>
        <span style={{ color: T.inkDim, fontSize: FZ.micro }}>year {e.year} · age {e.age}{e.city ? ` · ${e.city}` : ""}</span>
        <span style={{ flex: 1 }} />
        {d.certain && <Tag color={T.good}>never in doubt</Tag>}
        {against && <Tag color="#e08a6a">🎲 against the odds</Tag>}
        {!known && <Tag color={T.inkDim}>odds not recorded</Tag>}
      </div>
      <div style={{ fontFamily: SERIF, fontStyle: "italic", color: T.parchment, fontSize: FZ.base, lineHeight: 1.5, marginBottom: SPACE.sm }}>{d.prompt}</div>
      <div style={{ display: "grid", gridTemplateColumns: `repeat(${d.options.length}, 1fr)`, gap: SPACE.sm }}>
        {d.options.map((o, k) => (
          <div key={k} style={{
            border: `1px solid ${o.chosen ? km.color : T.lineSoft}`, borderRadius: RADIUS.sm, padding: SPACE.sm,
            background: o.chosen ? `${km.color}14` : "transparent", opacity: o.chosen ? 1 : 0.72,
          }}>
            <div style={{ display: "flex", alignItems: "baseline", gap: 5 }}>
              <span style={{ fontSize: FZ.small, fontWeight: 700, color: o.chosen ? T.ink : T.inkMid }}>{o.chosen ? "✓ " : ""}{o.label}</span>
              <span style={{ flex: 1 }} />
              {o.odds >= 0 && <span style={{ fontSize: FZ.small, color: o.chosen ? km.color : T.inkDim, fontVariantNumeric: "tabular-nums", fontWeight: 700 }}>{Math.round(o.odds * 100)}%</span>}
            </div>
            {o.odds >= 0 && (
              <div style={{ height: 4, borderRadius: 2, background: T.lineSoft, margin: "3px 0 4px", overflow: "hidden" }}>
                <div style={{ width: `${o.odds * 100}%`, height: "100%", background: o.chosen ? km.color : T.inkFaint }} />
              </div>
            )}
            <div style={{ fontSize: FZ.tiny, color: T.inkMid, lineHeight: 1.4 }}>{o.outcome}</div>
            <Consequences o={o} />
          </div>
        ))}
      </div>
      {(d.why.length > 0 || d.against.length > 0) && (
        <div style={{ fontSize: FZ.tiny, marginTop: SPACE.sm, lineHeight: 1.6 }}>
          {d.why.length > 0 && <div><span style={{ color: T.goodInk }}>✚ moved by </span><span style={{ color: T.ink }}>{d.why.join(", ")}</span></div>}
          {d.against.length > 0 && <div><span style={{ color: T.badInk }}>✖ despite </span><span style={{ color: T.inkMid }}>{d.against.join(", ")}</span></div>}
        </div>
      )}
      <ChangeLine e={e} p={p} />
    </div>
  );
}

function Consequences({ o }: { o: DecisionBrief["options"][number] }) {
  const bits: ReactNode[] = [];
  if (o.grants) bits.push(<TraitChip key="t" name={o.grants} compact={false} />);
  if (o.fame) bits.push(<span key="f" style={{ color: o.fame > 0 ? T.gold : T.badInk }}>★{o.fame > 0 ? "+" : ""}{Math.round(o.fame * 100)} fame</span>);
  if (o.modifier) bits.push(<span key="m" style={{ color: T.inkMid }}>⏳ {o.modifier}</span>);
  const ideo = (o.ideology ?? [0, 0, 0, 0]).map((v, i) => ({ v, a: IDEOLOGY_AXES[i] })).filter((x) => Math.abs(x.v) > 0.01);
  ideo.forEach(({ v, a }) => bits.push(<span key={a.name} style={{ color: a.color }}>{v > 0 ? "▲" : "▼"} {v > 0 ? a.pos : a.neg}</span>));
  if (bits.length === 0) return null;
  return <div style={{ display: "flex", flexWrap: "wrap", gap: 5, marginTop: 4, fontSize: FZ.micro, alignItems: "center" }}>{bits}</div>;
}

function ChangeLine({ e, p }: { e: LifeEntryBrief; p: IndividualBrief }) {
  const pr = pronouns(p);
  const c = e.change ?? "";
  if (!c) return null;
  const g = e.gained || e.decision?.options[e.decision.pick]?.grants || "";
  const text: Record<string, ReactNode> = {
    gained: <>{pr.They} became <TraitChip name={g} /></>,
    deepened: <>{pr.They} grew deeply <TraitChip name={g} strength={2} /></>,
    overturned: <><TraitChip name={e.lost ?? ""} /> gave way to <TraitChip name={g} /></>,
    weakened: <>{pr.their[0].toUpperCase() + pr.their.slice(1)} deep <TraitChip name={e.lost ?? ""} /> nature was shaken</>,
    confirmed: <>it confirmed what {pr.they} already were: <TraitChip name={g} strength={2} /></>,
    "no room": <>{pr.their[0].toUpperCase() + pr.their.slice(1)} character was already set — nothing new took root</>,
  };
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 5, flexWrap: "wrap", marginTop: SPACE.sm, paddingTop: SPACE.sm, borderTop: `1px dashed ${T.lineSoft}`, fontSize: FZ.tiny, color: T.inkMid }}>
      <span style={{ color: T.inkDim }}>What it did to {pr.them}:</span> {text[c] ?? c}
    </div>
  );
}

// ── Travels ──────────────────────────────────────────────────────────────

let thumbCache: { url: string; w: number; h: number } | null = null;
let thumbPending: Promise<{ url: string; w: number; h: number } | null> | null = null;
function worldThumb(): Promise<{ url: string; w: number; h: number } | null> {
  if (thumbCache) return Promise.resolve(thumbCache);
  if (!thumbPending) {
    thumbPending = renderWorldThumbnail(900).then((t) => {
      const bin = atob(t.rgba);
      const bytes = new Uint8ClampedArray(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      const c = document.createElement("canvas");
      c.width = t.width; c.height = t.height;
      const ctx = c.getContext("2d");
      if (!ctx) return null;
      ctx.putImageData(new ImageData(bytes, t.width, t.height), 0, 0);
      thumbCache = { url: c.toDataURL(), w: t.width, h: t.height };
      return thumbCache;
    }).catch(() => null);
  }
  return thumbPending;
}

const HOW_WORD: Record<string, string> = { debut: "began here", study: "went to study", exile: "fled into exile", return: "came home", settled: "settled", visit: "journeyed" };
const HOW_ICON: Record<string, string> = { debut: "★", study: "📚", exile: "⛓", return: "🏠", settled: "⌂", visit: "↗" };

/** The road as a map: distinct cities numbered by first arrival. X is
 *  unwrapped around the first stop so a road across the antimeridian stays
 *  one road (rule 6). */
function useRoad(p: IndividualBrief) {
  return useMemo(() => {
    const W = p.world_w || 1, places = p.places ?? [];
    const x0 = places[0]?.x ?? 0;
    const unwrap = (x: number) => { let v = x; while (v - x0 > W / 2) v -= W; while (x0 - v > W / 2) v += W; return v; };
    const order: number[] = [];
    const stop = new Map<number, { hub: number; x: number; y: number; city: string; n: number }>();
    places.forEach((pl) => {
      if (!stop.has(pl.hub)) { order.push(pl.hub); stop.set(pl.hub, { hub: pl.hub, x: unwrap(pl.x ?? 0), y: pl.y ?? 0, city: pl.city, n: order.length }); }
      if ((pl.from_hub ?? -1) >= 0 && !stop.has(pl.from_hub!)) { order.push(pl.from_hub!); stop.set(pl.from_hub!, { hub: pl.from_hub!, x: unwrap(pl.from_x ?? 0), y: pl.from_y ?? 0, city: pl.from_city ?? "", n: order.length }); }
    });
    const legs = places.filter((pl) => (pl.from_hub ?? -1) >= 0 && pl.from_hub !== pl.hub)
      .map((pl) => ({ from: stop.get(pl.from_hub!)!, to: stop.get(pl.hub)!, pl }));
    return { stops: [...stop.values()], legs, W, H: p.world_h || 1 };
  }, [p]);
}

function TravelsLeaf({ p }: { p: IndividualBrief }) {
  const places = p.places ?? [];
  const road = useRoad(p);
  const [thumb, setThumb] = useState(thumbCache);
  useEffect(() => { if (!thumb) worldThumb().then(setThumb); }, [thumb]);
  const onMap = useUIStore((s) => s.personRoad?.name === p.name);
  const pr = pronouns(p);

  const homes = places.filter((x) => !x.visit);
  const journeys = places.filter((x) => x.visit);
  const totalKm = places.reduce((a, x) => a + (x.km ?? 0) * (x.visit ? 2 : 1), 0);
  const longest = places.reduce<PlaceBrief | null>((a, x) => ((x.km ?? 0) > (a?.km ?? 0) ? x : a), null);
  if (places.length === 0) return <EmptyNote>No road recorded for {p.name}.</EmptyNote>;

  const showOnMap = () => {
    const st = useUIStore.getState();
    if (onMap) { st.setPersonRoad(null); return; }
    const lastHub = places.filter((x) => !x.visit).slice(-1)[0]?.hub ?? places[places.length - 1].hub;
    const homeHub = p.home_hub ?? -1;
    const r: PersonRoad = {
      name: p.name,
      legs: road.legs.map(({ pl }) => ({ ax: pl.from_x ?? 0, ay: pl.from_y ?? 0, bx: pl.x ?? 0, by: pl.y ?? 0, visit: pl.visit })),
      stops: road.stops.map((s) => {
        const pl = places.find((x) => x.hub === s.hub);
        const raw = pl ? { x: pl.x ?? 0, y: pl.y ?? 0 } : (() => { const q = places.find((x) => x.from_hub === s.hub); return { x: q?.from_x ?? 0, y: q?.from_y ?? 0 }; })();
        return { ...raw, city: s.city, n: s.n, home: s.hub === homeHub, last: s.hub === lastHub };
      }),
    };
    st.setPersonRoad(r);
    const focus = r.stops.find((s) => s.last) ?? r.stops[0];
    if (focus) useViewportStore.getState().focusOn(focus.x, focus.y);
  };

  return (
    <div>
      <div style={{ display: "flex", gap: SPACE.lg, background: T.card, border: `1px solid ${T.line}`, borderRadius: RADIUS.md, padding: SPACE.md, marginBottom: SPACE.md, fontSize: FZ.small, color: T.inkMid, flexWrap: "wrap", alignItems: "center" }}>
        <span><b style={{ color: T.ink, fontSize: FZ.head }}>{homes.length}</b> home{homes.length === 1 ? "" : "s"}</span>
        <span><b style={{ color: T.ink, fontSize: FZ.head }}>{journeys.length}</b> journey{journeys.length === 1 ? "" : "s"}</span>
        <span><b style={{ color: T.ink, fontSize: FZ.head }}>{road.stops.length}</b> cities seen</span>
        <span><b style={{ color: T.ink, fontSize: FZ.head }}>{Math.round(totalKm).toLocaleString()}</b> km on the road</span>
        {longest && (longest.km ?? 0) > 0 && <span>longest: {longest.from_city} → {longest.city} ({Math.round(longest.km ?? 0).toLocaleString()} km)</span>}
        <span style={{ flex: 1 }} />
        <button data-no-drag onClick={showOnMap} style={{
          fontSize: FZ.small, cursor: "pointer", borderRadius: RADIUS.pill, padding: "3px 10px", fontWeight: 700,
          color: onMap ? T.panel : GOLD, background: onMap ? GOLD : "transparent", border: `1px solid ${GOLD}`,
        }}>{onMap ? "✓ On the world map — clear" : "🗺 Show on the world map"}</button>
      </div>
      <RoadMap road={road} thumb={thumb} />
      <div style={{ display: "flex", gap: 12, fontSize: FZ.micro, color: T.inkDim, margin: "4px 0 0" }}>
        <span><span style={{ color: GOLD }}>━━</span> moved there to live</span>
        <span><span style={{ color: VISIT }}>╌╌</span> a journey there and back</span>
        <span>◉ home · ● where {pr.they} {p.alive ? "lives now" : "ended"}</span>
      </div>
      <SectionTitle>The itinerary</SectionTitle>
      <div style={{ display: "grid", gridTemplateColumns: "22px 70px 1fr 1.1fr 70px 70px", gap: "3px 8px", fontSize: FZ.tiny, alignItems: "baseline" }}>
        {["#", "Year", "Route", "Why", "Distance", "Stay?"].map((h) => (
          <span key={h} style={{ color: T.inkDim, fontSize: FZ.micro, textTransform: "uppercase", letterSpacing: 0.4, borderBottom: `1px solid ${T.line}` }}>{h}</span>
        ))}
        {places.map((pl, i) => (
          <Row key={i} i={i + 1} pl={pl} />
        ))}
      </div>
    </div>
  );
}

/** The reason in words, without doubling the verb ("went to study to study"). */
function whyWent(pl: PlaceBrief): string {
  const r = pl.reason && pl.reason !== "entering public life" ? pl.reason : "";
  if (pl.how === "visit") return `journeyed ${r || "abroad"}`;
  if (pl.how === "settled") return `settled ${r}`.trim();
  return r || (HOW_WORD[pl.how] ?? pl.how);
}

function Row({ i, pl }: { i: number; pl: PlaceBrief }) {
  const c = pl.visit ? VISIT : GOLD;
  return (
    <>
      <span style={{ color: c, fontWeight: 700 }}>{i}</span>
      <span style={{ color: T.inkMid, fontVariantNumeric: "tabular-nums" }}>{pl.year} <span style={{ color: T.inkFaint }}>({pl.age})</span></span>
      <span style={{ color: T.ink }}>
        <span aria-hidden>{HOW_ICON[pl.how] ?? "•"} </span>
        {pl.from_city ? <>{pl.from_city} <span style={{ color: c }}>{pl.visit ? "⇄" : "→"}</span> </> : null}<b>{pl.city}</b>
      </span>
      <span style={{ color: T.inkMid }}>{whyWent(pl)}</span>
      <span style={{ color: T.inkMid, fontVariantNumeric: "tabular-nums" }}>{(pl.km ?? 0) > 0 ? `${Math.round(pl.km ?? 0).toLocaleString()} km` : "—"}</span>
      <span style={{ color: (pl.stay_odds ?? -1) >= 0 ? (pl.visit ? T.inkDim : GOLD) : T.inkFaint }}>
        {(pl.stay_odds ?? -1) >= 0 ? `${Math.round((pl.stay_odds ?? 0) * 100)}% → ${pl.visit ? "went back" : "stayed"}` : "—"}
      </span>
    </>
  );
}

function RoadMap({ road, thumb }: { road: ReturnType<typeof useRoad>; thumb: { url: string; w: number; h: number } | null }) {
  const { stops, legs, W, H } = road;
  const VW = 740, VH = 300;
  // Frame the road with padding, at the box's own aspect ratio.
  const xs = stops.map((s) => s.x), ys = stops.map((s) => s.y);
  let minX = Math.min(...xs), maxX = Math.max(...xs), minY = Math.min(...ys), maxY = Math.max(...ys);
  const minSpan = W * 0.08;
  let sw = Math.max(maxX - minX, minSpan) * 1.35, sh = Math.max(maxY - minY, minSpan * VH / VW) * 1.35;
  if (sw / sh > VW / VH) sh = sw * VH / VW; else sw = sh * VW / VH;
  sw = Math.min(sw, W * 1.2); sh = Math.min(sh, H);
  const cx = (minX + maxX) / 2, cy = Math.min(Math.max((minY + maxY) / 2, sh / 2), H - sh / 2);
  minX = cx - sw / 2; minY = cy - sh / 2; maxX = cx + sw / 2; maxY = cy + sh / 2;
  const r = sw / VW * 7;
  const fs = sw / VW * 10;
  const lastHub = legs.length ? legs.filter((l) => !l.pl.visit).slice(-1)[0]?.to : stops[0];
  return (
    <svg viewBox={`${minX} ${minY} ${sw} ${sh}`} width="100%" style={{ display: "block", aspectRatio: `${VW} / ${VH}`, background: "#0e2436", borderRadius: RADIUS.md, border: `1px solid ${T.line}` }}>
      {thumb && [-W, 0, W].map((off) => (
        <image key={off} href={thumb.url} x={off} y={0} width={W} height={H} preserveAspectRatio="none" opacity={0.75} style={{ imageRendering: "pixelated" }} />
      ))}
      <defs>
        <marker id="pw-arrow-m" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
          <path d="M0,0 L10,5 L0,10 z" fill={GOLD} />
        </marker>
        <marker id="pw-arrow-v" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
          <path d="M0,0 L10,5 L0,10 z" fill={VISIT} />
        </marker>
      </defs>
      {legs.map(({ from, to, pl }, i) => {
        // A gentle arc so a there-and-back journey and a later move along the
        // same pair don't sit on top of each other.
        const mx = (from.x + to.x) / 2, my = (from.y + to.y) / 2;
        const dx = to.x - from.x, dy = to.y - from.y, len = Math.hypot(dx, dy) || 1;
        const bend = (pl.visit ? 0.22 : 0.1) * len;
        const qx = mx - dy / len * bend, qy = my + dx / len * bend;
        const c = pl.visit ? VISIT : GOLD;
        return (
          <path key={i} d={`M${from.x + 0.5},${from.y + 0.5} Q${qx + 0.5},${qy + 0.5} ${to.x + 0.5},${to.y + 0.5}`}
            fill="none" stroke={c} strokeWidth={sw / VW * (pl.visit ? 1.6 : 2.6)} strokeDasharray={pl.visit ? `${r * 0.6} ${r * 0.5}` : undefined}
            markerEnd={`url(#pw-arrow-${pl.visit ? "v" : "m"})`} opacity={0.95}>
            <title>{`${pl.year} (age ${pl.age}) · ${pl.from_city} → ${pl.city} · ${pl.reason ?? ""}${(pl.km ?? 0) > 0 ? ` · ${Math.round(pl.km ?? 0)} km` : ""}`}</title>
          </path>
        );
      })}
      {stops.map((s) => {
        const isLast = !!lastHub && s.hub === lastHub.hub;
        return (
          <g key={s.hub}>
            <circle cx={s.x + 0.5} cy={s.y + 0.5} r={r} fill={isLast ? GOLD : "#1a1410"} stroke={s.n === 1 ? "#fff4d6" : GOLD} strokeWidth={r * (s.n === 1 ? 0.35 : 0.2)} />
            <text x={s.x + 0.5} y={s.y + 0.5 + fs * 0.35} fontSize={fs} textAnchor="middle" fill={isLast ? "#1a1410" : "#fff4d6"} fontWeight={700}>{s.n}</text>
            <text x={s.x + 0.5 + r * 1.4} y={s.y + 0.5 + fs * 0.35} fontSize={fs * 1.05} fill="#fff4d6" stroke="#0a0806" strokeWidth={fs * 0.28} paintOrder="stroke" fontFamily="Georgia, serif">{s.city}</text>
          </g>
        );
      })}
    </svg>
  );
}

// ── Character ────────────────────────────────────────────────────────────

function CharacterLeaf({ p }: { p: IndividualBrief }) {
  const pr = pronouns(p);
  const open = (id: number) => useUIStore.getState().setPersonWindow(id);
  // One line per change: a dilemma and the milestone it caused are one event.
  const seen = new Set<string>();
  const arc = (p.life ?? []).filter((e) => ["gained", "deepened", "overturned", "weakened"].includes(e.change ?? "") || (e.kind !== "choice" && e.gained))
    .filter((e) => { const k = `${e.year}|${e.lost ?? ""}|${e.gained}`; if (seen.has(k)) return false; seen.add(k); return true; });
  const links = p.links ?? [];
  return (
    <div>
      <SectionTitle>Nature</SectionTitle>
      {p.traits.length === 0 && <EmptyNote>No marked traits.</EmptyNote>}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: SPACE.sm }}>
        {p.traits.map((t, i) => {
          const m = TRAIT_META[t];
          const st = p.trait_strength?.[i] ?? 1;
          return (
            <div key={t} style={{ display: "flex", gap: 8, alignItems: "flex-start", border: `1px solid ${traitColor(t)}44`, borderRadius: RADIUS.sm, padding: SPACE.sm, background: `${traitColor(t)}0c` }}>
              <span style={{ fontSize: 20, lineHeight: 1 }}>{m?.icon ?? "•"}</span>
              <div>
                <div style={{ color: traitColor(t), fontWeight: 700, fontSize: FZ.small }}>{st >= 2 ? "Deeply " : ""}{t}</div>
                <div style={{ color: T.inkMid, fontSize: FZ.tiny }}>{m?.blurb ?? ""}</div>
                <div style={{ color: T.inkDim, fontSize: FZ.micro, marginTop: 2 }}>{p.trait_origins?.[i] ?? ""}</div>
              </div>
            </div>
          );
        })}
      </div>
      {arc.length > 0 && (
        <>
          <SectionTitle>How life shaped {pr.them}</SectionTitle>
          <div style={{ borderLeft: `2px solid ${T.gold}44`, marginLeft: 4, paddingLeft: 10 }}>
            {arc.map((e, i) => (
              <div key={i} style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap", padding: "3px 0", fontSize: FZ.tiny }}>
                <span style={{ color: T.inkFaint, minWidth: 64, fontVariantNumeric: "tabular-nums" }}>{e.year} · age {e.age}</span>
                {e.change === "overturned" && e.lost ? <><TraitChip name={e.lost} /> <span style={{ color: T.inkDim }}>→</span> <TraitChip name={e.gained} /></>
                  : e.change === "weakened" ? <><TraitChip name={e.lost ?? ""} /> <span style={{ color: T.inkDim }}>shaken</span></>
                  : <><span style={{ color: T.inkDim }}>{e.change === "deepened" ? "grew deeply" : "became"}</span> <TraitChip name={e.gained} strength={e.change === "deepened" ? 2 : 1} /></>}
                <span style={{ color: T.inkMid }}>{e.choice ? `— chose “${e.choice}”` : `— ${e.text}`}</span>
              </div>
            ))}
          </div>
        </>
      )}
      <SectionTitle right={p.ideology_name ? <Tag color="#8a9ad8">⚑ leans toward {p.ideology_name}</Tag> : undefined}>Beliefs</SectionTitle>
      {IDEOLOGY_AXES.map((a, i) => {
        const v = Math.max(-5, Math.min(5, p.ideology?.[i] ?? 0));
        return (
          <div key={a.name} style={{ display: "grid", gridTemplateColumns: "80px 90px 1fr 90px 36px", gap: 6, alignItems: "center", fontSize: FZ.tiny, marginBottom: 4 }}>
            <span style={{ color: a.color, fontWeight: 700 }}>{a.name}</span>
            <span style={{ color: v < 0 ? T.ink : T.inkFaint, textAlign: "right" }}>{a.neg}</span>
            <div style={{ position: "relative", height: 8, background: T.lineSoft, borderRadius: 4 }}>
              <div style={{ position: "absolute", left: "50%", top: -2, bottom: -2, width: 1, background: T.inkFaint }} />
              <div style={{ position: "absolute", top: 0, bottom: 0, borderRadius: 4, background: a.color,
                left: v >= 0 ? "50%" : `${50 + v * 10}%`, width: `${Math.abs(v) * 10}%` }} />
            </div>
            <span style={{ color: v > 0 ? T.ink : T.inkFaint }}>{a.pos}</span>
            <span style={{ color: T.inkMid, textAlign: "right", fontVariantNumeric: "tabular-nums" }}>{v > 0 ? "+" : ""}{v.toFixed(1)}</span>
          </div>
        );
      })}
      {(p.modifiers ?? []).length > 0 && (
        <>
          <SectionTitle>Going through</SectionTitle>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
            {(p.modifiers ?? []).map((m, i) => (
              <Tag key={i} color={T.warn}>⏳ {m.name}{m.note ? ` — ${m.note}` : ""} · {m.years_left < 1 ? `${Math.max(1, Math.round(m.years_left * 12))} mo` : `${m.years_left.toFixed(1)} y`} left</Tag>
            ))}
          </div>
        </>
      )}
      {(links.length > 0 || (p.talent ?? 0) > 0) && (
        <>
          <SectionTitle>Ties</SectionTitle>
          <div style={{ display: "flex", flexDirection: "column", gap: 3, fontSize: FZ.small }}>
            {links.map((l, i) => (
              <div key={i}>
                <span style={{ color: T.inkDim, display: "inline-block", minWidth: 64 }}>{l.kind === "teacher" ? "📚 teacher" : l.kind === "student" ? "🎓 student" : l.kind === "house" ? "⚜ house" : l.kind}</span>
                <span data-no-drag style={{ color: T.accent, cursor: "pointer", textDecoration: "underline dotted" }}
                  onClick={() => l.kind === "house" ? useUIStore.getState().setDossierHouse(l.id) : open(l.id)}>{l.name}</span>
                {!l.alive && <span style={{ color: T.inkFaint }}> ✝</span>}
              </div>
            ))}
            {(p.talent ?? 0) > 0 && <div><span style={{ color: T.inkDim, display: "inline-block", minWidth: 64 }}>🎨 talent</span>{Math.round((p.talent ?? 0) * 100)} / 100</div>}
          </div>
        </>
      )}
    </div>
  );
}

/** A small "open the full life" button any person view can carry. */
export function OpenLifeButton({ id, label = "📖 Full life" }: { id: number | null | undefined; label?: string }) {
  if (id == null || id < 0) return null;
  return (
    <button data-no-drag onClick={(e) => { e.stopPropagation(); useUIStore.getState().setPersonWindow(id); }}
      title="Open this person's full life: decisions, travels and character"
      style={{ fontSize: FZ.micro, cursor: "pointer", borderRadius: RADIUS.pill, padding: "1px 8px", color: T.gold, background: "transparent", border: `1px solid ${T.gold}88`, whiteSpace: "nowrap" }}>
      {label}
    </button>
  );
}
