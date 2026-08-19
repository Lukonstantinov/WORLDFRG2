import { useCallback, useEffect, useState } from "react";
import {
  listCampaigns, campaignLibraryDir, setCampaignLibraryDir,
  revealCampaignLibrary, saveCampaignToLibrary, deleteCampaignFile,
} from "@bridge";
import type { CampaignFileInfo } from "@types";

/** The CAMPAIGN LIBRARY — a real folder on the user's disk, listed in-app.
 *
 *  Resuming a run used to mean remembering which `.campaign` file was which and
 *  finding it in an OS file dialog. This shows the folder's contents with each
 *  save's YEAR, so a campaign is picked by what it is rather than by filename.
 *
 *  The listing reads only each file's small header (see `campaign_library.rs`), so a
 *  folder of long runs opens instantly. A save whose world does not match the world
 *  currently open is shown but marked — it can be loaded, it just will not line up. */

interface Props {
  onClose: () => void;
  /** Load a save. The parent owns the reload of settlements/economy/overlays. */
  onOpen: (path: string) => Promise<void>;
  /** True when there is a running campaign that can be written to the library. */
  canSave: boolean;
}

export function CampaignLibraryPanel({ onClose, onOpen, canSave }: Props) {
  const [dir, setDir] = useState("");
  const [rows, setRows] = useState<CampaignFileInfo[]>([]);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState("");

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      setDir(await campaignLibraryDir());
      setRows(await listCampaigns());
      setNote("");
    } catch (e) {
      setNote(String(e));
    }
    setBusy(false);
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  const handleChangeFolder = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({ directory: true, title: "Choose a campaigns folder" });
      if (!picked) return;
      await setCampaignLibraryDir(picked as string);
      await refresh();
    } catch (e) {
      setNote("Could not change the folder: " + e);
    }
  };

  const handleSave = async () => {
    setBusy(true);
    try {
      const path = await saveCampaignToLibrary();
      setNote("Saved to " + path);
      await refresh();
    } catch (e) {
      setNote("Save failed: " + e);
      setBusy(false);
    }
  };

  const handleDelete = async (row: CampaignFileInfo) => {
    if (!confirm(`Delete "${row.name}" (year ${row.year})?\n\n${row.file_name} is removed from disk. This cannot be undone.`)) return;
    setBusy(true);
    try {
      await deleteCampaignFile(row.path);
      await refresh();
    } catch (e) {
      setNote("Delete failed: " + e);
      setBusy(false);
    }
  };

  const handleOpen = async (row: CampaignFileInfo) => {
    setBusy(true);
    try {
      await onOpen(row.path);
      onClose();
    } catch (e) {
      setNote("Open failed: " + e);
      setBusy(false);
    }
  };

  return (
    <div style={backdrop} onClick={onClose}>
      <div onClick={(e) => e.stopPropagation()} style={shell}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <h2 style={{ margin: 0, color: "#e6d3a3", fontSize: 17, fontWeight: 600 }}>📚 Campaigns</h2>
          <span onClick={onClose} title="Close"
            style={{ color: "#7090b0", cursor: "pointer", fontSize: 20, lineHeight: 1 }}>×</span>
        </div>

        <div style={{ color: muted, fontSize: 11, margin: "6px 0 10px" }}>
          Every campaign save in your campaigns folder, newest first. This is an ordinary
          folder on your PC — open it, back it up, or copy saves in and out.
        </div>

        {/* The folder itself: shown in full, because the point is that the user can find it. */}
        <div style={folderBar}>
          <span style={{ color: "#8fb0cc", fontSize: 10, flexShrink: 0 }}>📁</span>
          <code style={{ flex: 1, fontSize: 10, color: "#9fc0dc", wordBreak: "break-all" }}>
            {dir || "…"}
          </code>
          <button style={miniBtn} onClick={() => void revealCampaignLibrary()} title="Open this folder in your file manager">Open folder</button>
          <button style={miniBtn} onClick={handleChangeFolder} title="Use a different folder">Change…</button>
        </div>

        <div style={{ display: "flex", gap: 6, margin: "10px 0" }}>
          <button style={{ ...primaryBtn, opacity: canSave && !busy ? 1 : 0.45, cursor: canSave && !busy ? "pointer" : "not-allowed" }}
            disabled={!canSave || busy} onClick={handleSave}
            title={canSave ? "Write the running campaign into this folder" : "Start a campaign first"}>
            💾 Save current campaign here
          </button>
          <button style={miniBtn} onClick={() => void refresh()} disabled={busy}>↻ Refresh</button>
        </div>

        {note && <div style={noteBox}>{note}</div>}

        {rows.length === 0 && !busy && (
          <div style={{ ...noteBox, color: muted }}>
            No campaign saves in this folder yet. Start a campaign, then use
            “Save current campaign here”.
          </div>
        )}

        <div style={{ display: "flex", flexDirection: "column", gap: 6, marginTop: 6 }}>
          {rows.map((r) => (
            <div key={r.path} style={{
              ...card,
              // A save for a different world is legible but visibly set apart — it can
              // still be opened, it just will not line up with the map on screen.
              borderColor: r.world_match ? "#2c4a68" : "#4a3a22",
            }}>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                  <span style={{ color: "#e6d3a3", fontWeight: 600, fontSize: 13 }}>{r.name}</span>
                  <span style={{ color: "#7fa8cc", fontSize: 12 }}>
                    {r.tick > 0 || r.has_header ? `Year ${r.year}` : "not yet begun"}
                    {!r.has_header && r.tick > 0 ? " ≈" : ""}
                  </span>
                </div>
                <div style={{ color: muted, fontSize: 10, marginTop: 2 }}>
                  {r.world_name || "unknown world"}
                  {r.hubs > 0 && ` · ${r.hubs} cities`}
                  {r.houses > 0 && ` · ${r.houses} houses`}
                  {r.saved_at > 0 && ` · saved ${fmtDate(r.saved_at)}`}
                  {r.size_bytes > 0 && ` · ${fmtSize(r.size_bytes)}`}
                </div>
                {!r.world_match && (
                  <div style={{ color: "#c9a227", fontSize: 10, marginTop: 3 }}>
                    ⚠ Saved against a different world — open that world first, or the map
                    will not match.
                  </div>
                )}
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <button style={primaryBtn} disabled={busy} onClick={() => void handleOpen(r)}>Open</button>
                <button style={dangerBtn} disabled={busy} onClick={() => void handleDelete(r)}>Delete</button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function fmtDate(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) +
    " " + d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

function fmtSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const muted = "#6b8299";

const backdrop: React.CSSProperties = {
  position: "absolute", inset: 0, display: "flex",
  alignItems: "center", justifyContent: "center",
  background: "rgba(0,0,0,0.75)", zIndex: 100,
};

const shell: React.CSSProperties = {
  background: "#111820", border: "1px solid #2a3a4e", borderRadius: 10,
  padding: "20px 22px", width: 560, maxHeight: "84%", overflowY: "auto",
  boxShadow: "0 12px 40px rgba(0,0,0,0.5)", color: "#cfe2f6",
};

const folderBar: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: 8,
  background: "#0b1119", border: "1px solid #1a2838", borderRadius: 7, padding: "7px 9px",
};

const card: React.CSSProperties = {
  display: "flex", gap: 10, alignItems: "center",
  background: "#0d1520", border: "1px solid #2c4a68", borderRadius: 8, padding: "9px 11px",
};

const miniBtn: React.CSSProperties = {
  padding: "4px 9px", borderRadius: 6, fontSize: 11, cursor: "pointer",
  border: "1px solid #23405c", background: "#0f1a26", color: "#9fc0dc",
};

const primaryBtn: React.CSSProperties = {
  padding: "5px 12px", borderRadius: 6, fontSize: 11.5, cursor: "pointer", fontWeight: 600,
  border: "1px solid #3d6a45", background: "#16301d", color: "#bfe6c6",
};

const dangerBtn: React.CSSProperties = {
  padding: "4px 12px", borderRadius: 6, fontSize: 11, cursor: "pointer",
  border: "1px solid #5a2f2f", background: "#241214", color: "#d9a2a2",
};

const noteBox: React.CSSProperties = {
  background: "#0b1119", border: "1px solid #1a2838", borderRadius: 7,
  padding: "7px 9px", fontSize: 11, color: "#9fc0dc", marginBottom: 6,
};
