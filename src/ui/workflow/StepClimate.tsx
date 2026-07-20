import { useUIStore } from "@state/uiStore";
import { simClassifyClimate } from "@bridge";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

const BIOME_LEGEND = [
  { code: "Af", name: "Tropical Rainforest", color: "#0000ff", desc: "Hot and wet year-round, dense canopy forests" },
  { code: "Am", name: "Tropical Monsoon", color: "#0078ff", desc: "Seasonal heavy rains with short dry season" },
  { code: "Aw", name: "Tropical Savanna", color: "#46aafa", desc: "Distinct wet and dry seasons, grasslands with scattered trees" },
  { code: "BWh", name: "Hot Desert", color: "#ff0000", desc: "Extremely arid, scorching daytime temperatures" },
  { code: "BWk", name: "Cold Desert", color: "#ff9696", desc: "Arid with cold winters, sparse vegetation" },
  { code: "BSh", name: "Hot Steppe", color: "#f5a500", desc: "Semi-arid grasslands, hot climate" },
  { code: "BSk", name: "Cold Steppe", color: "#ffdc64", desc: "Semi-arid grasslands with cold winters" },
  { code: "Csa", name: "Mediterranean Hot", color: "#ffff00", desc: "Dry hot summers, mild wet winters" },
  { code: "Csb", name: "Mediterranean Warm", color: "#c8c800", desc: "Dry warm summers, mild wet winters" },
  { code: "Cfa", name: "Humid Subtropical", color: "#c8ff50", desc: "Hot humid summers, mild winters, year-round rain" },
  { code: "Cfb", name: "Oceanic", color: "#64c864", desc: "Mild temperatures, frequent rain, no dry season" },
  { code: "Dfa", name: "Continental Hot", color: "#00ff96", desc: "Hot summers, cold snowy winters" },
  { code: "Dfb", name: "Continental Warm", color: "#37c8ff", desc: "Warm summers, long cold winters" },
  { code: "Dfc", name: "Subarctic", color: "#007d7d", desc: "Brief cool summers, long severe winters" },
  { code: "ET", name: "Tundra", color: "#b2b2b2", desc: "Permafrost, only mosses and lichens survive" },
  { code: "EF", name: "Ice Cap", color: "#686868", desc: "Permanent ice cover, no vegetation" },
];

export function StepClimate({ invalidateTiles }: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const setLayer = useUIStore((s) => s.setLayer);

  const step3Done = stepCompleted[3] === true;

  const handleGenerate = async () => {
    if (simRunning) return;
    if (!step3Done) {
      setStatus("Step 3 required: Generate ocean & atmosphere first (wind, currents, temperature, precipitation)");
      return;
    }
    setSimRunning(true);
    setStatus("Classifying climate zones...");
    try {
      await simClassifyClimate();
      invalidateTiles();
      markStepCompleted(4);
      setLayer("climate");
      setStatus("K\u00F6ppen climate classification complete");
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step3Done && (
        <div style={{ color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3 }}>
          Complete Step 3 first (ocean & atmosphere needed for temperature/precipitation)
        </div>
      )}
      <button onClick={handleGenerate} disabled={simRunning || !step3Done} style={genBtn}>
        Generate Climate Zones
      </button>
      <div style={{ display: "flex", gap: 4, marginTop: 2 }}>
        <button onClick={() => setLayer("climate")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Climate
        </button>
        <button onClick={() => setLayer("biomes")}
          style={{ ...genBtn, fontSize: 10, padding: "3px 6px" }}>
          View Biomes
        </button>
      </div>

      <div style={{ color: "#607090", fontSize: 10, fontWeight: 600, marginTop: 4 }}>Biome Legend</div>
      <div style={{ maxHeight: 160, overflowY: "auto", display: "flex", flexDirection: "column", gap: 1 }}>
        {BIOME_LEGEND.map((b) => (
          <div key={b.code} style={{ display: "flex", alignItems: "start", gap: 4, padding: "1px 0" }}>
            <div style={{
              width: 10, height: 10, borderRadius: 2, flexShrink: 0, marginTop: 1,
              background: b.color,
            }} />
            <div>
              <span style={{ color: "#8090b0", fontSize: 10 }}>{b.code}</span>
              <span style={{ color: "#506080", fontSize: 10 }}> {b.name}</span>
              <div style={{ color: "#405060", fontSize: 9 }}>{b.desc}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
