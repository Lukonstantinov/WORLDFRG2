// EU4-style trade-good medallions, drawn as vector art directly on the map
// canvas (no image assets). Each good is a small roundel — a dimensional disc
// tinted by the good's colour with a metallic rim — carrying a duotone
// silhouette symbol. This replaces the emoji glyphs the overlay used before.
//
// Public API: `drawGoodIcon(ctx, name, cx, cy, r, opts)` renders one medallion
// centred at (cx, cy) with radius r in the current (already-transformed) canvas
// space. `opts.sublabel` selects the gem facet colour for the gemstones good.

// ── colour helpers ───────────────────────────────────────────────────────────
function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  const v = h.length === 3
    ? h.split("").map((c) => c + c).join("")
    : h.padEnd(6, "0").slice(0, 6);
  return [parseInt(v.slice(0, 2), 16), parseInt(v.slice(2, 4), 16), parseInt(v.slice(4, 6), 16)];
}
function rgb(r: number, g: number, b: number): string {
  return `rgb(${r | 0},${g | 0},${b | 0})`;
}
function mix([r, g, b]: [number, number, number], to: number, t: number): string {
  return rgb(r + (to - r) * t, g + (to - g) * t, b + (to - b) * t);
}
function luminance([r, g, b]: [number, number, number]): number {
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255;
}

/** Per-gemstone facet colour, keyed by the region sublabel (Ruby/Sapphire/…). */
export const GEM_COLORS: Record<string, string> = {
  Ruby: "#e0245e",
  Sapphire: "#2f6fe0",
  Emerald: "#2faa55",
  Diamond: "#cfeaf0",
  Topaz: "#e0a83a",
  Amethyst: "#9b59d0",
};

// ── shape primitives (assume ctx fill/stroke already set) ─────────────────────
function poly(ctx: CanvasRenderingContext2D, pts: number[][], close = true) {
  ctx.beginPath();
  ctx.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], pts[i][1]);
  if (close) ctx.closePath();
}
function dot(ctx: CanvasRenderingContext2D, x: number, y: number, rr: number) {
  ctx.beginPath();
  ctx.arc(x, y, rr, 0, Math.PI * 2);
  ctx.fill();
}

// Each symbol draws inside a box of half-size s around (x,y), filling with `ink`.
type Sym = (ctx: CanvasRenderingContext2D, x: number, y: number, s: number, ink: string, accent: string) => void;

const drop: Sym = (ctx, x, y, s, ink) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x, y - s);
  ctx.bezierCurveTo(x + s * 0.9, y - s * 0.1, x + s * 0.7, y + s * 0.9, x, y + s * 0.9);
  ctx.bezierCurveTo(x - s * 0.7, y + s * 0.9, x - s * 0.9, y - s * 0.1, x, y - s);
  ctx.fill();
};

const leaf: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x - s * 0.7, y + s * 0.7);
  ctx.quadraticCurveTo(x - s * 0.9, y - s * 0.9, x + s * 0.7, y - s * 0.7);
  ctx.quadraticCurveTo(x + s * 0.9, y + s * 0.5, x - s * 0.7, y + s * 0.7);
  ctx.fill();
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.12;
  ctx.beginPath();
  ctx.moveTo(x - s * 0.6, y + s * 0.6);
  ctx.lineTo(x + s * 0.6, y - s * 0.6);
  ctx.stroke();
};

const grain: Sym = (ctx, x, y, s, ink) => {
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.14;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x, y + s);
  ctx.lineTo(x, y - s * 0.7);
  ctx.stroke();
  ctx.fillStyle = ink;
  for (let i = 0; i < 4; i++) {
    const yy = y - s * 0.7 + i * s * 0.45;
    for (const dir of [-1, 1]) {
      ctx.beginPath();
      ctx.ellipse(x + dir * s * 0.42, yy + s * 0.18, s * 0.28, s * 0.14, dir * 0.7, 0, Math.PI * 2);
      ctx.fill();
    }
  }
};

const gem: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  poly(ctx, [
    [x, y - s], [x + s * 0.85, y - s * 0.2], [x + s * 0.45, y + s], [x - s * 0.45, y + s], [x - s * 0.85, y - s * 0.2],
  ]);
  ctx.fill();
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.1;
  ctx.beginPath();
  ctx.moveTo(x - s * 0.85, y - s * 0.2); ctx.lineTo(x + s * 0.85, y - s * 0.2);
  ctx.moveTo(x, y - s); ctx.lineTo(x - s * 0.3, y - s * 0.2); ctx.lineTo(x - s * 0.45, y + s);
  ctx.moveTo(x, y - s); ctx.lineTo(x + s * 0.3, y - s * 0.2); ctx.lineTo(x + s * 0.45, y + s);
  ctx.stroke();
};

const fish: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.ellipse(x + s * 0.05, y, s * 0.85, s * 0.45, 0, 0, Math.PI * 2);
  ctx.fill();
  poly(ctx, [[x - s * 0.7, y], [x - s, y - s * 0.5], [x - s, y + s * 0.5]]);
  ctx.fill();
  ctx.fillStyle = accent;
  dot(ctx, x + s * 0.45, y - s * 0.12, s * 0.1);
};

const whale: Sym = (ctx, x, y, s, ink) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x - s, y + s * 0.2);
  ctx.quadraticCurveTo(x - s * 0.2, y - s * 0.7, x + s * 0.7, y - s * 0.1);
  ctx.quadraticCurveTo(x + s, y, x + s * 0.8, y + s * 0.4);
  ctx.quadraticCurveTo(x, y + s * 0.7, x - s, y + s * 0.2);
  ctx.fill();
  poly(ctx, [[x + s * 0.6, y - s * 0.1], [x + s, y - s * 0.8], [x + s * 0.95, y - s * 0.1]]);
  ctx.fill();
};

const ingot: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  poly(ctx, [[x - s * 0.55, y - s * 0.4], [x + s * 0.55, y - s * 0.4], [x + s * 0.85, y + s * 0.4], [x - s * 0.85, y + s * 0.4]]);
  ctx.fill();
  ctx.fillStyle = accent;
  poly(ctx, [[x - s * 0.55, y - s * 0.4], [x + s * 0.55, y - s * 0.4], [x + s * 0.4, y - s * 0.15], [x - s * 0.4, y - s * 0.15]]);
  ctx.fill();
};

const pick: Sym = (ctx, x, y, s, ink) => {
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.16;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x - s * 0.1, y - s * 0.8); ctx.lineTo(x - s * 0.1, y + s * 0.9);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(x - s, y - s * 0.45);
  ctx.quadraticCurveTo(x, y - s * 0.95, x + s, y - s * 0.45);
  ctx.stroke();
};

const coin: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  dot(ctx, x, y, s * 0.9);
  ctx.fillStyle = accent;
  dot(ctx, x, y, s * 0.62);
  ctx.fillStyle = ink;
  ctx.font = `bold ${s}px serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("✦", x, y + s * 0.05);
};

const amphora: Sym = (ctx, x, y, s, ink) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x, y - s);
  ctx.quadraticCurveTo(x + s * 0.7, y - s * 0.6, x + s * 0.5, y + s * 0.2);
  ctx.quadraticCurveTo(x + s * 0.35, y + s, x, y + s);
  ctx.quadraticCurveTo(x - s * 0.35, y + s, x - s * 0.5, y + s * 0.2);
  ctx.quadraticCurveTo(x - s * 0.7, y - s * 0.6, x, y - s);
  ctx.fill();
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.14;
  ctx.beginPath();
  ctx.arc(x - s * 0.5, y - s * 0.35, s * 0.28, Math.PI * 0.2, Math.PI * 1.1);
  ctx.arc(x + s * 0.5, y - s * 0.35, s * 0.28, -Math.PI * 0.1, Math.PI * 0.8, true);
  ctx.stroke();
};

const goblet: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x - s * 0.6, y - s * 0.8);
  ctx.lineTo(x + s * 0.6, y - s * 0.8);
  ctx.quadraticCurveTo(x + s * 0.5, y + s * 0.1, x, y + s * 0.2);
  ctx.quadraticCurveTo(x - s * 0.5, y + s * 0.1, x - s * 0.6, y - s * 0.8);
  ctx.fill();
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.13;
  ctx.beginPath();
  ctx.moveTo(x, y + s * 0.2); ctx.lineTo(x, y + s * 0.8);
  ctx.moveTo(x - s * 0.45, y + s * 0.9); ctx.lineTo(x + s * 0.45, y + s * 0.9);
  ctx.stroke();
  ctx.fillStyle = accent;
  poly(ctx, [[x - s * 0.5, y - s * 0.75], [x + s * 0.5, y - s * 0.75], [x, y - s * 0.2]]);
  ctx.fill();
};

const bale: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.ellipse(x, y, s * 0.9, s * 0.8, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.1;
  for (let i = -1; i <= 1; i++) {
    ctx.beginPath();
    ctx.ellipse(x, y, s * 0.9, s * 0.8, 0, 0, Math.PI * 2);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(x + i * s * 0.4, y - s * 0.78);
    ctx.lineTo(x + i * s * 0.4, y + s * 0.78);
    ctx.stroke();
  }
};

const spool: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.fillRect(x - s * 0.5, y - s * 0.85, s, s * 1.7);
  ctx.fillStyle = accent;
  ctx.fillRect(x - s * 0.8, y - s * 0.85, s * 1.6, s * 0.22);
  ctx.fillRect(x - s * 0.8, y + s * 0.63, s * 1.6, s * 0.22);
};

const log: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.ellipse(x, y, s * 0.95, s * 0.6, Math.PI / 4, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = accent;
  ctx.beginPath();
  ctx.ellipse(x + s * 0.5, y - s * 0.5, s * 0.3, s * 0.22, Math.PI / 4, 0, Math.PI * 2);
  ctx.fill();
};

const animal: Sym = (ctx, x, y, s, ink) => {
  // generic quadruped silhouette (horses/cattle)
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.ellipse(x, y + s * 0.1, s * 0.8, s * 0.45, 0, 0, Math.PI * 2);
  ctx.fill();
  // neck + head
  poly(ctx, [[x + s * 0.5, y - s * 0.1], [x + s, y - s], [x + s * 1.05, y - s * 0.55], [x + s * 0.7, y + s * 0.1]]);
  ctx.fill();
  // legs
  ctx.fillRect(x - s * 0.6, y + s * 0.4, s * 0.18, s * 0.7);
  ctx.fillRect(x + s * 0.3, y + s * 0.4, s * 0.18, s * 0.7);
};

const tusk: Sym = (ctx, x, y, s, ink) => {
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.3;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x - s * 0.6, y - s * 0.7);
  ctx.quadraticCurveTo(x + s * 0.4, y - s * 0.2, x + s * 0.3, y + s * 0.8);
  ctx.stroke();
};

const pelt: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  poly(ctx, [
    [x, y - s], [x + s * 0.5, y - s * 0.6], [x + s, y - s * 0.7], [x + s * 0.6, y + s * 0.2],
    [x + s * 0.4, y + s], [x - s * 0.4, y + s], [x - s * 0.6, y + s * 0.2], [x - s, y - s * 0.7], [x - s * 0.5, y - s * 0.6],
  ]);
  ctx.fill();
  ctx.fillStyle = accent;
  dot(ctx, x, y, s * 0.18);
};

const crystals: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  poly(ctx, [[x - s * 0.8, y + s * 0.6], [x - s * 0.4, y - s * 0.4], [x, y + s * 0.6]]); ctx.fill();
  poly(ctx, [[x - s * 0.1, y + s * 0.6], [x + s * 0.35, y - s * 0.8], [x + s * 0.8, y + s * 0.6]]); ctx.fill();
  ctx.fillStyle = accent;
  poly(ctx, [[x + s * 0.2, y + s * 0.6], [x + s * 0.55, y + s * 0.05], [x + s * 0.9, y + s * 0.6]]); ctx.fill();
};

const bean: Sym = (ctx, x, y, s, ink, accent) => {
  for (const [dx, dy] of [[-s * 0.35, -s * 0.1], [s * 0.35, s * 0.1]]) {
    ctx.fillStyle = ink;
    ctx.beginPath();
    ctx.ellipse(x + dx, y + dy, s * 0.45, s * 0.62, 0.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = accent;
    ctx.lineWidth = s * 0.08;
    ctx.beginPath();
    ctx.moveTo(x + dx - s * 0.28, y + dy - s * 0.32);
    ctx.quadraticCurveTo(x + dx + s * 0.1, y + dy, x + dx + s * 0.28, y + dy + s * 0.32);
    ctx.stroke();
  }
};

const flower: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  for (let i = 0; i < 6; i++) {
    const a = (i / 6) * Math.PI * 2;
    ctx.beginPath();
    ctx.ellipse(x + Math.cos(a) * s * 0.5, y + Math.sin(a) * s * 0.5, s * 0.34, s * 0.2, a, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.fillStyle = accent;
  dot(ctx, x, y, s * 0.3);
};

const smoke: Sym = (ctx, x, y, s, ink) => {
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.16;
  ctx.lineCap = "round";
  for (const dx of [-s * 0.4, s * 0.4]) {
    ctx.beginPath();
    ctx.moveTo(x + dx, y + s);
    ctx.bezierCurveTo(x + dx - s * 0.6, y + s * 0.2, x + dx + s * 0.6, y - s * 0.4, x + dx, y - s);
    ctx.stroke();
  }
};

const palm: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.14;
  ctx.beginPath();
  ctx.moveTo(x, y + s); ctx.quadraticCurveTo(x - s * 0.2, y, x - s * 0.1, y - s * 0.4);
  ctx.stroke();
  ctx.strokeStyle = ink;
  for (let i = 0; i < 5; i++) {
    const a = -Math.PI / 2 + (i - 2) * 0.55;
    ctx.beginPath();
    ctx.moveTo(x - s * 0.1, y - s * 0.5);
    ctx.quadraticCurveTo(x + Math.cos(a) * s * 0.6, y - s * 0.5 + Math.sin(a) * s * 0.5,
      x + Math.cos(a) * s * 1.1, y - s * 0.4 + Math.sin(a) * s);
    ctx.stroke();
  }
  ctx.fillStyle = accent;
  dot(ctx, x + s * 0.1, y + s * 0.55, s * 0.16);
};

const berries: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  for (const [dx, dy] of [[-s * 0.4, -s * 0.2], [s * 0.4, -s * 0.2], [0, s * 0.4]]) dot(ctx, x + dx, y + dy, s * 0.42);
  ctx.fillStyle = accent;
  for (const [dx, dy] of [[-s * 0.4, -s * 0.2], [s * 0.4, -s * 0.2], [0, s * 0.4]]) dot(ctx, x + dx - s * 0.12, y + dy - s * 0.12, s * 0.12);
};

const scroll: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.fillRect(x - s * 0.7, y - s * 0.6, s * 1.4, s * 1.2);
  ctx.fillStyle = accent;
  ctx.beginPath(); ctx.ellipse(x - s * 0.7, y, s * 0.22, s * 0.6, 0, 0, Math.PI * 2); ctx.fill();
  ctx.beginPath(); ctx.ellipse(x + s * 0.7, y, s * 0.22, s * 0.6, 0, 0, Math.PI * 2); ctx.fill();
};

const pot: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x - s * 0.65, y - s * 0.5);
  ctx.quadraticCurveTo(x - s * 0.95, y + s * 0.2, x - s * 0.4, y + s * 0.8);
  ctx.lineTo(x + s * 0.4, y + s * 0.8);
  ctx.quadraticCurveTo(x + s * 0.95, y + s * 0.2, x + s * 0.65, y - s * 0.5);
  ctx.quadraticCurveTo(x, y - s * 0.85, x - s * 0.65, y - s * 0.5);
  ctx.fill();
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.1;
  ctx.beginPath(); ctx.moveTo(x - s * 0.7, y - s * 0.1); ctx.lineTo(x + s * 0.7, y - s * 0.1); ctx.stroke();
};

const shell: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x, y + s * 0.7);
  ctx.arc(x, y - s * 0.1, s * 0.85, Math.PI, 0);
  ctx.closePath();
  ctx.fill();
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.08;
  for (let i = -2; i <= 2; i++) {
    ctx.beginPath();
    ctx.moveTo(x, y + s * 0.6);
    ctx.lineTo(x + i * s * 0.35, y - s * 0.75);
    ctx.stroke();
  }
};

const grapes: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.12;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x, y - s * 0.55);
  ctx.lineTo(x + s * 0.12, y - s);
  ctx.stroke();
  ctx.fillStyle = accent;
  ctx.beginPath();
  ctx.ellipse(x + s * 0.42, y - s * 0.85, s * 0.3, s * 0.16, -0.5, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = ink;
  for (const [dx, dy] of [[-0.46, -0.45], [0, -0.5], [0.46, -0.45], [-0.26, -0.02], [0.26, -0.02], [0, 0.42]] as const) {
    dot(ctx, x + dx * s, y + dy * s, s * 0.29);
  }
};

const coral: Sym = (ctx, x, y, s, ink) => {
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.2;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  const branch = (bx: number, by: number, ang: number, len: number, depth: number) => {
    const ex = bx + Math.cos(ang) * len;
    const ey = by + Math.sin(ang) * len;
    ctx.beginPath();
    ctx.moveTo(bx, by);
    ctx.lineTo(ex, ey);
    ctx.stroke();
    if (depth > 0) {
      branch(ex, ey, ang - 0.6, len * 0.72, depth - 1);
      branch(ex, ey, ang + 0.5, len * 0.72, depth - 1);
    }
  };
  branch(x, y + s * 0.95, -Math.PI / 2, s * 0.62, 2);
};

const cinnamon: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  for (const dx of [-0.42, 0.02]) {
    ctx.save();
    ctx.translate(x + dx * s, y);
    ctx.rotate(0.12);
    ctx.fillRect(-s * 0.18, -s * 0.92, s * 0.4, s * 1.84);
    ctx.strokeStyle = accent;
    ctx.lineWidth = s * 0.09;
    ctx.beginPath(); ctx.ellipse(s * 0.02, -s * 0.92, s * 0.2, s * 0.09, 0, 0, Math.PI * 2); ctx.stroke();
    ctx.beginPath(); ctx.ellipse(s * 0.02, s * 0.92, s * 0.2, s * 0.09, 0, 0, Math.PI * 2); ctx.stroke();
    ctx.restore();
  }
};

const citrus: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  dot(ctx, x, y, s * 0.9);
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.1;
  for (let i = 0; i < 7; i++) {
    const a = (i / 7) * Math.PI * 2;
    ctx.beginPath();
    ctx.moveTo(x, y);
    ctx.lineTo(x + Math.cos(a) * s * 0.82, y + Math.sin(a) * s * 0.82);
    ctx.stroke();
  }
  ctx.fillStyle = accent;
  dot(ctx, x, y, s * 0.13);
};

const column: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.fillRect(x - s * 0.42, y - s * 0.62, s * 0.84, s * 1.2); // shaft
  ctx.fillStyle = accent;
  ctx.fillRect(x - s * 0.72, y - s * 0.9, s * 1.44, s * 0.24); // capital
  ctx.fillRect(x - s * 0.72, y + s * 0.6, s * 1.44, s * 0.28); // base
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.07;
  for (const dx of [-0.18, 0.18]) {
    ctx.beginPath();
    ctx.moveTo(x + dx * s, y - s * 0.55);
    ctx.lineTo(x + dx * s, y + s * 0.55);
    ctx.stroke();
  }
};

const ring: Sym = (ctx, x, y, s, ink, accent) => {
  // Carved jade disc (bì): solid rim with a hole, rendered as nested discs.
  ctx.fillStyle = ink;
  dot(ctx, x, y, s * 0.92);
  ctx.fillStyle = accent;
  dot(ctx, x, y, s * 0.58);
  ctx.fillStyle = ink;
  dot(ctx, x, y, s * 0.3);
};

const blob: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.fillStyle = ink;
  ctx.beginPath();
  ctx.moveTo(x - s * 0.8, y);
  ctx.bezierCurveTo(x - s * 0.7, y - s * 0.9, x + s * 0.6, y - s * 0.85, x + s * 0.82, y - s * 0.1);
  ctx.bezierCurveTo(x + s * 0.95, y + s * 0.7, x - s * 0.5, y + s * 0.92, x - s * 0.8, y);
  ctx.fill();
  ctx.fillStyle = accent;
  dot(ctx, x - s * 0.18, y - s * 0.22, s * 0.18);
  dot(ctx, x + s * 0.3, y + s * 0.2, s * 0.12);
};

const sheaf: Sym = (ctx, x, y, s, ink, accent) => {
  ctx.strokeStyle = ink;
  ctx.lineWidth = s * 0.13;
  ctx.lineCap = "round";
  for (const dx of [-0.4, 0, 0.4]) {
    ctx.beginPath();
    ctx.moveTo(x + dx * s * 0.7, y + s);
    ctx.lineTo(x + dx * s, y - s);
    ctx.stroke();
    ctx.fillStyle = ink;
    dot(ctx, x + dx * s, y - s, s * 0.15);
  }
  ctx.strokeStyle = accent;
  ctx.lineWidth = s * 0.16;
  ctx.beginPath();
  ctx.moveTo(x - s * 0.5, y + s * 0.3);
  ctx.lineTo(x + s * 0.5, y + s * 0.3);
  ctx.stroke();
};

const SYMBOLS: Record<string, Sym> = {
  silk: spool, cotton: bale, wool_fleece: bale, wool_llama: bale, flax: sheaf,
  wine: grapes, oliveoil: amphora, ceramics: pot, glassware: goblet,
  sugar: crystals, salt: crystals, bay_salt: crystals,
  frankincense: smoke, incense: smoke,
  stockfish: fish, dyes: drop, indigo: drop, tyrian_purple: shell,
  spices: leaf, tea: leaf, cloves: leaf, pepper: berries, tobacco: leaf,
  cinnamon: cinnamon, saffron: flower,
  coffee: bean, cacao: bean,
  furs: pelt, timber: log, hardwoods: log,
  amber: drop, pearls: shell, coral: coral, ambergris: blob,
  whaling: whale, wheat: grain, iron: pick, copper: ingot, tin: ingot,
  gold: coin, silver: ingot, lead: ingot,
  gemstones: gem, jade: ring, marble: column,
  horses: animal, ivory: tusk,
  paper: scroll, dates: palm, citrus: citrus,
};
const FALLBACK: Sym = (ctx, x, y, s, ink) => {
  ctx.fillStyle = ink;
  dot(ctx, x, y, s * 0.5);
};

export interface IconOpts { sublabel?: string; color?: string }

/** Draw one good's medallion centred at (cx, cy) with radius r. */
export function drawGoodIcon(
  ctx: CanvasRenderingContext2D, name: string, cx: number, cy: number, r: number,
  baseColor: string, opts: IconOpts = {},
) {
  // Gemstones take the per-stone facet colour from the sublabel.
  let color = baseColor;
  if (name === "gemstones" && opts.sublabel && GEM_COLORS[opts.sublabel]) {
    color = GEM_COLORS[opts.sublabel];
  }
  const base = hexToRgb(color);

  ctx.save();
  // Drop shadow.
  ctx.globalAlpha = 0.35;
  ctx.fillStyle = "#000";
  dot(ctx, cx + r * 0.06, cy + r * 0.08, r);
  ctx.globalAlpha = 1;

  // Dimensional disc: light top → dark bottom gradient in the good's colour.
  const grad = ctx.createLinearGradient(cx, cy - r, cx, cy + r);
  grad.addColorStop(0, mix(base, 255, 0.45));
  grad.addColorStop(0.55, color);
  grad.addColorStop(1, mix(base, 0, 0.45));
  ctx.fillStyle = grad;
  dot(ctx, cx, cy, r);

  // Metallic rim.
  ctx.lineWidth = r * 0.16;
  ctx.strokeStyle = "rgba(20,16,10,0.7)";
  ctx.beginPath(); ctx.arc(cx, cy, r * 0.97, 0, Math.PI * 2); ctx.stroke();
  ctx.lineWidth = r * 0.08;
  ctx.strokeStyle = "rgba(232,210,150,0.85)"; // gold inner ring
  ctx.beginPath(); ctx.arc(cx, cy, r * 0.86, 0, Math.PI * 2); ctx.stroke();

  // Symbol ink: dark on light medallion, light on dark.
  const lum = luminance(base);
  const ink = lum > 0.55 ? "rgba(28,22,14,0.92)" : "rgba(245,242,232,0.95)";
  const accent = lum > 0.55 ? "rgba(255,255,255,0.55)" : "rgba(0,0,0,0.4)";

  const sym = SYMBOLS[name] ?? FALLBACK;
  ctx.lineJoin = "round";
  sym(ctx, cx, cy, r * 0.62, ink, accent);

  ctx.restore();
}
