import { useEffect, useMemo, useState } from "react";
import { getGoodsReport } from "@bridge";
import type { GoodPlacementRow, GoodsPlacementReport } from "@types";

/**
 * THE POST-GENERATION GOODS REPORT (§8.20).
 *
 * Answers "which goods actually made it onto this world, and which quietly did
 * not" — a question that had no answer at all before the report existed. A good
 * whose climate is absent from a world still gets placed, because the seeder
 * falls back to the least-bad cell rather than letting a good silently vanish
 * (§8.16's rule applied to belts); the consequence is that a failed placement
 * used to look exactly like a successful one, just in an implausible spot.
 *
 * The panel is therefore organised around the FLAGS first and the catalogue
 * second: absent goods, then flagged ones, then everything healthy grouped by
 * category. That ordering is the backend's (`build_placement_report` sorts by
 * flag rank), so the two cannot drift.
 *
 * Read-only. It reads `metadata["goods_report"]`, so it survives a save and can
 * be reopened any time rather than being a one-shot toast the user can miss.
 */

/** What each flag means, in the user's terms rather than the code's. */
const FLAG_TEXT: Record<string, { label: string; tone: string; why: string }> = {
  absent: {
    label: "absent",
    tone: "#cc5544",
    why: "placed nothing — this world has no suitable climate for it",
  },
  fallback_seed: {
    label: "no real home",
    tone: "#cc8844",
    why:
      "placed only because the seeder fell back to the least-bad cell on the map. " +
      "This world may have no climate that genuinely suits it, so wherever it " +
      "landed is a compromise, not a homeland.",
  },
  ubiquitous: {
    label: "everywhere",
    tone: "#cc8844",
    why: "a non-staple covering over a quarter of the world — almost always a scoring mistake",
  },
  single_cell: {
    label: "a single cell",
    tone: "#cc8844",
    why: "a belt of two cells or fewer: technically present, effectively unreachable",
  },
};

const DIST_TEXT: Record<string, string> = {
  global: "everywhere suitable",
  local: "seeded homelands",
  endemic: "endemic — one landmass",
  deposits: "ore districts",
  manufactured: "made in cities",
};

export function GoodsReportPanel({ onClose }: { onClose: () => void }) {
  const [report, setReport] = useState<GoodsPlacementReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showHealthy, setShowHealthy] = useState(false);

  useEffect(() => {
    let live = true;
    getGoodsReport()
      .then((r) => { if (live) setReport(r); })
      .catch((e) => { if (live) setError(String(e)); });
    return () => { live = false; };
  }, []);

  const { absent, flagged, healthy } = useMemo(() => {
    const rows = report?.rows ?? [];
    return {
      absent: rows.filter((r) => r.flags.includes("absent")),
      flagged: rows.filter((r) => r.flags.length > 0 && !r.flags.includes("absent")),
      healthy: rows.filter((r) => r.flags.length === 0),
    };
  }, [report]);

  // Healthy goods grouped by their need category, so the list reads as a
  // catalogue ("what does this world offer") rather than as a flat dump.
  const byCategory = useMemo(() => {
    const m = new Map<string, GoodPlacementRow[]>();
    for (const r of healthy) {
      const k = r.category || "other";
      const arr = m.get(k);
      if (arr) arr.push(r); else m.set(k, [r]);
    }
    return [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  }, [healthy]);

  return (
    <div style={wrap}>
      <div style={header}>
        <span style={{ fontWeight: 600 }}>{"\u{1F33F}"} Goods placement</span>
        <button onClick={onClose} style={closeBtn}>{"×"}</button>
      </div>

      {error && <div style={note}>Could not read the report: {error}</div>}
      {!report && !error && <div style={note}>Reading…</div>}

      {report && report.rows.length === 0 && (
        <div style={note}>
          No report for this world. Run the <b>Biological</b> step (8) to generate
          goods — worlds made before the report existed carry no placement record.
        </div>
      )}

      {report && report.rows.length > 0 && (
        <>
          <div style={summary}>
            <Stat n={report.enabled} label="enabled" />
            <Stat n={report.placed} label="placed" />
            <Stat n={report.absent} label="absent" tone={report.absent > 0 ? "#cc5544" : undefined} />
            <Stat n={report.flagged} label="flagged" tone={report.flagged > 0 ? "#cc8844" : undefined} />
          </div>

          {absent.length > 0 && (
            <Section title="Absent — no suitable climate on this world">
              {absent.map((r) => <Row key={r.id} r={r} />)}
            </Section>
          )}

          {flagged.length > 0 && (
            <Section title="Flagged — placed, but read the reason">
              {flagged.map((r) => <Row key={r.id} r={r} />)}
            </Section>
          )}

          <div style={{ ...sectionTitle, cursor: "pointer" }} onClick={() => setShowHealthy((v) => !v)}>
            {showHealthy ? "▾" : "▸"} On the map ({healthy.length})
          </div>
          {showHealthy && byCategory.map(([cat, rows]) => (
            <Section key={cat} title={cat}>
              {rows.map((r) => <Row key={r.id} r={r} />)}
            </Section>
          ))}
        </>
      )}
    </div>
  );
}

function Stat({ n, label, tone }: { n: number; label: string; tone?: string }) {
  return (
    <div style={{ textAlign: "center", minWidth: 54 }}>
      <div style={{ fontSize: 16, fontWeight: 600, color: tone ?? "#d8e0e8" }}>{n}</div>
      <div style={{ fontSize: 9, color: "#7a8794" }}>{label}</div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ marginTop: 6 }}>
      <div style={sectionTitle}>{title}</div>
      {children}
    </div>
  );
}

function Row({ r }: { r: GoodPlacementRow }) {
  const [open, setOpen] = useState(false);
  const flag = r.flags[0] ? FLAG_TEXT[r.flags[0]] : undefined;
  return (
    <div style={row} onClick={() => setOpen((v) => !v)}>
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ width: 16 }}>{r.icon}</span>
        <span style={{ flex: 1, color: "#d8e0e8" }}>{r.name}</span>
        {flag && (
          <span style={{ ...pill, color: flag.tone, borderColor: flag.tone }}>{flag.label}</span>
        )}
        {!flag && (
          <span style={{ fontSize: 10, color: "#7a8794" }}>
            {/* Land share is the honest headline number: "how much of this world
                grows it", which a raw cell count cannot convey across world sizes. */}
            {(r.land_share * 100).toFixed(r.land_share < 0.01 ? 2 : 1)}%
          </span>
        )}
      </div>
      {open && (
        <div style={detail}>
          <div>{DIST_TEXT[r.distribution] ?? r.distribution}</div>
          <div>
            {r.cells.toLocaleString()} cells
            {r.origins > 0 && <> · {r.origins} {r.origins === 1 ? "homeland" : "homelands"}</>}
            {r.localities > 0 && <> · {r.localities} localities</>}
            {r.mean_grade > 0 && <> · mean grade {Math.round(r.mean_grade * 100)}%</>}
          </div>
          {r.notable.length > 0 && (
            <div style={{ color: "#9fb0c0" }}>
              Notable: {r.notable.slice(0, 6).join(", ")}
              {r.notable.length > 6 && ` +${r.notable.length - 6} more`}
            </div>
          )}
          {r.flags.map((f) => (
            <div key={f} style={{ color: FLAG_TEXT[f]?.tone ?? "#cc8844" }}>
              {FLAG_TEXT[f]?.why ?? f}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

const wrap: React.CSSProperties = {
  display: "flex", flexDirection: "column", gap: 2, fontSize: 11,
  background: "#0e1620", border: "1px solid #24313d", borderRadius: 4,
  padding: 8, maxHeight: "70vh", overflowY: "auto",
};
const header: React.CSSProperties = {
  display: "flex", alignItems: "center", justifyContent: "space-between",
  color: "#d8e0e8", marginBottom: 4,
};
const closeBtn: React.CSSProperties = {
  background: "none", border: "none", color: "#7a8794", fontSize: 15, cursor: "pointer",
};
const summary: React.CSSProperties = {
  display: "flex", gap: 10, justifyContent: "space-around",
  padding: "6px 0", borderBottom: "1px solid #1c2733", marginBottom: 2,
};
const sectionTitle: React.CSSProperties = {
  fontSize: 10, textTransform: "uppercase", letterSpacing: 0.5,
  color: "#7a8794", margin: "6px 0 2px",
};
const row: React.CSSProperties = {
  padding: "3px 4px", borderRadius: 3, cursor: "pointer", background: "#121b25",
  marginBottom: 2,
};
const detail: React.CSSProperties = {
  fontSize: 10, color: "#8fa0b0", padding: "3px 0 1px 22px",
  display: "flex", flexDirection: "column", gap: 2,
};
const pill: React.CSSProperties = {
  fontSize: 9, padding: "0 4px", border: "1px solid", borderRadius: 3,
};
const note: React.CSSProperties = { color: "#8fa0b0", fontSize: 10, padding: "6px 2px" };
