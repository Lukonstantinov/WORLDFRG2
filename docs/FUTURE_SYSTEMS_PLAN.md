# Future Systems — Initial Feature Plan

Scope chosen by the user (by importance rank): **Rank 1 (all), Rank 2 (diplomacy/
embargoes), Rank 3 (all), Rank 4 (money supply), Rank 8 reframed as a shadow
economy that EMERGES from Rank 1.** Plus a **house-split** redesign.

This is a *planning* document — design sketch + open questions. Nothing here is
implemented yet. Each system notes how it plugs into the existing `sim/tick.rs`
engine (which already has: production/consumption with substitution, market prices,
manufacturing chains, merchant houses + guilds + local merchants, estates/colonies,
banks/coin/crashes, speculation, economic war, tariffs/wealth tax, sentiment).

> **Confirmed for the user:** trade throughput IS attributed to three merchant
> classes — `tw_house` / `tw_local` / `tw_guild` — at both endpoints of every flow
> (house-owned voyages → houses, short hauls → local merchants, long hauls →
> guilds). Guilds are first-class trading entities (`is_guild` houses). So **local
> merchants and guilds are already part of trade dynamics.**

---

## Rank 1 — Labor markets & wages + Economic migration  *(foundational)*

**Why first:** today production scales with raw population and population only moves
via food/colony seeding. Labor + migration is the loop everything else plugs into
(inequality, unrest, shadow economy).

### 1a. Labor markets & wages
- Each hub gets a **labor pool** (working population) split skilled/unskilled, a
  **wage level** set by labor supply vs demand (production + manufacturing pull).
- Wages feed **prices** (cost-push) and **house margins** (labor is a cost of
  estates/manufactories), and define **unemployment** (idle labor).
- Manufactories already scale output by population → swap to scaling by *employed
  labor at the prevailing wage*.

### 1b. Economic migration
- Monthly/yearly, a fraction of a hub's people **migrate toward higher real wages /
  prosperity** and **away from war, famine, plague, unrest**, along the trade-route
  network (reuse the coarse cost grid for migration distance).
- Booms pull labor in (wages fall, output rises); busts push it out. Caps to avoid
  oscillation.

**Open questions (Rank 1):**
1. Wage model: a simple supply/demand clearing wage per hub, or a fuller
   skilled/unskilled two-tier market?
2. Should wages be paid by *houses/estates* (a real cost on their books) or abstractly
   by the city economy? (Former couples it to house wealth; latter is cheaper.)
3. Migration cadence — yearly (cheap, stable) or monthly (responsive, costlier)?
4. Should migration move *merchant-class* population too, or only the general
   populace?

---

## Rank 2 — Diplomacy & trade policy

**Why:** reshapes the trade graph more than war. Plugs into the existing polis/council
+ war systems.

- **Treaties / alliances** between poleis: lower mutual tariffs, shared defense.
- **Embargoes / sanctions:** a polis bars trade with a rival → routes reroute or
  goods grow scarce downstream (reuse `house_barred` + route reach machinery).
- **Tariff unions / most-favored-nation:** blocs with internal free trade.
- **Staple / navigation rights:** a polis forces trade through its port (mercantilism).
- Council policy (`decide_polis_policy`) gains a *diplomacy* decision each year.

**Open questions (Rank 2):**
1. Who drives diplomacy — the **polis council** (dominant house), or a separate
   state actor?
2. Should embargoes be **bilateral** (polis↔polis) or also **good-specific** (ban
   only war materiel / luxuries)?
3. Do treaties need a **trust/relations score** between poleis (like coin trust), or
   simple on/off pacts?

---

## Rank 3 — Social classes & inequality + Income-elastic demand

### 3a. Social classes & inequality
- Split each hub's population into **classes** (e.g. elite / artisan / commoner /
  poor) with distinct **consumption baskets** (poor → staples; elite → luxuries) and
  **political weight**.
- Track an **inequality / Gini-like** measure per hub from wages + house wealth; high
  inequality feeds **unrest** (Rank 6 hook) and the shadow economy (Rank 8).

### 3b. Income-elastic demand
- Luxury demand scales with **class income**, not just population — rich cities crave
  far more luxuries (extends the foreign-luxury desire already shipped).

**Open questions (Rank 3):**
1. How many classes — 3 (elite/middle/poor) or 4 (add clergy/nobility distinct from
   merchant elite)?
2. Should class shares be **fixed at worldgen** or **dynamic** (mobility as wages
   change — couples to Rank 1)?
3. Should inequality directly modify **prices/demand**, or only feed **unrest**?

---

## Rank 4 — Money supply → inflation

- A real **quantity-of-money** link: minted coin volume + bank notes issued raise the
  **regional price level** (inflation), beyond the current per-hub price solver.
- Debasement (already modeled) + a **silver/gold glut** (new minting) → visible
  region-wide inflation; tight money → deflation/credit crunch.
- Couples to banks (notes are money) and coinage (seigniorage already exists).

**Open questions (Rank 4):**
1. Inflation scope — **per-polis**, **per-component (continent)**, or **global**?
2. Should bank **notes** count toward the money supply (fractional-reserve inflation),
   or only minted **specie**?
3. Tie inflation into the existing **price index**, or a separate money-supply index
   shown in the finance panel?

---

## Rank 8 (reframed) — Shadow economy: smuggling, piracy, mafia, corruption

**Emerges from Rank 1.** Where labor is idle (unemployment) and inequality is high,
an informal economy grows:
- **Smuggling:** high tariffs/embargoes + poor enforcement → goods move off-book
  (tariff revenue lost, prices undercut). Already have piracy on sea routes.
- **Piracy/banditry:** scales with unemployment + weak rule of law near busy routes;
  raises voyage loss + insurance demand (Rank 8 economic hook).
- **Mafia / protection rackets:** in unequal, unenforced cities — skims a cut of
  trade, intimidates rival houses, can be allied with a house.
- **Corruption:** officials skim tariff/tax revenue (couples to tax-farming).

**Open questions (Rank 8):**
1. Model the shadow economy as a **per-hub "informality" score** (cheap, drives
   leakage/violence rates), or as **actual shadow actors** (smuggler bands / mafia
   houses)?
2. Should it be **suppressible** by polis spending (a guard/enforcement budget →
   couples to treasury)?
3. Should a **house** be able to ally with / run the mafia (a dark archetype), or is
   it purely an environmental drag?

---

## House split redesign  *(houses split too rarely)*

### Conditions to split (need to pick)
A house splits when internal pressure is high. Candidate triggers:
- **Succession crisis** — at a generation change (head dies) with multiple strong
  heirs / low cohesion.
- **Internal friction** — a large, wealthy house past a size/wealth threshold with a
  strong #2 figure (cadet) and rising tension.
- **Opportunity** — a very rich house with many offices spins off a peaceful branch.

### Naming
- Cadet branch keeps the parent surname + a new name: **`Medici` → `Medici-Gonzaga`**
  (parent-new). The new branch inherits some offices/wealth.

### History entries (color-coded in the chronicle)
- 🔴 **"House split due to internal friction"** → a **trade war** breaks out; if the
  cadet branch wins, **one of the parent's offices becomes the cadet's main HQ**.
- 🔴 **"House split due to a succession crisis"** (after a generation change).
- 🟢 **"The peaceful house split"** → an amicable cadet branch, no war.

**Open questions (house split):**
1. Split trigger weights — which dominates: succession (on head death), wealth/size
   threshold, or a friction/cohesion score?
2. What does the cadet branch **inherit** — a share of wealth + some offices + part of
   the fleet? What fraction?
3. Trade war on a friction split: reuse the existing **economic-war** machinery
   (levies/blockade) between the two branches, or a lighter house-vs-house feud?
4. How often should splits happen (target cadence) — e.g. a great house has an X%/yr
   chance once it clears the threshold?
5. House seeding follow-up (user note): seed/spawn houses **weighted by where trade
   actually happens**, with some randomness — partly covered by the existing dynamic
   `maybe_found_house` (new families emerge in active hubs over time) + the new
   cross-continent initial spread. Decide: also weight *initial* seeding by projected
   trade, or rely on dynamic founding?
