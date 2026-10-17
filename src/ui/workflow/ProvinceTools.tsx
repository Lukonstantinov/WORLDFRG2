import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore, decodeProvinceRaster } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import {
  simGenerateProvinces, simMergeSmallProvinces, simSplitLargeProvinces, simMergeSelectedProvinces,
  simCheckProvinces, simFixProvinceFragments,
} from "@bridge";
import type { ProvinceIssue, SimProvincesResult } from "@types";
import { genBtn } from "@ui/workflow/workflowStyles";

/** The Provinces box of the Settlements step: generate the partition, then shape it by
 *  hand — select provinces on the map (several at once), merge them into one, split
 *  them into smaller ones, repaint a border, and check the borders for cut-off pieces.
 *  Every line wraps inside the narrow step panel (nothing runs off to the right). */
export function ProvinceTools() {
  const { rivers, settlements, setSettlements, setProvinces, provinces } = useWorldStore();
  const simRunning = useUIStore((s) => s.simRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const marked = useUIStore((s) => s.markedProvinces);
  const toggleMarked = useUIStore((s) => s.toggleMarkedProvince);
  const clearMarked = useUIStore((s) => s.clearMarkedProvinces);
  const selectMode = useUIStore((s) => s.provinceSelectMode);
  const setSelectMode = useUIStore((s) => s.setProvinceSelectMode);
  const activeTool = useUIStore((s) => s.activeTool);
  const setTool = useUIStore((s) => s.setTool);
  const editTarget = useUIStore((s) => s.provinceEditTarget);
  const setEditTarget = useUIStore((s) => s.setProvinceEditTarget);
  const brush = useUIStore((s) => s.brushRadius);
  const setBrush = useUIStore((s) => s.setBrushRadius);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const provincesVisible = useUIStore((s) => s.overlayVisibility.provinces ?? false);
  const focusOn = useViewportStore((s) => s.focusOn);

  const [granularity, setGranularity] = useState(0.5);
  const [pieces, setPieces] = useState(2);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState("");
  const [issues, setIssues] = useState<ProvinceIssue[] | null>(null);

  const nameOf = (id: number) => provinces.find((p) => p.id === id)?.name ?? `#${id}`;
  const apply = (res: SimProvincesResult) => { setProvinces(res.provinces, decodeProvinceRaster(res)); clearMarked(); setIssues(null); };
  const run = async (label: string, fn: () => Promise<string>) => {
    if (busy || simRunning) return;
    setBusy(true); setMsg(label + "…");
    try { setMsg(await fn()); } catch (e) { setMsg(`Failed: ${e}`); }
    setBusy(false);
  };

  const generate = () => run("Partitioning the land into provinces", async () => {
    if (settlements.length === 0) return "Find settlements first — provinces seed from them.";
    const res = await simGenerateProvinces(settlements, rivers, granularity);
    apply(res);
    const founded = res.founded_settlements ?? [];
    if (founded.length > 0) setSettlements([...settlements, ...founded]);
    useUIStore.getState().borrowOverlay("provinces");
    return `Generated ${res.provinces.length} provinces${founded.length ? ` (founded ${founded.length} new seat${founded.length === 1 ? "" : "s"})` : ""}.`;
  });
  const mergeSelected = () => run(`Merging ${marked.length} provinces`, async () => {
    const before = provinces.length;
    apply(await simMergeSelectedProvinces(marked));
    return `Merged ${marked.length} provinces into one (${before} → ${useWorldStore.getState().provinces.length}).`;
  });
  const splitSelected = () => run(`Splitting ${marked.length} provinces into ${pieces}`, async () => {
    const cells = marked.map((id) => provinces.find((p) => p.id === id)?.cells ?? 0).filter((c) => c > 0);
    // The splitter cuts anything over `maxCells`; the smallest selected province / pieces
    // makes every selected one come out at least `pieces` times smaller.
    const maxCells = Math.max(2, Math.floor(Math.min(...cells) / pieces));
    const before = provinces.length;
    const res = await simSplitLargeProvinces(maxCells, rivers, marked);
    apply(res);
    return res.provinces.length > before ? `Split into ${res.provinces.length - before} more provinces.` : "Those provinces are too small to split.";
  });
  const mergeSmall = () => run("Merging slivers", async () => {
    const before = provinces.length;
    const res = await simMergeSmallProvinces(undefined, undefined);
    apply(res);
    return before > res.provinces.length ? `Merged ${before - res.provinces.length} small provinces away.` : "No small provinces to merge.";
  });
  const splitLarge = () => run("Splitting oversized provinces", async () => {
    const before = provinces.length;
    const res = await simSplitLargeProvinces(undefined, rivers, undefined);
    apply(res);
    return res.provinces.length > before ? `Split into ${res.provinces.length - before} more provinces.` : "No oversized provinces to split.";
  });
  const check = () => run("Checking borders", async () => {
    const found = await simCheckProvinces();
    setIssues(found);
    return found.length === 0 ? "Borders look correct: every province is one connected body." : `${found.length} thing${found.length === 1 ? "" : "s"} to look at (listed below).`;
  });
  const repair = () => run("Repairing cut-off pieces", async () => {
    const r = await simFixProvinceFragments();
    apply(r.result);
    return r.moved > 0 ? `Handed ${r.moved} cut-off piece${r.moved === 1 ? "" : "s"} to the neighbour they touch.` : "Nothing to repair.";
  });

  const editing = activeTool === "provinceBrush";
  const startEdit = () => {
    setEditTarget(marked[marked.length - 1]);
    setSelectMode(false);
    setOverlayVisible("provinces", true);
    setTool("provinceBrush");
  };
  const stopEdit = () => { setTool("select"); setEditTarget(null); };
  const toggleSelect = () => {
    const on = !selectMode;
    setSelectMode(on);
    if (on) { setOverlayVisible("provinces", true); if (editing) stopEdit(); setTool("select"); }
  };

  const small: React.CSSProperties = { color: "#6f8f6f", fontSize: 9, lineHeight: 1.4, overflowWrap: "anywhere" };
  const btn = (on?: boolean, off?: boolean): React.CSSProperties => ({
    ...genBtn, marginBottom: 0, textAlign: "center", fontSize: 10, padding: "4px 6px",
    background: on ? "#3f5f2f" : "#183024", color: off ? "#587058" : "#a8dcb8", opacity: off ? 0.55 : 1,
    cursor: off ? "default" : "pointer", whiteSpace: "normal",
  });
  const sec: React.CSSProperties = { color: "#8fbf9f", fontSize: 10, fontWeight: 600, marginTop: 4 };

  return (
    <div style={{ marginTop: 6, padding: "6px 7px", background: "#0e1a12", borderRadius: 4, border: "1px solid #1a3020",
      display: "flex", flexDirection: "column", gap: 5, minWidth: 0, overflowWrap: "anywhere" }}>
      <div style={{ color: "#6a9a6a", fontSize: 10, fontWeight: 600 }}>🗺 Provinces — partition the land</div>
      <div style={small}>
        Borders follow rivers, lakes and mountain ranges. Provinces seed from the settlements and are rebuilt after Goods &amp; Trade (step 8).
      </div>

      <div>
        <div style={{ display: "flex", justifyContent: "space-between", gap: 6, color: "#8a7d66", fontSize: 10 }}>
          <span>Province size</span>
          <span style={{ color: "#ac9d84" }}>{granularity < 0.34 ? "few · large" : granularity > 0.66 ? "many · small" : "balanced"}</span>
        </div>
        <input type="range" min={0} max={1} step={0.05} value={granularity}
          onChange={(e) => setGranularity(parseFloat(e.target.value))} style={{ width: "100%" }} />
        <div style={small}>Right = smaller, more numerous. Size also scales with the land (bigger in polar, desert and mountain country).</div>
      </div>
      <button onClick={generate} disabled={busy || simRunning} style={btn()}>
        {busy ? "Working…" : provinces.length ? `Regenerate provinces (${provinces.length})` : "Generate provinces"}
      </button>

      {provinces.length > 0 && (<>
        <label style={{ display: "flex", alignItems: "center", gap: 6, color: "#8fbf9f", fontSize: 10, cursor: "pointer" }}>
          <input type="checkbox" checked={provincesVisible} onChange={(e) => setOverlayVisible("provinces", e.target.checked)}
            style={{ accentColor: "#c09b5a", width: 12, height: 12 }} />
          Show provinces on map
        </label>

        <div style={sec}>Edit provinces</div>
        <button onClick={toggleSelect} style={btn(selectMode)}>
          {selectMode ? "✔ Selecting — click provinces on the map" : "☐ Select provinces on the map"}
        </button>
        <div style={small}>
          Click a province to mark it, click again to unmark (Shift-click works any time). Marked provinces are outlined in cyan.
        </div>
        {marked.length > 0 && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 3 }}>
            {marked.map((id) => (
              <span key={id} onClick={() => toggleMarked(id)} title="Unmark"
                style={{ background: "#14303a", border: "1px solid #2a6070", color: "#8fdcf0", borderRadius: 9, padding: "0 6px", fontSize: 9.5, cursor: "pointer" }}>
                {nameOf(id)} ✕
              </span>
            ))}
            <span onClick={clearMarked} style={{ color: "#c08080", fontSize: 9.5, cursor: "pointer", alignSelf: "center" }}>clear</span>
          </div>
        )}
        <div style={{ display: "flex", gap: 4 }}>
          <button onClick={mergeSelected} disabled={marked.length < 2 || busy} style={{ ...btn(false, marked.length < 2), flex: 1 }}
            title="Fold all marked provinces into one (the largest keeps its name and seat)">
            Merge {marked.length >= 2 ? `${marked.length} ` : ""}into one
          </button>
          <button onClick={splitSelected} disabled={marked.length < 1 || busy} style={{ ...btn(false, marked.length < 1), flex: 1 }}
            title="Cut each marked province into smaller, feature-following provinces">
            Split into {pieces}
          </button>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 6, color: "#8a7d66", fontSize: 10 }}>
          <span>Split pieces</span>
          {[2, 3, 4, 6].map((k) => (
            <span key={k} onClick={() => setPieces(k)}
              style={{ cursor: "pointer", padding: "0 6px", borderRadius: 8, border: `1px solid ${pieces === k ? "#8fbf9f" : "#2a4a38"}`, color: pieces === k ? "#cfeedd" : "#6f8f6f" }}>{k}</span>
          ))}
        </div>

        <button onClick={editing ? stopEdit : startEdit} disabled={!editing && marked.length < 1}
          style={btn(editing, !editing && marked.length < 1)}
          title="Paint land cells into the marked province to move its border">
          {editing ? "✔ Editing borders — Done" : "✎ Edit borders of the marked province"}
        </button>
        {editing && editTarget !== null && (
          <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
            <div style={small}>
              Drag over the map: every land cell you paint over is handed to <b style={{ color: "#cfeedd" }}>{nameOf(editTarget)}</b>.
              To pull a border back, mark the neighbour and paint into that one instead.
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", color: "#8a7d66", fontSize: 10 }}>
              <span>Brush</span><span>{brush}</span>
            </div>
            <input type="range" min={1} max={20} value={brush} onChange={(e) => setBrush(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
        )}

        <div style={sec}>Check the borders</div>
        <div style={{ display: "flex", gap: 4 }}>
          <button onClick={check} disabled={busy} style={{ ...btn(), flex: 1 }}>Check borders</button>
          <button onClick={repair} disabled={busy} style={{ ...btn(), flex: 1 }} title="Give every cut-off piece to the neighbour it touches most">Repair pieces</button>
        </div>
        {issues && issues.length > 0 && (
          <div style={{ maxHeight: 130, overflowY: "auto", display: "flex", flexDirection: "column", gap: 2 }}>
            {issues.slice(0, 80).map((it, k) => (
              <div key={k} onClick={() => focusOn(it.x, it.y)} title="Show on the map"
                style={{ cursor: "pointer", fontSize: 9.5, color: it.kind === "fragment" || it.kind === "empty" ? "#e0a070" : "#9fb89f",
                  borderBottom: "1px solid #1a3020", padding: "1px 0" }}>
                <b>{nameOf(it.province)}</b> · {it.kind.replace("_", " ")} — {it.detail}
              </div>
            ))}
          </div>
        )}

        <div style={sec}>Whole map</div>
        <div style={{ display: "flex", gap: 4 }}>
          <button onClick={mergeSmall} disabled={busy} style={{ ...btn(), flex: 1 }} title="Fold slivers into the neighbour they share most border with">Merge small</button>
          <button onClick={splitLarge} disabled={busy} style={{ ...btn(), flex: 1 }} title="Split the oversized non-polar provinces">Split large</button>
        </div>
      </>)}
      {msg && <div style={{ color: msg.startsWith("Failed") ? "#e08080" : "#bcd8a0", fontSize: 10 }}>{msg}</div>}
    </div>
  );
}
