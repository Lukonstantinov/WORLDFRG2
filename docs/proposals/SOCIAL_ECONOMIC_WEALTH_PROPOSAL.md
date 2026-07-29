# Social, Economic & Wealth — Analysis and Improvement Proposal

*Scope: the campaign tick simulation (`src-tauri/src/sim/tick.rs`), the worldgen
market (`sim/market.rs`), and the panels that surface them. Grounded in a real
50-year `simulate_decades_reports_dynamics` run (see Baseline below), not just a
code read.*

---

## 1. Baseline — what the living sim actually does today

A fresh 50-year run of the standing dynamics test (30 hubs, 6 goods, 10 seed
houses) produced:

```
yr  5: houses 6↑/4✝  banks 0  coins 10 (59%)  wars 0  crashes 0  richest 216560  silk 83%  thefts 0
yr 10: houses 2↑/11✝ banks 0  coins 10 (68%)  wars 0  crashes 0  richest 170299  silk 83%  thefts 0
yr 20: houses 2↑/15✝ banks 1  coins 12 (76%)  wars 0  crashes 0  richest 134259  silk 83%  thefts 0
yr 35: houses 2↑/15✝ banks 2  coins 12 (77%)  wars 0  crashes 0  richest 192511  silk 83%  thefts 0
yr 45: houses 2↑/15✝ banks 2  coins 12 (76%)  wars 0  crashes 0  richest 360827  silk 83%  thefts 0
yr 50: houses 2↑/16✝ banks 2  coins 12 (75%)  wars 1  crashes 0  richest 560878  silk 83%  thefts 0
over 50y: wealth ∈ [-5.1, 560878] · sustained (late) richest 560878
```

The economy is **bounded and dynamic enough to pass its asserts**, but six
systems that exist in code are effectively **dormant or cosmetic**:

| Symptom (50y run) | Reading |
|---|---|
| richest 216k → **560k**, still climbing in the final decade | The progressive wealth tax bends the curve but never **plateaus** the elite; no visible redistribution loop. |
| **0 crashes, 2 banks** ever | The whole DLC 3.5 banking + crash-contagion engine barely activates in a normal run. |
| houses settle at **~2 founded / 5y** vs ~15 dead | The merchant class decays toward a 2–4 house **oligarchy**; no replenishment ladder. |
| **1 war**, only at yr 50 | War is a rare tail event, not a recurring pressure. |
| **silk frozen at 83%**, **0 thefts** for 50y | Learning-by-doing + espionage (the productivity frontier) never move. |
| `mood` computed, charted, **consumed nowhere** | The social layer is a read-out with no feedback into the economy. |

The rest of this document proposes improvements along three pillars — **Social**,
**Economic**, **Wealth** — each tied to specific code, constants, and the digest
line it should move.

---

## 2. Social factors

### 2.1 Mood is inert — close the social→economic loop

**Current** (`update_sentiment`, tick.rs:3389): each hub eases three drivers
(`sent_food`, `sent_prosperity`, `sent_stability`) and blends them into `mood`
(tick.rs:3423). But:

- `mood` is **only** sampled for the history chart (tick.rs:4142). It changes
  nothing.
- `sent_food`/`sent_prosperity` feed carrying capacity (population, tick.rs:5072).
- `sent_stability` nudges coin trust (tick.rs:1762).

So a city can be miserable and it merely loses people slowly. There is no
**civil disorder** — no event when mood collapses, no consequence for the houses
that profit while the populace starves.

**Proposal — Civil Unrest events.** Add an unrest accumulator and a disorder
event class:

- Track `unrest: f32` per hub, rising when `mood` sits below a threshold
  (`UNREST_MOOD_FLOOR ≈ 0.35`) and decaying when content. High inequality
  (§2.2) multiplies the rise.
- When `unrest` crosses a trigger, fire a `"unrest"` event in `roll_events`
  (tick.rs:4789) / `active_events`: a temporary **production debuff** on the hub
  (reuse `event_production_mult`, tick.rs:4229), houses **avoid trading** through
  it for the duration (a freight/route penalty in `dispatch`), the polis spends
  **treasury** to restore order, and a severe riot can **unseat the dominant
  `council_house`** (tick.rs:789) — a direct social check on plutocracy.
- Surface it: an "Unrest"/"Disorder" row in `HubPanel` and a journal line so the
  Chronicle reads the social history, mirroring how wars/crashes already log.

**Why it matters:** it makes `mood` *load-bearing*, gives starvation/dearth real
teeth, and creates the missing feedback where extreme wealth concentration
provokes a backlash that resets local dominance.

**Touches:** `update_sentiment`, `roll_events`, `event_production_mult`,
`dispatch`, `decide_polis_policy`; new `unrest` field (serde-default → old saves
load); `HubPanel.tsx`, `types.ts`.

### 2.2 No inequality / social-class model

**Current:** the only path by which trade wealth "reaches the people" is
`civic_pool` (tick.rs:727) — an abstract scalar that houses pay conspicuous
consumption / endowment / overhead into (`apply_wealth_sinks`, tick.rs:3568) and
that nudges `sent_prosperity`. There is **no commoner-wealth state, no class
split, no inequality measure** anywhere in `sim/` (confirmed: no `gini`,
`commoner`, `wage`, `class` machinery exists).

So "the elite is at 560k" has no counterpart in "and the commoners are…". The
player cannot see, and the sim cannot react to, the *distribution* of wealth.

**Proposal — a two-class welfare model per hub:**

- **Commoner welfare** `commoner_wealth: f32`: fed by `civic_pool` spend, **wages**
  from local production/manufacture (a slice of `production[g]·price` scaled by
  the labor share), and public works; drained by **rents** (estate/manufactory
  owner cuts already exist, `ESTATE_OWNER_CUT`, tick.rs:84) and by dearth.
- **Elite wealth** at the hub = Σ resident-house wealth + treasury (already
  available).
- **Inequality index** `inequality: f32` (a cheap Gini-proxy = elite /
  (elite + commoner)). Feeds the unrest accumulator (§2.1) and is shown as a
  bar in `HubPanel`.

This turns the existing `merchant_pops` split (tick.rs:7237, currently
display-only) into a real **class structure**: commoners vs merchant houses vs
guilds, each with a wealth pool and a stake in the city's mood.

**Touches:** new `commoner_wealth`, `inequality` fields; wage credit in
`update_food_and_starvation`/production pass; `apply_wealth_sinks` already routes
elite outflow → just split its destination; `HubPanel.tsx`, `types.ts`,
`campaign_commands.rs` (HubDetail).

### 2.3 No social mobility — the merchant class doesn't replenish

**Current:** `maybe_found_house` (tick.rs:7003) spawns new houses, but the digest
shows founding collapsing to ~2/5y while ~15 houses die — the world trends to a
tiny survivor oligarchy.

**Proposal — a commoner→merchant ladder.** In a prosperous hub with **high
commoner welfare** (§2.2) and no overwhelming dominant house, let a successful
local merchant **graduate into a new House** (seeded from `commoner_wealth`, a
small starting fortune, a specialty in the city's surplus good). Gate it on
commoner welfare so mobility is a *reward* for a healthy, broadly-prosperous city
— and a pressure-release valve against the oligarchy. This directly lifts the
`houses ↑` line and keeps the merchant ecosystem diverse over centuries.

**Touches:** `maybe_found_house` (add the welfare-gated path), `commoner_wealth`.

---

## 3. Economic factors

### 3.1 Banking & financial crises are dormant

**Current:** `update_banks` (tick.rs:1912), `bank_pass` (tick.rs:1992),
`trigger_regional_crash` (tick.rs:2253), `maybe_pop_bubbles` (tick.rs:2313) all
exist — yet the run produced **2 banks and 0 crashes in 50 years**. The
speculation engine (`compute_speculation`, tick.rs:2573) ranks bubble drivers,
but nothing pops. The most elaborate subsystem in the file is barely exercised.

**Proposal:**

- **Lower the bank-founding bar** so a council seat with a banking-archetype
  house reliably charters a bank within a decade or two (tune `BANK_*` founding
  thresholds in `update_banks`), targeting ~1 bank per major polis.
- **Make bubbles actually pop:** verify `maybe_pop_bubbles` fires on sustained
  high `SpecCenter` risk and that `trigger_regional_crash` is reachable from it;
  add a guaranteed-eventually trigger when speculation risk stays maxed for N
  years. A crash should then **spike unrest** (§2.1) and write a `CrashRecord`.
- **Target metric:** the digest should show banks rising into double digits and
  **at least a handful of crashes across 50y** — the boom/bust the design
  promises.

**Touches:** `update_banks`, `bank_maybe_lend`, `compute_speculation`,
`maybe_pop_bubbles`, `trigger_regional_crash`. **Per the standing rule, re-run
`simulate_decades_reports_dynamics` until crashes/banks read healthy *and* wealth
stays bounded.**

### 3.2 Productivity frontier is frozen

**Current:** `update_good_quality` (tick.rs:2495) and `maybe_steal_quality`
(tick.rs:2523) implement learning-by-doing and espionage, but **silk stayed at
83% for all 50 years with 0 thefts**. The economy has no moving technological
frontier — quality is static, so there's no "rising productivity" story beyond
the flat +1.5%/yr `PROD_GROWTH_PER_YEAR` drift.

**Proposal:** make manufactories **climb quality with sustained output**
(learning curve) and let `maybe_steal_quality` actually fire between rival houses
(it currently never does in a normal run). A visible quality frontier (silk 83%→
95%+ over a century, occasional technique theft) is both an economic-dynamism win
and great Chronicle material.

**Touches:** `update_good_quality`, `maybe_steal_quality`; tune the learning rate
and theft chance.

### 3.3 War as a recurring pressure, not a tail event

**Current:** 1 war in 50 years (`maybe_declare_war`, tick.rs:2450). Wars levy
houses and move treasury — a meaningful economic shock that almost never happens.

**Proposal:** modestly raise war frequency between rival poleis (rivalry already
tracked, tick.rs:7109) so a campaign sees periodic regional conflicts that
disrupt trade lanes (blockades already modeled) and force house levies. Keep it
bounded so it doesn't dominate. **Re-run the dynamics test to confirm wealth and
population stay sane.**

---

## 4. Wealth

### 4.1 The elite drifts up instead of plateauing

**Current:** the progressive civic wealth tax (`apply_wealth_sinks`, tick.rs:3555)
is a flat base (`HOUSE_WEALTH_TAX_BASE` 0.004/mo) + a quadratic surcharge above
`HOUSE_WEALTH_SOFTCAP` (60k), capped at 40%/mo. It bends the curve — the test
comment notes it stopped an old ~1.25M runaway — but the run still shows the
richest **climbing 216k → 560k in the final decade**. It reins in *runaway* but
does not produce a stable elite ceiling.

**Proposal:**

- **Make the tax visibly fund welfare.** Today the proceeds go to `treasury`
  (tick.rs:3574). Route a share to **commoner welfare / public works** (§2.2) so
  redistribution is real and on-screen — the player sees the rich taxed *and* the
  people lifted, closing the loop the design implies.
- **Optional gentle stabilizer:** a slightly steeper surcharge slope or a lower
  soft cap so the sustained elite **plateaus** (e.g. settles ~300–400k) rather
  than drifting up. This is a *tuning* change — must be validated against the
  `late_max < 800_000` assert and, more importantly, against houses still being
  able to afford outposts (`OUTPOST_FOUND_WEALTH` 100k) so growth ambitions
  survive.

### 4.2 Dead wealth constants (cleanup)

`WEALTH_TAX_PROG` and `WEALTH_TAX_SCALE` (tick.rs:304–305) are **unused** — the
compiler warns on both. The progressive tax was reimplemented as the quadratic
surcharge (`HOUSE_WEALTH_TAX_*`). Remove the dead constants (or wire one of them
into the §4.1 stabilizer) to silence the warnings and keep the wealth tuning
legible.

### 4.3 Wealth distribution is invisible to the player

Even bounded, wealth today is a single number per house and an abstract
`civic_pool` per hub. With §2.2 in place, add a small **"Who holds the wealth"**
read-out (elite vs commoner vs treasury, plus the inequality bar) to `HubPanel`,
and a world-level inequality trend to the finance panel. *This is a visual change
→ ship an HTML before/after report in `docs/mockups/` per the standing rule.*

---

## 5. Prioritized roadmap

| # | Change | Effort | Payoff | Risk |
|---|--------|--------|--------|------|
| 1 | **Inequality + commoner-welfare model** (§2.2) | M | Unlocks everything below; makes wealth legible | Low — additive state, serde-default |
| 2 | **Civil unrest loop** (§2.1) | M | Makes `mood` load-bearing; social→economic feedback | Med — must not death-spiral cities |
| 3 | **Route wealth tax → welfare + plateau tuning** (§4.1) | S | Visible redistribution; stable elite | Med — re-tune against dynamics test |
| 4 | **Activate banking/crashes** (§3.1) | M | Boom/bust the design promises | Med — must stay bounded |
| 5 | **Commoner→merchant mobility** (§2.3) | S | Fixes the oligarchy decay | Low |
| 6 | **Quality frontier + espionage** (§3.2) | S | Economic dynamism + Chronicle | Low |
| 7 | **War frequency** (§3.3) + **dead-constant cleanup** (§4.2) | S | Recurring pressure; tidy tuning | Low |

**Suggested first slice:** #1 + #2 + #3 together — they form one coherent
"society reacts to wealth" feature, all anchored on the new inequality state, and
they make the existing-but-cosmetic `mood` system finally matter.

## 6. Testing & guardrails (non-negotiable per CLAUDE.md)

- Every change touching `tick.rs` → re-run
  `cargo test --lib simulate_decades_reports_dynamics -- --nocapture` and read the
  digest: wealth bounded & finite, houses turn over, and now also banks/crashes/
  unrest actually occur. The test hard-asserts bounded+finite wealth and turnover.
- Any visual change (HubPanel inequality bar, unrest row, finance trend) →
  self-contained before/after HTML in `docs/mockups/`, dark theme, sent to the
  user.
- All new struct fields `#[serde(default)]` so existing `.campaign` saves load.
