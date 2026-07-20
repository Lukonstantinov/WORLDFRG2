import { useState } from "react";

/** One chronicle entry, already reduced to a year. */
export interface ChronicleEntry {
  year: number;
  kind: string;
  text: string;
}

/** A chronicle grouped BY YEAR into collapsible rows: each year shows a one-line
 *  header (year · how many things happened · a glance of icons); click it to
 *  expand and read what actually happened that year. Newest year first, and the
 *  most recent year starts expanded. Shared by the house/guild timeline and the
 *  settlement history so all three read the same clean way. */
export function YearChronicle({
  entries,
  icons = {},
  colors = {},
  emptyText = "No recorded events yet.",
}: {
  entries: ChronicleEntry[];
  icons?: Record<string, string>;
  colors?: Record<string, string>;
  emptyText?: string;
}) {
  // Group into { year → entries }, newest year first.
  const byYear = new Map<number, ChronicleEntry[]>();
  for (const e of entries) {
    const arr = byYear.get(e.year);
    if (arr) arr.push(e); else byYear.set(e.year, [e]);
  }
  const years = [...byYear.keys()].sort((a, b) => b - a);

  // Most recent year expanded by default; the rest collapsed.
  const [open, setOpen] = useState<Set<number>>(() => new Set(years.slice(0, 1)));
  const toggle = (y: number) =>
    setOpen((prev) => {
      const next = new Set(prev);
      next.has(y) ? next.delete(y) : next.add(y);
      return next;
    });

  if (years.length === 0) return <div style={emptyStyle}>{emptyText}</div>;

  return (
    <div>
      {years.map((y) => {
        const evs = byYear.get(y)!;
        const isOpen = open.has(y);
        // A compact glance: the distinct event icons that year (deduped, max 6).
        const glance = [...new Set(evs.map((e) => icons[e.kind] ?? "•"))].slice(0, 6).join(" ");
        return (
          <div key={y} style={{ borderBottom: "1px solid #131f2c" }}>
            <div
              onClick={() => toggle(y)}
              style={{
                display: "flex", alignItems: "center", gap: 7, cursor: "pointer",
                padding: "4px 2px", userSelect: "none",
              }}
              title={isOpen ? "Collapse" : "Expand to see what happened"}
            >
              <span style={{ color: "#6a86a6", fontSize: 9, width: 9, textAlign: "center" }}>
                {isOpen ? "▾" : "▸"}
              </span>
              <span style={{ color: "#cfe0f4", fontWeight: 700, fontSize: 11, minWidth: 54 }}>Year {y}</span>
              <span style={{ color: "#56708e", fontSize: 9 }}>
                {evs.length} event{evs.length === 1 ? "" : "s"}
              </span>
              <span style={{ flex: 1 }} />
              {!isOpen && <span style={{ fontSize: 11, letterSpacing: 1 }}>{glance}</span>}
            </div>
            {isOpen && (
              <div style={{ position: "relative", paddingLeft: 16, paddingBottom: 5 }}>
                <div style={{ position: "absolute", left: 5, top: 2, bottom: 4, width: 2, background: "#1c2c40" }} />
                {evs.map((e, i) => (
                  <div key={i} style={{ position: "relative", marginBottom: 5 }}>
                    <span style={{ position: "absolute", left: -13, top: 0, fontSize: 11 }}>{icons[e.kind] ?? "•"}</span>
                    <div style={{ color: colors[e.kind] ?? "#c0d0e0", fontSize: 11, lineHeight: 1.3 }}>{e.text}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

const emptyStyle: React.CSSProperties = { color: "#506080", fontSize: 11, padding: "8px 4px" };
