import { useUIStore } from "@state/uiStore";

/*  Trade reach. "Global (cross any ocean)" is gone: an unbroken open-water run is
 *  capped by a WORLD RULE now (`MAX_OPEN_SEA_CROSSING_KM`, query_commands/mod.rs),
 *  which binds at every reach except 2, so no setting can put a trans-oceanic lane
 *  back. Reach 1 may only ask for something SHORTER than the rule.
 *
 *  `SEA_CAP_KM` mirrors that Rust constant for the SLIDER BOUND only — the backend
 *  clamps regardless (gated by `no_caller_can_ask_its_way_across_an_ocean`), so if
 *  the two ever drift the rule still holds and the slider merely stops short. */
const REACH_LABELS = ["Sea trade — short crossings only", "Coastal — custom crossing limit", "Continental only — no sea legs"];
const KM_EQUATOR = 40075;
const SEA_CAP_KM = 800;

/** Trade-route settings, read live by the route/flow overlays and by the
 *  Political (9) and Economy (10) queries. Moved out of step 8, which never
 *  used them for generation, so each step shows only what it uses. */
export function TradeSettings() {
  const bioParams = useUIStore((s) => s.bioParams);
  const setBioParams = useUIStore((s) => s.setBioParams);
  return (
    <>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#897a61", marginTop: 2 }}>
        <span>Economic regions</span><span style={{ color: "#bcab8e" }}>{bioParams.economicRegions}</span>
      </div>
      <input type="range" min={4} max={40} value={bioParams.economicRegions}
        onChange={(e) => setBioParams({ economicRegions: Number(e.target.value) })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#897a61" }}>
        Granularity of trade regions &amp; political hubs (also scales route hubs).
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#897a61", marginTop: 2 }}>
        <span>Demand bias</span>
        <span style={{ color: "#bcab8e" }}>
          {bioParams.luxuryBias < 0.4 ? "subsistence" : bioParams.luxuryBias > 0.6 ? "mercantile" : "balanced"}
        </span>
      </div>
      <input type="range" min={0} max={100} value={Math.round(bioParams.luxuryBias * 100)}
        onChange={(e) => setBioParams({ luxuryBias: Number(e.target.value) / 100 })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#897a61" }}>
        Subsistence worlds trade staples; mercantile worlds prize distant luxuries (silk, spices).
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#897a61", marginTop: 2 }}>
        <span>Piracy</span>
        <span style={{ color: "#bcab8e" }}>
          {bioParams.piracyLevel < 0.1 ? "safe seas" : bioParams.piracyLevel > 0.6 ? "infested" : "some raiders"}
        </span>
      </div>
      <input type="range" min={0} max={100} value={Math.round(bioParams.piracyLevel * 100)}
        onChange={(e) => setBioParams({ piracyLevel: Number(e.target.value) / 100 })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#897a61" }}>
        Raiders make coastal narrows &amp; straits costlier — trade hugs safe coasts, detours or goes overland.
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#897a61", marginTop: 2 }}>
        <span>Trade season</span>
        <span style={{ color: "#bcab8e" }}>
          {bioParams.tradeSeason === 0 ? "all year" : `moon ${bioParams.tradeSeason}/${bioParams.calendarMonths}`}
        </span>
      </div>
      <input type="range" min={0} max={bioParams.calendarMonths} value={bioParams.tradeSeason}
        onChange={(e) => setBioParams({ tradeSeason: Number(e.target.value) })}
        style={{ width: "100%" }} />
      <div style={{ fontSize: 9, color: "#897a61" }}>
        Seasonal closures: winter snows shut high mountain passes; monsoon/cyclone seas close their sailing windows, so routes detour. Routes redraw live as you slide.
      </div>

      <div style={{ fontSize: 10, color: "#897a61", marginTop: 2 }}>Trade reach</div>
      <select value={bioParams.tradeReach}
        onChange={(e) => setBioParams({ tradeReach: Number(e.target.value) })}
        style={{ width: "100%", background: "#110e09", color: "#cec4b2", border: "1px solid #3e3422", borderRadius: 4, fontSize: 10, padding: "2px 4px" }}>
        {REACH_LABELS.map((l, i) => <option key={i} value={i}>{l}</option>)}
      </select>

      {bioParams.tradeReach === 1 && (
        <>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, color: "#897a61", marginTop: 2 }}>
            <span>Max open-water crossing</span>
            <span style={{ color: "#bcab8e" }}>{Math.round(bioParams.maxCrossing * KM_EQUATOR)} km</span>
          </div>
          {/* Stated in KM, not "% of map width" — a fraction of the map is not a
              distance anyone can reason about, and reading it as one is how the
              shipped 12% came to mean a 4,809 km ocean traversal. */}
          <input type="range" min={50} max={SEA_CAP_KM} step={25}
            value={Math.round(bioParams.maxCrossing * KM_EQUATOR)}
            onChange={(e) => setBioParams({ maxCrossing: Number(e.target.value) / KM_EQUATOR })}
            style={{ width: "100%" }} />
          <div style={{ fontSize: 9, color: "#897a61" }}>
            The longest stretch of OPEN water a route may cross without sight of land.
            Coast-hugging and island-hopping are unlimited — shelf and coastal water
            reset the run — so a network still reaches a long way, one hop at a time.
          </div>
        </>
      )}

      <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 10, color: "#bcab8e", marginTop: 4, cursor: "pointer" }}>
        <input type="checkbox" checked={bioParams.desertRoutes}
          onChange={(e) => setBioParams({ desertRoutes: e.target.checked })} />
        Silk Road caravans (overland steppe &amp; desert)
      </label>
      <div style={{ fontSize: 9, color: "#897a61", marginTop: 1 }}>
        Trade threads overland steppe corridors and deserts (Silk-Road style) when storms/reefs make the seas perilous.
      </div>

    </>
  );
}
