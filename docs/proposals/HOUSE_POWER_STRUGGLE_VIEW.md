# The power struggle — two parties, named motives, a foreign hand

**Status: DESIGN. Nothing built.** Fourth in the series. ASCII schematics only.
Predecessors: `HOUSE_PEOPLE_PLAN.md` · `HOUSE_POWER_AND_POLITICS.md` ·
`HOUSE_SUCCESSION_CRISIS.md`.

Five more decisions taken:

| # | Decision | Binds |
|---|---|---|
| 1 | A crisis shows in **both** the houses list and the dossier | a ⚠ chip on the row + the struggle window |
| 2 | Foreign influence only through a **concrete commercial channel** (their office/bailo in our city; our member leasing in their city) | leverage is derived from `offices`/`bailos`/`office_leases`/`captor_house` — no abstract conspiracy |
| 3 | The **heir chooses**: joins the ruler, joins the plot, stands aside, or (rarely) splits — cordial regard is a 50/50 roll, and the choice is marked | heir allegiance is its own recorded event |
| 4 | If the ruler **dies mid-crisis**, leadership of the loyalists passes to the heir or a prominent loyalist, and the struggle may end there or run a fresh round | crisis survives its subject |
| 5 | **1–7** prominent figures, by *personal* power — including kin who run a manufactory or sit on a city council | `power_pct` is earned from what a person actually runs |

---

## 1. Power is earned from what a person runs (decision 5)

A figure's share is not assigned — it is derived from their actual position, which is why
a manufactory steward and a kinsman on the council can both matter:

| Source of personal power | Basis (all existing state) |
|---|---|
| Stewards a holding | the holding's throughput ÷ the house's total |
| Sits on a city council | `Official` seat weight (head 2.0 · treasurer 1.4 · other 1.0) |
| Runs the house bank | the bank's equity ÷ house wealth |
| Led a profitable venture | `Expedition.revenue` banked, decaying |
| Brought a marriage alliance | a standing bonus while the alliance lives |
| Named heir | a flat share for the expectation |

Prominence count therefore **emerges** — a house with one estate and no offices has one
or two figures; a sprawling Tier 1 house with a bank, four stewards and two kin on
councils has seven. Cap at 7; anyone below ~3% falls back into the pool.

> **Reconciling an earlier decision.** `Kin` and `Figure` remain separate populations, as
> decided. But a **`Kin` may occupy an `Official` seat** — `Official.kin: bool` already
> exists and is currently only a flag. Making it point at a kin index is what lets "city
> council members of the same family" hold power.

---

## 2. The foreign hand (decision 2)

Two channels, both concrete, both derived from state that exists. Neither invents a
conspiracy — they model **dependency**, which is how commercial pressure actually worked.

**Channel A — their presence in our city.** A rival house holds an office or bailo in the
city where our member sits. Proximity gives them a voice.

**Channel B — our member is their tenant.** Our member holds a lease
(`office_leases`) on an office or bailo in a city the rival *controls*
(`captor_house == rival`, or the rival is dominant there). The rival can threaten the
lease. This is the strong channel, because it is real dependency rather than mere contact.

```
leverage = 0.5·channel_A + 0.8·channel_B
         × rival_weight(wealth, prestige)
         × (1 + 0.5·feud_with_us)      // a rival already quarrelling with us is motivated
```

**What leverage does — and does not do:**

- It **deepens an existing grievance**: it lowers that member's regard for our head and
  adds a named annotation to their row.
- It **cannot create** the plot. A loyal, contented member is not turned by a rival's
  bailo. This is the line that keeps the mechanic as pressure rather than as the intrigue
  layer that was excluded.
- It is **always disclosed** — "bribed by House Okkath in 293 — she leases their bailo at
  Kelmar" names the rival, the year, and the actual dependency.

---

## 3. The heir chooses (decision 3)

Evaluated once, when the crisis opens, and recorded:

| Heir's regard | Choice | Marked as |
|---|---|---|
| ≥ 0.50 (friendly/loyal) | joins the **loyalists** | "the heir stood with the ruler" |
| < 0.25 (hostile) | joins the **plot** | "the heir turned" |
| 0.25–0.49 (cordial) | **50/50 roll** | "the heir wavered and chose the ruler" / "…the plot" |
| — | rare: **stands aside** | "the heir would not take a side" — share moves to undecided |
| — | very rare: **splits** | "the heir left with her own line" → schism, reason recorded |

An heir who stands aside is the most dangerous case for the head: the succession's own
weight sits in the undecided column where either side may still win it.

---

## 4. The ruler dies mid-struggle (decision 4)

Plague, age or a lost voyage can take him in round 2. The crisis does **not** simply
evaporate:

1. Leadership of the loyalists passes to the **heir** if the heir is in that camp;
   otherwise to the **highest-share prominent loyalist**.
2. Power re-seats: the new leader gains a portion of the dead head's share (the rest
   returns to the pool — a dead man's authority is not inherited whole).
3. The struggle re-evaluates immediately:
   - new leader's camp still ahead → **crisis ends, loyalists prevail**, the new leader is
     seated as head.
   - plot now ahead → **a fresh round sequence** opens under the new leader (round counter
     resets; a hard total-round cap still applies, per `every_crisis_terminates`).
   - no loyalist remains → **the plot wins outright**.

This is the "power should go to the new leader of the loyalists" case, and it makes a
natural death mid-crisis a genuine turn rather than an anticlimax.

---

## 5. The struggle window (the layout you specified)

```
┌ ⚔ House Vareni · POWER STRUGGLE · round 2 of 4 · opened 297 ────── ✕ ┐
│  cause: three years of falling funds                                  │
├────────────────────────────────────┬──────────────────────────────────┤
│  THE LOYALISTS                     │  THE KELMAR PARTY                │
│  (Ilvar's men)               52%   │  (Tanmo's faction)         36%   │
│  ████████████████████░░░░░░        │  ██████████████░░░░░░░░░         │
├────────────────────────────────────┼──────────────────────────────────┤
│ ★ Ilvar Vareni           RULER 41% │ ◆ Tanmo Vareni   PLOT LEADER 19% │
│   ▸ to hold what he built          │   ▸ passed over for the seat     │
│     bold · grasping · ⚠ reckless   │     bold · grasping              │
│                                    │                                  │
│ ◆ Sura Vareni             HEIR 12% │ ◇ Melqa Vareni               8%  │
│   ▸ the succession is hers         │   ▸ kept at the seat, given      │
│     ● friendly — "given the        │     little                       │
│       dyeworks by the head"        │   ⚠ FOREIGN HAND — House Okkath, │
│   ▐ the heir stood with the ruler  │     293: she leases their bailo  │
│                                    │     at Kelmar and they hold it   │
│ ◇ Hanno Vareni                 4%  │                                  │
│   ▸ kin by the Tavric match        │ ◇ Zaro Vareni                9%  │
│     ● loyal                        │   ▸ his ships went down on the   │
│                                    │     ruler's venture              │
├────────────────────────────────────┴──────────────────────────────────┤
│  UNDECIDED  ▒▒▒▒▒▒▒ 12%    two cordial cousins · the pool             │
├───────────────────────────────────────────────────────────────────────┤
│  ROUND 1 · spring 297   Ilvar launched a venture to the Sarkoth Reach  │
│    🎲 ✗ BACKFIRED — two ships lost, funds fell further                │
│         loyalists 61% → 52%                                           │
│  ROUND 2 · summer 297   Ilvar stood firm and named no concession       │
│    🎲 ○ no effect — hostility hardened                                │
│         plot 31% → 36%                                                │
│  ROUND 3 · autumn 297   …                                             │
├───────────────────────────────────────────────────────────────────────┤
│  AT STAKE    6 holdings · 2 cities · Banco Vareni · the Kelmar bailo   │
│  FOREIGN HAND  House Okkath ⚔ (in feud with us) — bailo at Kelmar     │
│  ▐ Ilvar is reckless and unskilled: he gambles where he should         │
│    concede, and the odds run against him.                             │
└───────────────────────────────────────────────────────────────────────┘
```

**Colour discipline** (`motive` coloured, per your sketch):
- loyalist column tinted the house colour; plot column tinted a contrasting claret
- motive line (`▸`) in the *figure's own* tint so a motive reads as personal
- ⚠ FOREIGN HAND always in the **rival's** colour — the rival is identified by hue too
- undecided in neutral grey, deliberately colourless: it belongs to no-one

### The crisis chip on the houses list (decision 1)

```
│ ⬤[arms] House Vareni    🏦 ⚖ 👑  ⚠ CRISIS r2/4   ████████ 42k │
│    Ilvar Vareni · Ostrahn · gen 4 · Ashkar                    │
│    ⚠ Tanmo's faction holds 36% against the ruler's 41%        │
```

One line, and it lets a turbulent era be *seen* — "three houses in crisis this decade" —
without opening anything.

---

## 6. New state this adds

```rust
pub struct HouseCrisis {
    // …fields from HOUSE_SUCCESSION_CRISIS.md…
    /// Party names, generated from the leaders / their seats.
    pub loyalist_name: String,     // "Ilvar's men"
    pub plot_name: String,         // "the Kelmar party"
    /// Per-member allegiance this round: kin index → camp.
    pub allegiance: Vec<(u32, u8)>, // 0 loyalist · 1 plot · 2 undecided
    /// The heir's recorded choice + how it was reached.
    pub heir_choice: u8,           // 0 ruler · 1 plot · 2 stood aside · 3 split
    pub heir_choice_text: String,
    /// Foreign leverage found, disclosed in full.
    pub foreign: Vec<ForeignHand>,
    /// Leadership changes mid-crisis (the ruler died and was replaced).
    pub leader_changes: Vec<(u32, u32, String)>, // tick, new leader kin, why
}

pub struct ForeignHand {
    pub rival_house: u32,
    pub target_kin: u32,
    /// 0 their office/bailo in our city · 1 our member leases in their city
    pub channel: u8,
    pub since_year: u32,
    pub strength: f32,
    pub text: String,   // "she leases their bailo at Kelmar and they hold it"
}
```

Each figure gains `motive: String` — one line, generated from their largest driver
(ambition, grievance, kinship, dependency), because a power score without a motive is
just a number in a column.

---

## 7. Sequence position

Amends `HOUSE_SUCCESSION_CRISIS.md` §6 — same slot, more inside it:

| # | Step | Gate |
|---|---|---|
| 8b | Crisis: open · parties · heir choice · rounds · resolve | dynamics bounded; **`every_crisis_terminates`**; deposition rate sane over 300 yrs |
| 8c | **Foreign hand** (leverage from offices/bailos/leases) | must NOT raise the deposition rate materially — leverage deepens a grievance, it does not manufacture one. Measure the rate with the channel off vs on. |
| 9 | Schism (crisis outcome 4, or the heir splitting) | `econ_` Gini in 0.60–0.85; dissolutions must not spike |

The gate on 8c is the important one and it is falsifiable: if turning the foreign channel
on measurably increases how often houses fall, then it is manufacturing plots rather than
colouring them, and the multiplier is wrong.

Invariants, now four:

- `power_shares_always_sum_to_100`
- `a_house_with_no_kin_is_bit_identical`
- `every_crisis_terminates`
- **`allegiance_partitions_the_house`** — every prominent figure is in exactly one camp,
  and loyalist + plot + undecided = 100. Without it the two columns can silently disagree
  with the pie, which is exactly the sort of bug nobody notices for months.
