import React from "react";

// Shared workflow button style. Lives in its OWN leaf module (no imports back into
// the workflow tree) to break a circular import: WorkflowPanel imports every
// Step*.tsx, and StepLandmass consumes this style at MODULE-INIT time
// (`const smallBtn = { ...genBtn }`). When it lived on WorkflowPanel that read hit
// the export before WorkflowPanel had finished initializing it, throwing
// "Cannot access 'genBtn' before initialization" and blanking the whole app.
// A leaf module is always fully evaluated before any consumer's body runs.
export const genBtn: React.CSSProperties = {
  width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid #3b311f",
  background: "#262017", color: "#b3a07f", cursor: "pointer", fontSize: 11, textAlign: "left" as const,
};
