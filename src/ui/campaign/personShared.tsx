import { useEffect, useRef } from "react";
import type { ReactNode } from "react";
import type { IndividualBrief } from "@types";
import { drawBust, individualPerson, kitForCulture } from "@ui/campaign/cultureDress";
import { TraitChip, IdeologyBars } from "@ui/campaign/traitIcons";
import { LifeStory } from "@ui/campaign/LifeStory";
import { OpenLifeButton } from "@ui/campaign/PersonWindow";

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

export function Bust({ person, size = 32 }: { person: IndividualBrief; size?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const dpr = 2;
    el.width = size * dpr; el.height = size * dpr;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, el.width, el.height);
    drawBust(ctx, 0, 0, size * dpr, kitForCulture(person.culture || "unknown"), {
      occasion: person.famous ? "ceremonial" : "national",
      person: individualPerson(person),
      cols: size >= 90 ? 52 : 44,
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
          {person.age ? ` · age ${person.age}` : ""}
          {person.roles.length > 1 ? ` · ${person.roles.join(" → ")}` : ""}
        </div>
        {person.traits.length > 0 && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: 3, marginBottom: 4 }}>
            {person.traits.map((t, i) => <TraitChip key={t} name={t} strength={person.trait_strength?.[i] ?? 1} />)}
          </div>
        )}
        {person.ideology && (
          <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4, fontSize: 9, color: "#9a8ae0" }}>
            <IdeologyBars v={person.ideology} />
            <span>{person.ideology_name ? `leans to ${person.ideology_name}` : "no settled doctrine"}</span>
          </div>
        )}
        <div style={{ marginBottom: 4 }}><OpenLifeButton id={person.id} /></div>
        <LifeStory life={person.life} places={person.places} fallback={person.life_log.slice().reverse()} />
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

// ── the new people vocabulary (design handoff "Council & People") ─────────

/** One colour per role, so a role reads the same on a ladder, a ring and a chip. */
export const ROLE_COLOR: Record<string, string> = {
  Doge: "#d8b24a", Jarl: "#d8b24a", Khan: "#d8b24a", Burgomaster: "#d8b24a", Treasurer: "#e0a83a", "Harbour Master": "#5fa8e8",
  "Master of the Horse": "#b0503a", Justiciar: "#9fb4cc", Councillor: "#8a9a4a", Alderman: "#8a9a4a", Admiral: "#5fa8e8",
  Demagogue: "#e0735a", "Master Craftsman": "#d8b24a", "Great Banker": "#5cc08a", Explorer: "#b48ae0", Diplomat: "#4ac0c0",
  "Merchant Prince": "#e0a83a", Guildmaster: "#c98a3a", Philosopher: "#8a7ad8", Scholar: "#6a8fd8", Ideologue: "#c85a7a",
  Physician: "#4ac09a", Artisan: "#d8a24a", Official: "#7a8ca0", "Horde Leader": "#b0503a", Factor: "#9a8a6a",
  Apprentice: "#7a7a6a", Journeyman: "#a08a5a", Student: "#6a8fd8", Captain: "#5fa8e8", Soldier: "#b07a5a", Merchant: "#c8a060",
  Envoy: "#4ac0c0", Exile: "#7a6a8a", Priest: "#b8a0d0", Performer: "#d890b0",
};
export const roleColor = (r: string | undefined): string => (r && ROLE_COLOR[r]) || "#9fb4cc";

/** The compact career ladder: each rung a role, rising — the latest in full ink.
 *  `roles` is the sim's own role list, oldest first (years are not served yet). */
export function CareerLadder({ roles, max = 4 }: { roles: string[]; max?: number }) {
  const shown = roles.slice(-max);
  return (
    <div style={{ display: "flex", alignItems: "flex-end", gap: 3, minWidth: 0, flexWrap: "wrap" }}>
      {roles.length > shown.length && <span style={{ fontSize: 10, color: T_FAINT }}>+{roles.length - shown.length}</span>}
      {shown.map((r, i) => {
        const last = i === shown.length - 1, c = roleColor(r);
        return (
          <span key={i} style={{ display: "inline-flex", alignItems: "flex-end", gap: 3 }}>
            {i > 0 && <span style={{ fontSize: 10, color: T_FAINT, paddingBottom: 1 }}>›</span>}
            <span style={{ display: "inline-flex", alignItems: "flex-end", gap: 4, fontSize: 10, lineHeight: 1.5, whiteSpace: "nowrap", color: last ? "#cfe2f6" : "#9fb4cc", fontWeight: last ? 600 : 400 }}>
              <span style={{ width: 4, height: 5 + i * 3, borderRadius: 1, background: c, opacity: last ? 1 : 0.65, marginBottom: 2 }} />
              {r}
            </span>
          </span>
        );
      })}
    </div>
  );
}
const T_FAINT = "#46586e";
