# Two Apps? — and the File-Upload Viability Audit

**Status: AUDIT COMPLETE (measured by reading every upload path) · PLAN PROPOSED, NOT BUILT.**

Two questions were asked together:

1. Should WorldForge become **two apps** — a world generator and a campaign player?
2. Or should it stay one app in which **opening a map file can start a campaign**?
3. And: **analyse every file upload and say whether that is viable now.**

Question 3 answers questions 1 and 2, so it goes first.

---

## 1. The answer up front

**It is not viable now.** Opening a `.worldforge` file and pressing *Begin Campaign*
fails, every time, on every world file the app has ever written. The failure is not a
bug in the dialog or the loader — both work. It is that **`save_world_as` deliberately
deletes the two rows a campaign cannot start without.**

```rust
// commands/file_commands.rs::save_world_as
let backup = rusqlite::backup::Backup::new(&conn, &mut dest)?;   // copies EVERY table
backup.run_to_completion(...)?;
dest.execute("DELETE FROM campaign", [])?;                        // ← settlements + economy die here
```

`CAMPAIGN_KEYS` (`campaign_commands/mod.rs:34`) is what lives in that table:

```
name · settlements · economy · bio_params · campaign_progress · world_ref · campaign_sim
```

and `campaign_start_sim` (`lifecycle.rs:208`) opens with:

```rust
let econ_json = metadata::campaign_get(&conn, "economy")
    .filter(|s| !s.is_empty())
    .ok_or_else(|| "Build the economy (step 10) before starting the campaign sim.")?;
```

So the exact user journey today is:

| Step | What happens |
|---|---|
| Generate a world, Finalize, **Save World** | `.worldforge` written — **without settlements, without economy** |
| Quit. Later, **Open** that `.worldforge` | Loads. `meta.frozen` is true, so `App.tsx:449` flips to **Chronicle** mode |
| Chronicle shows **▶ Begin Campaign** | `ChroniclePanel.tsx:62` → `campaign_start_sim` |
| Result | ❌ **"Build the economy (step 10) before starting the campaign sim."** |
| Can the user recover in Chronicle? | ❌ Chronicle renders `ChroniclePanel`, not `WorkflowPanel` (`App.tsx:694`). There is no step 10 button in that mode. |
| Can they switch to Forge and re-run? | ⚠️ Partly — see §3. Settlements (7) and Economy (10) are not freeze-gated, so they *can* re-run. **Provinces (7b) are freeze-gated and cannot.** |

The app is therefore **already a two-file product** — `.worldforge` + `.campaign` — and
the second file is mandatory, but nothing in the UI says so, and the Open dialog does
not even offer `.campaign` as an extension.

---

## 2. Full file-upload inventory

Every path by which a file enters the app. All six are **path-based** through
`@tauri-apps/plugin-dialog`; there is no drag-and-drop anywhere in `src/`.

| # | Surface | Command | Accepts | Carries in | Works? |
|---|---|---|---|---|---|
| 1 | Header **Open** | `open_world` | `.worldforge`, `.db` | tiles · metadata · objects · sim_state · campaign-if-present | ✅ loads · ❌ **cannot start a campaign** (§1) |
| 2 | Header **📂 Open Campaign** | `open_campaign` | `.campaign` | the 7 campaign keys | ✅ — but **requires a world already open** (`App.tsx:485`) |
| 3 | **Import Layers** | `import_world_layers` | another `.worldforge` | terrain/climate/hydrology/soil/hazards/goods columns | ✅ — grid sizes must match; freeze-gated |
| 4 | **Import Template** | `load_image_template` | any image | land/sea by 4-bit quantization | ✅ — freeze-gated |
| 5 | Goods Editor **Import .txt** | `import_goods_txt` | `.txt` | goods specs, add-only | ✅ |
| 6 | Legacy single-file save | `open_world` → `migrate_legacy_campaign_keys` | old `.worldforge` | settlements/economy from *metadata* | ✅ — and note **this is the only file format that has ever round-tripped a playable world in one file** |

Four findings fall out of the table.

**F1 — `.worldforge` is not a world you can play; `.campaign` is not a world.**
Neither file is self-sufficient. The user must keep a matched *pair*, in two separate
save actions, in two separate app modes, with no reminder and no bundling. Losing
either half loses the campaign.

**F2 — the legacy format already proved the bundle works.** `migrate_legacy_campaign_keys`
exists precisely because the app once shipped a single file that carried both halves,
and it still opens them correctly. A bundle format is not new engineering risk; it is
a format the loader already handles.

**F3 — regenerating the stripped halves silently corrupts provinces.**
`Province.settlements: Vec<String>` (`sim/shared/provinces.rs:101`) holds settlement
**ids**. Provinces live in world `metadata` and therefore *survive* `save_world_as`;
settlements live in `campaign` and do *not*. So re-running step 7 on a reopened world
— the only available recovery — produces a new settlement set whose ids no province
references, while `sim_generate_provinces` is freeze-gated and cannot be re-run to
repair them. The world looks fine and its political layer is quietly wrong. This is
the same class of failure §2.4 of `CLAUDE.md` warns about: it fails *silently*.

**F4 — the `prompt()` fallback on every dialog is dead code that hides errors.**
Each of the six surfaces wraps the dialog in `try { … } catch { prompt(…) }`. The
comment at `App.tsx:287` already records that **Tauri implements `window.prompt` on
none of its three webviews**. So if `plugin-dialog` ever fails to import, every file
operation degrades to a silent no-op rather than an error. The dialog plugin *is*
correctly registered (`lib.rs:23`, `capabilities/default.json`), so this is latent —
but it is a landmine, not a fallback.

---

## 3. Why "just re-run the missing steps" is not the fix

It is tempting to answer §1 by letting the user re-run steps 7→10 after opening a bare
world. The freeze rules make that a trap rather than a path:

| Step | Freeze-gated? | On a reopened `.worldforge` |
|---|---|---|
| 7 · Settlements | no | re-runs — **new ids, new culture map** (`store_and_activate`) |
| 7b · Provinces | **yes** (`sim_commands.rs:887`) | ❌ cannot re-run — stale ids stay stale |
| 8 · Biological | no | re-runs |
| 9 · Political | query-only | fine |
| 10 · Economy | query-only | fine |

Recovery therefore rebuilds the human layer against a political layer it can no longer
rebuild. **The fix has to be at the file format, not at the workflow.**

---

## 4. Two apps, or one? — recommendation

### Recommendation: **stay one app. Do not split the binary.**

The split is superficially attractive — the app already has a `Forge`/`Chronicle` mode
switch (`uiStore.appMode`), two panels, two toolbars, two save formats — so it looks
like the seam is already cut. It is not, and three facts say so:

1. **The campaign reads the world at start-up and only then stops.** `campaign_start_sim`
   needs the goods library, the culture map, `province_raster`, `world_ref`,
   `prov_good_belt` from `Province.good_belt`, and the tiles behind them. §3.4 of
   `CLAUDE.md` calls the interface a one-way snapshot — but *taking* that snapshot needs
   the whole world pipeline present. A campaign-only binary would have to ship the
   entire `sim/` tree anyway, at which point it is the same binary with a smaller menu.
2. **FIX_PLAN B1 points the other way.** Making the world↔campaign edge *two-way at
   province granularity* is an explicitly prioritised item. Splitting the binary now
   would put a process boundary exactly where the roadmap wants a tighter coupling.
3. **The map is the campaign's only view.** Every campaign panel draws on the same
   Pixi canvas, the same `OverlayManager`, the same tile pyramid, the same renderer.
   Two apps means two copies of the 4.6k-line overlay layer or an IPC-shaped rewrite.

**What the user actually wants from "two apps" — a clean separation between building a
world and playing one — the mode switch already delivers.** What it does not deliver is
a **file** that means "a world you can play." That is the real gap, and it is one
format change, not two products.

---

## 5. The plan

Five slices, smallest first, each with its own gate. Slices 1–2 alone close the
reported problem.

### Slice 1 — `.worldforge` carries a playable world (the actual fix)

Stop stripping what a campaign needs. Keep stripping what belongs to a *run*.

Split `CAMPAIGN_KEYS` into two sets:

```rust
/// The world's HUMAN LAYER — generated by steps 7-10, frozen with the world,
/// belongs in the .worldforge file.
const WORLD_HUMAN_KEYS: [&str; 3] = ["settlements", "economy", "bio_params"];

/// A RUN — one playthrough. Never in a world file.
const CAMPAIGN_RUN_KEYS: [&str; 4] = ["name", "campaign_progress", "world_ref", "campaign_sim"];
```

- `save_world_as` deletes only `CAMPAIGN_RUN_KEYS` from the destination, not the whole
  table.
- `save_campaign_as` keeps writing all seven (a `.campaign` stays a complete run).
- `open_world` needs no change — it already copies the campaign table when present.
- `campaign_start_sim` needs no change — its `economy` read now finds a value.

**Effect:** open any newly-saved `.worldforge`, land in Chronicle, press
*▶ Begin Campaign*, and it starts. Provinces, settlements and economy all agree,
because none of them were ever regenerated.

**Gate:** a new Rust test — `a_saved_world_file_can_start_a_campaign`: build a small
world through step 10, `save_world_as` to a temp path, open it into a fresh `WorldDb`,
assert `campaign_get(conn, "economy")` is non-empty and that `settlements` parses to
the same count. Plus `cargo test --lib econ_` and `simulate_decades_reports_dynamics`
unchanged (this slice touches no tick code, so both must be bit-identical).

**Back-compat:** old `.worldforge` files still have no economy. They open exactly as
today — so Slice 2 has to exist.

---

### Slice 2 — say so, instead of failing at the end

Three small honesty fixes, all frontend.

- **`campaign_start_sim`'s error must be reachable.** When Chronicle has no economy,
  `ChroniclePanel` shows a *"This world file predates playable saves — rebuild the
  human layer"* card instead of a *Begin Campaign* button that throws. The card offers
  one action that runs 7 → 8 → 9 → 10 in order, **and warns first** if
  `metadata["provinces"]` exists, because of F3.
- **`handleOpen` must not strand the user.** Today a frozen world with no economy sets
  `workflowStep = 10` *and* `appMode = "chronicle"` — a step number in a mode that has
  no step list. Land in **Forge** when the economy is missing; land in Chronicle only
  when it is present.
- **The Open dialog accepts `.campaign`.** Picking one when its world is not loaded
  should say which world it wants (`world_ref.world_name` is in the file) rather than
  being unreachable behind an `isLoaded` guard.

**Gate:** `npx tsc --noEmit`, plus a manual line in `IN_APP_VERIFICATION_CHECKLIST.md`:
open a pre-Slice-1 world → the card appears, not the error; open a post-Slice-1 world →
the campaign starts.

---

### Slice 3 — one file that is a world *and* a run (`.wf2`)

F2 says the loader already handles this shape. Add a single **Save Playable World**
action writing a bundle: the full SQLite backup with *nothing* deleted. `open_world`
already copies the campaign table when it is there, and `migrate_legacy_campaign_keys`
already no-ops on a split file — so the read path is done. What is needed is the write
action, the extension in the filters, and a decision about which the header offers by
default.

**Deliberately keep all three formats:** `.worldforge` (world, shareable, no run),
`.campaign` (run, small, needs its world), `.wf2` (everything, one file, largest). They
serve three real cases — publishing a map, sharing a save against a map someone already
has, and "I just want my game back."

**Gate:** round-trip test — save `.wf2`, open into a fresh DB, assert the campaign sim's
`tick` and total wealth survive unchanged.

---

### Slice 4 — the pair is checkable, not hopeful

`WorldRef.fingerprint` is `SUM(version), COUNT(*)` over lod-0 tiles. `open_campaign`
already compares it and warns. Two upgrades:

- Record `world_name` in the warning text so it says *which* world to open.
- On opening a `.worldforge`, if a `.campaign` with a matching fingerprint sits beside
  it in the same directory, offer to load it.

**Gate:** unit test on the mismatch branch; no sim exposure.

---

### Slice 5 — drag-and-drop (the literal "upload")

Tauri's core drag-drop event needs no plugin, only a capability entry. One handler on
the app root routes by extension: `.worldforge`/`.wf2` → `open_world`,
`.campaign` → `open_campaign`, image → template import (Forge only, unfrozen only),
`.txt` → goods import. Delete the `prompt()` fallbacks in the same commit (F4) and
surface a real error instead.

**Gate:** `npx tsc --noEmit` + a checklist line per file type.

---

## 6. Deliberately not built

- **A separate campaign binary.** §4 — the campaign needs the world pipeline to seed
  itself, and FIX_PLAN B1 wants that edge tighter, not process-separated.
- **Migrating existing `.worldforge` files in place.** Slice 1 cannot retro-fit data
  the file never contained. Slice 2's rebuild card is the honest answer, warning
  included.
- **Un-freezing provinces so step 7b can re-run after a reopen.** That would let the
  political layer be rebuilt — and would also let it drift out from under any campaign
  saved against it. The freeze is doing its job; F3 is a *format* bug, fixed in Slice 1.
- **A web/browser build.** Every upload path is a filesystem path handed to Rust. That
  is a separate product decision, not a file-format one.

## 7. Risks

- **R1 · World files get bigger.** `economy` and `settlements` are JSON blobs on a
  world that may hold 500 hubs. Measure before shipping Slice 1; if it matters, zstd
  them as the tiles already are.
- **R2 · Slice 1 changes what a `.worldforge` means.** A file saved after Slice 1 and
  opened by an older build will carry campaign rows the old `open_world` copies in
  happily — which is the *desired* behaviour, but it means the change is not reversible
  once files are in the wild. Land it deliberately.
- **R3 · The rebuild card in Slice 2 can corrupt provinces (F3).** It must warn, and it
  must be the user's choice. Never run it automatically on open.
