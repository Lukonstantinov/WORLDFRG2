# Houses as people — the build plan

**Status: PLAN. Nothing built.** Design and schematics: `HOUSE_PEOPLE_AND_TIERS.md`.
This file is the sequenced plan with a gate per step, per §2.4.

Six decisions taken by the maintainer, and what each one binds:

| # | Decision | Binds |
|---|---|---|
| 1 | Tiers reveal only during the campaign; Tier 1 may be empty early | tier is rank-banded with an absolute floor |
| 2 | Character on **every kin**, and on **anyone who rules a city or holds a role** | one shared `Character` block on `Kin`, `Official` and `Figure` |
| 3 | A schism splits **holdings AND wealth** | needs its own econ-oracle gate |
| 4 | A house may play by new rules, but **culture dominates** and drift must not be drastic | character is DERIVED from culture traits; individual drift bounded |
| 5 | Hired factors need clear logic | §4 below — a labour market, not a conspiracy |
| 6 | Goals autonomous, and more of them | 17 goal kinds, AI-chosen |

---

## 1. Character is a function of culture, not a free roll

This is the load-bearing consequence of decision 4. Character must not be an independent
random draw or a Punic merchant house and a Slavic agrarian house end up
indistinguishable — the thing culture is *for*.

The world already computes, deterministically, 2–3 `TRAITS` per culture
(`cultures.rs::kit_traits`, driven by a fixed per-kit archetype table plus mobility), and
already exposes them to the UI through `trait_briefs`. So the baseline is free:

```
character = clamp(Σ trait_contribution, −2, +2)   // the CULTURAL baseline
          + individual_drift                       // bounded, see below
```

Four axes, each already mapping to a knob in the code:

| Axis | − pole | + pole | Existing knob |
|---|---|---|---|
| **A** Caution ↔ Boldness | hoards, short terms | hulls, long voyages, expeditions | `decide_fleets`, expedition gate, contract `term_years` |
| **B** Honour ↔ Greed | keeps contracts, accepts arbitration | undercuts, escalates, debases | `FEUD_HEAT`, `arbitrate_feuds`, bribe spend |
| **C** Private ↔ Civic | hoards to the family | funds works, subsidises the city | `fund_public_works`, `HOUSE_CONSUMPTION_RATE` |
| **D** Rooted ↔ Expansive | deepens the seat | offices, bailos, colonies | office leasing, branch, colonisation |

Trait → axis contribution table (the whole mapping; A/B/C/D as above):

| Trait | A | B | C | D |
|---|---|---|---|---|
| 0 Mercantile 💰 | · | +1 | · | +1 |
| 1 Seafaring ⚓ | +2 | · | · | +1 |
| 2 Insular 🏝 | −1 | · | · | −2 |
| 3 Martial ⚔ | +2 | +1 | · | · |
| 4 Devout 🕯 | · | −1 | +1 | · |
| 5 Nomadic 🐎 | +1 | · | · | +1 |
| 6 Diaspora 🧳 | +1 | · | −1 | +2 |
| 7 Assimilative 🤝 | · | −1 | +1 | · |
| 8 Clannish 🩸 | · | · | −2 | −1 |
| 9 Scholarly 📜 | −1 | −1 | +1 | · |
| 10 Agrarian 🌾 | −1 | · | · | −2 |
| 11 Pastoral 🐐 | · | · | · | −1 |
| 12 Artisan 🔨 | −1 | · | +1 | · |
| 13 Xenophobic 🚫 | · | +1 | −1 | −2 |

Sanity check against real kits (before any code): Punic `[Mercantile, Seafaring]` →
bold, grasping, expansive — a maritime trading house. Slavic `[Agrarian, Clannish]` →
cautious, private, deeply rooted. Mongol `[Nomadic, Martial]` → very bold, expansive.
Sinitic `[Scholarly, Artisan]` → cautious, civic, honourable. Those read correctly, which
is the point of deriving rather than rolling.

**Individual drift, bounded** (decision 4: "not too drastic"):
- Ordinary person: **±1 on at most 2 axes**, seeded on their name.
- **Maverick** (8% roll): **±2 on one axis** — the house that plays by new rules. Flagged
  in the UI as *unusual for their people*, because the deviation only means something if
  the norm is legible.
- A drifted axis is still clamped to −2..+2, so no-one leaves the scale.

**Effect size:** each axis moves its knob by at most **±15%**. Character must be legible
without producing a house that plays a different game.

**Gate:** with every character at all-zero, `simulate_decades_reports_dynamics` must be
**bit-identical**. That is the proof character is a modifier and not a rewrite.

---

## 2. Character goes on three carriers (decision 2)

One shared struct, three carriers — so "anyone who rules a city or holds a role" has a
character, and the phrase generator is written once:

```rust
pub struct Character { pub axes: [i8; 4], pub maverick: bool }
```

| Carrier | Derived from | What it changes |
|---|---|---|
| `Kin` (new) | the house's seat culture | the head's drives the house AI; others matter on inheritance or defection |
| `Official` (exists) | the **city's** culture | a Greedy treasurer is cheaper to bribe; an Honourable magistrate resists capture and speeds arbitration; a Civic head funds works from the treasury; a Bold harbourmaster favours long-voyage charters |
| `Figure` (exists) | the city's culture | a Bold explorer raises expedition odds; a Scholarly master lifts craft quality |

`Official` and `Figure` already exist and are already persisted — this is one appended
serde-defaulted field each, and it makes the government layer (currently invisible
arithmetic, §1.2 of the earlier audit) start to read as people.

---

## 3. `Kin` roster

```rust
pub struct Kin {
    pub name: String, pub born_tick: u32, pub dies_tick: u32,
    /// 0 head · 1 heir · 2 steward · 3 idle · 4 married out · 5 dead
    pub role: u8,
    /// Hub of the holding they steward (−1 = at the seat).
    pub posted: i32,
    pub character: Character,
    pub loyalty: f32,   // 0..1 to the head
    pub skill: f32,     // 0..1 — multiplies what they run
    pub parent: i32,    // kin index, or −1 — the line a schism splits along
}
```

4–9 live kin per house, capped. Kin skill is drawn **lower** than a hired factor's
(nepotism is the cost of loyalty) and loyalty **higher**.

---

## 4. Stewards — the clear logic (decision 5)

Every holding — estate, manufactory, depot, office, bailo, bank — gets
`steward: i32` (index into the house's steward list, −1 = unattended).

Two kinds, and the whole design is that **they fail differently**:

| | Kin steward 👪 | Hired factor 💼 |
|---|---|---|
| Skill | drawn low (0.25–0.70) | drawn high (0.45–0.95) |
| Loyalty | starts high (0.75–0.95) | starts middling (0.35–0.65) |
| Cost | none (already fed by the family) | a monthly wage ∝ the holding's throughput |
| Failure mode | **resents** → schism (§6) | **skims** → embezzlement |
| Poachable | no | yes |

Six rules:

1. **Unattended holdings run at ×0.85 margin.** That is the pressure to staff at all.
2. **Steward effect:** the holding's margin × `(0.70 + 0.60 · skill)` — a good steward
   is worth roughly +30% over a poor one, so hiring well is a real decision.
3. **Wage:** a hired factor costs `WAGE_RATE × throughput` monthly, charged through the
   existing `apply_wealth_sinks`. Underpay (the house is illiquid) and loyalty falls.
4. **Skim:** at `loyalty < 0.5` a hired factor diverts
   `(0.5 − loyalty) × throughput × SKIM_RATE` into their own `skimmed` tally. Once
   `skimmed` passes a fraction of the holding's book it is **discovered** — chronicled,
   the factor dismissed, and the loss written off. Nothing is hidden from the player
   permanently; the discovery is the beat.
5. **Poaching is a labour market, not a scheme.** A rival with an office in the same city
   and Greed > 0 bids for the factor. The house may match the wage or lose them. Losing
   one transfers **no assets** — only the margin drops and the rival gains a trade edge
   there. This deliberately stays out of the intrigue layer that was excluded.
6. **Kin never skim.** A kin steward who is able, posted far, and disloyal is the schism
   candidate. Clean division: **hired staff steal money; family steals the house.**

---

## 5. Tiers (decision 1)

```
standing = 0.30·rank_norm(wealth) + 0.25·rank_norm(volume) + 0.20·reach
         + 0.15·seats + 0.10·rank_norm(prestige)
```

| Tier | Name | Band |
|---|---|---|
| 1 | Great house | top 8% **and** standing ≥ 0.55 |
| 2 | Major house | next 22% |
| 3 | Lesser house | next 40% |
| 4 | Marginal house | the rest |

Recomputed monthly with a **±0.04 dead band** (without hysteresis a house flickers
between 2 and 3 forever and floods the chronicle). Tier changes are chronicled. Tier 1
legitimately empty for the first decades — tiers reveal as the campaign runs.

---

## 6. Schism (decision 3 — holdings **and** wealth)

```
tension = 0.35·(1 − cohesion) + 0.25·(1 − mean kin loyalty)
        + 0.20·stretch + 0.10·feuds_running + 0.10·passed_over_able_heir
```

Three outcomes, never a silent number:

1. **Quarrel** — cohesion drops, one kin's loyalty craters, chronicled.
2. **Departure** — the disloyal kin leaves with the holdings they were **posted to**,
   plus a wealth share, founding a rival house (`found_branch` already does the founding).
3. **Rupture** (Tier 1–2 only, rare) — the house splits along `parent` lines; both halves
   drop a tier; a `FEUD_SUCCESSION` feud opens between them.

**Wealth split** = departing line's headcount ÷ total kin, clamped to **[0.20, 0.45]**.
Requires re-enabling `ENABLE_CADET_BRANCHES` — justified now not as "more houses" but as
a consequence.

**Gate (this step needs its own):** `econ_` Gini must stay inside 0.60–0.85 and
dissolutions/century must not spike — a schism moves the wealth distribution the oracle
scores, which is exactly why decision 3 is the riskiest of the six.

---

## 7. Goals — 17 kinds, AI-chosen (decision 6)

Checkable, with a deadline, so each can fail and be recorded. One active goal per house;
two for Tier 1.

| Goal | Success test | Preferred by |
|---|---|---|
| Corner the ⟨good⟩ trade | monopoly ≥ 60% held 5 yrs | Specialty · Greed |
| Break ⟨house⟩'s monopoly | their share < 30% | Greed + hot feud |
| Own the ⟨good⟩ chain | holds estate **and** manufactory for a recipe | Artisan · Mercantile |
| Seat the council of ⟨city⟩ | `captor_house == self` | Political · Civic |
| See ⟨kin⟩ seated at ⟨city⟩ | a kin `Official` holds a seat | Clannish |
| Raise a bailo at ⟨city⟩ | city in `bailos` | Fleet · Expansive |
| Mint a coin | seat city issues a named coin | Political |
| Charter a bank | owns a solvent bank 10 yrs | Banking |
| Reach ⟨province⟩ | an expedition returns from it | Bold · Expansive |
| Found a colony at ⟨province⟩ | colony survives 10 yrs | Expansive |
| Wed into ⟨house⟩ | an alliance with them | Clannish · Honour |
| Endow a great work | funds a civic wonder | Devout · Civic |
| Fill ⟨city⟩'s granary | civic food reserve above target | Civic |
| Outlast ⟨house⟩ | they go defunct while we live | Greed |
| Outlive the debt | solvent 5 yrs after insolvency | after a debt scare |
| Return to the great houses | Tier 1 regained | after demotion |
| Restore the house | wealth over its historic peak | after ruin or scandal |

A goal biases the **weights** of decisions the AI already makes. It never adds a new
action — that is what keeps this from becoming a second simulation.

---

## 8. Sequence, with the gate for each

| # | Step | New sim state | Gate |
|---|---|---|---|
| 1 | **Tiers** + list grouping | none (pure derivation) | `tsc`; dynamics untouched |
| 2 | **Culture dress figure** on the dossier (+3 house marks, register by tier) | none | `tsc`; schematic renders clean |
| 3 | **Expeditions tab** + province highlight | `Expedition.dest_province` | dynamics bit-identical (field unread by the tick) |
| 4 | **`Character`** on Kin/Official/Figure, derived from culture; phrase only, no effects yet | `Character` ×3 | **all-zero character ⇒ bit-identical** |
| 5 | **`Kin` roster** + holdings authorship | `Kin`, `steward` | no roster ⇒ bit-identical |
| 6 | **Stewards** — skill, wage, skim, poaching | steward fields | dynamics bounded + turnover; `econ_` bands hold |
| 7 | **Character wired to the four knobs** (±15% cap) | none | dynamics bounded; `econ_` bands hold; all-zero still bit-identical |
| 8 | **Goals** | `Goal` | dynamics bounded; goal achieve/fail rate sane over 200 yrs |
| 9 | **Schism** | tension + split | **`econ_` Gini in 0.60–0.85; dissolutions/century must not spike** |

Steps 1–3 cannot regress either oracle (no tick change). Steps 4–9 each need the
dynamics run re-read and the econ scorecard compared before the next begins.

**Also required by §2.7 as each step lands:** CLAUDE.md §5 (the new systems), §6/§7 (new
modules and panels), and a SCOREBOARD row whenever a measured number moves.
