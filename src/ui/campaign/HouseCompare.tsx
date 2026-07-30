/** House Comparison window — search two houses (or guilds), see their ruler
 *  figures, a full side-by-side stat comparison (each row highlighting whichever
 *  side leads), their trading strategy (specialties/top goods/monopolies/
 *  archetype), and a minimal operations map plotting where each actually holds
 *  seats, offices and controlled settlements — so a rivalry reads at a glance
 *  instead of needing two separate dossiers open side by side. */
import { useMemo, useState } from "react";
import { CoatOfArms } from "@ui/heraldry/CoatOfArms";
import { cultureFigureSVG, cultureSeed } from "@ui/campaign/cultureFigure";
import { useFloatingWindow, PANEL_TINTS } from "@ui/world/useFloatingWindow";
import type { HouseBrief } from "@types";

const fmt = (v: number | undefined) => {
  const n = v ?? 0;
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : n.toFixed(n >= 10 ? 0 : 2);
};

function HouseSearchField({ houses, value, onPick, placeholder }:
  { houses: HouseBrief[]; value: HouseBrief | null; onPick: (h: HouseBrief) => void; placeholder: string }) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const matches = useMemo(() => {
    if (!query.trim()) return houses.filter((h) => !h.defunct).slice(0, 8);
    const q = query.toLowerCase();
    return houses.filter((h) => h.name.toLowerCase().includes(q)).slice(0, 8);
  }, [houses, query]);
  return (
    <div style={{ position: "relative", flex: 1 }}>
      <input
        data-no-drag
        value={open ? query : (value?.name ?? "")}
        placeholder={placeholder}
        onFocus={() => { setOpen(true); setQuery(""); }}
        onChange={(e) => setQuery(e.target.value)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        style={{
          width: "100%", background: "#0a1119", border: "1px solid #24364e", borderRadius: 4,
          color: "#e8dcc0", fontSize: 12, padding: "6px 9px", fontFamily: "inherit",
        }}
      />
      {open && matches.length > 0 && (
        <div style={{
          position: "absolute", top: "100%", left: 0, right: 0, zIndex: 60, marginTop: 2,
          background: "#0f1822", border: "1px solid #24364e", borderRadius: 4,
          maxHeight: 220, overflowY: "auto", boxShadow: "0 8px 20px rgba(0,0,0,0.5)",
        }}>
          {matches.map((h) => (
            <div
              key={h.idx}
              onMouseDown={() => { onPick(h); setOpen(false); }}
              style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 9px", cursor: "pointer", fontSize: 11 }}
              onMouseEnter={(e) => (e.currentTarget.style.background = "#16202c")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
            >
              <span style={{ width: 8, height: 8, borderRadius: 2, background: h.color ?? "#888", flex: "0 0 auto" }} />
              <span style={{ color: h.defunct ? "#5a6f82" : "#cfe0f4" }}>{h.name}{h.defunct ? " (defunct)" : ""}</span>
              <span style={{ flex: 1 }} />
              <span style={{ color: "#6a86a6", fontSize: 9.5 }}>{fmt(h.wealth)}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** One comparison row: label + both values, the leading side highlighted. Pass
 *  `lowerIsBetter` for a metric where less is the stronger position (none currently
 *  used, kept for future rows like debt). */
function Row({ label, a, b, format, lowerIsBetter }:
  { label: string; a: number; b: number; format?: (v: number) => string; lowerIsBetter?: boolean }) {
  const f = format ?? ((v: number) => v.toFixed(2));
  const aWins = lowerIsBetter ? a < b : a > b;
  const bWins = lowerIsBetter ? b < a : b > a;
  const tie = a === b;
  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 110px 1fr", alignItems: "center", padding: "4px 0", borderBottom: "1px solid #16202c" }}>
      <span style={{ textAlign: "right", fontSize: 11, color: aWins && !tie ? "#7fd0a0" : "#cfe0f4", fontWeight: aWins && !tie ? 700 : 400, fontVariantNumeric: "tabular-nums" }}>
        {f(a)}
      </span>
      <span style={{ textAlign: "center", fontSize: 9.5, color: "#6a86a6", textTransform: "uppercase", letterSpacing: 0.3 }}>{label}</span>
      <span style={{ fontSize: 11, color: bWins && !tie ? "#7fd0a0" : "#cfe0f4", fontWeight: bWins && !tie ? 700 : 400, fontVariantNumeric: "tabular-nums" }}>
        {f(b)}
      </span>
    </div>
  );
}

/** Minimal operations map: every point either house's own brief already carries
 *  (seat, offices, controlled settlements), normalised to the bounding box of the
 *  TWO houses together (not the whole world) so their actual working region fills
 *  the frame instead of being a speck on a full atlas. */
function OperationsMap({ a, b }: { a: HouseBrief; b: HouseBrief }) {
  const points: { x: number; y: number; kind: string; color: string; label: string }[] = [];
  const add = (p: [number, number] | undefined, kind: string, color: string, label: string) => {
    if (p) points.push({ x: p[0], y: p[1], kind, color, label });
  };
  add(a.seat, "seat", a.color ?? "#b32d2d", a.name);
  add(b.seat, "seat", b.color ?? "#2a5fa0", b.name);
  (a.offices ?? []).forEach(([nm, p]) => add(p, "office", a.color ?? "#b32d2d", nm));
  (b.offices ?? []).forEach(([nm, p]) => add(p, "office", b.color ?? "#2a5fa0", nm));
  (a.controls ?? []).forEach((p) => add(p, "controls", a.color ?? "#b32d2d", ""));
  (b.controls ?? []).forEach((p) => add(p, "controls", b.color ?? "#2a5fa0", ""));

  if (points.length === 0) {
    return <div style={{ color: "#56708e", fontSize: 10, textAlign: "center", padding: 20 }}>No located operations to plot.</div>;
  }
  const xs = points.map((p) => p.x), ys = points.map((p) => p.y);
  const pad = Math.max(4, (Math.max(...xs) - Math.min(...xs)) * 0.15, (Math.max(...ys) - Math.min(...ys)) * 0.15);
  const minX = Math.min(...xs) - pad, maxX = Math.max(...xs) + pad;
  const minY = Math.min(...ys) - pad, maxY = Math.max(...ys) + pad;
  const w = Math.max(1, maxX - minX), h = Math.max(1, maxY - minY);
  const VB = 260;
  const px = (x: number) => ((x - minX) / w) * VB;
  const py = (y: number) => ((y - minY) / h) * VB;
  const rFor = (kind: string) => (kind === "seat" ? 6 : kind === "office" ? 4 : 3);

  return (
    <svg viewBox={`0 0 ${VB} ${VB}`} width="100%" height="220" style={{ background: "#0a1119", borderRadius: 6, border: "1px solid #1c2b3c" }}>
      <rect x="1" y="1" width={VB - 2} height={VB - 2} fill="none" stroke="#16202c" />
      {points.map((p, i) => (
        <g key={i}>
          <circle cx={px(p.x)} cy={py(p.y)} r={rFor(p.kind)} fill={p.color} opacity={p.kind === "controls" ? 0.55 : 0.9}
            stroke={p.kind === "seat" ? "#e8dcc0" : "none"} strokeWidth={p.kind === "seat" ? 1.4 : 0} />
          {p.kind === "seat" && (
            <text x={px(p.x) + 8} y={py(p.y) + 3} fontSize="9" fill="#cfe0f4">{p.label}</text>
          )}
        </g>
      ))}
    </svg>
  );
}

export function HouseCompareWindow({ houses, initialA, initialB, onClose }:
  { houses: HouseBrief[]; initialA?: HouseBrief | null; initialB?: HouseBrief | null; onClose: () => void }) {
  const [a, setA] = useState<HouseBrief | null>(initialA ?? houses.find((h) => !h.defunct) ?? null);
  const [b, setB] = useState<HouseBrief | null>(initialB ?? houses.filter((h) => !h.defunct)[1] ?? null);
  const { rootStyle, onPointerDown } = useFloatingWindow(PANEL_TINTS.houses);

  const Figure = ({ h }: { h: HouseBrief }) => {
    const svg = useMemo(() => cultureFigureSVG({
      kit: h.kit ?? -1, sex: h.head_female ? "f" : "m",
      seed: cultureSeed(h.head_name || h.name), occasion: "ceremonial",
    }), [h.kit, h.head_female, h.head_name, h.name]);
    return (
      <div style={{ position: "relative", width: 96, height: 218, margin: "0 auto" }}>
        <div style={{
          width: 96, height: 218, borderRadius: 6, overflow: "hidden", background: "#0a1119",
          border: `3px solid ${h.color ?? "#3a5570"}`,
        }} dangerouslySetInnerHTML={{ __html: svg }} />
        <div style={{ position: "absolute", top: -6, right: -6 }}><CoatOfArms name={h.name} size={24} guild={h.is_guild} /></div>
      </div>
    );
  };

  return (
    <div data-draggable style={{
      position: "absolute", top: 40, left: "50%", transform: "translateX(-50%)",
      width: 780, maxHeight: "88vh", overflowY: "auto",
      background: "#0c141e", border: "1px solid #24364e", borderRadius: 8,
      boxShadow: "0 8px 28px rgba(0,0,0,0.55)", zIndex: 50, padding: "12px 16px",
      ...rootStyle,
    }} onPointerDown={onPointerDown}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 10, cursor: "move" }}>
        <span style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13 }}>⚖ Compare Houses</span>
        <span style={{ flex: 1 }} />
        <span data-no-drag onClick={onClose} style={{ color: "#7090b0", cursor: "pointer", fontSize: 16, lineHeight: 1 }}>×</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 14 }}>
        <HouseSearchField houses={houses} value={a} onPick={setA} placeholder="Search a house or guild…" />
        <span style={{ color: "#c9a227", fontStyle: "italic", fontSize: 12 }}>vs.</span>
        <HouseSearchField houses={houses} value={b} onPick={setB} placeholder="Search a house or guild…" />
      </div>

      {!a || !b ? (
        <div style={{ color: "#56708e", fontSize: 11, textAlign: "center", padding: 30 }}>Pick two houses to compare.</div>
      ) : (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 14 }}>
            <div style={{ textAlign: "center" }}>
              <Figure h={a} />
              <div style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13, marginTop: 6 }}>{a.name}</div>
              <div style={{ color: "#9ab0c8", fontSize: 10 }}>{a.head_name} · {a.home_name} · gen {a.generation}</div>
            </div>
            <div style={{ textAlign: "center" }}>
              <Figure h={b} />
              <div style={{ color: "#e8dcc0", fontWeight: 700, fontSize: 13, marginTop: 6 }}>{b.name}</div>
              <div style={{ color: "#9ab0c8", fontSize: 10 }}>{b.head_name} · {b.home_name} · gen {b.generation}</div>
            </div>
          </div>

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, marginBottom: 4 }}>Standing</div>
          <Row label="Wealth" a={a.wealth} b={b.wealth} format={fmt} />
          <Row label="Prestige" a={a.prestige} b={b.prestige} format={(v) => v.toFixed(2)} />
          <Row label="Political power" a={a.political_power} b={b.political_power} format={(v) => v.toFixed(2)} />
          <Row label="Tier" a={5 - (a.tier || 4)} b={5 - (b.tier || 4)} format={() => (a.tier ? `${a.tier}` : "—")} />
          <Row label="Standing score" a={a.standing ?? 0} b={b.standing ?? 0} format={(v) => v.toFixed(2)} />
          <Row label="Founded" a={-(a.founded_year ?? 0)} b={-(b.founded_year ?? 0)} format={() => `${a.founded_year ?? "—"}`} />

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, margin: "10px 0 4px" }}>Trade &amp; transport</div>
          <Row label="Trade volume" a={a.volume ?? 0} b={b.volume ?? 0} format={fmt} />
          <Row label="Fleet (sea)" a={a.fleet_sea ?? 0} b={b.fleet_sea ?? 0} format={(v) => `${v}`} />
          <Row label="Fleet (river)" a={a.fleet_river ?? 0} b={b.fleet_river ?? 0} format={(v) => `${v}`} />
          <Row label="Fleet (caravan)" a={a.fleet_caravan ?? 0} b={b.fleet_caravan ?? 0} format={(v) => `${v}`} />
          <Row label="Offices" a={(a.offices ?? []).length} b={(b.offices ?? []).length} format={(v) => `${v}`} />
          <Row label="Estates" a={(a.estates ?? []).length} b={(b.estates ?? []).length} format={(v) => `${v}`} />
          <Row label="Monopolies" a={a.monopolies.length} b={b.monopolies.length} format={(v) => `${v}`} />

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, margin: "10px 0 4px" }}>Trading strategy</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, fontSize: 10.5 }}>
            {[a, b].map((h, i) => (
              <div key={i}>
                <div style={{ color: "#9fd0c0" }}>{h.archetype_label ?? "—"}{h.archetype_perk ? ` · ${h.archetype_perk}` : ""}</div>
                <div style={{ color: "#7a90a8", marginTop: 3 }}>Specialises in: {h.specialties.length ? h.specialties.join(", ") : "—"}</div>
                <div style={{ color: "#7a90a8", marginTop: 2 }}>Best known for: {h.top_goods.length ? h.top_goods.slice(0, 4).join(", ") : "—"}</div>
                {h.monopolies.length > 0 && (
                  <div style={{ color: "#c9a227", marginTop: 2 }}>
                    Monopolies: {h.monopolies.slice(0, 4).map(([g, s]) => `${g} (${Math.round(s * 100)}%)`).join(", ")}
                  </div>
                )}
              </div>
            ))}
          </div>

          <div style={{ color: "#6a86a6", fontSize: 9.5, textTransform: "uppercase", letterSpacing: 0.4, margin: "10px 0 4px" }}>
            Where they operate — {a.name} <span style={{ color: a.color }}>⬤</span> vs {b.name} <span style={{ color: b.color }}>⬤</span>
          </div>
          <OperationsMap a={a} b={b} />

          {a.rivals.includes(b.name) || b.rivals.includes(a.name) ? (
            <div style={{ color: "#e08a8a", fontSize: 10, marginTop: 8 }}>⚔ These houses are recorded rivals.</div>
          ) : null}
        </>
      )}
    </div>
  );
}
