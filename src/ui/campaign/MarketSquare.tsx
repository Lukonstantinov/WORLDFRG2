import { useEffect, useMemo, useRef } from "react";
import type { HubDetail } from "@types";
import { useGoodsStore } from "@state/goodsStore";
import { marketSquare, SQUARE_W, SQUARE_H, type Stall } from "@canvas/marketSquareArt";
import { kitForCulture } from "@ui/campaign/cultureDress";

/** Drawn at 2× and CSS-sized down, the same convention as the goods icons and
 *  dress plates — the pixel treatment needs the integer upscale. */
const SCALE = 2;
const STALLS = 4, PER_STALL = 3;

/** The city's MARKET SQUARE: its own skyline closing the square, four trestle
 *  stalls of the goods actually on sale here, and a crowd drawn from the
 *  peoples who actually live here. Purely a view of `detail` — no new query. */
export function MarketSquare({ detail }: { detail: HubDetail }) {
  const ref = useRef<HTMLCanvasElement | null>(null);
  const goodMeta = useGoodsStore((s) => s.meta);

  const stalls = useMemo<Stall[]>(() => {
    // Who keeps the stalls: the majority people, then each minority quarter.
    const peoples = [detail.culture, ...(detail.minorities ?? []).map(([n]) => n)]
      .filter((n): n is string => !!n);

    // What is on the boards: the goods this market actually holds, dearest first,
    // so a market with a real scarcity shows it on the front stall.
    const wares = detail.goods
      .filter((g) => g.stock > 0.01 || g.production > 0.01)
      .sort((a, b) => (b.price * b.stock) - (a.price * a.stock))
      .slice(0, STALLS * PER_STALL);
    if (wares.length === 0) return [];

    const out: Stall[] = [];
    for (let i = 0; i < STALLS; i++) {
      const mine = wares.slice(i * PER_STALL, i * PER_STALL + PER_STALL);
      if (mine.length === 0) break;
      const goods = mine.map((g) => [g.name, goodMeta(g.name).color] as [string, string]);
      // The chip names the stall's dearest ware against its own base value.
      const lead = mine.reduce((a, b) => (b.price / Math.max(1e-6, b.base_value) > a.price / Math.max(1e-6, a.base_value) ? b : a));
      out.push({
        kit: kitForCulture(peoples[i % Math.max(1, peoples.length)] ?? detail.name),
        goods,
        chip: [lead.name, goodMeta(lead.name).color, goodMeta(lead.name).name,
          lead.price / Math.max(1e-6, lead.base_value)],
      });
    }
    return out;
  }, [detail, goodMeta]);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.width = SQUARE_W * SCALE; el.height = SQUARE_H * SCALE;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(SCALE, 0, 0, SCALE, 0, 0);
    ctx.clearRect(0, 0, SQUARE_W, SQUARE_H);
    // A bigger town is a busier square.
    const pop = detail.population || 0;
    const crowd = Math.max(4, Math.min(22, Math.round(6 + Math.log10(Math.max(200, pop) / 200) * 7)));
    marketSquare(ctx, SQUARE_W, SQUARE_H, { stalls, crowd });
  }, [stalls, detail.population]);

  if (stalls.length === 0) return null;
  return (
    <canvas ref={ref} style={{
      display: "block", width: "100%", height: "auto", aspectRatio: `${SQUARE_W} / ${SQUARE_H}`,
      borderBottom: "1px solid #1e2e42",
    }} />
  );
}
