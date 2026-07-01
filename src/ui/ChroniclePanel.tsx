import { useState } from "react";
import { StepCampaign } from "./workflow/StepCampaign";
import { useViewportStore } from "../state/viewportStore";

/** The Chronicle subproduct's left panel — the campaign clock and its economy
 *  read-outs (reuses StepCampaign, the DLC-1 "Living Trade" control surface).
 *
 *  Presented as a distinct product face from Forge: the world is finalized and
 *  read-only here; prices, wealth and trade FLOW are simulated live from the
 *  campaign, not read from the static worldgen preview. Trade ROUTES (the
 *  network) are inherited from the world; only their live volumes are derived. */
export function ChroniclePanel() {
  const invalidateTiles = useViewportStore((s) => s.invalidateTiles);
  // The campaign RNG seed. Random per session start; the campaign, once begun,
  // carries its own state, so this only seeds a fresh "Begin Campaign".
  const [seed] = useState(() => Math.floor(Math.random() * 999999));

  return (
    <div style={{
      width: 220, background: "#0d1219", borderRight: "1px solid #1e2a38",
      padding: "8px", overflowY: "auto", display: "flex", flexDirection: "column",
      gap: 6, fontSize: 12,
    }}>
      <div style={{ color: "#3a80c0", fontWeight: 700, fontSize: 13 }}>
        Chronicle — Living Campaign
      </div>
      <div style={{ color: "#506080", fontSize: 10, lineHeight: 1.5, marginBottom: 2 }}>
        The map is finalized. Cities start at their <b style={{ color: "#7a98b8" }}>founding</b>
        {" "}conditions and the economy evolves <b style={{ color: "#7a98b8" }}>live</b> —
        prices, wealth and trade volumes are simulated, not the worldgen preview.
        Trade routes are inherited from the world; their activity is earned each year.
      </div>
      <StepCampaign seed={seed} plateCount={0} invalidateTiles={invalidateTiles} />
    </div>
  );
}
