import { useEffect, useMemo, useRef, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useViewportStore } from "@state/viewportStore";
import { campaignGetCultures, campaignGetCulturePresence, campaignGetCultureHistory, campaignGetMigrationRoutes, campaignGetCultureNotables } from "@bridge";
import type { CultureBrief, MigrationRouteBrief, NotablePerson } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { CultureFigures } from "@ui/campaign/CultureFigures";
import { CoatOfArms, houseColor } from "@ui/heraldry/CoatOfArms";
import { GOOD_DEFS } from "@goods";
import { T, SERIF } from "@ui/campaign/chronicleTheme";

const GOOD_BY_NAME = new Map(GOOD_DEFS.map((g) => [g.name, g]));

/** Relation chip styling by kind (kin / friendly / rival / hostile). */
const REL_STYLE: Record<string, { icon: string; color: string; title: string }> = {
  kin: { icon: "🤝", color: "#8fd0a0", title: "Kindred — same language family" },
  friendly: { icon: "🕊", color: "#7fb0e0", title: "Friendly — peaceable neighbours who share cities" },
  rival: { icon: "⚖", color: "#e0b060", title: "Rivals — competing merchant peoples" },
  hostile: { icon: "⚔", color: "#e07a6a", title: "Hostile — friction between these peoples" },
};

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

  // Living peoples vs the VANISHED (extinct) ones, filed separately.
  const living = useMemo(() => cultures.filter((c) => c.alive !== false), [cultures]);
  const dead = useMemo(() => cultures.filter((c) => c.alive === false), [cultures]);
  // Top-3 wealthiest living peoples get a 👑 in the list (richest-cultures ranking).
  const richest = useMemo(() => new Set(
    [...living].sort((a, b) => (b.wealth ?? 0) - (a.wealth ?? 0)).slice(0, 3).map((c) => c.name)
  ), [living]);

  if (!open) return null;

  return (
    <div data-draggable style={{ ...panel, ...rootStyle }}>
      <div style={{ ...header, cursor: "move" }} onPointerDown={onPointerDown}>
        <span style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: 14, letterSpacing: 0.4 }}>
          👥 Peoples
        </span>
        <span style={{ color: T.inkFaint, fontSize: 10, marginLeft: 8 }}>
          {active ? `${living.length} peoples${dead.length ? ` · ${dead.length} vanished` : ""} · Year ${year}` : "no campaign"}
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
          {/* ── Left: the culture list (living, then the vanished) ── */}
          <div style={{ width: 168, overflowY: "auto", borderRight: `1px solid ${T.lineSoft}`, padding: "4px 0" }}>
            {living.map((c) => {
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
                  {richest.has(c.name) && <span title="Among the richest peoples" style={{ fontSize: 10 }}>👑</span>}
                  {c.mobility >= 0.7 && <span title="Travel-prone merchant diaspora" style={{ fontSize: 10 }}>⚓</span>}
                  <span style={{ color: T.inkFaint, fontSize: 10 }}>{fmtNum(c.population)}</span>
                </div>
              );
            })}
            {dead.length > 0 && (
              <div style={{ color: T.inkFaint, fontSize: 9, fontWeight: 700, textTransform: "uppercase",
                letterSpacing: 0.6, padding: "9px 9px 3px", borderTop: `1px solid ${T.lineSoft}`, marginTop: 4 }}>
                ✝ Vanished peoples
              </div>
            )}
            {dead.map((c) => {
              const on = c.name === selected;
              return (
                <div key={c.name} onClick={() => setSelected(on ? null : c.name)}
                  title="An extinct people — no living settlement holds it"
                  style={{ display: "flex", alignItems: "center", gap: 7, padding: "5px 9px", cursor: "pointer",
                    background: on ? "rgba(255,255,255,0.05)" : "transparent", opacity: 0.55,
                    borderLeft: `3px solid ${on ? rgb(c.color) : "transparent"}` }}>
                  <span style={{ width: 11, height: 11, borderRadius: 3, background: rgb(c.color), flexShrink: 0, border: "1px solid rgba(0,0,0,0.4)", filter: "grayscale(0.5)" }} />
                  <span style={{ flex: 1, color: T.inkDim, fontSize: 12, fontStyle: "italic", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", textDecoration: "line-through" }}>
                    {c.name}
                  </span>
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
                {(sel.lingua_regions ?? 0) > 0 && (
                  <div title="This people's tongue is the trade language of one or more regions — it eases assimilation there." style={{
                    display: "inline-flex", alignItems: "center", gap: 5, fontSize: 10.5, marginBottom: 6,
                    padding: "3px 8px", borderRadius: 6, color: "#e0c98a",
                    background: "rgba(203,162,74,0.12)", border: "1px solid rgba(203,162,74,0.4)" }}>
                    🗣 <b>Lingua franca</b> — the trade tongue of {sel.lingua_regions} region{(sel.lingua_regions ?? 0) > 1 ? "s" : ""}
                  </div>
                )}
                {sel.origin && (
                  <div style={{ fontFamily: SERIF, fontStyle: "italic", color: T.inkMid, fontSize: 12,
                    lineHeight: 1.5, marginBottom: 10, padding: "7px 9px", background: T.card,
                    border: `1px solid ${T.lineSoft}`, borderRadius: 6 }}>
                    {sel.origin}
                  </div>
                )}
                {/* Obituary — for an extinct people. */}
                {sel.alive === false && sel.obituary && (
                  <div style={{ fontFamily: SERIF, fontStyle: "italic", color: "#c9a9a9", fontSize: 12,
                    lineHeight: 1.55, marginBottom: 10, padding: "8px 10px", background: "rgba(60,20,24,0.35)",
                    border: "1px solid rgba(150,70,70,0.4)", borderRadius: 6 }}>
                    ✝ {sel.obituary}
                  </div>
                )}
                {/* Character traits (2–3). */}
                {(sel.traits?.length ?? 0) > 0 && (
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 5, marginBottom: 10 }}>
                    {sel.traits!.map((t) => (
                      <span key={t.name} title={t.blurb}
                        style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 11, fontWeight: 600,
                          color: T.ink, border: `1px solid ${T.lineSoft}`, background: T.card, borderRadius: 12,
                          padding: "2px 9px", cursor: "help" }}>
                        <span>{t.emoji}</span>{t.name}
                      </span>
                    ))}
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
                  {sel.alive !== false && (sel.wealth ?? 0) > 0 && (
                    <Tile label="Wealth" value={fmtNum(sel.wealth!)} color={richest.has(sel.name) ? "#e8c860" : T.inkMid} />
                  )}
                </div>
                {/* Relations — kin, friends, rivals, foes. */}
                {(sel.relations?.length ?? 0) > 0 && (
                  <>
                    <div style={sectionHdr}>Relations with other peoples</div>
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 5, marginBottom: 10 }}>
                      {sel.relations!.map((r) => {
                        const s = REL_STYLE[r.kind] ?? REL_STYLE.friendly;
                        return (
                          <span key={r.name + r.kind} title={s.title}
                            onClick={() => setSelected(r.name)}
                            style={{ display: "inline-flex", alignItems: "center", gap: 4, fontSize: 11, cursor: "pointer",
                              color: s.color, border: `1px solid ${s.color}55`, background: `${s.color}18`,
                              borderRadius: 12, padding: "2px 9px" }}>
                            <span>{s.icon}</span>{r.name}
                          </span>
                        );
                      })}
                    </div>
                  </>
                )}

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

                {/* Population over time — sampled every 6 months. */}
                <CulturePopChart name={sel.name} color={sel.color} tick={snapshot?.clock?.tick ?? 0} />

                {/* Migration — where this people is moving (from the live migration flows). */}
                <CultureMigration name={sel.name} color={sel.color}
                  hubs={snapshot?.hubs ?? []} tick={snapshot?.clock?.tick ?? 0} />

                {/* Notable people — magnates & dynastic heads, with clickable cities. */}
                <NotablePeople name={sel.name} tick={snapshot?.clock?.tick ?? 0}
                  onLocate={(x, y) => setSearchPin(x, y)} />

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

                {sel.alive !== false && (
                  <>
                    <button onClick={() => setSelected(sel.name)} style={{ ...mapBtn, marginTop: 12, borderColor: rgb(sel.color) }}>
                      ◧ colour the map by {sel.name}
                    </button>
                    <div style={{ color: T.inkFaint, fontSize: 10, marginTop: 6, lineHeight: 1.4 }}>
                      On the map: 75%+ solid ring &amp; fill · 45–74% half · 20–44% quarter · 5–19% ring only.
                    </div>
                  </>
                )}
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
      const dom = grid.dominant;
      for (let p = 0; p < grid.data.length; p++) {
        const raw = grid.data[p];
        const isLand = grid.land[p] === 1;
        const isDominant = dom ? dom[p] === 1 : raw >= 250;
        // Intensity: full colour where this people is DOMINANT (homeland / local
        // majority); a faint ~30% wash where it's only a MINORITY presence; a dim
        // backdrop for other land. This makes "where they rule" read apart from
        // "where they merely live among others".
        let v = raw / 255;
        if (!isDominant && raw > 40) v = Math.min(v, 0.3); // non-dominant presence → ~30%
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
            Full colour = where this people is <b>dominant</b> (homeland &amp; majority) · faint
            wash = where it lives as a <b>minority</b> · dim = other land.
          </div>
        </>
      )}
    </div>
  );
}

function fmtK(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return Math.round(n / 1_000) + "k";
  return String(Math.round(n));
}

/** Population of one people over time — a line chart sampled every 6 months. */
function CulturePopChart({ name, color, tick }: { name: string; color: [number, number, number]; tick: number }) {
  const [pts, setPts] = useState<[number, number][]>([]);
  useEffect(() => {
    let ok = true;
    campaignGetCultureHistory(name).then((r) => { if (ok) setPts(r); }).catch(() => {});
    return () => { ok = false; };
  }, [name, tick]);
  if (pts.length < 2) return null;
  const w = 260, h = 82, pad = 4;
  const xs = pts.map((p) => p[0]), ys = pts.map((p) => p[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  const y1 = Math.max(...ys, 1);
  const sx = (x: number) => pad + (x1 > x0 ? (x - x0) / (x1 - x0) : 0) * (w - 2 * pad);
  const sy = (y: number) => (h - pad) - (y / y1) * (h - 2 * pad);
  const rgb = `rgb(${color[0]},${color[1]},${color[2]})`;
  const line = pts.map((p, i) => `${i ? "L" : "M"}${sx(p[0]).toFixed(1)} ${sy(p[1]).toFixed(1)}`).join(" ");
  const area = `${line} L${sx(x1).toFixed(1)} ${h - pad} L${sx(x0).toFixed(1)} ${h - pad} Z`;
  const now = pts[pts.length - 1][1];
  return (
    <div style={{ marginBottom: 11 }}>
      <div style={{ ...sectionHdr, display: "flex", justifyContent: "space-between" }}>
        <span>Population over time</span>
        <span style={{ color: rgb, fontWeight: 700 }}>{fmtK(now)}</span>
      </div>
      <svg viewBox={`0 0 ${w} ${h}`} style={{ width: "100%", height: 82, display: "block" }}>
        <path d={area} fill={rgb} opacity={0.14} />
        <path d={line} fill="none" stroke={rgb} strokeWidth={1.6} />
        <circle cx={sx(x1)} cy={sy(now)} r={2.4} fill={rgb} />
        <text x={pad} y={h - 1} fill={T.inkFaint} fontSize={7}>Yr {Math.round(x0)}</text>
        <text x={w - pad} y={h - 1} fill={T.inkFaint} fontSize={7} textAnchor="end">Yr {Math.round(x1)}</text>
        <text x={pad} y={9} fill={T.inkFaint} fontSize={7}>{fmtK(y1)}</text>
      </svg>
      <div style={{ color: T.inkFaint, fontSize: 9, marginTop: 1 }}>Sampled every 6 months.</div>
    </div>
  );
}

/** Notable people — the great merchant magnates / dynastic heads of this culture,
 *  with a short bio and the cities they reached (clickable to locate on the map). */
function NotablePeople({ name, tick, onLocate }: {
  name: string; tick: number; onLocate: (x: number, y: number) => void;
}) {
  const [people, setPeople] = useState<NotablePerson[]>([]);
  useEffect(() => {
    let ok = true;
    campaignGetCultureNotables(name).then((r) => { if (ok) setPeople(r); }).catch(() => {});
    return () => { ok = false; };
  }, [name, tick]);
  if (people.length === 0) return null;
  return (
    <div style={{ marginBottom: 11 }}>
      <div style={sectionHdr}>Notable people</div>
      {people.map((p, i) => (
        <div key={i} style={{ padding: "6px 0", borderBottom: `1px solid ${T.lineSoft}` }}>
          <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
            <span style={{ fontFamily: SERIF, color: p.alive ? T.ink : T.inkDim, fontSize: 13, fontWeight: 700,
              fontStyle: p.alive ? "normal" : "italic" }}>{p.name}</span>
            <span style={{ color: T.inkFaint, fontSize: 10 }}>· {p.house}</span>
            {!p.alive && <span style={{ color: T.inkFaint, fontSize: 9 }}>✝</span>}
          </div>
          <div style={{ color: T.inkFaint, fontSize: 9.5, marginBottom: 2 }}>{p.era}</div>
          <div style={{ fontFamily: SERIF, fontStyle: "italic", color: T.inkMid, fontSize: 11.5, lineHeight: 1.45 }}>
            {p.known_for}.
          </div>
          {p.cities.length > 0 && (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 3, alignItems: "center" }}>
              <span style={{ color: T.inkFaint, fontSize: 9.5 }}>
                {p.cities.length > 1 ? "seat & reach:" : "seat:"}
              </span>
              {p.cities.map((c, ci) => (
                <span key={ci} onClick={() => onLocate(c.x, c.y)}
                  title={c.role === "seat" ? "Home seat — click to locate" : "A city this house reached — click to locate"}
                  style={{ cursor: "pointer", fontSize: 10.5, color: c.role === "seat" ? "#e6cf9a" : "#7fb0e0",
                    border: `1px solid ${c.role === "seat" ? "#e6cf9a55" : "#7fb0e055"}`, borderRadius: 10, padding: "1px 7px" }}>
                  {c.role === "seat" ? "★ " : "→ "}{c.name}
                </span>
              ))}
            </div>
          )}
        </div>
      ))}
      <div style={{ color: T.inkFaint, fontSize: 9, marginTop: 3 }}>
        Great merchant houses of this people — their magnates, monopolies and the cities their agents reached.
      </div>
    </div>
  );
}

/** Migration — where this people is moving now, aggregated from live migration flows. */
function CultureMigration({ name, color, hubs, tick }: {
  name: string; color: [number, number, number]; hubs: { id: number; name: string }[]; tick: number;
}) {
  const [routes, setRoutes] = useState<MigrationRouteBrief[]>([]);
  useEffect(() => {
    let ok = true;
    campaignGetMigrationRoutes().then((r) => { if (ok) setRoutes(r); }).catch(() => {});
    return () => { ok = false; };
  }, [tick]);
  const dests = useMemo(() => {
    const m = new Map<number, number>();
    for (const r of routes) if (r.culture === name && r.to_hub >= 0) m.set(r.to_hub, (m.get(r.to_hub) ?? 0) + r.volume);
    return [...m.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6);
  }, [routes, name]);
  const total = dests.reduce((s, [, v]) => s + v, 0);
  const rgb = `rgb(${color[0]},${color[1]},${color[2]})`;
  const nameOf = (i: number) => hubs[i]?.name ?? `#${i}`;
  return (
    <div style={{ marginBottom: 11 }}>
      <div style={sectionHdr}>Where they move</div>
      {dests.length === 0 ? (
        <div style={{ color: T.inkFaint, fontSize: 10 }}>No recent migration of this people.</div>
      ) : (
        <>
          <div style={{ fontSize: 10, color: T.inkDim, marginBottom: 3 }}>{fmtK(total)} recently on the move →</div>
          {dests.map(([hub, vol]) => (
            <div key={hub} style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11, padding: "2px 2px" }}>
              <span style={{ width: 6, height: 6, borderRadius: 3, background: rgb, flex: "0 0 auto" }} />
              <span style={{ flex: 1, color: T.inkMid, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{nameOf(hub)}</span>
              <span style={{ color: rgb }}>{fmtK(vol)}</span>
            </div>
          ))}
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
