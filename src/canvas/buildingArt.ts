// Isometric BUILDING SPRITES for the city view — the fifteen types in
// CityView's `SPRITE_MAP`, differentiated by ARCHITECTURAL FORM rather than by
// palette: a stone plinth, shaded walls with courses, lit windows, a door, a
// roof whose shape reads the type (pitch / flat / crenel / dome / spire), and a
// per-type detail (tower, chimney, crane, arch, dock) or a whole alternate form
// (bank portico, palace wings, granary barn on staddles).
//
// Ownership is NOT carried in the walls — it reads from the ground wash and the
// heraldic flag the caller draws on top, which is what lets each building keep
// its own real material.

export interface BuildingSpec {
  w: number; h: number;
  roof: "pitch" | "flat" | "crenel" | "dome" | "spire";
  pal: "stone" | "timber" | "marble" | "brick";
  extra?: "tower" | "chimney" | "crane" | "arch" | "dock";
  form?: "portico" | "wings" | "barn";
  dormer?: number;
}

/** Keyed by the sprite stem CityView's `SPRITE_MAP` already uses. */
export const BUILDING_SPECS: Record<string, BuildingSpec> = {
  house: { w: 16, h: 9, roof: "pitch", pal: "brick" },
  guildhall: { w: 19, h: 15, roof: "pitch", pal: "brick", extra: "tower" },
  workshop: { w: 19, h: 11, roof: "pitch", pal: "timber", extra: "chimney", dormer: 1 },
  granary: { w: 19, h: 14, roof: "pitch", pal: "timber", form: "barn" },
  warehouse: { w: 20, h: 11, roof: "flat", pal: "stone", extra: "crane" },
  shipyard: { w: 20, h: 9, roof: "flat", pal: "timber", extra: "crane" },
  fondaco: { w: 19, h: 14, roof: "flat", pal: "brick", extra: "arch", dormer: 1 },
  cathedral: { w: 18, h: 17, roof: "spire", pal: "marble" },
  temple: { w: 19, h: 13, roof: "dome", pal: "marble", extra: "arch" },
  citadel: { w: 20, h: 15, roof: "crenel", pal: "stone", extra: "tower" },
  palace: { w: 22, h: 17, roof: "crenel", pal: "marble", form: "wings" },
  council_hall: { w: 19, h: 12, roof: "crenel", pal: "marble", extra: "arch" },
  mint: { w: 17, h: 12, roof: "dome", pal: "stone", extra: "chimney" },
  bank: { w: 19, h: 13, roof: "flat", pal: "marble", form: "portico" },
  harbor: { w: 20, h: 6, roof: "flat", pal: "timber", extra: "dock" },
};

export const PAL: Record<string, { a: string; b: string; c: string; roof: string; roof2: string }> = {
  stone: { a: "#94968f", b: "#70747a", c: "#4e5257", roof: "#5c6670", roof2: "#414c56" },
  timber: { a: "#af8759", b: "#84603d", c: "#5e442b", roof: "#9c5b45", roof2: "#6f3c2e" },
  marble: { a: "#e2dbc8", b: "#b7b09d", c: "#8b8677", roof: "#c9a24a", roof2: "#8f7130" },
  brick: { a: "#ad6742", b: "#84462c", c: "#5f3220", roof: "#9c5b45", roof2: "#6f3c2e" },
};

type Ctx = CanvasRenderingContext2D;
type RGB = [number, number, number];

export function hx(h: string): RGB {
  let s = (h || "#888888").replace("#", "");
  if (s.length === 3) s = s.split("").map((c) => c + c).join("");
  return [parseInt(s.slice(0, 2), 16), parseInt(s.slice(2, 4), 16), parseInt(s.slice(4, 6), 16)];
}

function shade(c: RGB, f: number): string {
  return f >= 1
    ? `rgb(${Math.round(c[0] + (255 - c[0]) * (f - 1))},${Math.round(c[1] + (255 - c[1]) * (f - 1))},${Math.round(c[2] + (255 - c[2]) * (f - 1))})`
    : `rgb(${Math.round(c[0] * f)},${Math.round(c[1] * f)},${Math.round(c[2] * f)})`;
}

function lgrad(ctx: Ctx, x0: number, y0: number, x1: number, y1: number, base: string) {
  const g = ctx.createLinearGradient(x0, y0, x1, y1);
  g.addColorStop(0, shade(hx(base), 1.25)); g.addColorStop(0.55, base); g.addColorStop(1, shade(hx(base), 0.72));
  return g;
}

/** The tier scale `building` applies to `spec.h` (0 = cottage, 2 = grand). */
export function tierScale(tier: number): number {
  return tier === 2 ? 1.42 : tier === 0 ? 0.7 : 1;
}

/** Where a flag pole roots on top of a building drawn by `building(...)` —
 *  the same anchor the design's own city panorama uses. */
export function buildingPeakY(by: number, s: number, spec: BuildingSpec, tier: number): number {
  const sc = s / 20;
  return by - (s / 2) / 2 - spec.h * tierScale(tier) * sc - s * 0.34;
}

/** One isometric building, base-anchored at (cx, by).
 *  Built from a stone plinth, shaded walls with courses, lit windows, a door,
 *  a roof whose SHAPE and material read the type, and per-type detail. */
export function building(ctx: Ctx, cx: number, by: number, s: number, spec: BuildingSpec, tier: number) {
  const p = PAL[spec.pal] || PAL.stone;
  const sc = s / 20, w = spec.w * sc, h = spec.h * tierScale(tier) * sc;
  const hw = w / 2, hh = w / 4;
  const quad = (pts: number[][], f: string | CanvasGradient) => {
    ctx.beginPath(); ctx.moveTo(pts[0][0], pts[0][1]);
    for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], pts[i][1]);
    ctx.closePath(); ctx.fillStyle = f; ctx.fill();
  };
  const line = (x1: number, y1: number, x2: number, y2: number, c: string, lw: number) => {
    ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.strokeStyle = c; ctx.lineWidth = lw; ctx.stroke();
  };

  // ground shadow
  ctx.save(); ctx.globalAlpha = 0.35;
  quad([[cx + w * 0.1, by - hh * 0.9], [cx + hw * 1.15, by + hh * 0.15], [cx + w * 0.1, by + hh * 1.05], [cx - hw * 0.85, by + hh * 0.15]], "#05080c");
  ctx.restore();

  // plinth
  const pl = Math.max(1.5, 2.2 * sc);
  quad([[cx - hw, by - pl], [cx, by - hh - pl], [cx + hw, by - pl], [cx + hw, by], [cx, by + hh], [cx - hw, by]], shade(hx(p.c), 0.85));
  const B = by - pl;
  const T = B - hh - h, tp = [[cx, T], [cx + hw, T + hh], [cx, T + hh * 2], [cx - hw, T + hh]];

  // walls
  quad([[cx - hw, B - h], [cx, B - h + hh], [cx, B + hh], [cx - hw, B]], p.c);
  quad([[cx, B - h + hh], [cx + hw, B - h], [cx + hw, B], [cx, B + hh]], p.b);
  quad(tp, p.a);
  // stone courses
  const courses = Math.max(2, Math.round(h / (3.4 * sc)));
  ctx.save(); ctx.globalAlpha = 0.18;
  for (let i = 1; i < courses; i++) {
    const yy = i * (h / courses);
    line(cx - hw, B - h + yy, cx, B - h + hh + yy, "#05080c", Math.max(0.6, 0.5 * sc));
    line(cx, B - h + hh + yy, cx + hw, B - h + yy, "#05080c", Math.max(0.6, 0.5 * sc));
  }
  ctx.restore();
  // windows, lit
  const rows = Math.max(1, Math.floor(h / (5.5 * sc))), ww = Math.max(1, 1.7 * sc), wh = Math.max(1.4, 2.4 * sc);
  for (let r = 0; r < rows; r++) {
    const t = (r + 0.62) / (rows + 0.35), yb = B - h + t * h;
    for (let k = 0; k < 2; k++) {
      const fx = cx - hw * 0.62 + k * hw * 0.42, fy = yb + (fx - (cx - hw)) / hw * hh * 0.5;
      ctx.fillStyle = "rgba(12,18,26,0.8)"; ctx.fillRect(fx, fy, ww, wh);
      ctx.fillStyle = "rgba(246,214,140,0.55)"; ctx.fillRect(fx, fy, ww, wh * 0.45);
      const gx = cx + hw * 0.22 + k * hw * 0.42, gy = yb + ((cx + hw) - gx) / hw * hh * 0.5;
      ctx.fillStyle = "rgba(12,18,26,0.8)"; ctx.fillRect(gx, gy, ww, wh);
      ctx.fillStyle = "rgba(246,214,140,0.35)"; ctx.fillRect(gx, gy, ww, wh * 0.45);
    }
  }
  // door on the front-left face
  const dw = Math.max(2, w * 0.13), dh = Math.max(3, h * 0.34);
  quad([[cx - dw * 1.2, B - dh + hh * 0.55], [cx - dw * 0.2, B - dh + hh * 0.05], [cx - dw * 0.2, B + hh * 0.05], [cx - dw * 1.2, B + hh * 0.55]], "rgba(30,20,12,0.85)");

  // ── per-type forms ──
  if (spec.form === "portico") {                       // Bank: colonnade + strongroom
    const cn = 5, cw = Math.max(1.2, w * 0.055), ch = h * 0.72;
    for (let i = 0; i < cn; i++) {
      const t = i / (cn - 1), x = cx - hw * 0.78 + t * hw * 1.05, y = B + hh * 0.5 - t * hh * 0.95;
      quad([[x, y - ch], [x + cw, y - ch + cw * 0.5], [x + cw, y], [x, y - cw * 0.5]], shade(hx(p.a), 1.08));
      quad([[x, y - ch], [x - cw * 0.5, y - ch - cw * 0.3], [x - cw * 0.5, y - cw * 0.8], [x, y - cw * 0.5]], p.b);
    }
    quad([[cx - hw * 0.9, B - h * 0.78], [cx + hw * 0.35, B - h * 0.78 - hh * 0.9], [cx + hw * 0.5, B - h * 0.68 - hh * 0.9], [cx - hw * 0.78, B - h * 0.68]], p.a);
    quad([[cx - hw * 0.9, B - h * 0.78], [cx - hw * 0.16, B - h * 1.02 - hh * 0.5], [cx + hw * 0.5, B - h * 0.78 - hh * 0.9]], shade(hx(p.roof), 0.95));
    // strongroom annexe
    quad([[cx + hw * 0.35, B - h * 0.5], [cx + hw * 0.95, B - h * 0.5 + hh * 0.5], [cx + hw * 0.95, B + hh * 0.5], [cx + hw * 0.35, B]], p.c);
    quad([[cx + hw * 0.35, B - h * 0.5], [cx + hw * 0.65, B - h * 0.62], [cx + hw * 0.95, B - h * 0.5 + hh * 0.5], [cx + hw * 0.65, B - h * 0.38 + hh * 0.5]], p.a);
    ctx.fillStyle = "#c9a24a"; ctx.beginPath(); ctx.arc(cx + hw * 0.66, B - h * 0.16, Math.max(1.2, w * 0.05), 0, 7); ctx.fill();
    return;
  }
  if (spec.form === "wings") {                         // Palace: central block + wings
    const wingH = h * 0.46;
    for (const d of [-1, 1]) {
      const bx = cx + d * hw * 0.72;
      quad([[bx - hw * 0.34, B - wingH - hh * 0.2], [bx + hw * 0.34, B - wingH + hh * 0.2], [bx + hw * 0.34, B + hh * 0.2], [bx - hw * 0.34, B - hh * 0.2]], d < 0 ? p.c : p.b);
      quad([[bx - hw * 0.34, B - wingH - hh * 0.2], [bx, B - wingH - hh * 0.5], [bx + hw * 0.34, B - wingH + hh * 0.2], [bx, B - wingH + hh * 0.5]], p.a);
      for (let i = 0; i < 3; i++) {
        const t = (i + 0.5) / 3;
        ctx.fillStyle = "rgba(246,214,140,0.5)";
        ctx.fillRect(bx - hw * 0.28 + t * hw * 0.5, B - wingH * 0.62 + (d < 0 ? -hh * 0.1 + t * hh * 0.3 : hh * 0.1 + t * hh * 0.3), Math.max(1, 1.6 * sc), Math.max(1.4, 2.4 * sc));
      }
    }
    quad([[cx - hw * 0.42, B - h - hh * 0.2], [cx + hw * 0.42, B - h + hh * 0.2], [cx + hw * 0.42, B + hh * 0.3], [cx - hw * 0.42, B - hh * 0.1]], p.b);
    quad([[cx - hw * 0.42, B - h - hh * 0.2], [cx, B - h - hh * 0.5], [cx + hw * 0.42, B - h + hh * 0.2], [cx, B - h + hh * 0.5]], p.a);
    const m = Math.max(1.4, w * 0.06);
    for (let i = 0; i < 4; i++) {
      const t = i / 3, x = cx - hw * 0.42 + t * hw * 0.84, y = B - h - hh * 0.2 + t * hh * 0.4;
      quad([[x - m, y - m * 1.6], [x + m, y - m * 1.6 + m * 0.4], [x + m, y + m * 0.4], [x - m, y]], shade(hx(p.a), 1.15));
    }
    quad([[cx - w * 0.09, B - h * 0.36], [cx + w * 0.09, B - h * 0.36 + hh * 0.18], [cx + w * 0.09, B + hh * 0.2], [cx - w * 0.09, B + hh * 0.02]], "rgba(24,16,10,0.85)");
    ctx.fillStyle = "#c9a24a"; ctx.fillRect(cx - Math.max(0.5, 0.4 * sc), B - h - hh * 0.5 - Math.max(3, h * 0.14), Math.max(1, 0.9 * sc), Math.max(3, h * 0.14));
    return;
  }
  if (spec.form === "barn") {                          // Granary: raised barn on staddles
    for (const [sx, sy] of [[cx - hw * 0.6, B + hh * 0.25], [cx + hw * 0.6, B + hh * 0.25], [cx, B + hh * 0.62], [cx, B - hh * 0.1]]) {
      quad([[sx - w * 0.05, sy - h * 0.28], [sx + w * 0.05, sy - h * 0.28], [sx + w * 0.05, sy], [sx - w * 0.05, sy]], shade(hx(p.c), 0.9));
      quad([[sx - w * 0.09, sy - h * 0.28], [sx + w * 0.09, sy - h * 0.28], [sx + w * 0.09, sy - h * 0.32], [sx - w * 0.09, sy - h * 0.32]], shade(hx(p.a), 0.95));
    }
    const L2 = B - h * 0.32;
    quad([[cx - hw, L2 - hh], [cx, L2 - hh * 2], [cx + hw, L2 - hh], [cx + hw, L2 - hh + h * 0.62], [cx, L2 + h * 0.62], [cx - hw, L2 - hh + h * 0.62]], p.c);
    quad([[cx, L2 - hh * 2], [cx + hw, L2 - hh], [cx + hw, L2 - hh + h * 0.62], [cx, L2 + h * 0.62]], p.b);
    for (let i = 1; i < 5; i++) {
      const t = i / 5;
      line(cx - hw + hw * t, L2 - hh - hh * t + h * 0.12, cx - hw + hw * t, L2 - hh - hh * t + h * 0.5, "rgba(0,0,0,0.28)", Math.max(0.7, 0.6 * sc));
    }
    const RT = L2 - hh * 2, rh2 = Math.max(3, w * 0.42);
    quad([[cx, RT - rh2], [cx + hw, RT + hh], [cx, RT + hh * 2]], p.roof);
    quad([[cx, RT - rh2], [cx - hw, RT + hh], [cx, RT + hh * 2]], p.roof2);
    quad([[cx - w * 0.16, L2 + h * 0.06], [cx + w * 0.16, L2 + h * 0.06], [cx + w * 0.16, L2 + h * 0.5], [cx - w * 0.16, L2 + h * 0.5]], "rgba(28,18,10,0.85)");
    line(cx, RT - rh2 * 0.4, cx, RT - rh2 * 1.5, shade(hx(p.c), 0.9), Math.max(1, 1.1 * sc));
    line(cx, RT - rh2 * 1.5, cx + w * 0.3, RT - rh2 * 1.5, shade(hx(p.c), 0.9), Math.max(1, 1.1 * sc));
    return;
  }

  const rh = Math.max(3, w * 0.4);
  if (spec.roof === "pitch") {
    const A = [cx, T - rh];
    quad([A, tp[0], tp[3]], p.roof2); quad([A, tp[0], tp[1]], p.roof);
    quad([A, tp[3], tp[2]], p.roof2); quad([A, tp[1], tp[2]], shade(hx(p.roof), 1.12));
    // tile courses + ridge
    ctx.save(); ctx.globalAlpha = 0.22;
    for (let i = 1; i < 4; i++) {
      const t = i / 4;
      line(cx - hw * t, (T - rh) + (tp[3][1] - (T - rh)) * t, cx, (T - rh) + (tp[2][1] - (T - rh)) * t, "#150c08", Math.max(0.6, 0.5 * sc));
      line(cx, (T - rh) + (tp[2][1] - (T - rh)) * t, cx + hw * t, (T - rh) + (tp[1][1] - (T - rh)) * t, "#150c08", Math.max(0.6, 0.5 * sc));
    }
    ctx.restore();
    line(A[0], A[1], tp[2][0], tp[2][1], shade(hx(p.roof), 1.35), Math.max(1, 0.8 * sc));
    if (spec.dormer) quad([[cx - hw * 0.3, T + hh * 0.5], [cx - hw * 0.05, T + hh * 0.35], [cx - hw * 0.05, T + hh * 0.9], [cx - hw * 0.3, T + hh * 1.05]], p.roof2);
  } else if (spec.roof === "flat") {
    quad(tp, p.roof2);
    quad([tp[3], tp[2], [tp[2][0], tp[2][1] - Math.max(1, 1.4 * sc)], [tp[3][0], tp[3][1] - Math.max(1, 1.4 * sc)]], p.a);
    quad([tp[1], tp[2], [tp[2][0], tp[2][1] - Math.max(1, 1.4 * sc)], [tp[1][0], tp[1][1] - Math.max(1, 1.4 * sc)]], p.b);
  } else if (spec.roof === "crenel") {
    quad(tp, p.roof2);
    const m = Math.max(1.6, w * 0.075), mh = Math.max(2, rh * 0.5);
    for (let i = 0; i <= 4; i++) {
      for (const [ax, ay, bx, by2] of [[tp[3][0], tp[3][1], tp[2][0], tp[2][1]], [tp[2][0], tp[2][1], tp[1][0], tp[1][1]]]) {
        const x = ax + (bx - ax) * (i / 4), y = ay + (by2 - ay) * (i / 4);
        quad([[x - m, y - mh], [x + m, y - mh], [x + m, y], [x - m, y]], p.a);
        quad([[x - m, y - mh], [x, y - mh - m * 0.5], [x + m, y - mh], [x, y - mh + m * 0.5]], shade(hx(p.a), 1.15));
      }
    }
  } else if (spec.roof === "dome") {
    quad(tp, p.roof2);
    const dr = hw * 0.72, dy = T + hh * 0.5;
    ctx.beginPath(); ctx.ellipse(cx, dy, dr, rh * 1.05, 0, Math.PI, 0);
    ctx.fillStyle = lgrad(ctx, cx - dr, dy, cx + dr, dy, p.roof); ctx.fill();
    ctx.beginPath(); ctx.ellipse(cx, dy, dr, rh * 0.35, 0, 0, Math.PI * 2);
    ctx.strokeStyle = shade(hx(p.roof), 0.7); ctx.lineWidth = Math.max(0.8, 0.6 * sc); ctx.stroke();
    ctx.fillStyle = "#d8b24a"; ctx.fillRect(cx - Math.max(0.5, 0.4 * sc), dy - rh * 1.05 - Math.max(2, rh * 0.5), Math.max(1, 0.9 * sc), Math.max(2, rh * 0.5));
    ctx.beginPath(); ctx.arc(cx, dy - rh * 1.05 - Math.max(2, rh * 0.5), Math.max(1, 0.9 * sc), 0, 7); ctx.fillStyle = "#f0d98a"; ctx.fill();
  } else if (spec.roof === "spire") {
    quad(tp, p.roof2);
    // tower block then a tall spire
    const tb = Math.max(3, h * 0.3);
    quad([[cx - hw * 0.34, T + hh - tb], [cx, T + hh * 0.5 - tb], [cx + hw * 0.34, T + hh - tb], [cx + hw * 0.34, T + hh], [cx, T + hh * 1.5], [cx - hw * 0.34, T + hh]], p.a);
    quad([[cx, T - rh * 2.6], [cx + hw * 0.34, T + hh - tb], [cx, T + hh * 1.5 - tb], [cx - hw * 0.34, T + hh - tb]], p.roof);
    quad([[cx, T - rh * 2.6], [cx - hw * 0.34, T + hh - tb], [cx, T + hh * 1.5 - tb]], p.roof2);
    ctx.fillStyle = "#d8b24a"; ctx.fillRect(cx - Math.max(0.5, 0.4 * sc), T - rh * 3.2, Math.max(1, 0.9 * sc), Math.max(2, rh * 0.6));
    ctx.fillRect(cx - Math.max(1.5, 1.4 * sc), T - rh * 3.0, Math.max(3, 2.8 * sc), Math.max(1, 0.7 * sc));
  }

  const e = spec.extra;
  if (e === "tower") {
    const tw = w * 0.34, th = h * 1.15, bx = cx - hw * 1.28, byy = B + hh * 0.3;
    quad([[bx, byy - th - tw * 0.5], [bx + tw, byy - th], [bx + tw, byy], [bx, byy + tw * 0.5]], p.b);
    quad([[bx, byy - th - tw * 0.5], [bx - tw * 0.55, byy - th - tw * 0.2], [bx - tw * 0.55, byy + tw * 0.3], [bx, byy + tw * 0.5]], p.c);
    quad([[bx - tw * 0.55, byy - th - tw * 0.2], [bx, byy - th - tw * 0.5], [bx + tw, byy - th], [bx + tw * 0.45, byy - th + tw * 0.3]], p.a);
    quad([[bx + tw * 0.2, byy - th - tw * 1.1], [bx + tw * 1.1, byy - th - tw * 0.1], [bx + tw * 0.2, byy - th + tw * 0.5], [bx - tw * 0.7, byy - th - tw * 0.5]], p.roof);
  }
  if (e === "chimney") {
    const chx = cx + hw * 0.4, chy = T - rh * 1.7;
    quad([[chx, chy], [chx + w * 0.1, chy + w * 0.05], [chx + w * 0.1, chy + rh * 1.9], [chx, chy + rh * 1.95]], p.c);
    ctx.save(); ctx.globalAlpha = 0.35; ctx.fillStyle = "#c8d2dc";
    for (let i = 0; i < 3; i++) { ctx.beginPath(); ctx.arc(chx + w * 0.05 + i * w * 0.03, chy - rh * (0.4 + i * 0.45), Math.max(1, w * 0.06 - i * 0.4), 0, 7); ctx.fill(); }
    ctx.restore();
  }
  if (e === "crane") {
    const jx = cx + hw * 0.55;
    line(jx, B - h * 0.1, jx, T - rh * 1.9, shade(hx(p.c), 0.9), Math.max(1, 1.1 * sc));
    line(jx, T - rh * 1.9, jx + hw * 0.8, T - rh * 1.1, shade(hx(p.c), 0.9), Math.max(1, 1.1 * sc));
    line(jx + hw * 0.8, T - rh * 1.1, jx + hw * 0.8, T - rh * 0.2, "#3b4a58", Math.max(0.8, 0.7 * sc));
    ctx.fillStyle = "#7a5f3a"; ctx.fillRect(jx + hw * 0.8 - Math.max(1, 1.4 * sc), T - rh * 0.2, Math.max(2, 2.8 * sc), Math.max(2, 2.2 * sc));
  }
  if (e === "arch") {
    const ax = cx, ay = B + hh * 0.1, aw = Math.max(2, w * 0.15), ah = Math.max(3, h * 0.42);
    ctx.beginPath(); ctx.moveTo(ax - aw, ay); ctx.lineTo(ax - aw, ay - ah + aw);
    ctx.quadraticCurveTo(ax, ay - ah - aw * 0.4, ax + aw, ay - ah + aw); ctx.lineTo(ax + aw, ay); ctx.closePath();
    ctx.fillStyle = "rgba(12,16,22,0.85)"; ctx.fill();
    ctx.strokeStyle = shade(hx(p.a), 1.1); ctx.lineWidth = Math.max(0.7, 0.6 * sc); ctx.stroke();
  }
  if (e === "dock") {
    quad([[cx - hw * 1.25, B + hh * 0.2], [cx + hw * 0.6, B - hh * 0.7], [cx + hw * 1.0, B - hh * 0.45], [cx - hw * 0.85, B + hh * 0.45]], shade(hx(p.c), 1.05));
    for (let i = 0; i < 4; i++) {
      const t = i / 3;
      const x = cx - hw * 1.1 + t * hw * 1.7, y = B + hh * 0.32 - t * hh * 0.75;
      ctx.fillStyle = shade(hx(p.c), 0.7); ctx.fillRect(x, y, Math.max(1, 1.2 * sc), Math.max(2, 3 * sc));
    }
  }
}

/** Flag + banner on top of a landmark. */
export function flag(ctx: Ctx, cx: number, topY: number, size: number, color: string) {
  ctx.strokeStyle = "#1b2833"; ctx.lineWidth = Math.max(1, size * 0.06);
  ctx.beginPath(); ctx.moveTo(cx, topY); ctx.lineTo(cx, topY - size * 0.5); ctx.stroke();
  ctx.fillStyle = color; ctx.beginPath();
  ctx.moveTo(cx, topY - size * 0.5); ctx.lineTo(cx + size * 0.32, topY - size * 0.42); ctx.lineTo(cx, topY - size * 0.3); ctx.closePath(); ctx.fill();
}
