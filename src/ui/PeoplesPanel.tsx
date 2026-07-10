import { useEffect, useMemo, useRef, useState } from "react";
import { useUIStore } from "../state/uiStore";
import { useCampaignStore } from "../state/campaignStore";
import { useViewportStore } from "../state/viewportStore";
import { campaignGetCultures, campaignGetCulturePresence } from "../bridge/tauri";
import type { CultureBrief } from "../types";
import { useFloatingWindow, PANEL_TINTS } from "./useFloatingWindow";
import { CultureFigures } from "./CultureFigures";
import { CoatOfArms, houseColor } from "./CoatOfArms";
import { GOOD_DEFS } from "../goods";
import { T, SERIF } from "./chronicleTheme";

const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));

/** #1/#23 · The PEOPLES panel — the living cultures of the world. A two-pane census:
 *  every people (with its colour), its population, homelands and merchant houses;
 *  click one to shade the map by that culture's share of each settlement. Travel-
 *  prone merchant diasporas (Hansa/Jewish/Armenian-style) are flagged. */
export function PeoplesPanel() {
  const open = useUIStore((s) => s.showPeoples);
  const setOpen = useUIStore((s) => s.setShowPeoples);
  const selected = useUIStore((s) => s.selectedCulture);
  const setSelected = useUIStore((s) => s.setSelectedCulture);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const setSearchPin = useViewportStore((s) => s.setSearchPin);
  const [cultures, setCultures] = useState<CultureBrief[]>([]);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.figures);

  const active = snapshot?.active === true;
  const year = snapshot ? Math.floor(snapshot.clock.tick / 365) : 0;

  useEffect(() => {
    if (!open || !active) return;
    let alive = true;
    campaignGetCultures().then((r) => { if (alive) setCultures(r); }).catch(() => {});
    return () => { alive = false; };
  }, [open, active, year]);

  // Closing the panel clears any culture isolation on the map.
  useEffect(() => { if (!open) setSelected(null); }, [open, setSelected]);

  const sel = useMemo(() => cultures.find((c) => c.name === selected) ?? null, [cultures, selected]);
  const rgb = (c: [number, number, number]) => `rgb(${c[0]},${c[1]},${c[2]})`;

  if (!open) return null;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: 14, letterSpacing: 0.4 }}>
          👥 Peoples
        </span>
        <span style={{ color: T.inkFaint, fontSize: 10, marginLeft: 8 }}>
          {active ? `${cultures.length} peoples · Year ${year}` : "no campaign"}
        </span>
        <span style={{ flex: 1 }} />
        {selected && (
          <button data-no-drag onClick={() => setSelected(null)} title="Clear the map colouring"
            style={clearBtn}>clear map</button>
        )}
        <span data-no-drag style={{ cursor: "pointer", color: "#7a90a8", marginLeft: 8 }} onClick={() => setOpen(false)}>✕</span>
      </div>

      {!active && <div style={hint}>Begin a campaign — the peoples of the world fill in as it lives.</div>}
      {active && cultures.length === 0 && <div style={hint}>No peoples recorded yet — advance a year.</div>}

      {active && cultures.length > 0 && (
        <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
          {/* ── Left: the culture list ── */}
          <div style={{ width: 168, overflowY: "auto", borderRight: `1px solid ${T.lineSoft}`, padding: "4px 0" }}>
            {cultures.map((c) => {
              const on = c.name === selected;
              return (
                <div key={c.name} onClick={() => setSelected(on ? null : c.name)}
                  title="Click to colour the map by this people's share"
                  style={{
                    display: "flex", alignItems: "center", gap: 7, padding: "5px 9px", cursor: "pointer",
                    background: on ? "rgba(255,255,255,0.06)" : "transparent",
                    borderLeft: `3px solid ${on ? rgb(c.color) : "transparent"}`,
                  }}>
                  <span style={{ width: 11, height: 11, borderRadius: 3, background: rgb(c.color), flexShrink: 0, border: "1px solid rgba(0,0,0,0.4)" }} />
                  <span style={{ flex: 1, color: on ? T.ink : T.inkMid, fontSize: 12, fontWeight: on ? 700 : 400, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {c.name}
                  </span>
                  {c.mobility >= 0.7 && <span title="Travel-prone merchant diaspora" style={{ fontSize: 10 }}>⚓</span>}
                  <span style={{ color: T.inkFaint, fontSize: 10 }}>{fmtNum(c.population)}</span>
                </div>
              );
            })}
          </div>

          {/* ── Right: the selected people's detail ── */}
          <div style={{ flex: 1, overflowY: "auto", padding: "10px 12px" }}>
            {!sel && <div style={hint}>Select a people to see its homelands, houses and spread — and to colour the map.</div>}
            {sel && (
              <>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                  <span style={{ width: 16, height: 16, borderRadius: 4, background: rgb(sel.color), border: "1px solid rgba(0,0,0,0.4)" }} />
                  <span style={{ fontFamily: SERIF, color: T.ink, fontSize: 16, fontWeight: 700 }}>{sel.name}</span>
                  {sel.family?.startsWith("Creole") && <span style={creoleBadge}>✶ creole people</span>}
                  {sel.mobility >= 0.7 && (
                    <span style={diasporaBadge}>⚓ merchant diaspora</span>
                  )}
                </div>
                {/* Language family + STATIC origin card (Cultures 2.0). */}
                {sel.family && (
                  <div style={{ color: T.inkDim, fontSize: 10.5, marginBottom: 6 }}>
                    Language family: <b style={{ color: T.inkMid }}>{sel.family}</b>
                  </div>
                )}
                {sel.origin && (
                  <div style={{ fontFamily: SERIF, fontStyle: "italic", color: T.inkMid, fontSize: 12,
                    lineHeight: 1.5, marginBottom: 10, padding: "7px 9px", background: T.card,
                    border: `1px solid ${T.lineSoft}`, borderRadius: 6 }}>
                    {sel.origin}
                  </div>
                )}
                {/* Full-body figures in national dress (man + woman; creole blends parents). */}
                {sel.kit != null && sel.kit >= 0 && (
                  <div style={{ marginBottom: 12 }}>
                    <div style={sectionHdr}>In national dress</div>
                    <CultureFigures name={sel.name} kit={sel.kit} kit2={sel.kit2} color={sel.color} />
                  </div>
                )}
                <div style={{ display: "flex", gap: 6, marginBottom: 10 }}>
                  <Tile label="People" value={fmtNum(sel.population)} />
                  <Tile label="Homelands" value={String(sel.towns)} color="#7fd0a0" />
                  <Tile label="Present in" value={String(sel.presence)} color="#7fb0e0" />
                  <Tile label="Mobility" value={`${Math.round(sel.mobility * 100)}%`} color={sel.mobility >= 0.7 ? "#e0b060" : T.inkMid} />
                </div>

                {/* Cultural taste — the goods this people prizes and its markets seek. */}
                {(sel.desired_goods?.length ?? 0) > 0 && (
                  <>
                    <div style={sectionHdr}>Prizes &amp; seeks</div>
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 5, marginBottom: 10 }}>
                      {sel.desired_goods!.map((g) => {
                        const d = GOOD_BY_NAME.get(g);
                        return (
                          <span key={g} style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 11,
                            color: T.inkMid, border: `1px solid ${T.lineSoft}`, background: T.card, borderRadius: 12, padding: "2px 8px" }}>
                            <span>{d?.emoji ?? "•"}</span>{d?.label ?? g}
                          </span>
                        );
                      })}
                    </div>
                  </>
                )}

                {/* Where they live — a mini world map (toggle), same look as the goods preview. */}
                {sel.kit != null && sel.kit >= 0 && (
                  <CulturePresence name={sel.name} color={sel.color} />
                )}

                <div style={sectionHdr}>Chief cities</div>
                {sel.top_cities.length === 0 ? <div style={hint}>—</div> : sel.top_cities.map(([name, pop]) => (
                  <div key={name} onClick={() => { const h = snapshot?.hubs.find((x) => x.name === name); if (h) setSearchPin(h.x, h.y); }}
                    title="Pin this city on the map"
                    style={{ display: "flex", justifyContent: "space-between", padding: "3px 2px", cursor: "pointer", fontSize: 12, borderBottom: `1px solid ${T.lineSoft}` }}>
                    <span style={{ color: T.inkMid }}>{name}</span>
                    <span style={{ color: T.inkDim }}>{fmtNum(pop)}</span>
                  </div>
                ))}

                <div style={{ ...sectionHdr, marginTop: 10 }}>Merchant houses of this people</div>
                {sel.houses.length === 0 ? (
                  <div style={hint}>No trading houses of this people (yet).</div>
                ) : (
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
                    {sel.houses.map((h) => (
                      <span key={h} style={{ ...housePill, display: "inline-flex", alignItems: "center", gap: 5, paddingLeft: 4 }}>
                        <CoatOfArms name={h} size={16} />
                        <span style={{ width: 8, height: 8, borderRadius: 2, background: houseColor(h), border: "1px solid rgba(0,0,0,0.35)" }} />
                        {h}
                      </span>
                    ))}
                  </div>
                )}

                <button onClick={() => setSelected(sel.name)} style={{ ...mapBtn, marginTop: 12, borderColor: rgb(sel.color) }}>
                  ◧ colour the map by {sel.name}
                </button>
                <div style={{ color: T.inkFaint, fontSize: 10, marginTop: 6, lineHeight: 1.4 }}>
                  On the map: 75%+ solid ring &amp; fill · 45–74% half · 20–44% quarter · 5–19% ring only.
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

/** A toggleable mini world-map of where a people lives — homeland territory brightest,
 *  live cities lit too. Drawn exactly like the Goods Editor's suitability preview. */
function CulturePresence({ name, color }: { name: string; color: [number, number, number] }) {
  const [show, setShow] = useState(false);
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    if (!show) return;
    let cancelled = false;
    campaignGetCulturePresence(name).then((grid) => {
      const cv = ref.current;
      if (cancelled || !cv || grid.width === 0) return;
      cv.width = grid.width; cv.height = grid.height;
      const ctx = cv.getContext("2d"); if (!ctx) return;
      const img = ctx.createImageData(grid.width, grid.height);
      for (let p = 0; p < grid.data.length; p++) {
        const v = grid.data[p] / 255;
        const isLand = grid.land[p] === 1;
        const o = p * 4;
        const bR = isLand ? 26 : 8, bG = isLand ? 32 : 11, bB = isLand ? 42 : 16;
        img.data[o] = Math.round(bR + color[0] * v);
        img.data[o + 1] = Math.round(bG + color[1] * v);
        img.data[o + 2] = Math.round(bB + color[2] * v);
        img.data[o + 3] = 255;
      }
      ctx.putImageData(img, 0, 0);
    }).catch(() => {});
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [show, name]);
  return (
    <div style={{ marginBottom: 11 }}>
      <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", marginBottom: 5,
        color: T.inkDim, fontSize: 9.5, fontWeight: 700, textTransform: "uppercase", letterSpacing: 0.5 }}>
        <input type="checkbox" checked={show} onChange={(e) => setShow(e.target.checked)}
          style={{ accentColor: "#9b7ad0", width: 11, height: 11 }} />
        Where they live
      </label>
      {show && (
        <>
          <canvas ref={ref} style={{ width: "100%", height: 118, imageRendering: "pixelated",
            border: `1px solid ${T.lineSoft}`, borderRadius: 5, background: "#06090d", display: "block" }} />
          <div style={{ color: T.inkFaint, fontSize: 9, marginTop: 3 }}>
            Bright = homeland &amp; cities where this people lives · faint = other land.
          </div>
        </>
      )}
    </div>
  );
}

function Tile({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div style={{ flex: 1, background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: 6, padding: "5px 7px" }}>
      <div style={{ color: T.inkDim, fontSize: 8, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.4 }}>{label}</div>
      <div style={{ color: color ?? T.ink, fontSize: 13, fontWeight: 700 }}>{value}</div>
    </div>
  );
}

function fmtNum(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
  if (n >= 10_000) return (n / 1_000).toFixed(0) + "k";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "k";
  return Math.round(n).toLocaleString();
}

const panel: React.CSSProperties = {
  position: "absolute", top: 56, left: 300, width: 520, height: 420, zIndex: 120,
  display: "flex", flexDirection: "column",
  border: `1px solid ${T.line}`, borderRadius: 8,
  boxShadow: "0 12px 34px rgba(0,0,0,0.55)", color: T.ink, fontSize: 12,
};
const header: React.CSSProperties = {
  display: "flex", alignItems: "center", padding: "7px 10px", borderBottom: `1px solid ${T.line}`,
};
const sectionHdr: React.CSSProperties = {
  color: T.inkDim, fontSize: 9.5, fontWeight: 700, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 4,
};
const hint: React.CSSProperties = { color: T.inkDim, fontSize: 11, padding: 10 };
const clearBtn: React.CSSProperties = {
  padding: "2px 8px", borderRadius: 5, fontSize: 10, cursor: "pointer",
  border: `1px solid ${T.goldDim}`, background: "transparent", color: T.gold,
};
const mapBtn: React.CSSProperties = {
  padding: "5px 10px", borderRadius: 6, fontSize: 11, cursor: "pointer", width: "100%",
  border: `1px solid ${T.line}`, background: "rgba(255,255,255,0.04)", color: T.ink,
};
const diasporaBadge: React.CSSProperties = {
  fontSize: 9.5, color: "#e0b060", border: "1px solid #6a5426", background: "#241d0c",
  borderRadius: 10, padding: "1px 7px",
};
const creoleBadge: React.CSSProperties = {
  fontSize: 9.5, color: "#c8a0e0", border: "1px solid #4a3466", background: "#1e1430",
  borderRadius: 10, padding: "1px 7px",
};
const housePill: React.CSSProperties = {
  fontSize: 11, color: T.inkMid, border: `1px solid ${T.lineSoft}`, background: T.card,
  borderRadius: 5, padding: "2px 7px",
};
