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
