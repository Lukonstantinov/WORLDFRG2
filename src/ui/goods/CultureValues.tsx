import { useEffect, useState } from "react";
import { getCultureValues, setCultureValues } from "@bridge";
import type { GoodSpec, CultureValue, CultureValueProfile, ValueKind } from "@types";
import { GoodIcon } from "./GoodIcon";

const KINDS: { kind: ValueKind; label: string; hint: string }[] = [
  { kind: "prestige", label: "Prestige", hint: "the rich pay more for it and display it" },
  { kind: "sacred", label: "Sacred", hint: "steady demand, bought in famine years too" },
  { kind: "staple", label: "Staple", hint: "their own staple, preferred over its substitutes" },
  { kind: "taboo", label: "Taboo", hint: "not bought at all where this people lives" },
  { kind: "currency", label: "Currency", hint: "the treasure metal they mint and hoard (gold or silver)" },
];
const KIND_COLOR: Record<ValueKind, string> = {
  prestige: "#d8b24a", sacred: "#b08ad8", staple: "#8ac070", taboo: "#d86a5a", currency: "#c8ccd6",
};

/** Goods Builder · Culture values (plan §9.2, G7): each people's few goods it values
 *  its own way. Derived from its traits and tastes by default; set by hand here. A
 *  hand-set profile replaces the derived one for that people. Read at campaign start. */
export function CultureValues({ specs }: { specs: GoodSpec[] }) {
  const [profiles, setProfiles] = useState<CultureValueProfile[] | null>(null);
  const [custom, setCustom] = useState<Record<string, CultureValue[]>>({});
  const [status, setStatus] = useState("");
  useEffect(() => {
    getCultureValues().then((p) => {
      setProfiles(p);
      setCustom(Object.fromEntries(p.filter((x) => x.custom.length).map((x) => [x.culture, x.custom])));
    }).catch((e) => setStatus(String(e)));
  }, []);

  if (!profiles) return <div style={{ color: "#9d917d", padding: 12 }}>Loading peoples… {status}</div>;
  if (profiles.length === 0) return <div style={{ color: "#9d917d", padding: 12 }}>No peoples yet — run the People stage first.</div>;

  const goods = specs.filter((g) => g.enabled);
  const nameOf = (id: string) => specs.find((g) => g.id === id)?.name ?? id;
  const colorOf = (id: string) => specs.find((g) => g.id === id)?.color ?? "#9a8a70";
  const save = async () => {
    try { await setCultureValues(Object.values(custom).flat()); setStatus("Saved — new campaigns use these values"); }
    catch (e) { setStatus(`Error: ${e}`); }
  };
  const edit = (culture: string, rows: CultureValue[] | null) => setCustom((c) => {
    const next = { ...c };
    if (rows) next[culture] = rows; else delete next[culture];
    return next;
  });

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 8, color: "#9d917d" }}>
        <span style={{ flex: 1 }}>Each people values a few goods its own way. Derived from its traits by default; set any people by hand.</span>
        <button onClick={save} style={{ background: "#8d7448", color: "#fff", border: "1px solid #a88a57", borderRadius: 4, padding: "4px 9px", cursor: "pointer" }}>Save values</button>
      </div>
      <div style={{ color: "#9d917d", minHeight: 14, marginBottom: 4 }}>{status}</div>
      {profiles.map((p) => {
        const mine = custom[p.culture];
        const rows = mine ?? p.auto;
        return (
          <div key={p.culture} style={{ borderBottom: "1px solid #342e25", padding: "6px 0" }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <b style={{ color: "#d8c49a" }}>{p.culture}</b>
              <span style={{ color: "#8a7d66" }}>{p.traits.join(" · ")}</span>
              <span style={{ flex: 1 }} />
              <span style={{ color: mine ? "#f0c070" : "#8a7d66" }}>{mine ? "set by hand" : "derived"}</span>
              {mine
                ? <button onClick={() => edit(p.culture, null)} title="Drop the hand-set values; use the derived profile">Use derived</button>
                : <button onClick={() => edit(p.culture, p.auto.slice())} title="Start from the derived profile and edit it">Set by hand</button>}
            </div>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 4 }}>
              {rows.length === 0 && <span style={{ color: "#6f6553" }}>no special values</span>}
              {rows.map((v, k) => (
                <span key={k} style={{ display: "inline-flex", alignItems: "center", gap: 4, padding: "1px 6px", borderRadius: 10, border: `1px solid ${KIND_COLOR[v.kind]}`, color: KIND_COLOR[v.kind] }}>
                  <GoodIcon name={v.good} color={colorOf(v.good)} size={14} />
                  {mine ? (
                    <>
                      <select value={v.good} onChange={(e) => edit(p.culture, rows.map((r, j) => j === k ? { ...r, good: e.target.value } : r))}>
                        {goods.map((g) => <option key={g.id} value={g.id}>{g.name}</option>)}
                      </select>
                      <select value={v.kind} onChange={(e) => edit(p.culture, rows.map((r, j) => j === k ? { ...r, kind: e.target.value as ValueKind } : r))}>
                        {KINDS.map((x) => <option key={x.kind} value={x.kind} title={x.hint}>{x.label}</option>)}
                      </select>
                      <button onClick={() => edit(p.culture, rows.filter((_, j) => j !== k))} title="Remove">✕</button>
                    </>
                  ) : <span title={KINDS.find((x) => x.kind === v.kind)?.hint}>{nameOf(v.good)} · {v.kind}</span>}
                </span>
              ))}
              {mine && rows.length < 6 && goods.length > 0 && (
                <button onClick={() => edit(p.culture, [...rows, { culture: p.culture, good: goods[0].id, kind: "prestige" }])}>+ value</button>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
