import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import type { IndividualBrief } from "@types";
import { drawBust, individualKit } from "@ui/campaign/cultureDress";

/** Shared "who holds this office / this role" chip — a bust portrait (the
 *  same `cultureDress` renderer `NotablesPanel.tsx` already draws with),
 *  a name, an optional caption, and a click-to-expand life story. Used by
 *  the Government tab's seats (`HubPanel.tsx`) and the Ideology section's
 *  scholar list (`SettlementOverviewTab.tsx`) so the bust + blue "scholarly"
 *  tint + expand behaviour cannot drift between the two call sites. */

/** `role_name()`'s exact three "carries a doctrine" roles (individuals.rs) —
 *  matches `FiguresPanel.tsx`'s own `ROLE` map, so a scholarly figure reads
 *  the same blue wherever this app shows them. */
const SCHOLARLY_ROLES = ["Scholar", "Philosopher", "Ideologue"];
/** Same blue `FiguresPanel.tsx` already uses for its "Scholar" role card —
 *  reused here rather than inventing a second blue, per this task's own
 *  "pick one blue and use it everywhere" rule. */
export const SCHOLAR_BLUE = "#6a8fd8";

/** True when a person is one of the scholarly roles, OR when the seat they
 *  hold was won by `PATH_SCHOLAR` ("scholar/orator") — a seat installed
 *  under that path IS this mechanism even on the rare chance the resident's
 *  own role list has since changed. */
export function isScholarly(person: IndividualBrief | null | undefined, pathOverride?: string): boolean {
  if (pathOverride === "scholar/orator") return true;
  if (!person) return false;
  return person.roles.some((r) => SCHOLARLY_ROLES.includes(r));
}

function Bust({ person, size = 32 }: { person: IndividualBrief; size?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const dpr = 2;
    el.width = size * dpr; el.height = size * dpr;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    const kit = individualKit(person.culture || "unknown", person.face_seed);
    drawBust(ctx, 0, 0, size * dpr, kit, {
      occasion: person.famous ? "ceremonial" : "national",
      female: person.female,
      features: person.features,
    });
  }, [person, size]);
  return <canvas ref={ref} style={{ width: size, height: size, borderRadius: 6, flexShrink: 0, background: "#0d1622" }} />;
}

/** The expanded life-story block — bigger bust, the full trait list, and the
 *  full life log (not just its last line). */
function PersonExpanded({ person }: { person: IndividualBrief }) {
  return (
    <div style={{ marginTop: 5, display: "flex", gap: 8 }}>
      <Bust person={person} size={48} />
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ color: "#7a8aa0", fontSize: 9, marginBottom: 3 }}>
          {person.alive ? `Debuted ${person.debut_year}` : `${person.debut_year} – ${person.death_year} (${person.death_cause})`}
        </div>
        {person.traits.length > 0 && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 3, marginBottom: 4 }}>
            {person.traits.map((t) => (
              <span key={t} style={{ fontSize: 8.5, color: "#e8d9b0", border: "1px solid #4a4030", borderRadius: 8, padding: "1px 6px" }}>{t}</span>
            ))}
          </div>
        )}
        {person.life_log.length === 0 ? (
          <div style={{ color: "#6a86a6", fontSize: 9, fontStyle: "italic" }}>A quiet life, so far.</div>
        ) : (
          person.life_log.slice().reverse().map((line, i) => (
            <div key={i} style={{ color: "#9fb4cc", fontSize: 9.5, padding: "2px 0", borderTop: i > 0 ? "1px solid #1e2e42" : "none" }}>
              {line}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

/** A person chip: bust + name (+ caption) + (optionally) an inline expand.
 *  `pathOverride` lets a caller pass the SEAT's own `path` string (e.g.
 *  `"scholar/orator"`) so the blue tint applies even before the fetched
 *  `IndividualBrief`'s own roles would say so. */
export function PersonChip({
  person, label, sub, pathOverride, size = 32, onClick, expanded,
}: {
  person: IndividualBrief;
  label?: ReactNode;
  sub?: ReactNode;
  pathOverride?: string;
  size?: number;
  onClick?: () => void;
  expanded?: boolean;
}) {
  const scholarly = isScholarly(person, pathOverride);
  const tint = scholarly ? SCHOLAR_BLUE : undefined;
  return (
    <div data-no-drag onClick={onClick}
      style={{ display: "flex", gap: 7, alignItems: "flex-start", padding: "2px 0", cursor: onClick ? "pointer" : "default" }}>
      <div style={{ borderRadius: 7, boxShadow: tint ? `0 0 0 2px ${tint}` : "0 0 0 1px transparent" }}>
        <Bust person={person} size={size} />
      </div>
      <div style={{ minWidth: 0, flex: 1 }}>
        <div style={{ color: tint ?? "#e8dcc0", fontWeight: 600, fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {label ?? person.name}
        </div>
        {sub && <div style={{ color: "#7a90a8", fontSize: 9 }}>{sub}</div>}
        {expanded && <PersonExpanded person={person} />}
      </div>
    </div>
  );
}
