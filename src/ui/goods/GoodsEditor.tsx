import { useEffect, useRef, useState } from "react";
import { useGoodsStore } from "@state/goodsStore";
import { useUIStore } from "@state/uiStore";
import { useWorldStore } from "@state/worldStore";
import { previewGoodScore, importGoodsTxt } from "@bridge";
import type { GoodSpec, GoodDomain, GoodDistribution, GoodEnvelope, RecipeInput, GoodDna, GoodRole, GoodOrigin, GoodKeeping } from "@types";
import { GoodIcon } from "./GoodIcon";
import { PixelIconEditor } from "./PixelIconEditor";
import { WorldBalance } from "./WorldBalance";

const DOMAINS: GoodDomain[] = ["marine", "coastal", "continental", "island"];
const CATEGORIES = ["cereal", "protein", "oil", "fiber", "drink", "sweetener", "preservative", "metal", "bullion", "gem", "construction", "dye", "aromatic", "craft", "prestige", "livestock", "manufacture", "misc"] as const;
const DISTRIBUTIONS: GoodDistribution[] = ["global", "local", "endemic", "deposits", "manufactured"];
/** G4 · the Reach slider: one honest scale over distribution + reach_km. */
const REACHES: { label: string; hint: string; distribution: GoodDistribution; reach_km: number | null }[] = [
  { label: "Endemic", hint: "only its one homeland landmass; cannot be grown elsewhere", distribution: "endemic", reach_km: null },
  { label: "Local", hint: "the homeland and about 300 km of similar land", distribution: "local", reach_km: 300 },
  { label: "Regional", hint: "the homeland's region, about 1,200 km", distribution: "local", reach_km: 1200 },
  { label: "Continental", hint: "every connected suitable cell from the homeland", distribution: "local", reach_km: null },
  { label: "Global", hint: "every suitable cell anywhere; no homeland", distribution: "global", reach_km: null },
];
function reachIndex(g: GoodSpec): number {
  if (g.distribution === "endemic") return 0;
  if (g.distribution === "global") return 4;
  const r = g.reach_km ?? null;
  return r === null ? 3 : r <= 600 ? 1 : 2;
}
const ROLES: GoodRole[] = ["staple", "essential", "strategic", "comfort", "luxury", "treasure"];
const ORIGINS: GoodOrigin[] = ["grown", "raised", "gathered", "caught", "mined", "crafted"];
const KEEPINGS: GoodKeeping[] = ["perishable", "seasonal", "storable", "eternal"];
const TIERS = ["Bulk ×1", "Common ×3-10", "Valuable ×30-100", "Precious ×300-1,000", "Treasure ×2,000+"];
/** Shown while a spec has no `dna` yet (the backend fills it on load). */
const DNA_FALLBACK: GoodDna = { role: "comfort", origin: "grown", tier: 1, difficulty: 2, keeping: "storable" };

/** Builder presets (plan §4.5): which goods are switched on. Presets only toggle
 *  `enabled`; nothing is deleted, so switching back is always possible. */
const PRESET_MINIMAL = ["wheat", "stockfish", "salt", "wool_fleece", "timber", "iron", "wine", "pepper", "silver", "gold"];
const PRESET_CLASSIC = [...PRESET_MINIMAL, "barley", "rice", "herring", "bay_salt", "flax", "hides", "copper", "tin", "lead",
  "horses", "oliveoil", "honey", "beer", "furs", "amber", "cinnamon", "silk", "incense", "pearls", "ruby", "marble",
  "cloth", "linen", "leather_goods", "metalware", "armour", "ceramics", "jewelry"];
/** Shipped disabled as duplicates or out of period (`default_list`). */
const RETIRED = ["tobacco", "frankincense", "indigo", "gemstones", "dyes"];
const ROSTER_KEY = "wf2.goodsRoster";
const PRESETS: { label: string; hint: string; ids: (all: GoodSpec[]) => Set<string> }[] = [
  { label: "Minimal", hint: "about ten goods: one per essential need, a wine, a spice, silver and gold", ids: () => new Set(PRESET_MINIMAL) },
  { label: "Classic", hint: "about forty goods of a medieval world", ids: () => new Set(PRESET_CLASSIC) },
  { label: "Spice Age", hint: "every shipped good (the default)", ids: (all) => new Set(all.map((g) => g.id).filter((id) => !RETIRED.includes(id))) },
];
const ROLE_LABEL: Record<GoodRole, string> = {
  staple: "Staples — eaten every day", essential: "Essentials — salt, fibre, fuel, building",
  strategic: "Strategic — tools, war and ships", comfort: "Comforts — bought when there is money",
  luxury: "Luxuries — status and pleasure, carried far", treasure: "Treasure — hoarded, minted, given",
};
/** The six essential needs (plan §4.5): a world lacking one cannot grow cities. */
const NEEDS: { label: string; ok: (g: GoodSpec) => boolean }[] = [
  { label: "Staple grain", ok: (g) => g.category === "cereal" },
  { label: "Protein", ok: (g) => g.category === "protein" },
  { label: "Preservative", ok: (g) => g.category === "preservative" },
  { label: "Fibre", ok: (g) => g.category === "fiber" && (g.dna?.role === "essential" || g.dna?.role === "staple") },
  { label: "Fuel & building", ok: (g) => g.category === "construction" || g.category === "fuel" },
  { label: "Tool metal", ok: (g) => g.category === "metal" && g.distribution !== "manufactured" },
];
const STARS = (n: number) => "★".repeat(n) + "☆".repeat(Math.max(0, 5 - n));
const TIER_SHORT = ["Bulk", "Common", "Valuable", "Precious", "Treasure"];

// Climate presets → Köppen zone codes (see sim/koppen.rs).
const CLIMATE_PRESETS: { label: string; codes: number[] }[] = [
  { label: "Tropical", codes: [1, 2, 3, 23, 24] },
  { label: "Arid", codes: [4, 5, 6, 7] },
  { label: "Mediterranean", codes: [8, 9, 10] },
  { label: "Temperate", codes: [11, 12, 13, 25] },
  { label: "Continental", codes: [14, 15, 16, 17, 27, 28, 29] },
  { label: "Polar", codes: [21, 22] },
];

const panel: React.CSSProperties = {
  position: "fixed", inset: 0, background: "rgba(0,0,0,0.55)", zIndex: 1000,
  display: "flex", alignItems: "center", justifyContent: "center",
};
const sheet: React.CSSProperties = {
  width: "min(880px, 94vw)", maxHeight: "88vh", overflow: "auto",
  background: "#201d18", border: "1px solid #3f392f", borderRadius: 8,
  padding: 14, color: "#e2dbd0", fontSize: 12,
};
const btn: React.CSSProperties = {
  background: "#352f25", color: "#e2dbd0", border: "1px solid #564c3b",
  borderRadius: 4, padding: "4px 9px", cursor: "pointer", fontSize: 11,
};
const primaryBtn: React.CSSProperties = { ...btn, background: "#8d7448", borderColor: "#a88a57", color: "#fff" };
const row: React.CSSProperties = {
  display: "grid", gridTemplateColumns: "20px 22px 1.5fr 0.8fr 0.8fr 0.9fr 0.8fr 56px",
  gap: 6, alignItems: "center", padding: "4px 0", borderBottom: "1px solid #342e25",
};
const input: React.CSSProperties = {
  background: "#17140f", color: "#e2dbd0", border: "1px solid #3f392f", borderRadius: 3,
  padding: "2px 4px", fontSize: 11, width: "100%",
};

function num(v: number) { return Math.round(v * 100) / 100; }

export function GoodsEditor() {
  const {
    specs, editorOpen, dirty, setOpen, update, addCustom, duplicate, remove,
    applyToWorld, saveToLibrary, loadFromLibrary, resetDefaults, loadFromWorld,
  } = useGoodsStore();
  const [expanded, setExpanded] = useState<number | null>(null);
  const [status, setStatus] = useState("");
  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<"goods" | "balance">("goods");

  useEffect(() => { if (editorOpen && specs.length === 0) void loadFromWorld(); }, [editorOpen, specs.length, loadFromWorld]);
  // Load the world's goods on open too, so author-drawn icons show everywhere.
  const worldName = useWorldStore((s) => s.meta?.name ?? null);
  useEffect(() => { if (worldName) void loadFromWorld(); }, [worldName, loadFromWorld]);

  if (!editorOpen) return null;

  const act = (label: string, fn: () => Promise<void>) => async () => {
    try { await fn(); setStatus(label); } catch (e) { setStatus(`Error: ${e}`); }
  };

  // Import a .txt goods file (docs/DEPOSITS_AND_MINING_PLAN.md slice 3): pick a
  // path via the OS dialog, import ADD-ONLY into the global library, then
  // reload the editor from that library so new goods show up immediately.
  const handleImportTxt = async () => {
    try {
      let path: string | null = null;
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const result = await open({ filters: [{ name: "Goods", extensions: ["txt"] }], title: "Import goods .txt" });
        if (result) path = result as string;
      } catch {
        const input = prompt("Enter path to a goods .txt file:");
        if (input) path = input;
      }
      if (!path) return;
      const report = await importGoodsTxt(path);
      await loadFromLibrary();
      const parts = [`${report.added.length} added`];
      if (report.rejected.length) parts.push(`${report.rejected.length} rejected`);
      if (report.defaulted.length) parts.push(`${report.defaulted.length} field(s) defaulted`);
      setStatus(`Import: ${parts.join(", ")}${report.rejected.length ? " — " + report.rejected.map(([id, why]) => `${id}: ${why}`).join("; ") : ""}`);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  };

  const enabledCount = specs.filter((s) => s.enabled).length;
  const applyIds = (ids: Set<string>) => specs.forEach((g, i) => {
    if (g.enabled !== ids.has(g.id)) update(i, { enabled: ids.has(g.id) });
  });
  const saveRoster = () => {
    try { localStorage.setItem(ROSTER_KEY, JSON.stringify(specs.filter((g) => g.enabled).map((g) => g.id))); setStatus("Saved as My roster"); }
    catch { setStatus("Could not save the roster"); }
  };
  const loadRoster = () => {
    try {
      const ids = JSON.parse(localStorage.getItem(ROSTER_KEY) ?? "null") as string[] | null;
      if (ids) { applyIds(new Set(ids)); setStatus("Applied My roster"); } else setStatus("No roster saved yet");
    } catch { setStatus("Could not read the roster"); }
  };
  const on = specs.filter((g) => g.enabled);
  const q = query.trim().toLowerCase();
  const groups = ROLES.map((role) => ({
    role,
    rows: specs.map((g, i) => ({ g, i }))
      .filter(({ g }) => (g.dna?.role ?? "comfort") === role)
      .filter(({ g }) => !q || g.name.toLowerCase().includes(q) || g.id.includes(q))
      .sort((a, b) => a.g.name.localeCompare(b.g.name)),
  })).filter((grp) => grp.rows.length > 0);

  return (
    <div style={panel} onClick={() => setOpen(false)}>
      <div style={sheet} onClick={(e) => e.stopPropagation()}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <span style={{ display: "flex", gap: 10, alignItems: "center" }}>
            <strong style={{ fontSize: 14 }}>Goods Builder</strong>
            {(["goods", "balance"] as const).map((t) => (
              <button key={t} style={{ ...btn, background: tab === t ? "#5a4a30" : btn.background }} onClick={() => setTab(t)}>
                {t === "goods" ? "Goods" : "World balance"}
              </button>
            ))}
          </span>
          <span style={{ color: "#9d917d" }}>
            {enabledCount} enabled · {specs.length} total{dirty ? " · unsaved" : ""}
          </span>
        </div>

        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
          <button style={btn} onClick={addCustom}>+ Add custom</button>
          <button style={btn} onClick={act("Loaded global library", loadFromLibrary)}>Load library</button>
          <button style={btn} title="Add-only: import an INI-ish .txt of new goods (never replaces an existing id)" onClick={handleImportTxt}>Import .txt</button>
          <button style={btn} onClick={act("Reset to defaults", resetDefaults)}>Reset to defaults</button>
          <button style={btn} title="Review planted vs manufactured goods + the recipe schematic" onClick={() => useUIStore.getState().openChainReview()}>⚙ Review chains</button>
          <div style={{ flex: 1 }} />
          <button style={btn} onClick={act("Saved to global library", saveToLibrary)}>Save to library</button>
          <button style={primaryBtn} onClick={act("Applied to this world", applyToWorld)}>Apply to world</button>
          <button style={btn} onClick={() => setOpen(false)}>Close</button>
        </div>

        {tab === "balance" ? <WorldBalance specs={specs} /> : (<>
        <div style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap", marginBottom: 8 }}>
          <span style={{ color: "#b5a68d" }}>Preset</span>
          {PRESETS.map((pr) => (
            <button key={pr.label} style={btn} title={pr.hint}
              onClick={() => { applyIds(pr.ids(specs)); setStatus(`Applied ${pr.label}`); }}>{pr.label}</button>
          ))}
          <span style={{ color: "#564c3b" }}>|</span>
          <button style={btn} title="Switch on exactly the goods you saved" onClick={loadRoster}>My roster</button>
          <button style={btn} title="Remember the goods switched on now as My roster" onClick={saveRoster}>Save as my roster</button>
          <div style={{ flex: 1 }} />
          <input style={{ ...input, width: 160 }} placeholder="Find a good…" value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>

        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 10 }}
          title="A world needs at least one good for each of these, or its cities cannot grow. Checked for the whole world; per region comes with the world-balance view.">
          {NEEDS.map((n) => {
            const have = on.filter(n.ok);
            return (
              <span key={n.label} style={{
                padding: "2px 7px", borderRadius: 10, fontSize: 11,
                background: have.length ? "#22311f" : "#3a2a17", border: `1px solid ${have.length ? "#4c6a3f" : "#8a6a30"}`,
                color: have.length ? "#bfe0a8" : "#f0c070",
              }} title={have.length ? have.map((g) => g.name).join(", ") : "nothing switched on covers this need"}>
                {have.length ? "✓" : "⚠"} {n.label}{have.length ? ` · ${have.length}` : ""}
              </span>
            );
          })}
        </div>

        <div style={{ ...row, fontWeight: 600, color: "#b5a68d", borderBottom: "1px solid #564c3b" }}>
          <span>On</span><span></span><span>Name</span><span>Value</span><span>Difficulty</span>
          <span>Reach</span><span>Origin</span><span></span>
        </div>

        {groups.map((grp) => (
          <div key={grp.role}>
            <div style={{ marginTop: 10, padding: "3px 0", color: "#d8c49a", fontWeight: 600, borderBottom: "1px solid #4a4031" }}>
              {ROLE_LABEL[grp.role]} <span style={{ color: "#8a7d66", fontWeight: 400 }}>· {grp.rows.filter(({ g }) => g.enabled).length}/{grp.rows.length} on</span>
            </div>
            {grp.rows.map(({ g, i }) => (
              <GoodRow
                key={`${g.id}_${i}`} g={g} i={i} expanded={expanded === i}
                onToggleExpand={() => setExpanded(expanded === i ? null : i)}
                update={update} duplicate={duplicate} remove={remove} allSpecs={specs}
              />
            ))}
          </div>
        ))}
        </>)}

        <div style={{ marginTop: 8, color: "#9d917d", minHeight: 16 }}>{status}</div>
        <div style={{ marginTop: 4, color: "#7b715f", fontSize: 10 }}>
          Apply to world, then re-run the Resources stage to place goods with these settings.
          Built-in goods keep their original scoring unless duplicated into a custom good.
        </div>
      </div>
    </div>
  );
}

function GoodRow({ g, i, expanded, onToggleExpand, update, duplicate, remove, allSpecs }: {
  g: GoodSpec; i: number; expanded: boolean; onToggleExpand: () => void;
  update: (i: number, patch: Partial<GoodSpec>) => void;
  duplicate: (i: number) => void; remove: (i: number) => void; allSpecs: GoodSpec[];
}) {
  const manufactured = g.distribution === "manufactured";
  const [drawIcon, setDrawIcon] = useState(false);
  return (
    <>
      <div style={row}>
        <input type="checkbox" checked={g.enabled} onChange={(e) => update(i, { enabled: e.target.checked })} />
        <GoodIcon name={g.id} color={g.color} size={20} title={g.id} />
        <input style={input} value={g.name} onChange={(e) => update(i, { name: e.target.value })} />
        <span style={{ color: "#c8b48a" }} title={TIERS[g.dna?.tier ?? 1]}>{TIER_SHORT[g.dna?.tier ?? 1]}</span>
        <span style={{ color: "#e0b050", letterSpacing: -1 }} title={`difficulty ${g.dna?.difficulty ?? 2} of 5`}>{STARS(g.dna?.difficulty ?? 2)}</span>
        <span style={{ color: "#9db0c0" }}>{manufactured ? "made in cities" : g.distribution === "deposits" ? "deposits" : REACHES[reachIndex(g)].label}{g.homeland?.length ? " 📍" : ""}</span>
        <span style={{ color: "#a99c84" }}>{g.dna?.origin ?? "—"}</span>
        <div style={{ display: "flex", gap: 3 }}>
          <button style={{ ...btn, padding: "2px 5px" }} title="Icon / color / envelope" onClick={onToggleExpand}>⋯</button>
          {!g.builtin && <button style={{ ...btn, padding: "2px 5px", color: "#e08080" }} title="Delete" onClick={() => remove(i)}>✕</button>}
        </div>
      </div>
      {expanded && (
        <div style={{ padding: "8px 6px 12px 28px", borderBottom: "1px solid #342e25", display: "flex", flexWrap: "wrap", gap: 14 }}>
          <button style={btn} title="Draw this good's icon, start from another, or upload an image" onClick={() => setDrawIcon(!drawIcon)}>
            {drawIcon ? "Close icon editor" : "✎ Draw icon"}
          </button>
          {drawIcon && <PixelIconEditor id={g.id} color={g.color} pixels={g.pixels} onChange={(pixels) => update(i, { pixels })} />}
          <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
            Color <input type="color" value={g.color} onChange={(e) => update(i, { color: e.target.value })} />
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="Where it can grow at all (distribution, below, says how far it spreads)">
            Domain <select style={input} value={g.domain} onChange={(e) => update(i, { domain: e.target.value as GoodDomain })}>
              {DOMAINS.map((d) => <option key={d} value={d}>{d}</option>)}
            </select>
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
            Distribution <select style={input} value={g.distribution} onChange={(e) => update(i, { distribution: e.target.value as GoodDistribution })}>
              {DISTRIBUTIONS.map((d) => <option key={d} value={d}>{d}</option>)}
            </select>
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center", width: 170 }} title="Higher = a smaller, harder-to-seed belt">
            Rarity <Slider value={g.rarity} onChange={(v) => update(i, { rarity: v })} />
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center", width: 170 }} title="How much people want it">
            Desire <Slider value={g.desire} onChange={(v) => update(i, { desire: v })} />
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }}>
            <input type="checkbox" checked={g.network_luxury} onChange={(e) => update(i, { network_luxury: e.target.checked })} /> network luxury
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="Goods in the same category substitute for each other in city markets">
            Category <select style={input} value={g.category} onChange={(e) => update(i, { category: e.target.value })}>
              {CATEGORIES.map((c) => <option key={c} value={c}>{c}</option>)}
            </select>
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="0 = basic need (filled first), 1 = comfort, 2 = luxury">
            Need <select style={input} value={g.need_tier} onChange={(e) => update(i, { need_tier: Number(e.target.value) })}>
              <option value={0}>basic</option><option value={1}>comfort</option><option value={2}>luxury</option>
            </select>
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="World-standard value in grain-equivalent (wheat = 1)">
            Value <input type="number" min={0.1} step={0.1} style={{ ...input, width: 56 }} value={g.base_value}
              onChange={(e) => update(i, { base_value: Number(e.target.value) || 1 })} /> ×grain
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="Freight weight/volume — 1 = silk-light; 3-4 = bulky (timber, grain, ore) so it stays regional">
            Bulk <input type="number" min={0.1} step={0.1} style={{ ...input, width: 50 }} value={g.bulk ?? 1}
              onChange={(e) => update(i, { bulk: Number(e.target.value) || 1 })} />
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="Spoilage: extra freight per travel-day. 0 = durable; high for fresh fish/fruit (can't travel far)">
            Perish <input type="number" min={0} step={0.01} style={{ ...input, width: 50 }} value={g.perishable ?? 0}
              onChange={(e) => update(i, { perishable: Math.max(0, Number(e.target.value) || 0) })} />
          </label>
          <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="Demand cadence (days between a person consuming a unit). Food ~7, comfort ~45, durables/luxuries ~180. Long cadence = weak local pull → mostly sold wholesale to merchants">
            Cadence <input type="number" min={1} step={1} style={{ ...input, width: 50 }} value={g.consumption_interval ?? 30}
              onChange={(e) => update(i, { consumption_interval: Math.max(1, Number(e.target.value) || 30) })} /> d
          </label>
          <DnaEditor dna={g.dna ?? DNA_FALLBACK} onChange={(dna) => update(i, { dna })} />
          <button style={btn} onClick={() => duplicate(i)}>Duplicate as custom</button>
          {manufactured
            ? <RecipeEditor i={i} g={g} allSpecs={allSpecs} update={update} />
            : g.builtin
              ? <span style={{ color: "#9d917d", flexBasis: "100%" }}>Built-in scoring (duplicate to customize its climate/temperature envelope).</span>
              : <EnvelopeEditor i={i} env={g.scoring ?? null} update={update} />}
          {!manufactured && g.distribution !== "deposits" && <ReachEditor g={g} onChange={(patch) => update(i, patch)} />}
          {!manufactured && <PreviewCanvas spec={g} onPin={g.distribution === "global" || g.distribution === "deposits" ? undefined
            : (cell) => update(i, { homeland: cell ? [cell] : [] })} />}
        </div>
      )}
    </>
  );
}

/** The six goods properties (plan §4.1). Geography is the Distribution column. */
function DnaEditor({ dna, onChange }: { dna: GoodDna; onChange: (d: GoodDna) => void }) {
  const set = (patch: Partial<GoodDna>) => onChange({ ...dna, ...patch });
  const pick = <T extends string>(label: string, title: string, value: T, opts: readonly T[], f: (v: T) => void) => (
    <label style={{ display: "flex", gap: 4, alignItems: "center" }} title={title}>
      {label} <select style={input} value={value} onChange={(e) => f(e.target.value as T)}>
        {opts.map((o) => <option key={o} value={o}>{o}</option>)}
      </select>
    </label>
  );
  return (
    <div style={{ flexBasis: "100%", display: "flex", flexWrap: "wrap", gap: 12, padding: "6px 0", borderTop: "1px dashed #3f392f" }}>
      {pick("Role", "Why people want it: staples are eaten daily, treasure is hoarded and minted", dna.role, ROLES, (role) => set({ role }))}
      {pick("Origin", "How it is obtained; each origin has its own way to fail", dna.origin, ORIGINS, (origin) => set({ origin }))}
      <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="Value per weight, as a multiple of grain">
        Value tier <select style={input} value={dna.tier} onChange={(e) => set({ tier: Number(e.target.value) })}>
          {TIERS.map((t, k) => <option key={k} value={k}>{t}</option>)}
        </select>
      </label>
      <label style={{ display: "flex", gap: 4, alignItems: "center" }} title="1 = anyone can; 5 = specialists, capital and years">
        Difficulty <span style={{ display: "inline-flex", gap: 1 }}>
          {[1, 2, 3, 4, 5].map((k) => (
            <button key={k} onClick={() => set({ difficulty: k })} title={`${k} of 5`}
              style={{ background: "none", border: "none", padding: 0, cursor: "pointer", fontSize: 13, color: k <= dna.difficulty ? "#e0b050" : "#4a4336" }}>★</button>
          ))}
        </span>
      </label>
      {pick("Keeping", "How long it keeps; only eternal goods can serve as money", dna.keeping, KEEPINGS, (keeping) => set({ keeping }))}
    </div>
  );
}

/** Recipe rows for a Manufactured good: inputs (good × qty) + a labor slider.
 *  Highlights inputs that reference an unknown/disabled good or itself (a cycle). */
function RecipeEditor({ i, g, allSpecs, update }: {
  i: number; g: GoodSpec; allSpecs: GoodSpec[];
  update: (i: number, patch: Partial<GoodSpec>) => void;
}) {
  const inputs: RecipeInput[] = g.inputs ?? [];
  const setInputs = (next: RecipeInput[]) => update(i, { inputs: next });
  const byId = new Map(allSpecs.map((s) => [s.id, s]));
  const options = allSpecs.filter((s) => s.id !== g.id && s.enabled);
  const problem = (inp: RecipeInput): string | null => {
    if (inp.good === g.id) return "self-reference (cycle)";
    const s = byId.get(inp.good);
    if (!s) return "unknown good";
    if (!s.enabled) return "disabled good";
    return null;
  };
  const summary = inputs.length === 0
    ? "no recipe — add inputs below"
    : "needs " + inputs.map((inp) => `${num(inp.qty)}× ${byId.get(inp.good)?.name ?? inp.good}`).join(" + ");

  return (
    <div style={{ flexBasis: "100%", display: "flex", flexDirection: "column", gap: 6 }}>
      <div style={{ color: "#b5a68d" }}>
        Recipe — made in cities from these raws (no map belt). <span style={{ color: "#9d917d" }}>{summary}</span>
      </div>
      {inputs.map((inp, k) => {
        const prob = problem(inp);
        return (
          <div key={k} style={{ display: "flex", gap: 6, alignItems: "center" }}>
            <select style={{ ...input, width: 180, borderColor: prob ? "#c06060" : "#3f392f" }} value={inp.good}
              onChange={(e) => setInputs(inputs.map((x, j) => j === k ? { ...x, good: e.target.value } : x))}>
              {!byId.has(inp.good) && <option value={inp.good}>{inp.good} (?)</option>}
              {options.map((s) => <option key={s.id} value={s.id}>{s.icon} {s.name}</option>)}
            </select>
            <span style={{ color: "#9d917d" }}>×</span>
            <input type="number" min={0} step={0.1} style={{ ...input, width: 56 }} value={inp.qty}
              onChange={(e) => setInputs(inputs.map((x, j) => j === k ? { ...x, qty: Math.max(0, Number(e.target.value) || 0) } : x))} />
            {prob && <span style={{ color: "#e08080", fontSize: 10 }}>{prob}</span>}
            <button style={{ ...btn, padding: "2px 6px", color: "#e08080" }} onClick={() => setInputs(inputs.filter((_, j) => j !== k))}>✕</button>
          </div>
        );
      })}
      <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
        <button style={btn} onClick={() => setInputs([...inputs, { good: options[0]?.id ?? "", qty: 1 }])}>+ Add input</button>
        <label style={{ display: "flex", gap: 6, alignItems: "center", color: "#b5a68d" }} title="Output rate ∝ city population × this. Higher = a major manufacturing centre.">
          Labor {num(g.labor ?? 1)}
          <input type="range" min={0.2} max={3} step={0.1} value={g.labor ?? 1}
            onChange={(e) => update(i, { labor: parseFloat(e.target.value) })} />
        </label>
      </div>
    </div>
  );
}

function EnvelopeEditor({ i, env, update }: {
  i: number; env: GoodEnvelope | null;
  update: (i: number, patch: Partial<GoodSpec>) => void;
}) {
  const e: GoodEnvelope = env ?? { climate: [], fertility: 0.4, coast_bonus: 0 };
  const setEnv = (patch: Partial<GoodEnvelope>) => update(i, { scoring: { ...e, ...patch } });
  const activeCodes = new Set(e.climate.map(([c]) => c));
  const presetOn = (codes: number[]) => codes.every((c) => activeCodes.has(c));
  const togglePreset = (codes: number[]) => {
    const on = presetOn(codes);
    const next = new Set(activeCodes);
    for (const c of codes) { if (on) next.delete(c); else next.add(c); }
    setEnv({ climate: [...next].map((c) => [c, 1.0] as [number, number]) });
  };

  return (
    <div style={{ flexBasis: "100%", display: "flex", flexDirection: "column", gap: 6 }}>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        <span style={{ color: "#b5a68d", alignSelf: "center" }}>Climate:</span>
        {CLIMATE_PRESETS.map((p) => (
          <button key={p.label}
            style={{ ...btn, background: presetOn(p.codes) ? "#8d7448" : "#352f25", padding: "2px 7px" }}
            onClick={() => togglePreset(p.codes)}>{p.label}</button>
        ))}
        <span style={{ color: "#7b715f", alignSelf: "center" }}>(none = any climate)</span>
      </div>
      <Range label="Temp °C" lo={e.temp?.[0] ?? 18} width={e.temp?.[1] ?? 8}
        onLo={(v) => setEnv({ temp: [v, e.temp?.[1] ?? 8] })} onWidth={(v) => setEnv({ temp: [e.temp?.[0] ?? 18, v] })}
        loMin={-20} loMax={35} wMin={2} wMax={25} />
      <BandRow label="Precip mm" band={e.precip ?? [400, 1600, 500]} onChange={(b) => setEnv({ precip: b })} max={3000} step={50} />
      <BandRow label="Elevation" band={e.elevation ?? [0, 0.5, 0.25]} onChange={(b) => setEnv({ elevation: b })} max={1} step={0.02} />
      <div style={{ display: "flex", gap: 16 }}>
        <Labeled label={`Fertility ${num(e.fertility)}`}>
          <input type="range" min={0} max={1} step={0.05} value={e.fertility} onChange={(ev) => setEnv({ fertility: parseFloat(ev.target.value) })} />
        </Labeled>
        <Labeled label={`Coast bonus ${num(e.coast_bonus)}`}>
          <input type="range" min={0} max={1} step={0.05} value={e.coast_bonus} onChange={(ev) => setEnv({ coast_bonus: parseFloat(ev.target.value) })} />
        </Labeled>
      </div>
    </div>
  );
}

function Slider({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
      <input type="range" min={0} max={1} step={0.05} value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))} style={{ width: "100%" }} />
      <span style={{ color: "#b5a68d", width: 24, fontSize: 10 }}>{num(value)}</span>
    </div>
  );
}

function Labeled({ label, children }: { label: string; children: React.ReactNode }) {
  return <label style={{ display: "flex", flexDirection: "column", gap: 2, color: "#b5a68d" }}>{label}{children}</label>;
}

function Range({ label, lo, width, onLo, onWidth, loMin, loMax, wMin, wMax }: {
  label: string; lo: number; width: number; onLo: (v: number) => void; onWidth: (v: number) => void;
  loMin: number; loMax: number; wMin: number; wMax: number;
}) {
  return (
    <div style={{ display: "flex", gap: 16 }}>
      <Labeled label={`${label} center ${num(lo)}`}>
        <input type="range" min={loMin} max={loMax} step={1} value={lo} onChange={(e) => onLo(parseFloat(e.target.value))} />
      </Labeled>
      <Labeled label={`width ${num(width)}`}>
        <input type="range" min={wMin} max={wMax} step={1} value={width} onChange={(e) => onWidth(parseFloat(e.target.value))} />
      </Labeled>
    </div>
  );
}

function BandRow({ label, band, onChange, max, step }: {
  label: string; band: [number, number, number]; onChange: (b: [number, number, number]) => void;
  max: number; step: number;
}) {
  const [lo, hi, edge] = band;
  return (
    <div style={{ display: "flex", gap: 16 }}>
      <Labeled label={`${label} lo ${num(lo)}`}>
        <input type="range" min={0} max={max} step={step} value={lo} onChange={(e) => onChange([parseFloat(e.target.value), hi, edge])} />
      </Labeled>
      <Labeled label={`hi ${num(hi)}`}>
        <input type="range" min={0} max={max} step={step} value={hi} onChange={(e) => onChange([lo, parseFloat(e.target.value), edge])} />
      </Labeled>
      <Labeled label={`falloff ${num(edge)}`}>
        <input type="range" min={0} max={max} step={step} value={edge} onChange={(e) => onChange([lo, hi, parseFloat(e.target.value)])} />
      </Labeled>
    </div>
  );
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const h = hex.replace("#", "");
  const n = h.length === 3
    ? h.split("").map((c) => c + c).join("")
    : h.padEnd(6, "0").slice(0, 6);
  const v = parseInt(n, 16);
  return { r: (v >> 16) & 255, g: (v >> 8) & 255, b: v & 255 };
}

/** G4 · Reach at year 0 (5 steps) for a placed good. */
function ReachEditor({ g, onChange }: { g: GoodSpec; onChange: (p: Partial<GoodSpec>) => void }) {
  const k = reachIndex(g);
  return (
    <div style={{ flexBasis: "100%", display: "flex", gap: 10, alignItems: "center" }}>
      <span title="How far this good has spread at year 0">Reach</span>
      <input type="range" min={0} max={4} step={1} value={k} style={{ width: 160 }}
        onChange={(e) => { const r = REACHES[Number(e.target.value)]; onChange({ distribution: r.distribution, reach_km: r.reach_km }); }} />
      <b>{REACHES[k].label}</b>
      <span style={{ color: "#9d917d" }}>{REACHES[k].hint}</span>
    </div>
  );
}

/** Live suitability heatmap for a good spec (debounced; brighter = better fit).
 *  With `onPin`, a click pins the homeland there (G4); the pin draws as a ring. */
function PreviewCanvas({ spec, onPin }: { spec: GoodSpec; onPin?: (cell: [number, number] | null) => void }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const meta = useWorldStore((s) => s.meta);
  const pin = spec.homeland?.[0] ?? null;
  const key = JSON.stringify(spec);
  useEffect(() => {
    let cancelled = false;
    const t = setTimeout(() => {
      previewGoodScore(spec).then((grid) => {
        if (cancelled) return;
        const cv = ref.current;
        if (!cv || grid.width === 0) return;
        cv.width = grid.width;
        cv.height = grid.height;
        const ctx = cv.getContext("2d");
        if (!ctx) return;
        const img = ctx.createImageData(grid.width, grid.height);
        const col = hexToRgb(spec.color);
        for (let p = 0; p < grid.data.length; p++) {
          const v = grid.data[p] / 255;
          const o = p * 4;
          img.data[o] = Math.round(8 + col.r * v);
          img.data[o + 1] = Math.round(10 + col.g * v);
          img.data[o + 2] = Math.round(14 + col.b * v);
          img.data[o + 3] = 255;
        }
        ctx.putImageData(img, 0, 0);
        if (pin && meta) {
          const px = (pin[0] / meta.grid_width) * grid.width, py = (pin[1] / meta.grid_height) * grid.height;
          ctx.strokeStyle = "#fff"; ctx.lineWidth = 1.5;
          ctx.beginPath(); ctx.arc(px, py, 4, 0, Math.PI * 2); ctx.stroke();
        }
      }).catch(() => {});
    }, 350);
    return () => { cancelled = true; clearTimeout(t); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return (
    <div style={{ flexBasis: "100%" }}>
      <div style={{ fontSize: 10, color: "#9d917d", marginBottom: 3 }}>
        Suitability preview (brighter = better fit) — updates as you edit
        {onPin && <> · <b>click to pin the homeland</b>{pin && <> · <button style={{ ...btn, padding: "0 5px" }} onClick={() => onPin(null)}>clear pin</button></>}</>}
      </div>
      <canvas ref={ref} onClick={(e) => {
        if (!onPin || !meta) return;
        const r = e.currentTarget.getBoundingClientRect();
        const fx = (e.clientX - r.left) / r.width, fy = (e.clientY - r.top) / r.height;
        onPin([Math.min(meta.grid_width - 1, Math.floor(fx * meta.grid_width)), Math.min(meta.grid_height - 1, Math.floor(fy * meta.grid_height))]);
      }} style={{ cursor: onPin ? "crosshair" : undefined,
        width: 280, height: 140, imageRendering: "pixelated",
        border: "1px solid #3f382d", borderRadius: 4, background: "#0c0a07",
      }} />
    </div>
  );
}
