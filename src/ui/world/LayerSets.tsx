import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import type { ActiveLayer } from "@types";

/** MY LAYER SETS (GENERATOR_AND_GOODS_REDESIGN.md §13) — the user's own saved
 *  views: base layer + every overlay switch + elevation style. A plate is a view
 *  the app ships; a layer set is one the user made. Stored per user in
 *  localStorage (an appearance convenience, like label typography), never in the
 *  world file. */
interface LayerSet {
  name: string;
  layer: ActiveLayer;
  overlays: Record<string, boolean>;
  elevationStyle: string | null;
}

const KEY = "wf2.layerSets";

function load(): LayerSet[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) || "[]");
    return Array.isArray(v) ? v : [];
  } catch { return []; }
}
function store(sets: LayerSet[]) {
  try { localStorage.setItem(KEY, JSON.stringify(sets)); } catch { /* storage blocked: sets live for this session */ }
}

export function LayerSets() {
  const [sets, setSets] = useState<LayerSet[]>(load);
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState("");

  const save = () => {
    const n = name.trim();
    if (!n) return;
    const s = useUIStore.getState();
    const entry: LayerSet = {
      name: n, layer: s.activeLayer,
      overlays: { ...s.overlayVisibility },
      elevationStyle: s.elevationStyle,
    };
    const next = [...sets.filter((x) => x.name !== n), entry];
    setSets(next); store(next); setNaming(false); setName("");
  };
  // Saved switches are laid OVER the current ones, so an overlay added to the app
  // after the set was saved keeps its present state instead of vanishing.
  const apply = (x: LayerSet) =>
    useUIStore.setState((s) => ({
      activeLayer: x.layer,
      overlayVisibility: { ...s.overlayVisibility, ...x.overlays },
      borrowedOverlays: [],
      elevationStyle: x.elevationStyle,
      activeMapTheme: null,
    }));
  const remove = (n: string) => { const next = sets.filter((x) => x.name !== n); setSets(next); store(next); };

  return (
    <div style={{ marginTop: 6 }}>
      <div style={{ fontSize: 9, color: "#897a61", textTransform: "uppercase", letterSpacing: 0.6, marginBottom: 3 }}>My layer sets</div>
      {sets.length === 0 && !naming && (
        <div style={{ fontSize: 9.5, color: "#6e6250", marginBottom: 3 }}>Set up the map the way you like it, then save it here.</div>
      )}
      {sets.map((x) => (
        <div key={x.name} style={{ display: "flex", gap: 3, marginBottom: 2 }}>
          <button onClick={() => apply(x)} title={`Show “${x.name}”`}
            style={{ flex: 1, minWidth: 0, textAlign: "left", padding: "3px 6px", borderRadius: 4, fontSize: 10.5, cursor: "pointer",
              border: "1px solid #453925", background: "#221c12", color: "#ac9c81",
              whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{x.name}</button>
          <button onClick={() => remove(x.name)} aria-label={`Delete layer set ${x.name}`} title="Delete"
            style={{ padding: "0 6px", borderRadius: 4, fontSize: 10, cursor: "pointer", border: "1px solid #453925", background: "#1b1610", color: "#8a7a5e" }}>×</button>
        </div>
      ))}
      {naming ? (
        <form onSubmit={(e) => { e.preventDefault(); save(); }} style={{ display: "flex", gap: 3 }}>
          <input autoFocus value={name} onChange={(e) => setName(e.target.value)} placeholder="Name this view"
            aria-label="Layer set name"
            style={{ flex: 1, minWidth: 0, fontSize: 10.5, padding: "3px 5px", borderRadius: 4, border: "1px solid #5d4a2a", background: "#14110d", color: "#ecdcb5" }} />
          <button type="submit" disabled={!name.trim()}
            style={{ padding: "0 7px", borderRadius: 4, fontSize: 10, cursor: "pointer", border: "1px solid #c09b5a", background: "#4e4028", color: "#f0dcaa" }}>Save</button>
        </form>
      ) : (
        <button onClick={() => setNaming(true)}
          style={{ width: "100%", padding: "3px 6px", borderRadius: 4, fontSize: 10.5, cursor: "pointer",
            border: "1px dashed #5d4a2a", background: "transparent", color: "#ac9c81" }}>+ Save current view</button>
      )}
    </div>
  );
}
