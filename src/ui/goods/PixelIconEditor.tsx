import { useEffect, useRef, useState } from "react";
import { PIXEL_GOODS, SPRITE_GRID as N, spritePixels, outlinePixels, encodePixels, decodePixels } from "@canvas/goodArt";

/** The Goods Builder's icon editor (plan §13 decision 3): a 24×24 pixel grid that
 *  starts from the good's current icon or any shipped one, recolours, and turns an
 *  uploaded image into house-style pixels (8 colours + the dark outline). */
export function PixelIconEditor({ id, color, pixels, onChange }: {
  id: string; color: string; pixels: string | null | undefined;
  onChange: (enc: string | null) => void;
}) {
  const [grid, setGrid] = useState<(string | null)[]>(() => pixels ? decodePixels(pixels) : spritePixels(id, color));
  const [ink, setInk] = useState("#c8a050");
  const [erase, setErase] = useState(false);
  const [from, setFrom] = useState("");
  const ref = useRef<HTMLCanvasElement>(null);
  const drawing = useRef(false);
  const S = 11; // screen px per cell

  useEffect(() => {
    const ctx = ref.current?.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, N * S, N * S);
    for (let i = 0; i < N * N; i++) {
      const x = (i % N) * S, y = Math.floor(i / N) * S;
      ctx.fillStyle = grid[i] ?? (((i % N) + Math.floor(i / N)) % 2 ? "#1d1a15" : "#24201a");
      ctx.fillRect(x, y, S, S);
    }
  }, [grid]);

  const commit = (g: (string | null)[]) => { setGrid(g); onChange(encodePixels(g)); };
  const paintAt = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    const x = Math.floor(((e.clientX - r.left) / r.width) * N), y = Math.floor(((e.clientY - r.top) / r.height) * N);
    if (x < 0 || y < 0 || x >= N || y >= N) return;
    const v = erase ? null : ink;
    if (grid[y * N + x] === v) return;
    const g = grid.slice(); g[y * N + x] = v; commit(g);
  };
  const recolour = () => {
    // Replace the most common colour (the body of most icons) with the ink.
    const count = new Map<string, number>();
    for (const c of grid) if (c) count.set(c, (count.get(c) ?? 0) + 1);
    const top = [...count.entries()].sort((a, b) => b[1] - a[1])[0]?.[0];
    if (top) commit(grid.map((c) => (c === top ? ink : c)));
  };
  const upload = (file: File) => {
    const img = new Image();
    img.onload = () => {
      const cv = document.createElement("canvas"); cv.width = cv.height = N - 2;
      const ctx = cv.getContext("2d"); if (!ctx) return;
      const k = Math.min((N - 2) / img.width, (N - 2) / img.height);
      ctx.drawImage(img, ((N - 2) - img.width * k) / 2, ((N - 2) - img.height * k) / 2, img.width * k, img.height * k);
      const d = ctx.getImageData(0, 0, N - 2, N - 2).data;
      const raw: ([number, number, number] | null)[] = [];
      for (let i = 0; i < (N - 2) * (N - 2); i++) raw.push(d[i * 4 + 3] < 128 ? null : [d[i * 4], d[i * 4 + 1], d[i * 4 + 2]]);
      const pal = palette(raw, 8);
      const out: (string | null)[] = new Array(N * N).fill(null);
      raw.forEach((c, i) => {
        if (!c) return;
        const x = (i % (N - 2)) + 1, y = Math.floor(i / (N - 2)) + 1;
        out[y * N + x] = nearest(pal, c);
      });
      commit(outlinePixels(out));
    };
    img.src = URL.createObjectURL(file);
  };

  return (
    <div style={{ flexBasis: "100%", display: "flex", gap: 12, alignItems: "flex-start" }}>
      <canvas ref={ref} width={N * S} height={N * S}
        style={{ border: "1px solid #3f382d", cursor: "crosshair", imageRendering: "pixelated" }}
        onMouseDown={(e) => { drawing.current = true; paintAt(e); }}
        onMouseMove={(e) => { if (drawing.current) paintAt(e); }}
        onMouseUp={() => { drawing.current = false; }} onMouseLeave={() => { drawing.current = false; }} />
      <div style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: 11 }}>
        <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
          Ink <input type="color" value={ink} onChange={(e) => { setInk(e.target.value); setErase(false); }} />
          <label><input type="checkbox" checked={erase} onChange={(e) => setErase(e.target.checked)} /> eraser</label>
        </label>
        <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
          Start from <select value={from} onChange={(e) => { setFrom(e.target.value); if (e.target.value) commit(spritePixels(e.target.value, color)); }}>
            <option value="">— a shipped icon —</option>
            {PIXEL_GOODS.slice().sort().map((n) => <option key={n} value={n}>{n}</option>)}
          </select>
        </label>
        <button onClick={recolour} title="Paint the icon's main colour with the ink">Recolour body</button>
        <label>Upload image <input type="file" accept="image/*" onChange={(e) => { const f = e.target.files?.[0]; if (f) upload(f); }} /></label>
        <button onClick={() => { setGrid(spritePixels(id, color)); onChange(null); }} title="Drop the drawn icon; use the shipped art (or the crate)">Reset to default</button>
        <span style={{ color: "#8a7d66", maxWidth: 220 }}>Uploads are shrunk to 22×22, cut to 8 colours and given the dark house outline.</span>
      </div>
    </div>
  );
}

/** Up to `k` representative colours: the most common 4-bit-per-channel buckets. */
function palette(px: ([number, number, number] | null)[], k: number): [number, number, number][] {
  const buckets = new Map<number, { n: number; sum: [number, number, number] }>();
  for (const c of px) {
    if (!c) continue;
    const key = ((c[0] >> 4) << 8) | ((c[1] >> 4) << 4) | (c[2] >> 4);
    const b = buckets.get(key) ?? { n: 0, sum: [0, 0, 0] };
    b.n++; b.sum[0] += c[0]; b.sum[1] += c[1]; b.sum[2] += c[2];
    buckets.set(key, b);
  }
  return [...buckets.values()].sort((a, b) => b.n - a.n).slice(0, k)
    .map((b) => [b.sum[0] / b.n, b.sum[1] / b.n, b.sum[2] / b.n].map(Math.round) as [number, number, number]);
}

function nearest(pal: [number, number, number][], c: [number, number, number]): string {
  let best = pal[0], bd = Infinity;
  for (const p of pal) {
    const d = (p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2 + (p[2] - c[2]) ** 2;
    if (d < bd) { bd = d; best = p; }
  }
  return "#" + best.map((v) => v.toString(16).padStart(2, "0")).join("");
}
