import { useEffect, useMemo, useRef, useState } from "react";
import { useUIStore } from "@state/uiStore";
import { useCampaignStore } from "@state/campaignStore";
import { campaignGetNotableIndividuals, campaignGetHallOfDead } from "@bridge";
import type { IndividualBrief } from "@types";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import { T, FZ, SPACE, SERIF, RADIUS } from "@ui/campaign/chronicleTheme";
import { Panel, PanelHeader, PanelBody, Chip, EmptyNote } from "@ui/kit";
import { drawBust, individualKit } from "@ui/campaign/cultureDress";

/** 02_PEOPLE.md (Living World row 02) · the 40-cap notable roster and the
 *  Hall of the Dead in one window — a plain list, PLUS (02.7) a real portrait
 *  on the detail view: the culture's own dress plate, varied per person by
 *  `face_seed` and carrying their real `female`/`features` (an eyepatch, grey
 *  hair, a scar…) straight off the stored `Individual`. Not yet the full
 *  gallery redesign (`FiguresPanel.tsx` becoming the roster's front page is
 *  still future work) — this panel still reads exactly what the backend
 *  serves and invents nothing beyond the one portrait. */
export function NotablesPanel() {
  const open = useUIStore((s) => s.showNotables);
  const close = () => useUIStore.getState().setShowNotables(false);
  const snapshot = useCampaignStore((s) => s.snapshot);
  const tick = snapshot?.clock?.tick ?? 0;
  const active = !!snapshot?.active;

  const [tab, setTab] = useState<"living" | "dead">("living");
  const [living, setLiving] = useState<IndividualBrief[]>([]);
  const [dead, setDead] = useState<IndividualBrief[]>([]);
  const [picked, setPicked] = useState<number | null>(null);

  useEffect(() => {
    if (!open || !active) return;
    campaignGetNotableIndividuals().then(setLiving).catch(() => setLiving([]));
    campaignGetHallOfDead().then(setDead).catch(() => setDead([]));
  }, [open, active, tick]);

  const rows = tab === "living" ? living : dead;
  const pickedRow = useMemo(() => rows.find((r) => r.id === picked) ?? null, [rows, picked]);

  const portraitRef = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = portraitRef.current;
    if (!el || !pickedRow) return;
    const size = 72, dpr = 2;
    el.width = size * dpr; el.height = size * dpr;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    const kit = individualKit(pickedRow.culture || "unknown", pickedRow.face_seed);
    drawBust(ctx, 0, 0, size * dpr, kit, {
      occasion: pickedRow.famous ? "ceremonial" : "national",
      female: pickedRow.female,
      features: pickedRow.features,
    });
  }, [pickedRow]);

  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.notables);
  if (!open) return null;

  return (
    <Panel onPointerDown={onPointerDown} width={380} maxHeight="80%" style={{ top: 70, right: 420, zIndex: 117, ...rootStyle }}>
      <PanelHeader icon="👥" title="Notables" onDragStart={onPointerDown} onClose={close} />
      <PanelBody style={{ display: "flex", flexDirection: "column", flex: 1, padding: `${SPACE.md}px ${SPACE.lg}px ${SPACE.lg}px` }}>
        {!active && <EmptyNote>Start the campaign to see notable people.</EmptyNote>}
        {active && (
          <>
            <div style={{ display: "flex", gap: 6, marginBottom: SPACE.md }}>
              <Chip on={tab === "living"} onClick={() => { setTab("living"); setPicked(null); }}>Living ({living.length})</Chip>
              <Chip on={tab === "dead"} onClick={() => { setTab("dead"); setPicked(null); }}>Hall of the Dead ({dead.length})</Chip>
            </div>
            {rows.length === 0 && (
              <EmptyNote>{tab === "living" ? "No one has become notable yet." : "No notable has died yet."}</EmptyNote>
            )}
            {pickedRow ? (
              <div>
                <div data-no-drag onClick={() => setPicked(null)} style={{ cursor: "pointer", color: T.inkDim, fontSize: FZ.tiny, marginBottom: 6 }}>
                  ← back to the list
                </div>
                <div style={{ display: "flex", gap: SPACE.md, alignItems: "flex-start" }}>
                  <canvas ref={portraitRef} style={{ width: 72, height: 72, borderRadius: RADIUS.sm, flexShrink: 0, background: T.card }} />
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontFamily: SERIF, color: T.parchment, fontSize: FZ.title }}>{pickedRow.name}</div>
                    <div style={{ color: T.inkMid, fontSize: FZ.small, marginBottom: 6 }}>
                      {pickedRow.roles.join(", ") || "—"} · {pickedRow.city || "—"} · {pickedRow.culture || "unknown culture"}
                    </div>
                  </div>
                </div>
                <div style={{ color: T.inkDim, fontSize: FZ.tiny, marginBottom: 8 }}>
                  {pickedRow.alive ? `Debuted ${pickedRow.debut_year}` : `${pickedRow.debut_year} – ${pickedRow.death_year} (${pickedRow.death_cause})`}
                </div>
                {pickedRow.traits.length > 0 && (
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 8 }}>
                    {pickedRow.traits.map((t) => <Chip key={t}>{t}</Chip>)}
                  </div>
                )}
                <div style={{ borderTop: `1px solid ${T.lineSoft}`, paddingTop: 6 }}>
                  {pickedRow.life_log.length === 0 && <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>A quiet life, so far.</div>}
                  {pickedRow.life_log.slice().reverse().map((line, i) => (
                    <div key={i} style={{ color: T.inkMid, fontSize: FZ.small, padding: "3px 0", borderBottom: i < pickedRow.life_log.length - 1 ? `1px solid ${T.lineSoft}` : "none" }}>
                      {line}
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div style={{ overflowY: "auto", flex: 1 }}>
                {rows.map((p) => (
                  <div key={p.id} data-no-drag onClick={() => setPicked(p.id)}
                    style={{
                      display: "flex", justifyContent: "space-between", alignItems: "center",
                      padding: "6px 4px", cursor: "pointer", borderRadius: RADIUS.sm,
                      borderBottom: `1px solid ${T.lineSoft}`,
                    }}>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ color: T.parchment, fontSize: FZ.small, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.name}</div>
                      <div style={{ color: T.inkDim, fontSize: FZ.tiny }}>{p.roles[0] ?? "Notable"} · {p.city || "—"}</div>
                    </div>
                    <div style={{ color: T.inkDim, fontSize: FZ.tiny, flexShrink: 0 }}>
                      {tab === "living" ? `fame ${p.fame.toFixed(2)}` : `d. ${p.death_year}`}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </PanelBody>
    </Panel>
  );
}
