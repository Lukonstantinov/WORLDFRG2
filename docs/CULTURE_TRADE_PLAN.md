# Culture → Trade: making the culture layer load-bearing

**Status: STEP 0 BUILT (the instrument). Steps 1-6 approved in shape, not started.
STEP 7 IS BLOCKED BY ITS OWN MEASUREMENT — see §5 F4.**

---

## 1. The finding this plan exists to fix

WorldForge has **three** well-built culture subsystems — 14 traits (`kit_traits`),
creoles (ethnogenesis), and lingua franca (`compute_lingua`) — and they form a
**closed loop**. Every one of them feeds only back into the others:

```
   TRAITS ──┐
   CREOLES ─┼──► traits_resist_assimilation ──► assimilation_pass ──► hub_minorities
   LINGUA ──┘                                          ▲                    │
                                                       └── diaspora_pass ◄──┘

   hub_minorities is read by SIX places:
     4 × display commands · 1 × city naming (cities.rs) · the culture passes themselves
   ─────────────────────────────────────────────────────────────────────────────
   ZERO economic readers. No edge to production, dispatch, war, money or realms.
```

`diaspora_pass` faithfully moves settlers along trade ties for 500 years and builds
a patchwork of minority quarters across the map. **Not one coin moves differently
because of it.**

Of the 14 traits, exactly **three** are read by anything:
`Insular`/`Xenophobic`/`Assimilative`/`Diaspora` (assimilation resistance),
`Clannish` (a matriliny precondition, read once at seeding), and the rest — including
`Mercantile`, `Seafaring`, `Martial`, `Agrarian`, `Scholarly`, `Artisan` — drive
**nothing**. Every economy in the world is the same economy wearing a different
name-bank.

## 2. The goal, as a gate rather than an aspiration

> Wire six traits and the diaspora return edge such that peoples measurably differ
> in economic behaviour, **without** moving `econ_` top-10% wealth share out of
> 0.60–0.90 or Gini out of its band, and **without** any single culture holding a
> minority quarter in more than `X`% of cities — where `X` is set from Step 0's
> measurement, not guessed.

## 3. Order

```
  STEP 0  the INSTRUMENT + the no-op helper            ← BUILT
  STEP 1  Mercantile   → dispatch margin
  STEP 2  Seafaring    → voyage loss + expedition reach
  STEP 3  Martial      → war declare chance + round bias
  STEP 4  Agrarian     → province yield
  STEP 5  Scholarly    → bank charter odds
  STEP 6  Artisan      → manufacture labour term
  ────────────────────────────────────────────────────────────────────────
  STEP 6b BOUND `diaspora_pass`      ← NEW, and now a PREREQUISITE, not an
                                       optional cleanup (§5 F4)
  STEP 7  the RETURN EDGE — kin_pull + lingua_pull in dispatch
          BLOCKED until 6b lands and this diagnostic is re-run.
```

Steps 1-6 are unaffected by F4: none of them touches the diaspora loop.

One trait per commit, each with its own `econ_` before/after. Six at once and a
band moves with no way to attribute it (§2.4: "never tune a constant without a gate
that isn't the target").

## 4. Step 0 — what was actually built

Step 0 writes **no production behaviour**. Its gate is that the dynamics run is
bit-identical.

### 4.1 `CampaignSim.culture_kits` — the sim stops reading a process global

`culture_trait_ids` resolved a culture's kit through `cultures::kit_of_people`,
which reads the **process-global** `CultureMap`. Two consequences:

- §5 documents the tick as "Pure & deterministic per `(seed, tick)` — no DB, no
  global RNG, no tile access". Reading a process-global culture map contradicts
  that claim; this is the first fix to it.
- §8.19 forbids a test from calling `cultures::set_active` (it raced two existing
  tests). So in **every** test, `culture_trait_ids` returned empty and
  `traits_resist_assimilation` returned 1.0 — the one wired trait reader was
  invisible to every gate in the project. That is why eleven dead traits went
  unnoticed: the live one was untestable too.

`culture_kits: Vec<CultureKit>` (`{ culture, kit, mut_seed }`, serde-defaulted
empty) is a sim-local registry consulted BEFORE the global, seeded once from the
culture map and lazily backfilled on load (the same "backfill on demand" pattern
`ensure_province_land` uses). Empty ⇒ falls through to the old global path ⇒
existing saves and the real app are unchanged.

**Why the INPUTS and not the derived traits are persisted.** Storing `kit` +
`mut_seed` keeps `kit_traits` the single source of truth (§8.18's "one copy cannot
drift"). The accepted, documented cost: a future retune of `kit_traits` would move
an existing save's behaviour once traits are load-bearing. The mitigation is
already mandated by `TRAITS`' own doc — append at the end, never reorder.

### 4.2 `culture_trait_factor(culture, axis) -> f32` — the no-op bus

Mirrors `head_character_factor` exactly, which is the precedent in this codebase
for "a knob that is a TRUE 1.0 when the data is absent, not an approximation":

- returns exactly `1.0` for a culture with no traits — no special case at any
  future call site;
- bounded to ±`TRAIT_KNOB_CAP` (0.15) of 1.0 (rule 18: any new multiplier needs a
  ceiling);
- **has no call sites in this step.** It is plumbing, so Steps 1-6 are one-line
  commits with their own gates.

### 4.3 `culture_reference_world()` + `econ_measure_diaspora_reach`

A SEPARATE fixture from `reference_world()`, kept separate for the same reason
`realm_reference_world` is: `hub_culture`/`prov_culture` feed migration, so changing
the scorecard's world to express cultures would move the scorecard.

Six peoples over 42 cities, their names chosen by **measured** mobility rather than
hope (the generator is bimodal — see §5) — two travel-prone, four rooted — with
kits assigned through `culture_kits` so traits actually resolve without the global.

`econ_measure_diaspora_reach` (`#[ignore]`d, 300 years, the shape of
`econ_measure_foreign_hand_conjunction`) prints:

- cities holding a quarter ≥2%, per culture, and as a share of all cities;
- **the maximum any single culture reaches** — this is the number that sets Step 7's
  assertion threshold;
- concentration (Herfindahl) across quarter-holding cultures;
- mean ethnic overlap across actual trading pairs — the raw material `kin_pull`
  would multiply, so if this is ~0 the return edge is not worth building;
- each culture's resolved traits and its `traits_resist_assimilation` value —
  the brake, measured where it matters.

## 5. Findings that shaped this plan (measured, not reasoned)

**F1 · The mobility generator is bimodal; there is no mid band.**
`culture_mobility` maps `r > 0.80` to `0.7..1.0` and everything else to `0.1..0.5`.
Nothing can land in `[0.5, 0.68)`. Therefore `DIASPORA_MOBILITY_GATE` (0.5) and the
Diaspora-trait gate (0.68) are **the same gate in practice**, and `kit_traits`'
`else if mobility >= 0.5 { Nomadic }` branch is **unreachable for every culture** —
Nomadic can only ever arrive as an archetype trait (Turkic, Mongol) or the seeded
flavour. Not fixed here: it is a behaviour change, and this step changes no
behaviour.

**F2 · `kit_traits` drops Insular from every diaspora culture.** The doc promises a
mobile people "gains Diaspora + Insular (the scattered, self-contained
merchant-minority profile)". The code pushes both onto a list that already holds two
archetype traits and then calls `out.truncate(3)`. No kit's archetype pair contains
Diaspora, so a high-mobility culture always ends `[arch0, arch1, Diaspora]` and
**Insular is always discarded**. `traits_resist_assimilation` therefore returns
`0.40`, not the intended `0.18` — quarters dissolve **2.2× faster than designed**.
Not fixed here, for the same reason as F1; the diagnostic prints both the actual and
the intended value so the fix can be commissioned with a number attached.

**F3 · `culture_mobility` (cities.rs) and `people_mobility` (cultures.rs) are
byte-identical duplicates.** Deduplicated in this step by delegation — provably
bit-identical, and it removes a drift risk before traits become load-bearing.

**F4 · THE CULTURE LAYER ALREADY CONVERGES ON ONE PEOPLE — measured, and it
blocks step 7.** This is what the instrument was built to find, and it found it
before a line of coupling was written. Over 300 years on
`culture_reference_world()`:

```
  people        mobility   quarters   majority in   traits
  Numidian         0.973   40 of 40   39 of 40      Mercantile+Seafaring+Diaspora
  Ilyric           0.758   30 of 40    0            Mercantile+Pastoral+Diaspora
  Kolchis          0.467    0          0            Martial+Seafaring+Scholarly
  Astaran          0.486    0          0            Martial+Scholarly+Mercantile
  Sarmatian        0.391   18 of 40    0            Agrarian+Clannish+Xenophobic
  Vendran          0.377   20 of 40    1            Scholarly+Artisan+Clannish

  SATURATION in year 120 — and held for the remaining 180 years.
  2 of 6 peoples hold no quarter anywhere · 4 of 6 rule no city at all.
  mean ethnic overlap across trading pairs: 0.47, still rising at year 300.
```

One travel-prone people ends as the **majority in 39 of 40 cities**. The
maintainer's stated invariant for this workstream — *cultural diversity must never
converge on one colour* — is **already violated today**, with no economic coupling
whatsoever. `diaspora_pass` has no cap on how many cities one people may reach, and
F2 means the assimilation brake that should oppose it runs 2.2× weaker than designed
on exactly the peoples doing the spreading.

The consequence for the plan is not a tuning note, it is a **stop**: `kin_pull` is a
reinforcing term, and a reinforcing term on a mechanism already pinned at its ceiling
can only make a runaway worse. Step 7 does not proceed until `diaspora_pass` is
bounded (new step 6b) and this diagnostic is re-run.

There is a second, quieter reading worth keeping: overlap at 0.47 means `kin_pull`
would have had plenty to multiply. The return edge is not a bad idea — it is a good
idea aimed at a loop that has no brake yet.

## 6. Risks

- **The return edge is a positive feedback loop.** Quarters raise `kin_pull`, which
  raises trade on that tie, which is where `diaspora_pass` sends the next wave.
  Three brakes exist — the multiplier cap, `assimilation_pass`, and
  `DIASPORA_MAX_MINORITY` (0.45) — but the last two bound the SIZE of a quarter,
  not the NUMBER of cities holding one. That unbounded axis is exactly what Step 0
  measures, and F2 means the assimilation brake is currently 2.2× weaker than
  designed precisely where the loop is strongest.
- **Assimilation must stay permanently partial** (maintainer's decision). A realm
  that only grows converges on one colour; so does a culture layer that only
  spreads. **F4 shows this is not a hypothetical risk but the current behaviour.**
- **Step 6b is where the judgement is**, and it should not be guessed. Bounding a
  diaspora's REACH (how many cities) is a different fix from strengthening the
  assimilation BRAKE (F2), and they have different side effects: the first is a hard
  cap and reads as a rule, the second is a rate and reads as a pressure. The
  instrument now exists to compare them, which is the point of having built it
  first.

## 7. Deliberately not built

- **Nomadic / Pastoral as mechanics.** The interesting version is seat-less
  provinces with seasonal rural migration — a structural change threatening
  `province_demography_pass`, not a multiplier. Deferred, not folded in silently.
- **Devout.** Needs a religion layer; ruled out of scope by the maintainer.
- **Widening trait assignment per world** (environment-driven archetypes, creoles
  born with port traits). Deferred so it cannot confound attribution while trait
  effects are being wired one at a time.
- **F1 and F2's fixes.** Real bugs, measured and recorded; each is a behaviour
  change needing its own gate, and Step 0 changes no behaviour.
- **Step 6b's fix itself.** F4 says a bound is needed and the instrument can now
  compare candidates, but choosing between a reach cap and a stronger brake is a
  design decision with a visible effect on every world, so it is commissioned
  separately rather than folded into a measurement step.
