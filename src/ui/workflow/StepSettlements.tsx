import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { useViewportStore } from "@state/viewportStore";
import { simGenerateSettlements, simGenerateProvinces, setCultureCount as saveCultureCount, getCultureCount } from "@bridge";
import { genBtn } from "@ui/workflow/WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

export function StepSettlements({ seed, invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const setLayer = useUIStore((s) => s.setLayer);
  const settlementRealism = useUIStore((s) => s.settlementRealism);
  const setSettlementRealism = useUIStore((s) => s.setSettlementRealism);
  const settlementCap = useUIStore((s) => s.settlementCap);
  const setSettlementCap = useUIStore((s) => s.setSettlementCap);
  const focusOn = useViewportStore((s) => s.focusOn);
  const { rivers, settlements, setSettlements, setProvinces, provinces } = useWorldStore();

  // Province partition — generated straight from this step (seeds off the settlements
  // above). Borders follow rivers / lakes / mountain ranges (see sim/shared/provinces.rs).
  const [provGranularity, setProvGranularity] = useState(0.5);
  const [provBusy, setProvBusy] = useState(false);

  // Cultures 2.0 · how many cultures/peoples the world starts with (0 = auto by land
  // area). Persisted to world metadata so the culture map picks it up at generation.
  const [cultureCount, setCultureCount] = useState<number>(0);
  useEffect(() => { getCultureCount().then(setCultureCount).catch(() => {}); }, []);
  const onCultureCount = (v: number) => {
    setCultureCount(v);
    saveCultureCount(v).catch(() => {});
  };

  const step6Done = stepCompleted[6] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step6Done) {
      setStatus("Step 6 required: Generate soil & fertility first (settlements need habitability data)");
      return;
    }
    setSimRunning(true);
    setStatus("Scoring habitability & placing cities...");
    try {
      const result = await simGenerateSettlements(
        seed, JSON.stringify(rivers), settlementRealism,
        settlementCap > 0 ? settlementCap : undefined);
      setSettlements(result.settlements);
      markStepCompleted(7);
      // Show the habitability heatmap + ranked city dots.
      setLayer("habitability");
      invalidateTiles();
      setOverlayVisible("settlements", true);

      const capitals = result.settlements.filter((s) => s.size === "capital").length;
      const cities = result.settlements.filter((s) => s.size === "city").length;
      const towns = result.settlements.filter((s) => s.size === "town").length;
      const villages = result.settlements.filter((s) => s.size === "village").length;
      setStatus(`${result.settlements.length} sites: ${capitals} capitals, ${cities} cities, ${towns} towns, ${villages} villages`);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const handleGenerateProvinces = async () => {
    if (provBusy || simRunning) return;
    if (settlements.length === 0) {
      setStatus("Find settlements first — provinces seed from them");
      return;
    }
    setProvBusy(true);
    setStatus("Partitioning the land into provinces…");
    try {
      const res = await simGenerateProvinces(settlements, rivers, provGranularity);
      setProvinces(res.provinces, {
        data: res.raster, w: res.raster_w, h: res.raster_h, gridW: res.grid_w, gridH: res.grid_h,
      });
      setOverlayVisible("provinces", true);
      setStatus(`Generated ${res.provinces.length} provinces`);
    } catch (e) {
      setStatus(`Province generation failed: ${e}`);
    } finally {
      setProvBusy(false);
    }
  };

  const dot = (size: string) =>
    size === "capital" ? "★" : size === "city" ? "●" : size === "town" ? "○" : "·";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step6Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 6 first (settlements need fertility & habitability data)
        </div>
      )}
      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        <div style={{ display: "flex", justifyContent: "space-between", color: "#8090b0", fontSize: 10 }}>
          <span>Density / realism</span>
          <span>{settlementRealism <= 0.33 ? "Sparse · strict" : settlementRealism >= 0.7 ? "Dense" : "Balanced"} ({Math.round(settlementRealism * 100)})</span>
        </div>
        <input type="range" min={0} max={1} step={0.05} value={settlementRealism}
          onChange={(e) => setSettlementRealism(parseFloat(e.target.value))} />
        <div style={{ color: "#405060", fontSize: 9 }}>
          Low = fewer, only genuinely viable sites (no marginal polar/desert specks);
          high = denser and more permissive.
        </div>
      </div>

      {/* Hard count cap (20..1000; 0 = auto). Overrides the density slider's count. */}
      <div style={{ display: "flex", flexDirection: "column", gap: 2, marginTop: 2 }}>
        <div style={{ display: "flex", justifyContent: "space-between", color: "#8090b0", fontSize: 10 }}>
          <span>Limit total settlements</span>
          <span>{settlementCap === 0 ? "Auto (no limit)" : `${settlementCap}`}</span>
        </div>
        <input type="range" min={0} max={1000} step={10} value={settlementCap}
          onChange={(e) => { const v = parseInt(e.target.value, 10); setSettlementCap(v === 0 ? 0 : Math.max(20, v)); }} />
        <div style={{ color: "#405060", fontSize: 9 }}>
          0 = auto (density slider decides). Otherwise a hard cap of 20–1000 settlements
          scattered across the world (keeps the strongest sites).
        </div>
      </div>

      {/* Cultures 2.0 · initial number of cultures/peoples (0 = auto by land area). */}
      <div style={{ display: "flex", flexDirection: "column", gap: 2, marginTop: 2 }}>
        <div style={{ display: "flex", justifyContent: "space-between", color: "#8090b0", fontSize: 10 }}>
          <span>Cultures at world start</span>
          <span>{cultureCount === 0 ? "Auto (by land area)" : `${cultureCount}`}</span>
        </div>
        <input type="range" min={0} max={24} step={1} value={cultureCount}
          onChange={(e) => onCultureCount(parseInt(e.target.value, 10))} />
        <div style={{ color: "#405060", fontSize: 9 }}>
          How many distinct peoples/language-hearths seed the map (0 = auto, ~5–14 by
          size). More = a patchier cultural map. Applied when settlements are generated.
        </div>
      </div>

      <button onClick={handleGenerate} disabled={simRunning || !step6Done} style={genBtn}>
        Find Settlements
      </button>

      {settlements.length > 0 && (
        <div style={{ maxHeight: 140, overflowY: "auto", display: "flex", flexDirection: "column", gap: 1 }}>
          {[...settlements].sort((a, b) => b.population - a.population).slice(0, 60).map((s, i) => (
            <div key={s.id}
              onClick={() => focusOn(s.x, s.y)}
              title="Click to locate on the map"
              style={{ display: "flex", justifyContent: "space-between", padding: "1px 3px",
                cursor: "pointer", borderRadius: 2 }}
              onMouseEnter={(e) => (e.currentTarget.style.background = "#16202e")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}>
              <span style={{ color: "#8090b0", fontSize: 10 }}>
                {dot(s.size)} #{i + 1} {s.size}
              </span>
              <span style={{ color: "#7a8aa0", fontSize: 10 }}>
                {s.population >= 1000 ? `${(s.population / 1000).toFixed(s.population >= 10000 ? 0 : 1)}k` : s.population}
              </span>
            </div>
          ))}
          {settlements.length > 60 && (
            <div style={{ color: "#405060", fontSize: 9 }}>... and {settlements.length - 60} more</div>
          )}
        </div>
      )}

      <div style={{ color: "#405060", fontSize: 10 }}>
        The Habitability heatmap shows where civilization is most likely
        (climate 40% + fertility 20% + water 20% + terrain 10% + trade 10%).
        Cities are placed highest-probability first; dot size = settlement rank.
      </div>

      {settlements.length > 0 && (
        <div style={{ marginTop: 6, padding: "6px 7px", background: "#0e1a12", borderRadius: 4,
          border: "1px solid #1a3020", display: "flex", flexDirection: "column", gap: 5 }}>
          <div style={{ color: "#6a9a6a", fontSize: 10, fontWeight: 600 }}>🗺 Provinces — partition the land</div>
          <div style={{ color: "#4a6a4a", fontSize: 9, lineHeight: 1.4 }}>
            Splits the land into provinces whose borders follow rivers, lakes and
            mountain ranges (lowland plains divide organically). Seeds from the
            settlements above — open the Provinces panel (top bar) to inspect them.
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{ color: "#607090", fontSize: 10 }}>Granularity</span>
            <input type="range" min={0} max={1} step={0.05} value={provGranularity}
              onChange={(e) => setProvGranularity(parseFloat(e.target.value))} style={{ flex: 1 }} />
            <span style={{ width: 66, color: "#8090b0", fontSize: 9, textAlign: "right" }}>
              {provGranularity < 0.34 ? "few · large" : provGranularity > 0.66 ? "many · small" : "balanced"}
            </span>
          </div>
          <button onClick={handleGenerateProvinces} disabled={provBusy || simRunning}
            style={{ ...genBtn, marginBottom: 0, background: "#183024", color: "#8fd0a0", textAlign: "center" }}>
            {provBusy ? "Partitioning…" : provinces.length ? `Regenerate Provinces (${provinces.length})` : "Generate Provinces"}
          </button>
        </div>
      )}
    </div>
  );
}
