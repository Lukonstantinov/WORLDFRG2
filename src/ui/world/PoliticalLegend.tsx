import { useMemo } from "react";
import { useUIStore } from "@state/uiStore";
import { usePoliticalStore, goodCensus } from "@state/politicalStore";
import { GOOD_DEFS } from "@goods";
import { GoodIcon } from "@ui/goods/GoodIcon";
import { Chip } from "@ui/kit";
import { T, FZ } from "@ui/campaign/chronicleTheme";

const label = (g: string) => GOOD_DEFS.find((d) => d.name === g)?.label ?? g.replace(/_/g, " ");

/** The political map's key and switches: what colours the land, what is drawn, and the
 *  goods the provinces mostly live on. Press a good to light every province where it is
 *  at least 30% of production (press it again to clear). */
export function PoliticalLegend() {
  const on = useUIStore((s) => s.activeLayer === "political");
  const p = usePoliticalStore();
  const census = useMemo(() => goodCensus(p.shares), [p.shares]);
  if (!on) return null;
  const lit = p.goodFilter ? census.find((c) => c.good === p.goodFilter)?.provinces ?? 0 : 0;
  return (
    <div data-no-drag style={{
      position: "absolute", left: 8, bottom: 8, zIndex: 12, width: 292, maxHeight: "62%", overflowY: "auto",
      background: "rgba(19,16,10,0.93)", border: `1px solid ${T.line}`, borderRadius: 8, padding: "8px 10px",
      color: T.ink, fontSize: FZ.small, boxShadow: "0 6px 18px rgba(0,0,0,0.45)",
    }}>
      <div style={{ color: T.inkDim, fontSize: FZ.micro, letterSpacing: 1, textTransform: "uppercase", marginBottom: 4 }}>Colour the land by</div>
      <div style={{ display: "flex", gap: 5, flexWrap: "wrap", marginBottom: 7 }}>
        {(["realm", "culture", "order"] as const).map((f) => (
          <Chip key={f} on={p.fill === f} onClick={() => p.patch({ fill: f })}>{f === "realm" ? "Realms" : f === "culture" ? "Cultures" : "Public order"}</Chip>
        ))}
      </div>
      <div style={{ display: "flex", gap: 5, flexWrap: "wrap", marginBottom: 7 }}>
        <Chip on={p.borders} onClick={() => p.patch({ borders: !p.borders })}>Borders</Chip>
        <Chip on={p.names} onClick={() => p.patch({ names: !p.names })}>Names</Chip>
        <Chip on={p.goods} onClick={() => p.patch({ goods: !p.goods })}>Goods</Chip>
      </div>
      <div style={{ color: T.inkDim, fontSize: FZ.micro, letterSpacing: 1, textTransform: "uppercase", marginBottom: 3 }}>People on each province</div>
      <div style={{ display: "flex", gap: 5, flexWrap: "wrap", marginBottom: 7 }}>
        {([["off", "Off"], ["all", "Everyone"], ["rural", "Countryside"], ["urban", "Towns"]] as const).map(([m, l]) => (
          <Chip key={m} on={p.popMode === m} onClick={() => p.patch({ popMode: m })}>{l}</Chip>
        ))}
      </div>
      <div style={{ color: T.inkDim, fontSize: FZ.micro, letterSpacing: 1, textTransform: "uppercase", marginBottom: 3 }}>
        Main goods — an icon shows at 30% of a province's production
      </div>
      {census.length === 0 && <div style={{ color: T.inkDim }}>No production recorded yet.</div>}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
        {census.map((c) => {
          const sel = p.goodFilter === c.good;
          return (
            <span key={c.good} onClick={() => p.patch({ goodFilter: sel ? null : c.good })}
              title={`${label(c.good)} — ${c.provinces} province${c.provinces === 1 ? "" : "s"} live mainly on it. Click to show them.`}
              style={{
                display: "inline-flex", alignItems: "center", gap: 3, padding: "1px 6px 1px 3px", borderRadius: 10, cursor: "pointer",
                border: `1px solid ${sel ? T.gold : T.lineSoft}`, background: sel ? "#3a2f1a" : "rgba(255,255,255,0.03)",
                color: sel ? T.gold : T.inkMid,
              }}>
              <GoodIcon name={c.good} size={16} />{c.provinces}
            </span>
          );
        })}
      </div>
      {p.goodFilter && (
        <div style={{ marginTop: 6, color: T.gold }}>
          {label(p.goodFilter)}: lit in {lit} province{lit === 1 ? "" : "s"} where it is at least 30% of production.
        </div>
      )}
    </div>
  );
}
