import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { simSoilFertility, simClassifyBiomes, getBiomeStats } from "@bridge";
import type { BiomeStat } from "@types";
import { genBtn } from "@ui/workflow/WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

/** Swatch colours mirroring `render/tile_image.rs::biome_color`. Keep the two in
 *  sync — the legend is worthless if it disagrees with the map. */
const BIOME_SWATCH: Record<number, string> = {
  1: "#0d4a1f", 2: "#1c632c", 3: "#4c7c30", 4: "#2e6e5c", 5: "#2c584a",
  6: "#a8a854", 7: "#968a52",
  8: "#205c42", 9: "#4a7a34", 10: "#3e6c3c", 11: "#2c563e", 12: "#3a7436",
  13: "#7c8646", 14: "#928a58",
  15: "#264c3a", 16: "#566a52",
  17: "#9ea458", 18: "#b0aa6c", 19: "#8c9460",
  20: "#7e9a6e", 21: "#969c8c", 22: "#a69e92", 23: "#ced2d8",
  24: "#d8bc82", 25: "#c4baa8", 26: "#bab094", 27: "#c0b27e", 28: "#e8e4de",
  29: "#e4c68a", 30: "#b09878",
  31: "#9aa698", 32: "#bcc0be", 33: "#eef4fa", 34: "#d6e6f4",
  35: "#5e644a", 36: "#346c3a", 37: "#78944e", 38: "#628260", 39: "#305a3e",
  40: "#808c6a", 41: "#3c8246",
};

export function StepSoilResources({ invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setLayer = useUIStore((s) => s.setLayer);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const rivers = useWorldStore((s) => s.rivers);
  const lakes = useWorldStore((s) => s.lakes);

  const step4Done = stepCompleted[4] === true;
  const step5Done = stepCompleted[5] === true;
  const step6Done = stepCompleted[6] === true;

  // Biome legend: fetched on demand after a classification run.
  const [biomes, setBiomes] = useState<BiomeStat[] | null>(null);
  const [showLegend, setShowLegend] = useState(false);

  const refreshBiomeStats = async () => {
    try { setBiomes(await getBiomeStats()); } catch { setBiomes(null); }
  };

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step4Done) {
      setStatus("Step 4 required: Generate climate zones first (soil types depend on climate)");
      return;
    }
    if (!step5Done) {
      setStatus("Step 5 required: Generate rivers first (fertility depends on river proximity)");
      return;
    }
    setSimRunning(true);
    setStatus("Computing soil, fertility & fisheries...");
    try {
      await simSoilFertility(JSON.stringify(rivers));
      // Phase 6b follows immediately: biomes read the soil fingerprints this pass
      // just wrote, plus the rivers & lakes from step 5.
      setStatus("Classifying biomes...");
      await simClassifyBiomes(JSON.stringify(rivers), JSON.stringify(lakes));
      invalidateTiles();
      markStepCompleted(6);
      await refreshBiomeStats();
      setStatus("Soil, fertility, fisheries & biomes computed");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  /** Re-classify biomes alone — cheap, and lets the user re-run after editing
   *  rivers or climate without redoing soil. */
  const handleReclassifyBiomes = async () => {
    if (simRunning) return;
    if (!step6Done) { setStatus("Generate soil & fertility first"); return; }
    setSimRunning(true);
    setStatus("Classifying biomes...");
    try {
      await simClassifyBiomes(JSON.stringify(rivers), JSON.stringify(lakes));
      invalidateTiles();
      setLayer("biomes");
      await refreshBiomeStats();
      setShowLegend(true);
      setStatus("Biomes reclassified");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  const totalCells = biomes?.reduce((a, b) => a + b.cells, 0) ?? 0;
  // Group the legend the way the classifier does, so it reads as an ecology
  // rather than an alphabetical list.
  const grouped = (biomes ?? []).reduce<Record<string, BiomeStat[]>>((acc, b) => {
    (acc[b.group] ??= []).push(b);
    return acc;
  }, {});

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step4Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 4 first (soil types depend on climate classification)
        </div>
      )}
      {step4Done && !step5Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 5 first (fertility and the wetland biomes depend on rivers &amp; lakes)
        </div>
      )}
      <button onClick={handleGenerate} disabled={simRunning || !step4Done || !step5Done} style={genBtn}>
        Generate Soil, Fertility &amp; Biomes
      </button>
      <div style={{ display: "flex", gap: 4, marginTop: 2 }}>
        <button onClick={() => setLayer("soil")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Soil</button>
        <button onClick={() => setLayer("fertility")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Fertility</button>
        <button onClick={() => setLayer("fisheries")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Fisheries</button>
        <button onClick={() => setLayer("biomes")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>Biomes</button>
      </div>

      <div style={{ color: "#405060", fontSize: 10 }}>
        Soil types derived from K{"ö"}ppen climate. Fertility = soil (30%) + precipitation (20%)
        + temperature (15%) + river proximity (20%) + coast (10%) + volcanic (5%).
        Fisheries from upwelling zones + river mouths.
      </div>

      {/* ── Biomes (phase 6b) ── */}
      <div style={{ background: "#0a1018", border: "1px solid #223c30", borderRadius: 4, padding: 6, marginTop: 2 }}>
        <div style={{ color: "#6ab08a", fontSize: 10, fontWeight: 600, marginBottom: 4 }}>
          🌿 Biomes
        </div>
        <div style={{ color: "#506880", fontSize: 9, lineHeight: 1.4, marginBottom: 5 }}>
          41 ecological biomes from biotemperature (heat plants can actually use)
          × the potential-evapotranspiration ratio × K{"ö"}ppen's seasonality,
          then overridden by local water: mangrove and salt marsh on tidal
          shores, gallery forest and oases along rivers in dry country, marsh,
          swamp and peat bog around lakes, salt flats on terminal-lake basins.
          The treeline is a TEMPERATURE, so it drops from ~4000&nbsp;m in the
          tropics to sea level near the poles. Descriptive only — reclassifying
          never moves a city or a trade belt.
        </div>
        <button onClick={handleReclassifyBiomes} disabled={simRunning || !step6Done}
          style={{ ...genBtn, background: "#13301f", color: "#a8d8b8", textAlign: "center", marginBottom: 0 }}>
          🌿 Reclassify Biomes
        </button>
        <button
          onClick={async () => {
            const next = !showLegend;
            setShowLegend(next);
            if (next && !biomes) await refreshBiomeStats();
          }}
          style={{ ...genBtn, marginTop: 4, marginBottom: 0, fontSize: 10, textAlign: "center" }}>
          {showLegend ? "▲ Biome legend" : "▼ Biome legend"}
        </button>

        {showLegend && (
          <div style={{ marginTop: 5, maxHeight: 260, overflowY: "auto" }}>
            {totalCells === 0 && (
              <div style={{ color: "#506880", fontSize: 9 }}>
                No biomes classified yet — run the step above.
              </div>
            )}
            {Object.entries(grouped).map(([group, items]) => (
              <div key={group} style={{ marginBottom: 4 }}>
                <div style={{
                  fontSize: 9, color: "#3a5e70", textTransform: "uppercase",
                  letterSpacing: 0.6, marginBottom: 2,
                }}>
                  {group}
                </div>
                {items.sort((a, b) => b.cells - a.cells).map((b) => (
                  <div key={b.code} style={{ display: "flex", alignItems: "center", gap: 5, padding: "1px 0" }}>
                    <div style={{
                      width: 11, height: 11, borderRadius: 2, flexShrink: 0,
                      background: BIOME_SWATCH[b.code] ?? "#6e7660",
                      border: "1px solid #00000060",
                    }} />
                    <span style={{ color: "#7f95ad", fontSize: 10, flex: 1 }}>{b.name}</span>
                    <span style={{ color: "#4a6076", fontSize: 9, fontFamily: "monospace" }}>
                      {((b.cells / totalCells) * 100).toFixed(1)}%
                    </span>
                  </div>
                ))}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
