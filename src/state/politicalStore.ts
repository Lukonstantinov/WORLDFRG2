import { create } from "zustand";
import { computeStates, campaignProvinceGoodShares, getStartRealms } from "@bridge";
import { useWorldStore } from "./worldStore";
import { useGoodsStore } from "./goodsStore";
import { GOOD_SHARE_MIN, type PoliticalFill } from "@canvas/RealmLayer";

type RGB = [number, number, number];
export interface Share { good: string; share: number }

/** Everything the political layer shows, plus the switches on its legend. */
interface PoliticalState {
  owner: Map<number, { realm: number; color: RGB }>;
  shares: Map<number, Share[]>;
  unrest: Map<number, number>;
  culture: Map<number, string>;
  version: number;
  fill: PoliticalFill;
  borders: boolean; goods: boolean; names: boolean;
  /** Show only provinces where this good is at least 30% of production. */
  goodFilter: string | null;
  patch: (p: Partial<PoliticalState>) => void;
}

export const usePoliticalStore = create<PoliticalState>((set) => ({
  owner: new Map(), shares: new Map(), unrest: new Map(), culture: new Map(), version: 0,
  fill: "realm", borders: true, goods: true, names: true, goodFilter: null,
  patch: (p) => set(p),
}));

/** Per good: how many provinces it is a main product (≥30%) of — the legend's list. */
export function goodCensus(shares: Map<number, Share[]>): { good: string; provinces: number }[] {
  const n = new Map<string, number>();
  for (const list of shares.values()) for (const s of list) if (s.share >= GOOD_SHARE_MIN) n.set(s.good, (n.get(s.good) ?? 0) + 1);
  return [...n].map(([good, provinces]) => ({ good, provinces })).sort((a, b) => b.provinces - a.provinces);
}

/** (Re)load realms, culture, order and goods shares. In a campaign they are live;
 *  before one, realms are the world's authored starting realms and the goods are the
 *  land's own quality ranking. */
export async function loadPolitical(campaignActive: boolean): Promise<void> {
  const owner = new Map<number, { realm: number; color: RGB }>();
  const shares = new Map<number, Share[]>();
  const unrest = new Map<number, number>();
  const culture = new Map<number, string>();
  const provinces = useWorldStore.getState().provinces;
  for (const p of provinces) culture.set(p.id, p.culture);

  if (campaignActive) {
    const [states, per] = await Promise.all([
      computeStates().catch(() => []), campaignProvinceGoodShares().catch(() => []),
    ]);
    states.forEach((st) => st.province_ids.forEach((p) => owner.set(p, { realm: st.id, color: st.color })));
    for (const row of per) {
      shares.set(row.province, row.goods.map((g) => ({ good: g.good, share: g.amount })));
      unrest.set(row.province, row.unrest);
      if (row.culture) culture.set(row.province, row.culture);
    }
  } else {
    const start = await getStartRealms().catch(() => []);
    start.forEach((st, k) => st.provinces.forEach((p) => owner.set(p, { realm: k, color: st.color })));
    const specs = useGoodsStore.getState().specs;
    for (const p of provinces) {
      const top = [...p.goods].sort((a, b) => b.quality - a.quality).slice(0, 6);
      const total = top.reduce((a, g) => a + g.quality, 0);
      if (total <= 0) continue;
      shares.set(p.id, top.map((g) => ({ good: specs[g.good]?.id ?? "", share: g.quality / total })).filter((s) => s.good));
    }
  }
  usePoliticalStore.setState((s) => ({ owner, shares, unrest, culture, version: s.version + 1 }));
}
