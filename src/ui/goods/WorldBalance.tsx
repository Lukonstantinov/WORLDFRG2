import { useEffect, useState } from "react";
import { useWorldStore } from "@state/worldStore";
import { getDepositSites } from "@bridge";
import type { GoodSpec, DepositSite } from "@types";
import { GoodIcon } from "./GoodIcon";

/** Goods Builder · World balance (plan §4.7 item 5): how much of each good this world
 *  makes at year 0, how many towns make it, and what share of all output VALUE it
 *  is — the readout that shows whether gold is rare or casual. Reads the frozen
 *  economy (Report stage) and the deposit list; nothing is recomputed here.
 *  shortcut: world-wide only; per-region needs and the regional gold:silver ratio
 *  need a region id on each economy hub — queued in the plan doc (G6). */
export function WorldBalance({ specs }: { specs: GoodSpec[] }) {
  const economy = useWorldStore((s) => s.economy);
  const [sites, setSites] = useState<DepositSite[]>([]);
  useEffect(() => { getDepositSites().then(setSites).catch(() => setSites([])); }, [economy]);

  if (!economy || economy.hubs.length === 0) {
    return <div style={{ color: "#9d917d", padding: 12 }}>Run the Report stage (Economy) to see this world's balance.</div>;
  }
  const rows = specs.filter((g) => g.enabled).map((g) => {
    const gi = economy.goods.indexOf(g.id);
    let output = 0, towns = 0;
    if (gi >= 0) for (const h of economy.hubs) {
      const p = h.produces.find((x) => x.good === gi);
      if (p) { output += p.amount; towns++; }
    }
    const mine = sites.filter((d) => d.good === g.id);
    return { g, output, towns, value: output * g.base_value, known: mine.filter((d) => d.known).length, workings: mine.length };
  });
  const totalValue = rows.reduce((a, r) => a + r.value, 0) || 1;
  const byId = (id: string) => rows.find((r) => r.g.id === id);
  const gold = byId("gold"), silver = byId("silver");
  const treasureShare = rows.filter((r) => r.g.dna?.role === "treasure").reduce((a, r) => a + r.value, 0) / totalValue;
  rows.sort((a, b) => b.value - a.value);
  const pct = (v: number) => `${(v * 100).toFixed(v < 0.01 ? 2 : 1)}%`;

  return (
    <div>
      <div style={{ display: "flex", gap: 18, flexWrap: "wrap", margin: "4px 0 10px", color: "#d8c49a" }}>
        <span title="Share of the value of everything this world makes at year 0">Treasure: <b>{pct(treasureShare)}</b> of output value</span>
        {gold && silver && gold.output > 0 && (
          <span title="World output of silver for each unit of gold (history: about 10-15)">Silver per gold: <b>{(silver.output / gold.output).toFixed(1)}</b> : 1</span>
        )}
        {gold && <span>Gold: <b>{gold.towns}</b> towns · {gold.known}/{gold.workings} workings known</span>}
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "22px 1.4fr 60px 70px 1.6fr 90px", gap: 6, fontWeight: 600, color: "#b5a68d", borderBottom: "1px solid #564c3b", padding: "3px 0" }}>
        <span></span><span>Good</span><span>Towns</span><span>Output</span><span>Share of value</span><span>Workings</span>
      </div>
      {rows.map((r) => (
        <div key={r.g.id} style={{ display: "grid", gridTemplateColumns: "22px 1.4fr 60px 70px 1.6fr 90px", gap: 6, alignItems: "center", padding: "2px 0", borderBottom: "1px solid #2c271f" }}>
          <GoodIcon name={r.g.id} color={r.g.color} size={18} />
          <span>{r.g.name}{r.towns === 0 && <span style={{ color: "#f0c070" }} title="No town makes it at year 0"> · absent</span>}</span>
          <span>{r.towns}</span>
          <span title="Normalised year-0 output (the campaign multiplies by 1,000 per day)">{r.output.toFixed(2)}</span>
          <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <span style={{ height: 8, width: `${Math.max(1, (r.value / totalValue) * 300)}px`, maxWidth: 160, background: r.g.color, borderRadius: 2 }} />
            {pct(r.value / totalValue)}
          </span>
          <span style={{ color: "#9db0c0" }}>{r.workings ? `${r.known}/${r.workings} known` : ""}</span>
        </div>
      ))}
    </div>
  );
}
