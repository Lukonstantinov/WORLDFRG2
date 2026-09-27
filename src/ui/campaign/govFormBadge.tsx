import type { CSSProperties } from "react";

/** Government-form badge — an icon + a colour so the three regime forms
 *  `govt_type_name()` ever returns (`"Merchant Council (Doge)"` /
 *  `"Principality (Prince)"` / `"Free Commune (Mayor)"`, `GovernmentBrief.
 *  form`) read apart at a glance, without new procedural art. ONE shared
 *  mapping — the Government tab (`HubPanel.tsx`), the floating Government
 *  panel (`GovernmentPanel.tsx`) and the Overview tab
 *  (`SettlementOverviewTab.tsx`) all render through `<GovFormBadge/>`, so
 *  the icon/colour can never drift between the three. */
export const GOV_FORM_META: Record<string, { icon: string; color: string; short: string }> = {
  "Merchant Council (Doge)": { icon: "🏛", color: "#7a94c0", short: "Council" },
  "Principality (Prince)": { icon: "👑", color: "#d8b24a", short: "Principality" },
  "Free Commune (Mayor)": { icon: "🗳", color: "#7fd0a0", short: "Commune" },
};
const FALLBACK = { icon: "⚖", color: "#9fb4cc", short: "" };
export const govFormMeta = (form: string) => GOV_FORM_META[form] ?? FALLBACK;

export function GovFormBadge({ form, style }: { form: string; style?: CSSProperties }) {
  const m = govFormMeta(form);
  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: 4, whiteSpace: "nowrap",
      color: m.color, border: `1px solid ${m.color}66`, background: `${m.color}22`,
      borderRadius: 10, padding: "1px 8px", fontSize: 11, fontWeight: 700,
      ...style,
    }}>
      <span aria-hidden>{m.icon}</span><span>{form}</span>
    </span>
  );
}
