import { useEffect, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { useViewportStore } from "@state/viewportStore";
import { campaignGetTradingPosts } from "@bridge";
import type { TradingPost } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { Panel, PanelHeader, PanelBody, EmptyNote } from "@ui/kit";
import { T, FZ } from "@ui/campaign/chronicleTheme";

/** TRADE_STAGING_AND_POSTS_PLAN.md Slice 7 — the Trading Posts window.
 *
 *  A scoped-down reading of the plan's own window: the ROSTER (owner, motive,
 *  writ, rung, transit, trend) — the concrete, well-specified part. "Who calls
 *  here" / the ban list / a trajectory chart / map symbology (posts as squares,
 *  lanes drawn by leg, a provisioning-range wash) are NOT built here — they
 *  need either new per-post query commands this slice didn't add, or a Canvas
 *  2D rendering change nobody could visually verify in this session. Named so
 *  a later session doesn't assume they were forgotten (the codebase's own
 *  "deliberately not built" discipline, CLAUDE.md §6). */
export function TradingPostsPanel() {
  const open = useUIStore((s) => s.showTradingPosts);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const focusOn = useViewportStore((s) => s.focusOn);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;
  const year = Math.floor(tick / 365);

  const [posts, setPosts] = useState<TradingPost[]>([]);

  useEffect(() => {
    let alive = true;
    if (!open || !active) return;
    campaignGetTradingPosts().then((p) => { if (alive) setPosts(p); }).catch(() => { if (alive) setPosts([]); });
    return () => { alive = false; };
  }, [open, active, year]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.route);
  if (!open) return null;
  const close = () => useUIStore.getState().setShowTradingPosts(false);
  const rungLabel = (r: number) => r >= 2 ? "entrepôt" : r >= 1 ? "trade hub" : "ordinary";
  const motiveLabel = (m: number) => m === 4 ? "route" : "resource";

  return (
    <Panel onPointerDown={onPointerDown} width={720} maxHeight="78vh"
      style={{ top: 90, left: 340, zIndex: 41, ...rootStyle }}>
      <PanelHeader icon="🐫" title="Trading Posts" onDragStart={onPointerDown} onClose={close} />
      {!active && <EmptyNote>Begin the campaign (Step 11) — no posts have been founded yet.</EmptyNote>}
      {active && posts.length === 0 && (
        <EmptyNote>No live outposts or route posts yet — a house needs real wealth (and, for a
          route post, a genuinely stranded lane) before either is founded.</EmptyNote>
      )}
      {active && posts.length > 0 && (
        <PanelBody style={{ overflowY: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: FZ.small }}>
            <thead>
              <tr style={{ color: T.inkDim, textAlign: "left", borderBottom: `1px solid ${T.line}` }}>
                <th style={{ padding: "4px 6px" }}>post</th>
                <th style={{ padding: "4px 6px" }}>motive</th>
                <th style={{ padding: "4px 6px" }}>owner</th>
                <th style={{ padding: "4px 6px" }}>rung</th>
                <th style={{ padding: "4px 6px" }}>pop</th>
                <th style={{ padding: "4px 6px" }}>transit/yr</th>
                <th style={{ padding: "4px 6px" }}>writ</th>
                <th style={{ padding: "4px 6px" }}>age</th>
              </tr>
            </thead>
            <tbody>
              {posts.map((p) => (
                <tr key={p.hub}
                  onClick={() => focusOn(p.x, p.y)}
                  style={{ cursor: "pointer", borderBottom: `1px solid ${T.lineSoft}` }}>
                  <td style={{ padding: "4px 6px", color: T.ink }}>
                    {p.name}
                    {p.decline_years > 0 && <span style={{ color: T.warn, marginLeft: 4 }}>↓</span>}
                    {p.barred_houses.length > 0 && <span style={{ color: T.bad, marginLeft: 4 }}>⛔</span>}
                  </td>
                  <td style={{ padding: "4px 6px" }}>{motiveLabel(p.motive)}</td>
                  <td style={{ padding: "4px 6px" }}>{p.owner_house || "—"}</td>
                  <td style={{ padding: "4px 6px" }}>{p.graduated ? rungLabel(p.rung) : "founding"}</td>
                  <td style={{ padding: "4px 6px" }}>{Math.round(p.population).toLocaleString()}</td>
                  <td style={{ padding: "4px 6px" }}>{Math.round(p.transit_year).toLocaleString()}</td>
                  <td style={{ padding: "4px 6px" }}>{p.writ_holder || "free"}</td>
                  <td style={{ padding: "4px 6px" }}>{p.age_years}y</td>
                </tr>
              ))}
            </tbody>
          </table>
        </PanelBody>
      )}
    </Panel>
  );
}
