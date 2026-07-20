import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { simGenerateToponyms, saveToponyms, getToponyms } from "@bridge";
import type { Toponym } from "@types";
import { genBtn } from "@ui/workflow/WorkflowPanel";

const KIND_ICON: Record<Toponym["kind"], string> = {
  river: "\u{1F30A}", mountain: "\u{1F3D4}\u{FE0F}", lake: "\u{1F4A7}", region: "\u{1F3F3}\u{FE0F}",
};

/** #26 · optional, GATED toponym step. Names rivers/mountains/lakes/regions in the
 *  local culture's style; the list is editable (rename any feature) and persisted.
 *  Gated on the Rivers (5) and Settlements (7) steps — the backend also refuses to
 *  run before the culture map + rivers exist. */
export function StepToponyms() {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const rivers = useWorldStore((s) => s.rivers);
  const lakes = useWorldStore((s) => s.lakes);
  const toponyms = useWorldStore((s) => s.toponyms);
  const setToponyms = useWorldStore((s) => s.setToponyms);

  const riversDone = stepCompleted[5] === true;
  const settlementsDone = stepCompleted[7] === true;
  const ready = riversDone && settlementsDone;

  const [dirty, setDirty] = useState(false);
  const [filter, setFilter] = useState<"" | Toponym["kind"]>("");

  // Load any persisted list when the step opens.
  useEffect(() => {
    if (toponyms.length === 0) {
      getToponyms().then((t) => { if (t.length) setToponyms(t); }).catch(() => {});
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const generate = async () => {
    if (simRunning) return;
    if (!ready) {
      setStatus("Toponyms need the Rivers (5) and Settlements (7) steps first.");
      return;
    }
    setSimRunning(true);
    setStatus("Naming the world's rivers, peaks, lakes and regions…");
    try {
      const list = await simGenerateToponyms(
        rivers.map((r) => ({ points: r.points })),
        lakes.map((l) => ({ cells: l.cells })),
      );
      setToponyms(list);
      setDirty(false);
      setOverlayVisible("toponyms", true);
      markStepCompleted(12);
      setStatus(`Named ${list.length} features.`);
    } catch (err) {
      setStatus(`Error: ${err}`);
    }
    setSimRunning(false);
  };

  const rename = (i: number, name: string) => {
    const next = toponyms.map((t, k) => (k === i ? { ...t, name } : t));
    setToponyms(next);
    setDirty(true);
  };

  const persist = async () => {
    try {
      await saveToponyms(toponyms);
      setDirty(false);
      setStatus("Saved place names.");
    } catch (err) { setStatus(`Error: ${err}`); }
  };

  const counts = toponyms.reduce<Record<string, number>>((a, t) => { a[t.kind] = (a[t.kind] ?? 0) + 1; return a; }, {});
  const shown = toponyms
    .map((t, i) => ({ t, i }))
    .filter(({ t }) => filter === "" || t.kind === filter);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {!ready && (
        <div style={{ color: "#cc8844", fontSize: 10 }}>
          Requires Rivers (Step 5){!settlementsDone ? " and Settlements (Step 7)" : ""} — those create the rivers and culture map names draw from.
        </div>
      )}

      <button onClick={generate} disabled={simRunning || !ready} style={genBtn}>
        {toponyms.length > 0 ? "Regenerate place names" : "Generate place names"}
      </button>

      {toponyms.length > 0 && (
        <>
          <div style={{ display: "flex", gap: 4, flexWrap: "wrap", alignItems: "center" }}>
            <Chip on={filter === ""} onClick={() => setFilter("")}>{`All ${toponyms.length}`}</Chip>
            {(["region", "river", "mountain", "lake"] as const).map((k) =>
              counts[k] ? (
                <Chip key={k} on={filter === k} onClick={() => setFilter(k)}>{`${KIND_ICON[k]} ${counts[k]}`}</Chip>
              ) : null,
            )}
            <label style={{ display: "flex", alignItems: "center", gap: 4, marginLeft: "auto", color: "#7a98b8", fontSize: 10, cursor: "pointer" }}>
              <input type="checkbox" defaultChecked onChange={(e) => setOverlayVisible("toponyms", e.target.checked)}
                style={{ accentColor: "#3a80c0" }} />
              labels
            </label>
          </div>

          <div style={{ maxHeight: 240, overflowY: "auto", display: "flex", flexDirection: "column", gap: 2 }}>
            {shown.map(({ t, i }) => (
              <div key={i} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span style={{ width: 16, textAlign: "center" }} title={t.kind}>{KIND_ICON[t.kind]}</span>
                <input value={t.name} onChange={(e) => rename(i, e.target.value)} style={nameInput} />
              </div>
            ))}
          </div>

          <button onClick={persist} disabled={!dirty} style={{
            ...genBtn, textAlign: "center",
            background: dirty ? "#1a4a5a" : "#151d28", color: dirty ? "#a0d0e0" : "#506070",
          }}>
            {dirty ? "Save renamed places" : "Saved"}
          </button>
        </>
      )}
    </div>
  );
}

function Chip({ on, onClick, children }: { on: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button onClick={onClick} style={{
      padding: "2px 7px", borderRadius: 10, fontSize: 10, cursor: "pointer",
      border: `1px solid ${on ? "#3a80c0" : "#1e2e42"}`,
      background: on ? "#19324a" : "#0c1622", color: on ? "#cfe2f6" : "#7a98b8",
    }}>{children}</button>
  );
}

const nameInput: React.CSSProperties = {
  flex: 1, background: "#080c12", border: "1px solid #1e2e42", color: "#cfe0f4",
  padding: "2px 5px", borderRadius: 4, fontSize: 11, minWidth: 0,
};
