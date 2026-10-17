import { drawPixelIcon } from "./goodArt";
import { GOOD_DEFS } from "@goods";
import type { ProvinceRaster } from "@state/worldStore";

/** THE POLITICAL LAYER — the whole map drawn the way the realm window's minimap is:
 *  dark sea, every province opaque in its realm's colour (or culture / public order),
 *  thin province borders, heavy realm frontiers, the province's name and the goods it
 *  mostly lives on. A good's icon appears only where that good is at least 30% of the
 *  province's production, so a one-icon province is a specialist and a three-icon one
 *  is balanced. Picking a province (or a good) keeps that ground bright and dims the rest.
 *
 *  Self-contained: it owns its offscreen fill canvas and border paths and is drawn by
 *  `MapCanvas` once per visible world copy, under the ordinary overlays. */

type RGB = [number, number, number];
export type PoliticalFill = "realm" | "culture" | "order";
/** A good's icon shows only at/above this share of the province's production. */
export const GOOD_SHARE_MIN = 0.30;

export interface RealmLayerData {
  raster: ProvinceRaster | null;
  owner: Map<number, { realm: number; color: RGB }>;
  names: Map<number, string>;
  cultures: Map<number, string>;
  unrest: Map<number, number>;
  shares: Map<number, { good: string; share: number }[]>;
}
export interface RealmLayerView {
  fill: PoliticalFill; borders: boolean; goods: boolean; names: boolean;
  selected: number | null; goodFilter: string | null;
}

const NO = 0xffffffff;
const SEA: RGB = [30, 44, 52];
const FREE: RGB = [120, 112, 96];
const SERIF = "Georgia, 'Times New Roman', serif";
const PALETTE: RGB[] = [
  [176, 71, 58], [63, 111, 168], [201, 154, 58], [79, 138, 90], [138, 79, 160], [47, 143, 138],
  [168, 96, 58], [106, 125, 47], [176, 74, 122], [74, 90, 160], [154, 138, 58], [58, 122, 90],
  [192, 106, 80], [90, 74, 138], [122, 154, 74], [168, 58, 90],
];
const goodColor = new Map(GOOD_DEFS.map((g) => [g.name, g.color]));

function hashColor(s: string): RGB {
  let h = 2166136261;
  for (const c of s) { h ^= c.charCodeAt(0); h = Math.imul(h, 16777619); }
  return PALETTE[(h >>> 0) % PALETTE.length];
}
function orderColor(u: number): RGB {
  return u > 0.6 ? [163, 38, 42] : u > 0.4 ? [192, 106, 42] : u > 0.2 ? [201, 162, 58] : [79, 138, 90];
}

interface Stat { x: number; y: number; r: number }

export class RealmLayer {
  private data: RealmLayerData = { raster: null, owner: new Map(), names: new Map(), cultures: new Map(), unrest: new Map(), shares: new Map() };
  private view: RealmLayerView = { fill: "realm", borders: true, goods: true, names: true, selected: null, goodFilter: null };
  private fillCanvas: HTMLCanvasElement | null = null;
  private provPath: Path2D | null = null;
  private realmPath: Path2D | null = null;
  private coastPath: Path2D | null = null;
  private selPath: Path2D | null = null;
  private stats = new Map<number, Stat>();
  private dirtyFill = true;
  private dirtyGeo = true;
  private dirtySel = true;

  /** True once there are provinces to draw (otherwise the ordinary tiles show). */
  get ready(): boolean { return !!this.data.raster; }

  /** New data (raster / owners / shares): everything is rebuilt on the next draw. */
  setData(d: RealmLayerData) {
    this.data = d;
    this.dirtyFill = this.dirtyGeo = this.dirtySel = true;
  }

  setView(v: RealmLayerView) {
    const o = this.view;
    if (v.fill !== o.fill || v.selected !== o.selected || v.goodFilter !== o.goodFilter) this.dirtyFill = true;
    if (v.selected !== o.selected) this.dirtySel = true;
    this.view = v;
  }

  /** The ground a world cell sits in, or null at sea (a single array read). */
  provinceAt(wx: number, wy: number): number | null {
    const r = this.data.raster;
    if (!r) return null;
    const rx = Math.floor((wx / r.gridW) * r.w), ry = Math.floor((wy / r.gridH) * r.h);
    if (rx < 0 || ry < 0 || rx >= r.w || ry >= r.h) return null;
    const id = r.data[ry * r.w + rx];
    return id === NO ? null : id;
  }

  private realmOf(p: number): number { return this.data.owner.get(p)?.realm ?? -1; }

  /** Does this province hold the focus (selected / its realm / the filtered good)? */
  private focusOf(p: number): "hot" | "warm" | "lit" | "dim" {
    const { selected, goodFilter } = this.view;
    if (goodFilter) {
      const has = (this.data.shares.get(p) ?? []).some((s) => s.good === goodFilter && s.share >= GOOD_SHARE_MIN);
      if (!has) return "dim";
      return p === selected ? "hot" : "lit";
    }
    if (selected === null) return "lit";
    if (p === selected) return "hot";
    const sr = this.realmOf(selected);
    return sr >= 0 && this.realmOf(p) === sr ? "warm" : "dim";
  }

  private colourOf(p: number): RGB {
    const d = this.data;
    const own = d.owner.get(p);
    let c: RGB = own?.color ?? FREE;
    if (this.view.fill === "culture") c = hashColor(d.cultures.get(p) || String(own?.realm ?? p));
    else if (this.view.fill === "order") c = orderColor(d.unrest.get(p) ?? 0.2);
    const f = this.focusOf(p);
    const grey = (c[0] + c[1] + c[2]) / 3;
    const tone = 0.86 + ((p * 37) % 7) * 0.045; // each province a touch different from its neighbours
    const k = f === "warm" ? tone * 0.9 : tone;
    if (f === "dim") return c.map((v) => Math.round((v * 0.35 + grey * 0.65) * 0.55)) as RGB;
    const out = c.map((v) => v * k);
    if (f === "hot") return out.map((v) => Math.min(255, Math.round(v * 1.12 + 22))) as RGB;
    return out.map((v) => Math.round(v)) as RGB;
  }

  private rebuildFill() {
    this.dirtyFill = false;
    const r = this.data.raster;
    if (!r) { this.fillCanvas = null; return; }
    const { w, h, data } = r;
    const cv = this.fillCanvas ?? document.createElement("canvas");
    cv.width = w; cv.height = h;
    const ictx = cv.getContext("2d");
    if (!ictx) return;
    const img = ictx.createImageData(w, h);
    const px = img.data;
    const cache = new Map<number, RGB>();
    for (let i = 0; i < w * h; i++) {
      const id = data[i];
      let c: RGB = SEA;
      if (id !== NO) {
        let cc = cache.get(id);
        if (!cc) { cc = this.colourOf(id); cache.set(id, cc); }
        c = cc;
      }
      const o = i * 4;
      px[o] = c[0]; px[o + 1] = c[1]; px[o + 2] = c[2]; px[o + 3] = 255;
    }
    ictx.putImageData(img, 0, 0);
    this.fillCanvas = cv;
  }

  /** Border paths + per-province centroid/size, once per raster / ownership change. */
  private rebuildGeo() {
    this.dirtyGeo = false;
    this.provPath = this.realmPath = this.coastPath = null;
    this.stats.clear();
    const r = this.data.raster;
    if (!r) return;
    const { w, h, data, gridW, gridH } = r;
    const sx = gridW / w, sy = gridH / h;
    const prov = new Path2D(), realm = new Path2D(), coast = new Path2D();
    const acc = new Map<number, { n: number; s: number; c: number; y: number }>();
    const edge = (a: number, b: number, x: number, y0: number, y1: number, x0: number, x1: number, y: number, vertical: boolean) => {
      if (a === b) return;
      const path = a === NO || b === NO ? coast
        : this.realmOf(a) !== this.realmOf(b) ? realm : prov;
      if (vertical) { path.moveTo(x, y0); path.lineTo(x, y1); } else { path.moveTo(x0, y); path.lineTo(x1, y); }
      // A frontier between two realms is drawn heavy; give it the province line too so
      // the dark stroke has a base to sit on.
      if (path === realm) { if (vertical) { prov.moveTo(x, y0); prov.lineTo(x, y1); } else { prov.moveTo(x0, y); prov.lineTo(x1, y); } }
    };
    for (let ry = 0; ry < h; ry++) {
      for (let rx = 0; rx < w; rx++) {
        const i = ry * w + rx;
        const id = data[i];
        if (id !== NO) {
          const a = acc.get(id) ?? { n: 0, s: 0, c: 0, y: 0 };
          const ang = (2 * Math.PI * (rx + 0.5)) / w; // circular mean: a province may straddle the seam
          a.n++; a.s += Math.sin(ang); a.c += Math.cos(ang); a.y += ry + 0.5;
          acc.set(id, a);
        }
        const x1 = (rx + 1) * sx, y0 = ry * sy, y1 = (ry + 1) * sy;
        edge(id, data[ry * w + ((rx + 1) % w)], x1, y0, y1, 0, 0, 0, true);
        if (ry + 1 < h) edge(id, data[(ry + 1) * w + rx], 0, 0, 0, rx * sx, x1, y1, false);
      }
    }
    for (const [id, a] of acc) {
      let ang = Math.atan2(a.s, a.c); if (ang < 0) ang += 2 * Math.PI;
      this.stats.set(id, { x: (ang / (2 * Math.PI)) * gridW, y: (a.y / a.n) * sy, r: Math.sqrt((a.n * sx * sy) / Math.PI) });
    }
    this.provPath = prov; this.realmPath = realm; this.coastPath = coast;
  }

  private rebuildSel() {
    this.dirtySel = false;
    this.selPath = null;
    const r = this.data.raster, id = this.view.selected;
    if (!r || id === null) return;
    const { w, h, data, gridW, gridH } = r;
    const sx = gridW / w, sy = gridH / h;
    const path = new Path2D();
    for (let ry = 0; ry < h; ry++) {
      for (let rx = 0; rx < w; rx++) {
        if (data[ry * w + rx] !== id) continue;
        const x0 = rx * sx, x1 = (rx + 1) * sx, y0 = ry * sy, y1 = (ry + 1) * sy;
        if (rx === 0 || data[ry * w + rx - 1] !== id) { path.moveTo(x0, y0); path.lineTo(x0, y1); }
        if (rx + 1 >= w || data[ry * w + rx + 1] !== id) { path.moveTo(x1, y0); path.lineTo(x1, y1); }
        if (ry === 0 || data[(ry - 1) * w + rx] !== id) { path.moveTo(x0, y0); path.lineTo(x1, y0); }
        if (ry + 1 >= h || data[(ry + 1) * w + rx] !== id) { path.moveTo(x0, y1); path.lineTo(x1, y1); }
      }
    }
    this.selPath = path;
  }

  /** Draw one world copy. `vis` is the visible world rectangle in this copy's own
   *  coordinates (used only to skip names/icons that are off screen). */
  render(ctx: CanvasRenderingContext2D, scale: number, vis: { x0: number; x1: number; y0: number; y1: number }) {
    const r = this.data.raster;
    if (!r) return;
    if (this.dirtyGeo) this.rebuildGeo();
    if (this.dirtyFill) this.rebuildFill();
    if (this.dirtySel) this.rebuildSel();
    if (!this.fillCanvas) return;
    const inv = 1 / scale;
    ctx.save();
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(this.fillCanvas, 0, 0, r.w, r.h, 0, 0, r.gridW, r.gridH);
    ctx.lineJoin = "round";
    if (this.view.borders) {
      if (this.provPath) { ctx.strokeStyle = "rgba(0,0,0,0.38)"; ctx.lineWidth = 1 * inv; ctx.stroke(this.provPath); }
      if (this.realmPath) { ctx.strokeStyle = "rgba(14,10,6,0.95)"; ctx.lineWidth = 2.2 * inv; ctx.stroke(this.realmPath); }
      if (this.coastPath) { ctx.strokeStyle = "rgba(8,12,14,0.7)"; ctx.lineWidth = 1.3 * inv; ctx.stroke(this.coastPath); }
    }
    if (this.selPath) {
      ctx.strokeStyle = "rgba(6,12,20,0.9)"; ctx.lineWidth = 4.5 * inv; ctx.stroke(this.selPath);
      ctx.strokeStyle = "rgba(255,226,138,0.98)"; ctx.lineWidth = 2 * inv; ctx.stroke(this.selPath);
    }
    if (this.view.names || this.view.goods) this.renderLabels(ctx, scale, vis);
    ctx.restore();
  }

  private renderLabels(ctx: CanvasRenderingContext2D, scale: number, vis: { x0: number; x1: number; y0: number; y1: number }) {
    const inv = 1 / scale;
    ctx.textAlign = "center"; ctx.textBaseline = "middle";
    for (const [id, s] of this.stats) {
      if (s.x + s.r < vis.x0 || s.x - s.r > vis.x1 || s.y + s.r < vis.y0 || s.y - s.r > vis.y1) continue;
      const diam = 2 * s.r * scale; // on-screen width of the province, px
      if (diam < 46) continue;
      if (this.focusOf(id) === "dim" && this.view.goodFilter) continue;
      const icons = this.view.goods && diam >= 84
        ? (this.data.shares.get(id) ?? []).filter((g) => g.share >= GOOD_SHARE_MIN).slice(0, 3) : [];
      const fpx = Math.max(9, Math.min(15, diam * 0.11));
      const isz = Math.max(16, Math.min(26, diam * 0.13));
      if (this.view.names) {
        const name = (this.data.names.get(id) ?? "").toUpperCase();
        if (name) {
          const ny = s.y - (icons.length ? (isz * 0.45) * inv : 0);
          ctx.font = `600 ${fpx * inv}px ${SERIF}`;
          ctx.lineWidth = 3 * inv; ctx.strokeStyle = "rgba(10,8,5,0.8)"; ctx.fillStyle = "#f5efe2";
          ctx.strokeText(name, s.x, ny); ctx.fillText(name, s.x, ny);
        }
      }
      icons.forEach((g, k) => {
        const ix = s.x + (k - (icons.length - 1) / 2) * (isz + 5) * inv;
        const iy = s.y + ((this.view.names ? fpx * 0.5 : 0) + isz * 0.65) * inv;
        const sel = this.view.goodFilter === g.good;
        ctx.fillStyle = sel ? "rgba(255,226,138,0.97)" : "rgba(245,239,226,0.88)";
        ctx.beginPath(); ctx.arc(ix, iy, isz * 0.62 * inv, 0, Math.PI * 2); ctx.fill();
        drawPixelIcon(ctx, ix, iy, isz * 0.9 * inv, goodColor.get(g.good) ?? "#9a8a70", g.good);
      });
    }
    ctx.textAlign = "start"; ctx.textBaseline = "alphabetic";
  }
}
