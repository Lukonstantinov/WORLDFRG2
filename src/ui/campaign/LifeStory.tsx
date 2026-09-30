import { useMemo, useState } from "react";
import type { LifeEntryBrief, PlaceBrief } from "@types";
import { T, FZ, SERIF, RADIUS } from "@ui/campaign/chronicleTheme";
import { TraitChip } from "@ui/campaign/traitIcons";

/** A person's life as a STRUCTURED story (2026-09-30): the choices they made
 *  (and why), the milestones of their career, and the ordinary happenings in
 *  between — grouped into stages of life, filterable to the KEY moments, with
 *  their road (every city they lived in or visited) drawn as a strip on top.
 *  Shared by Notable Figures, the council chamber and every person chip. */

const STAGES: { name: string; from: number; to: number }[] = [
  { name: "Youth", from: 0, to: 29 },
  { name: "Prime", from: 30, to: 49 },
  { name: "Maturity", from: 50, to: 64 },
  { name: "Old age", from: 65, to: 999 },
];

const KIND_ICON: Record<string, string> = { choice: "⚖", milestone: "✦", event: "·" };
const HOW_ICON: Record<string, string> = { debut: "★", study: "📚", exile: "⛓", return: "🏠", settled: "⌂", visit: "↗" };
const HOW_WORD: Record<string, string> = { debut: "began", study: "studied", exile: "exiled to", return: "came home", settled: "settled", visit: "visited" };

export function LifeRoad({ places, color = T.gold }: { places: PlaceBrief[]; color?: string }) {
  if (!places || places.length === 0) return null;
  const homes = places.filter((p) => !p.visit);
  const visits = places.filter((p) => p.visit).length;
  return (
    <div style={{ marginBottom: 6 }}>
      <div style={{ fontSize: FZ.micro, color: T.inkDim, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 3 }}>
        The road · {homes.length} home{homes.length === 1 ? "" : "s"}{visits > 0 ? ` · ${visits} journey${visits === 1 ? "" : "s"}` : ""}
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 2, rowGap: 4 }}>
        {places.map((p, i) => (
          <span key={i} style={{ display: "inline-flex", alignItems: "center", gap: 2 }}>
            {i > 0 && <span style={{ color: T.inkFaint, fontSize: FZ.tiny, margin: "0 2px" }}>{p.visit ? "⇢" : "→"}</span>}
            <span title={`${HOW_WORD[p.how] ?? p.how} ${p.city} · year ${p.year}, age ${p.age}`}
              style={{
                display: "inline-flex", alignItems: "center", gap: 3,
                fontSize: FZ.tiny, lineHeight: 1.5, padding: "0 6px", borderRadius: RADIUS.pill,
                color: p.visit ? T.inkMid : T.ink,
                background: p.visit ? "transparent" : `${color}1c`,
                border: `1px ${p.visit ? "dashed" : "solid"} ${p.visit ? T.line : `${color}66`}`,
              }}>
              <span aria-hidden>{HOW_ICON[p.how] ?? "•"}</span>
              <span>{p.city}</span>
              <span style={{ color: T.inkFaint, fontSize: FZ.micro }}>{p.year}</span>
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}

export function LifeStory({ life, places, color = T.gold, fallback }: {
  life?: LifeEntryBrief[];
  places?: PlaceBrief[];
  color?: string;
  /** Plain text lines to show when no structured life was served (old save). */
  fallback?: string[];
}) {
  const entries = life ?? [];
  const keyCount = entries.filter((e) => e.key).length;
  const [keyOnly, setKeyOnly] = useState(true);
  const shown = useMemo(() => (keyOnly && keyCount > 0 ? entries.filter((e) => e.key) : entries), [entries, keyOnly, keyCount]);
  const groups = useMemo(() => {
    const out: { stage: string; items: LifeEntryBrief[] }[] = [];
    for (const e of shown) {
      const st = STAGES.find((s) => e.age >= s.from && e.age <= s.to)?.name ?? "Old age";
      const last = out[out.length - 1];
      if (last && last.stage === st) last.items.push(e); else out.push({ stage: st, items: [e] });
    }
    return out;
  }, [shown]);

  if (entries.length === 0) {
    if (fallback && fallback.length > 0) {
      return (
        <div>
          {fallback.map((l, i) => (
            <div key={i} style={{ color: T.inkMid, fontSize: FZ.small, padding: "2px 0", borderTop: i ? `1px solid ${T.lineSoft}` : "none" }}>{l}</div>
          ))}
        </div>
      );
    }
    return <div style={{ color: T.inkDim, fontSize: FZ.tiny, fontStyle: "italic" }}>A quiet life, so far.</div>;
  }

  const choices = entries.filter((e) => e.kind === "choice").length;
  return (
    <div data-no-drag>
      <LifeRoad places={places ?? []} color={color} />
      <div style={{ display: "flex", alignItems: "center", gap: 4, marginBottom: 4 }}>
        {(["key", "all"] as const).map((m) => {
          const on = (m === "key") === keyOnly;
          return (
            <button key={m} onClick={() => setKeyOnly(m === "key")}
              style={{
                fontSize: FZ.micro, cursor: "pointer", borderRadius: RADIUS.pill, padding: "1px 7px",
                color: on ? T.panel : T.inkMid, background: on ? color : "transparent",
                border: `1px solid ${on ? color : T.line}`, fontWeight: on ? 700 : 400,
              }}>
              {m === "key" ? `✦ Key moments (${keyCount})` : `Whole life (${entries.length})`}
            </button>
          );
        })}
        <span style={{ flex: 1 }} />
        <span style={{ fontSize: FZ.micro, color: T.inkDim }}>⚖ {choices} choice{choices === 1 ? "" : "s"}</span>
      </div>
      {groups.map((g, gi) => (
        <div key={gi} style={{ marginTop: gi ? 6 : 2 }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6, marginBottom: 2 }}>
            <span style={{ fontFamily: SERIF, color, fontSize: FZ.body, fontWeight: 700 }}>{g.stage}</span>
            <span style={{ fontSize: FZ.micro, color: T.inkFaint }}>
              ages {g.items[0].age}–{g.items[g.items.length - 1].age} · {g.items[0].year}–{g.items[g.items.length - 1].year}
            </span>
            <span style={{ flex: 1, borderBottom: `1px solid ${T.lineSoft}` }} />
          </div>
          <div style={{ borderLeft: `2px solid ${color}44`, marginLeft: 4, paddingLeft: 8 }}>
            {g.items.map((e, i) => <LifeLine key={i} e={e} color={color} />)}
          </div>
        </div>
      ))}
    </div>
  );
}

function LifeLine({ e, color }: { e: LifeEntryBrief; color: string }) {
  const isChoice = e.kind === "choice";
  const isMs = e.kind === "milestone";
  return (
    <div style={{ position: "relative", padding: "3px 0" }}>
      <span style={{
        position: "absolute", left: -15, top: 4, width: 12, height: 12, borderRadius: "50%",
        display: "grid", placeItems: "center", fontSize: 8,
        background: isChoice ? color : isMs ? T.raised : T.panel,
        color: isChoice ? T.panel : isMs ? color : T.inkDim,
        border: `1px solid ${isChoice || isMs ? color : T.line}`,
      }}>{KIND_ICON[e.kind] ?? "·"}</span>
      <div style={{ display: "flex", gap: 6, alignItems: "baseline" }}>
        <span style={{ fontSize: FZ.micro, color: T.inkFaint, fontVariantNumeric: "tabular-nums", flex: "0 0 auto", minWidth: 42 }}>
          {e.year} · {e.age}
        </span>
        <span style={{ fontSize: FZ.small, lineHeight: 1.4, color: isChoice || isMs ? T.ink : T.inkMid, fontWeight: isMs ? 600 : 400 }}>
          {e.text}
        </span>
      </div>
      {(isChoice || e.gained) && (
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 4, marginLeft: 48, marginTop: 2 }}>
          {isChoice && (
            <span style={{ fontSize: FZ.micro, color }}>
              chose to <b>{e.choice}</b>
              {e.why.length > 0 && <span style={{ color: T.inkDim }}> — because {e.why.join(", ")}</span>}
            </span>
          )}
          {e.gained && <span style={{ fontSize: FZ.micro, color: T.inkDim }}>→</span>}
          {e.gained && <TraitChip name={e.gained} />}
        </div>
      )}
    </div>
  );
}
