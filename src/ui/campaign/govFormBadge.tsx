import type { CSSProperties } from "react";

/** Government-form badge — an icon + a colour so the three regime forms
 *  `govt_type_name()` ever returns (`"Merchant Council (Doge)"` /
 *  `"Principality (Prince)"` / `"Free Commune (Mayor)"`, `GovernmentBrief.
 *  form`) read apart at a glance, without new procedural art. ONE shared
 *  mapping — the Government tab (`HubPanel.tsx`), the floating Government
 *  panel (`GovernmentPanel.tsx`) and the Overview tab
 *  (`SettlementOverviewTab.tsx`) all render through `<GovFormBadge/>`, so
 *  the icon/colour can never drift between the three. */
const KIND_META: Record<string, { icon: string; color: string; short: string }> = {
  council: { icon: "🏛", color: "#7a94c0", short: "Council" },
  ruler: { icon: "👑", color: "#d8b24a", short: "One ruler" },
  assembly: { icon: "🗳", color: "#7fd0a0", short: "Assembly" },
};
const FALLBACK = { icon: "⚖", color: "#9fb4cc", short: "" };

/** Which of the three kinds a form NAME is, for callers that only have the
 *  name (a culture's own word — "Althing", "Signoria" — or the old generic). */
function kindOfName(form: string): string {
  const f = form.toLowerCase();
  if (/(principal|lordship|kingship|kingdom|tyrann|satrap|jarldom|chiefdom|emirate|rajya|marquis|knyaz|altepetl|khanate|domain|curacazgo|principate)/.test(f)) return "ruler";
  if (/(commune|assembly|ekklesia|comitia|althing|veche|jamaa|gana|compact|calpolli|kurultai|tajmaat|ayllu|gbara|anjoman|óenach|oenach|age-set|village council)/.test(f)) return "assembly";
  if (f.length > 0) return "council";
  return "";
}
export const govFormMeta = (form: string, kind?: string) => KIND_META[kind || kindOfName(form)] ?? FALLBACK;

export function GovFormBadge({ form, kind, style }: { form: string; kind?: string; style?: CSSProperties }) {
  const m = govFormMeta(form, kind);
  return (
    <span title={m.short} style={{
      display: "inline-flex", alignItems: "center", gap: 4, whiteSpace: "nowrap",
      color: m.color, border: `1px solid ${m.color}66`, background: `${m.color}22`,
      borderRadius: 10, padding: "1px 8px", fontSize: 11, fontWeight: 700,
      ...style,
    }}>
      <span aria-hidden>{m.icon}</span><span>{form}</span>
    </span>
  );
}
