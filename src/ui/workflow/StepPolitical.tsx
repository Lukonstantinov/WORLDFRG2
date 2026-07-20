import { useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { computePolitical } from "@bridge";
import type { PoliticalCenter } from "@types";
import { GOOD_DEFS } from "@goods";
import { genBtn } from "./WorkflowPanel";

interface Props {
  seed: number;
  plateCount: number;
  invalidateTiles: () => void;
}

const emojiFor = (name: string) => GOOD_DEFS.find((g) => g.name === name)?.emoji ?? "";

export function StepPolitical(_props: Props) {
  const simRunning = useUIStore((s) => s.simRunning);
  const setSimRunning = useUIStore((s) => s.setSimRunning);
  const setStatus = useUIStore((s) => s.setStatus);
  const markStepCompleted = useUIStore((s) => s.markStepCompleted);
  const setOverlayVisible = useUIStore((s) => s.setOverlayVisible);
  const stepCompleted = useUIStore((s) => s.stepCompleted);
  const bioParams = useUIStore((s) => s.bioParams);
  const settlements = useWorldStore((s) => s.settlements);
  const rivers = useWorldStore((s) => s.rivers);
  const [centers, setCenters] = useState<PoliticalCenter[]>([]);

  const step8Done = stepCompleted[8] === true;

  const handleCompute = async () => {
    if (simRunning) return;
    if (!step8Done) {
      setStatus("Step 8 required: generate the Biological-Trade layer first (trade power needs goods + routes)");
      return;
    }
    setSimRunning(true);
    setStatus("Ranking settlements by trade power & computing influence...");
    try {
      const result = await computePolitical(
        settlements.map((s) => ({ x: s.x, y: s.y, score: s.score, population: s.population })),
        rivers.map((r) => ({ points: r.points })),
        bioParams.tradeReach,
        bioParams.maxCrossing,
        bioParams.desertRoutes,
        bioParams.economicRegions,
        bioParams.piracyLevel,
      );
      setCenters(result);
      markStepCompleted(9);
      setOverlayVisible("politicalInfluence", true);
      setStatus(`Political layer: ${result.length} powers ranked by trade position & monopoly`);
    } catch (err) { setStatus(`Error: ${err}`); }
    setSimRunning(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {!step8Done && (
        <div style={warn}>Complete Step 8 first (trade power is built from goods, routes & monopolies)</div>
      )}

      <button onClick={handleCompute} disabled={simRunning || !step8Done} style={genBtn}>
        Compute Political Influence
      </button>

      {centers.length > 0 && (
        <div style={{ marginTop: 4, maxHeight: 220, overflowY: "auto" }}>
          {centers.slice(0, 14).map((c) => (
            <div key={c.rank} style={{
              display: "flex", justifyContent: "space-between", alignItems: "center",
              fontSize: 10, padding: "2px 4px", borderBottom: "1px solid #14202e", color: "#a8c0d8",
            }}>
              <span>#{c.rank + 1}</span>
              <span style={{ color: "#8aa0c0" }}>power {c.power.toFixed(2)}</span>
              <span style={{ minWidth: 70, textAlign: "right" }}>
                {c.monopolies.slice(0, 4).map((m) => emojiFor(m)).join("")}
              </span>
            </div>
          ))}
        </div>
      )}

      <div style={{ color: "#405060", fontSize: 10, marginTop: 2 }}>
        Re-ranks settlements by <b>trade power</b> = base habitability + route
        centrality + good monopoly. Bigger magenta discs = greater influence.
        Monopolized goods (shown as emoji) come from being the sole producer of a
        seeded good.
      </div>
    </div>
  );
}

const warn: React.CSSProperties = {
  color: "#cc6644", fontSize: 10, padding: "3px 6px", background: "#1a1410", borderRadius: 3,
};
