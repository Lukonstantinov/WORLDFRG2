/** WorldForge shared UI kit — the small set of presentational primitives every
 *  floating panel should compose from, built on the `chronicleTheme` tokens so
 *  the whole app reads as one designed surface instead of ~60 files of ad-hoc
 *  inline hexes.
 *
 *  These are PURELY presentational (no store access, no data fetching) and they
 *  COMPOSE with the existing floating-window machinery rather than replacing it:
 *  a panel still calls `useFloatingWindow(...)` for drag/tint, then wraps its
 *  body in <Panel> / <PanelHeader> / <Section> etc. See CityRankingPanel for the
 *  reference adoption.
 */
import type { CSSProperties, ReactNode, PointerEvent } from "react";
import { T, SERIF, RADIUS, SHADOW, SPACE, FZ, TONE, type Tone } from "@ui/campaign/chronicleTheme";

// ── Panel shell ──────────────────────────────────────────────────────────────

/** A floating-window surface. Merge the hook's `rootStyle` (position/tint) in via
 *  `style`; keep `data-draggable` on this node so the drag hook can find it. */
export function Panel({
  children, style, width, maxHeight = "78vh", onPointerDown,
}: { children: ReactNode; style?: CSSProperties; width?: number; maxHeight?: CSSProperties["maxHeight"];
  /** The drag hook's `onPointerDown` — pass it so the whole window body drags, not
   *  just the header. Presses on interactive controls are ignored by the hook. */
  onPointerDown?: (e: PointerEvent<HTMLElement>) => void }) {
  return (
    <div
      data-draggable
      onPointerDown={onPointerDown}
      style={{
        position: "absolute",
        width,
        maxHeight,
        display: "flex",
        flexDirection: "column",
        background: T.panel,
        border: `1px solid ${T.line}`,
        borderRadius: RADIUS.lg,
        boxShadow: SHADOW.panel,
        color: T.ink,
        ...style,
      }}
    >
      {children}
    </div>
  );
}

/** The draggable title bar: an icon + serif title on the left, an optional close
 *  ✕ on the right. Pass the hook's `onPointerDown` as `onDragStart`. */
export function PanelHeader({
  icon, title, onClose, onDragStart, right,
}: {
  icon?: ReactNode; title: ReactNode; onClose?: () => void;
  onDragStart?: (e: PointerEvent<HTMLElement>) => void; right?: ReactNode;
}) {
  return (
    <div
      onPointerDown={onDragStart}
      style={{
        display: "flex", alignItems: "center", gap: SPACE.md,
        padding: "8px 10px", borderBottom: `1px solid ${T.line}`,
        cursor: onDragStart ? "move" : undefined, flex: "0 0 auto",
      }}
    >
      {icon != null && <span style={{ fontSize: FZ.head }}>{icon}</span>}
      <span style={{ fontFamily: SERIF, color: T.gold, fontWeight: 700, fontSize: FZ.head, letterSpacing: 0.3 }}>
        {title}
      </span>
      <span style={{ flex: 1 }} />
      {right}
      {onClose && (
        <span
          data-no-drag
          onClick={onClose}
          title="Close"
          style={{ cursor: "pointer", color: T.inkDim, fontSize: FZ.base, padding: "0 2px" }}
        >
          ✕
        </span>
      )}
    </div>
  );
}

/** Scrolling body region for a panel's content. */
export function PanelBody({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <div style={{ overflowY: "auto", padding: `${SPACE.sm}px ${SPACE.md}px ${SPACE.lg}px`, ...style }}>{children}</div>;
}

// ── Structure ────────────────────────────────────────────────────────────────

/** A small-caps section header with an optional right-aligned accessory + body. */
export function Section({
  title, right, children, style,
}: { title?: ReactNode; right?: ReactNode; children?: ReactNode; style?: CSSProperties }) {
  return (
    <div style={{ marginBottom: SPACE.lg, ...style }}>
      {(title != null || right != null) && (
        <div style={{ display: "flex", alignItems: "center", marginBottom: SPACE.sm }}>
          {title != null && (
            <span style={{ color: T.inkDim, fontSize: FZ.small, fontWeight: 600, textTransform: "uppercase", letterSpacing: 0.6 }}>
              {title}
            </span>
          )}
          <span style={{ flex: 1 }} />
          {right}
        </div>
      )}
      {children}
    </div>
  );
}

/** A raised inset card. */
export function Card({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return (
    <div style={{ background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: RADIUS.md, padding: "7px 9px", ...style }}>
      {children}
    </div>
  );
}

/** A hairline divider. */
export function Divider({ style }: { style?: CSSProperties }) {
  return <div style={{ height: 1, background: T.lineSoft, margin: `${SPACE.md}px 0`, ...style }} />;
}

// ── Data display ─────────────────────────────────────────────────────────────

/** A responsive grid of label/value stat tiles. */
export function StatGrid({ cols = 2, children, style }: { cols?: number; children: ReactNode; style?: CSSProperties }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: `repeat(${cols}, 1fr)`, gap: SPACE.sm, ...style }}>
      {children}
    </div>
  );
}

/** One label-over-value stat tile. `tone` tints the value. */
export function Stat({
  label, value, hint, tone,
}: { label: ReactNode; value: ReactNode; hint?: ReactNode; tone?: Tone }) {
  return (
    <div style={{ background: T.card, border: `1px solid ${T.lineSoft}`, borderRadius: RADIUS.sm, padding: "5px 7px" }}>
      <div style={{ color: T.inkDim, fontSize: FZ.tiny, textTransform: "uppercase", letterSpacing: 0.4 }}>{label}</div>
      <div style={{ color: tone ? TONE[tone].ink : T.ink, fontSize: FZ.head, fontWeight: 700, fontFamily: SERIF }}>{value}</div>
      {hint != null && <div style={{ color: T.inkFaint, fontSize: FZ.micro }}>{hint}</div>}
    </div>
  );
}

/** A semantic pill. */
export function Badge({ tone = "neutral", children, style }: { tone?: Tone; children: ReactNode; style?: CSSProperties }) {
  const c = TONE[tone];
  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: 3,
      background: c.fill, color: c.ink, border: `1px solid ${c.line}`,
      borderRadius: RADIUS.pill, padding: "1px 7px", fontSize: FZ.tiny, fontWeight: 600,
      lineHeight: 1.5, whiteSpace: "nowrap", ...style,
    }}>
      {children}
    </span>
  );
}

/** A horizontal progress/proportion bar. `value`/`max` set the fill fraction. */
export function Meter({
  value, max = 1, color = T.gold, height = 4, track = T.card, style,
}: { value: number; max?: number; color?: string; height?: number; track?: string; style?: CSSProperties }) {
  const pct = Math.max(0, Math.min(1, max > 0 ? value / max : 0)) * 100;
  return (
    <div style={{ flex: 1, height, background: track, borderRadius: RADIUS.sm, overflow: "hidden", ...style }}>
      <div style={{ width: `${pct}%`, height: "100%", background: color }} />
    </div>
  );
}

/** A two-tone directional bar: how much of a total came IN against how much went
 *  OUT, on one track. Direction reads without a label, which a single-colour
 *  volume bar cannot do however it is sized.
 *
 *  `max` is the scale the bar is drawn to — pass the largest row's total so bars
 *  are comparable down a list; leave it out and each bar fills its own track. */
export function SplitBar({
  inV, outV, max, width, height = 7, inColor = "#5fd0ff", outColor = "#ffce5f", style,
}: {
  inV: number; outV: number; max?: number; width?: number | string; height?: number;
  inColor?: string; outColor?: string; style?: CSSProperties;
}) {
  const total = Math.max(0, inV) + Math.max(0, outV);
  const scale = max && max > 0 ? max : total;
  const frac = scale > 0 ? Math.min(1, total / scale) : 0;
  const inShare = total > 0 ? Math.max(0, inV) / total : 0;
  return (
    <div style={{
      width: width ?? undefined, flex: width === undefined ? 1 : undefined,
      height, background: T.card, borderRadius: RADIUS.sm, overflow: "hidden",
      display: "flex", ...style,
    }}>
      <div style={{ width: `${frac * inShare * 100}%`, background: inColor }} />
      <div style={{ width: `${frac * (1 - inShare) * 100}%`, background: outColor }} />
    </div>
  );
}

/** One slice of a `Donut`. */
export type Slice = { label: string; value: number; color: string };

/** A donut chart — for a PART-OF-A-WHOLE reading only (a city's exports by good,
 *  who carries its trade). A ranking belongs in bars: a donut makes two similar
 *  slices genuinely hard to order, which is exactly what a ranked list is for.
 *
 *  Drawn as stroked arcs on one circle, so every slice shares one scale by
 *  construction. Slices under `minSlice` of the whole are folded into one
 *  "others" arc rather than drawn as unreadable slivers — and the fold is
 *  reported in the returned legend, never silently dropped.
 *
 *  `center`/`sub` print inside the hole: the total, and what it is a total OF. */
export function Donut({
  slices, size = 108, thickness = 15, center, sub, minSlice = 0.02, restColor = T.inkFaint,
}: {
  slices: Slice[]; size?: number; thickness?: number;
  center?: string; sub?: string; minSlice?: number; restColor?: string;
}) {
  const clean = slices.filter((s) => s.value > 0);
  const total = clean.reduce((a, b) => a + b.value, 0);
  if (total <= 0) {
    return (
      <div style={{ width: size, height: size, display: "grid", placeItems: "center",
        color: T.inkFaint, fontSize: FZ.micro }}>no trade</div>
    );
  }
  // Fold the slivers, then draw. Sorted so the arcs run largest-first from 12
  // o'clock, which is what makes a donut readable at all.
  const big = clean.filter((s) => s.value / total >= minSlice).sort((a, b) => b.value - a.value);
  const restV = total - big.reduce((a, b) => a + b.value, 0);
  const drawn = restV > 0
    ? [...big, { label: `${clean.length - big.length} others`, value: restV, color: restColor }]
    : big;

  const r = (size - thickness) / 2;
  const c = size / 2;
  const circ = 2 * Math.PI * r;
  let offset = 0;
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img"
      aria-label={drawn.map((s) => `${s.label} ${Math.round((s.value / total) * 100)}%`).join(", ")}>
      <circle cx={c} cy={c} r={r} fill="none" stroke={T.raised} strokeWidth={thickness} />
      <g transform={`rotate(-90 ${c} ${c})`}>
        {drawn.map((s, i) => {
          const len = (s.value / total) * circ;
          const el = (
            <circle key={i} cx={c} cy={c} r={r} fill="none" stroke={s.color} strokeWidth={thickness}
              strokeDasharray={`${len} ${circ - len}`} strokeDashoffset={-offset}>
              <title>{`${s.label} · ${Math.round((s.value / total) * 100)}%`}</title>
            </circle>
          );
          offset += len;
          return el;
        })}
      </g>
      {center && (
        <text x={c} y={c - 1} textAnchor="middle" fill={T.ink}
          style={{ fontSize: Math.round(size * 0.145), fontWeight: 600 }}>{center}</text>
      )}
      {sub && (
        <text x={c} y={c + Math.round(size * 0.115)} textAnchor="middle" fill={T.inkDim}
          style={{ fontSize: Math.round(size * 0.082), letterSpacing: 0.4 }}>{sub}</text>
      )}
    </svg>
  );
}

/** The legend a `Donut` needs to be readable — swatch, label, value, share. Kept
 *  separate from the chart so a caller can place it beside or beneath. */
export function DonutKey({
  slices, total, fmt, minSlice = 0.02, restColor = T.inkFaint, onPick, picked,
}: {
  slices: Slice[]; total?: number; fmt?: (v: number) => string; minSlice?: number;
  restColor?: string; onPick?: (label: string | null) => void; picked?: string | null;
}) {
  const clean = slices.filter((s) => s.value > 0);
  const sum = total ?? clean.reduce((a, b) => a + b.value, 0);
  if (sum <= 0) return null;
  const big = clean.filter((s) => s.value / sum >= minSlice).sort((a, b) => b.value - a.value);
  const restV = sum - big.reduce((a, b) => a + b.value, 0);
  const rows = restV > 0
    ? [...big, { label: `${clean.length - big.length} others`, value: restV, color: restColor }]
    : big;
  const f = fmt ?? ((v: number) => v.toFixed(0));
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 1, minWidth: 0 }}>
      {rows.map((s) => (
        <div key={s.label} data-no-drag
          onClick={onPick ? () => onPick(picked === s.label ? null : s.label) : undefined}
          style={{
            display: "flex", alignItems: "center", gap: SPACE.sm, fontSize: FZ.base,
            cursor: onPick ? "pointer" : "default", borderRadius: RADIUS.sm,
            padding: "0 2px", background: picked === s.label ? T.card : "transparent",
          }}>
          <span style={{ width: 8, height: 8, borderRadius: 2, background: s.color, flex: "0 0 auto" }} />
          <span style={{ flex: 1, minWidth: 0, color: T.ink, overflow: "hidden",
            textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.label}</span>
          <span style={{ color: T.inkMid, fontVariantNumeric: "tabular-nums" }}>{f(s.value)}</span>
          <span style={{ width: 30, textAlign: "right", color: T.inkDim,
            fontVariantNumeric: "tabular-nums" }}>{((s.value / sum) * 100).toFixed(0)}%</span>
        </div>
      ))}
    </div>
  );
}

// ── Interaction ──────────────────────────────────────────────────────────────

/** A row of tabs. `tabs` is `[value, label]` pairs (label optional → value used). */
export function Tabs<V extends string>({
  tabs, active, onSelect, style,
}: { tabs: readonly (readonly [V, ReactNode])[]; active: V; onSelect: (v: V) => void; style?: CSSProperties }) {
  return (
    <div style={{ display: "flex", gap: 2, borderBottom: `1px solid ${T.line}`, ...style }}>
      {tabs.map(([v, label]) => {
        const on = v === active;
        return (
          <button
            key={v}
            data-no-drag
            onClick={() => onSelect(v)}
            style={{
              background: "transparent", border: "none", cursor: "pointer",
              padding: "5px 9px", fontSize: FZ.body, fontWeight: on ? 700 : 500,
              color: on ? T.gold : T.inkDim,
              borderBottom: `2px solid ${on ? T.gold : "transparent"}`, marginBottom: -1,
            }}
          >
            {label ?? v}
          </button>
        );
      })}
    </div>
  );
}

/** A selectable filter chip. `on` = currently-selected state. */
export function Chip({
  on, children, onClick, style,
}: { on?: boolean; children: ReactNode; onClick?: () => void; style?: CSSProperties }) {
  return (
    <span
      data-no-drag
      onClick={onClick}
      style={{
        fontSize: FZ.small, padding: "2px 8px", borderRadius: RADIUS.pill, cursor: "pointer",
        border: `1px solid ${on ? TONE.accent.line : T.line}`,
        background: on ? TONE.accent.fill : "transparent",
        color: on ? T.ink : T.inkDim, whiteSpace: "nowrap", ...style,
      }}
    >
      {children}
    </span>
  );
}

/** A button. `variant`: primary (accent), ghost (hairline), subtle (text-only). */
export function Button({
  variant = "ghost", children, onClick, disabled, title, style,
}: {
  variant?: "primary" | "ghost" | "subtle"; children: ReactNode;
  onClick?: () => void; disabled?: boolean; title?: string; style?: CSSProperties;
}) {
  const base: CSSProperties = {
    cursor: disabled ? "default" : "pointer", opacity: disabled ? 0.5 : 1,
    borderRadius: RADIUS.sm, padding: "4px 10px", fontSize: FZ.body, fontWeight: 600,
    fontFamily: "inherit",
  };
  const skins: Record<string, CSSProperties> = {
    primary: { background: T.accent, color: "#eaf3ff", border: `1px solid ${T.accent}` },
    ghost: { background: "transparent", color: T.ink, border: `1px solid ${T.line}` },
    subtle: { background: "transparent", color: T.inkMid, border: "none" },
  };
  return (
    <button data-no-drag onClick={disabled ? undefined : onClick} disabled={disabled} title={title} style={{ ...base, ...skins[variant], ...style }}>
      {children}
    </button>
  );
}

/** A muted "no data yet / do X first" note for empty states. */
export function EmptyNote({ children }: { children: ReactNode }) {
  return <div style={{ color: T.inkDim, fontSize: FZ.body, padding: SPACE.lg, lineHeight: 1.5 }}>{children}</div>;
}

/** A tiny footnote / legend line. */
export function FootNote({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <div style={{ color: T.inkFaint, fontSize: FZ.micro, marginTop: SPACE.xs, ...style }}>{children}</div>;
}
