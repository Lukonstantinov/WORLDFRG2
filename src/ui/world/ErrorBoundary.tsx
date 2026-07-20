import { Component, type ErrorInfo, type ReactNode } from "react";

/** Catches render/runtime throws in a subtree so ONE broken panel can't blank
 *  the whole app to a black screen. Instead of unmounting everything, it shows a
 *  small dismissible error card with the actual message + stack (also logged to
 *  the console) so the fault is legible and the rest of the app keeps running.
 *
 *  `label` names the guarded region; `onReset` (optional) runs when the user
 *  dismisses so a parent can clear the state that triggered the throw. */
export class ErrorBoundary extends Component<
  { label: string; children: ReactNode; onReset?: () => void },
  { error: Error | null; info: string }
> {
  state = { error: null as Error | null, info: "" };

  static getDerivedStateFromError(error: Error) {
    return { error, info: "" };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Surface it — the DevTools console is where the real stack lives.
    console.error(`[${this.props.label}] render error:`, error, info.componentStack);
    this.setState({ info: info.componentStack ?? "" });
  }

  dismiss = () => {
    this.setState({ error: null, info: "" });
    this.props.onReset?.();
  };

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <div style={card} role="alert">
        <div style={{ display: "flex", alignItems: "baseline", gap: 8, marginBottom: 6 }}>
          <span style={{ fontSize: 15 }}>⚠️</span>
          <span style={{ color: "#f0c674", fontWeight: 700, fontSize: 13 }}>
            {this.props.label} hit an error
          </span>
          <span style={{ flex: 1 }} />
          <button onClick={this.dismiss} style={btn}>Dismiss</button>
        </div>
        <div style={{ color: "#e6b0b0", fontSize: 12, fontFamily: "monospace", wordBreak: "break-word" }}>
          {error.message || String(error)}
        </div>
        {this.state.info && (
          <pre style={pre}>{this.state.info.split("\n").slice(0, 6).join("\n")}</pre>
        )}
      </div>
    );
  }
}

const card: React.CSSProperties = {
  position: "absolute", top: 70, left: "50%", transform: "translateX(-50%)",
  zIndex: 400, maxWidth: 520, width: "min(520px, 90%)",
  background: "#1a1012", border: "1px solid #7a3a3a", borderRadius: 8,
  padding: "10px 14px", boxShadow: "0 12px 34px rgba(0,0,0,0.6)",
};
const btn: React.CSSProperties = {
  padding: "2px 10px", borderRadius: 5, fontSize: 11, cursor: "pointer",
  border: "1px solid #7a3a3a", background: "transparent", color: "#f0c674",
};
const pre: React.CSSProperties = {
  color: "#9a8a8a", fontSize: 10, marginTop: 6, maxHeight: 120, overflow: "auto",
  whiteSpace: "pre-wrap", lineHeight: 1.35,
};
