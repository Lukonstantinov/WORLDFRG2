# The house as a body politic — power shares, relations, deposition

**Status: DESIGN. Nothing built.** Extends `HOUSE_PEOPLE_PLAN.md`. ASCII schematics only.

Four more decisions taken, then the new system:

| # | Decision | Binds |
|---|---|---|
| 1 | Characters **visible** — in the city panel for officials, in the house panel for family | two new UI blocks, not just silent effects |
| 2 | `Kin` and `Figure` stay **separate populations** | the house's power roster is KIN; world notables stay `Figure` |
| 3 | Schism wealth **moves**, is not destroyed | neutral to total wealth; tier demotion is the punishment |
| 4 | Goals are the **head's**, and a bad head can be **fatal** | needs competence, vice, and a way for the house to react |

---

## 1. The core invariant: power is a 100% pie with a derived remainder

Every prominent member of a house holds a **power share**. The remainder is the pool.

```
pool_pct = 100 − Σ prominent shares        // DERIVED, never stored
```

Storing the pool would let rounding drift break the invariant. Deriving it means the pie
always sums to 100 by construction, and it makes the pool mean something real: **influence
held by members too minor to name.**

That gives the centralisation reading you asked for:

| Shape | Reads as |
|---|---|
| head 58%, pool 8% | **centralised** — one man is the house |
| head 24%, 4 figures at ~15%, pool 16% | **an oligarchy of cousins** |
| head 19%, pool 61% | **diffuse** — no-one holds a third; the house acts by inertia |

`centralisation = head_pct + 0.5·heir_pct` for the one-line phrase; the full shape is the
bar. A diffuse house is *harder to depose but slower to act* — that is the trade the
number should express.

---

## 2. What moves a share

Zero-sum, and **gains come out of the pool first**, then pro-rata from the others. Never
renormalise the whole pie — that silently moves everyone and makes the modifier list a
lie.

Every change writes a `PowerModifier`, because you asked to see *why*:

```rust
pub struct PowerModifier {
    pub tick: u32,
    pub delta: f32,        // percentage points, signed
    pub reason: String,    // "silk monopoly won", "three years of falling funds"
    pub kind: u8,          // 0 economic · 1 goal · 2 character/vice · 3 tier
                           // · 4 venture · 5 politics · 6 kinship
}
```

Head's share moves from:

| Source | Direction | Basis (all existing state) |
|---|---|---|
| **Funds rising / falling** | ± | `wealth_history` year-on-year — this is your "power gained when good economic decisions are made" |
| **Goal achieved / failed** | ± | `Goal.state` |
| **Tier promotion / demotion** | ± | the standing score |
| **Vice** | − | derived from character + skill (§4) |
| **Venture returned / lost** | ± | `Expedition.status`, `revenue` |
| **A feud lost / won** | ± | `Feud.damage_*` |
| **Long successful tenure** | + small/yr | consolidation |
| **Long failing tenure** | − | erosion |

Other figures gain from the mirror image: a steward whose holding thrives, a kin who
brought a marriage alliance, a cousin who led a profitable venture.

---

## 3. Relation to the ruler — typed, with a reason

Each prominent figure carries a relation to the current head:

```rust
pub relation: i8,        // −2..+2
pub relation_reason: String,
```

| Value | Label | Effect |
|---|---|---|
| +2 | **devoted** | lends their share to the head in a deposition vote |
| +1 | **friendly** | votes with the head |
| 0 | **cordial** | abstains |
| −1 | **cold** | votes against |
| −2 | **hostile** | votes against and may lead a faction |

> **Open question — one term.** Your list read *"friendly, postal, cordial, hostile"*.
> "postal" does not map to anything; the ladder above assumes you meant a neutral or
> distant rung. Confirm the five labels you want.

Relation drifts yearly, and the **reason is the largest contributing term** — so the text
is always true rather than decorative:

| Cause | Direction | Reason text |
|---|---|---|
| Passed over for the succession | −2 | "passed over when Sura was named heir" |
| Posted far from the seat | −1 | "kept at Kelmar, far from the seat" |
| Character distance from the head | −1 | "has no patience for the head's ventures" |
| Character similarity | +1 | "shares the head's temper" |
| Raised up by the head (given a holding) | +1 | "given the Vaskeld estate by the head" |
| Married in / alliance kin | +1 | "kin by the Tavric match" |
| The head's feud closed their market | −1 | "lost her Kelmar trade to the head's quarrel" |
| The head's venture lost their ships | −2 | "her brother's ships went down on the head's venture" |
| A goal failed on their watch | −1 | "blamed for the failed charter" |

---

## 4. Competence and vice — how a head becomes fatal

Two things beyond the four character axes, both **derived** so there is no third random
layer (culture still dominates, per the earlier decision):

- **Competence** = `Kin.skill` (already in the plan). Multiplies the quality of the
  house's decisions: a low-skill head picks worse goals and mistimes fleet purchases.
- **Vice** = a named consequence of character extremes plus low skill:

| Vice | Derived from | Mechanism (existing knob) |
|---|---|---|
| **Lavish** | Civic ≥ +1 **and** skill ≤ 0.4 | raises `HOUSE_CONSUMPTION_RATE` — the house's own spending bleeds it |
| **Reckless** | Bold ≥ +2 | over-buys hulls, launches thin ventures — `decide_fleets`, expedition gate |
| **Rapacious** | Greed ≥ +2 | escalates feuds it cannot win — `FEUD_HEAT` |
| **Miserly** | Bold ≤ −2 **and** Private ≤ −1 | under-invests; volume decays and standing slides |
| **Parochial** | Rooted ≤ −2 | refuses offices; misses expansion entirely |

A vice is **not** a hidden penalty — it is named on the dossier and it appears as a
`PowerModifier` every year it costs the house money. That is what makes a bad head
legible instead of merely unlucky.

---

## 5. The house reacts — four outcomes, in one ordered decision

Yearly `house_politics_pass`. Deterministic and explainable — the order *is* the rule:

```
discontent = 0.45·falling_funds        // years of decline, normalised
           + 0.25·failed_goals_recent
           + 0.20·mean_hostility        // Σ share-weighted negative relations
           + 0.10·vice_severity
```

Below the threshold: nothing happens (and nothing is written — silence is the common case).
Above it, the **first** applicable outcome fires:

| Order | Outcome | Condition | What happens |
|---|---|---|---|
| 1 | **Suffer** | `head_pct ≥ 55` | The head is too strong to remove. Discontent is recorded, the house keeps bleeding, hostility rises further. *Your "suffer from the ruler if he has enough power".* |
| 2 | **Depose** | a challenger's share + allied shares > head's share + allied shares | The head is removed (role → idle, share collapses to a fifth), the challenger becomes head with their own character and goals. Reason recorded. |
| 3 | **Venture** | the house is liquid and a Bold/Expansive faction holds ≥ 20% | A pressure valve: an expedition or colony is launched. Buys the head time; the outcome swings their share hard either way. *Your "send ventures".* |
| 4 | **Split** | rare — hostility ≥ 0.7 **and** a hostile figure holds ≥ 25% | Schism (holdings + wealth, wealth moves). **The reason is recorded on both halves.** |

Each outcome writes a `HouseEvent` **and** a journal beat, so a deposition is as readable
a century later as a bankruptcy is now.

---

## 6. Prominence — who is on the list, and how they leave

- **Yearly**, one member may rise into prominence, taking a share out of the pool. Chance
  scales with their skill and whatever they run.
- They leave the list by: **deposition** (share collapses, drops out), **death**, or
  **departure** (schism / married out).
- **Death causes are real, not a dice roll:** age (`dies_tick`), **plague** (hook the
  existing epidemic state — a figure in a locked-down city can die of it), a lost voyage
  (they led an expedition that sank), war levies.
- A dead or deposed figure's share **returns to the pool**, which is why the pool must be
  derived rather than stored.
- Departed figures stay on the record with how they went — the family's permanent list.

---

## 7. Schematics

### 7a. The house's body politic

```
┌ ⚜️ House Vareni · 👥 Figures ─────────────────────── ✕ ┐
│ POWER  head 41% · heir 12% · 3 figures 31% · pool 16%  │
│ ████████████████░░░░░▒▒▒▒▒▒▒▒▒▒▒░░░░░░                 │
│ ▐ centralised under Ilvar — but not beyond challenge    │
├────────────────────────────────────────────────────────┤
│ ★ RULER  Ilvar Vareni                          41% ▼   │
│   bold · grasping · ⚠ RECKLESS        62y, head since  │
│   [figure]                                   256       │
│   WHY HIS POWER MOVED                                  │
│    +8  silk monopoly won                        271    │
│    +3  raised to the great houses               271    │
│    −6  three years of falling funds             294    │
│    −4  the Sarkoth venture lost two ships       296    │
│    −2  reckless — hulls bought the house cannot use    │
│   ▸ 9 earlier modifiers                                │
│                                                        │
│ ◆ HEIR   Sura Vareni                           12% ▲   │
│   cautious · honourable               34y   relation:   │
│   ● friendly — "given the dyeworks by the head"        │
│                                                        │
│ ── PROMINENT (3) ───────────────── appear yearly ────── │
│ ◇ Tanmo Vareni   steward, Kelmar               19% ▲   │
│   bold · grasping                                      │
│   ● HOSTILE — "passed over when Sura was named heir"   │
│   ⚠ his share and Ilvar's are within 22 points          │
│ ◇ Melqa Vareni   led the Ashen venture          8% ▲   │
│   ● cordial — "kept at the seat, given little"         │
│ ◇ Hanno Vareni   the Tavric match               4% ─   │
│   ● friendly — "kin by the Tavric match"               │
│                                                        │
│ ── POOL · unaligned kin ──────────────────────── 16% ─ │
│   Six members hold no separate weight. A large pool     │
│   makes the house hard to seize — and slow to act.      │
│                                                        │
│ ── DISCONTENT ────────────────────────────────────────  │
│   ███████░░░ 0.68   ⚠ above the threshold               │
│   falling funds · one failed charter · Tanmo hostile   │
│   → Ilvar holds 41%: NOT strong enough to simply       │
│     endure this. A challenge is possible.              │
│                                                        │
│ ── GONE ───────────────────────────────────────────────│
│  † Bodo Vareni    d. 288  plague at Kelmar             │
│  ✕ Zaro Vareni    deposed 264 — "the lost decade"      │
│  → Odarra Vareni  married out 271 → House Sedhri       │
└────────────────────────────────────────────────────────┘
```

### 7b. Goals, with failures kept (decision 4)

```
┌ 🎯 Ambitions ─────────────────────────────────────────┐
│ PURSUING (set by Ilvar, reckless)                     │
│  corner the silk trade      ██████░░░░ 61%   by 302   │
│ ACHIEVED (3)                                          │
│  ✓ 284  raised a bailo at Kelmar                      │
│  ✓ 271  cornered the dyestuff trade                   │
│  ✓ 268  chartered Banco Vareni                        │
│ FAILED (2)                          ← kept, capped 6  │
│  ✗ 296  reach the Sarkoth Reach — two ships lost      │
│  ✗ 279  seat the council of Ostrahn — outbid          │
│ ▸ full record (11)                                    │
└───────────────────────────────────────────────────────┘
```

Last 6 of each shown, full list behind a disclosure — the record is permanent (like the
family chronicle) but the panel stays readable over 500 years.

### 7c. Character in the city panel (decision 1)

```
┌ 🏙 Ostrahn · Government ──────────────────────────────┐
│ Council · captured by House Vareni                    │
│ 👤 Doge        Ilvar Vareni      👪 Vareni    ●●●●● 1.0│
│    bold · grasping · ⚠ reckless                        │
│ 👤 Treasurer   Bodo Ashken       —           ●●●○○ .62│
│    greedy · private        ⚠ cheap to buy              │
│ 👤 Harbourm.   Sura Kelmet       —           ●○○○○ .18│
│    bold · civic                                        │
│ 👤 Magistrate  Tanit Okkath      👪 Okkath    ●●●●○ .81│
│    honourable · civic      resists capture              │
└───────────────────────────────────────────────────────┘
```

Officials' characters come from the **city's** culture, not the house's — so a captured
council still staffs itself from local people, and a greedy treasurer being cheap to bribe
becomes visible rather than invisible arithmetic.

---

## 8. Where this fits the sequence

Insert into `HOUSE_PEOPLE_PLAN.md` §8 after step 5 (`Kin` roster), because power shares
need kin to attach to:

| # | Step | Gate |
|---|---|---|
| 5 | `Kin` roster + holdings authorship | no roster ⇒ bit-identical |
| **5b** | **Power shares + relations + modifiers** (read-only: no deposition yet) | no roster ⇒ bit-identical; shares always sum to 100 |
| 6 | Stewards | dynamics bounded; `econ_` bands hold |
| 7 | Character → knobs (±15%) | all-zero ⇒ bit-identical |
| **7b** | **Competence + vice** | dynamics bounded; house death-rate must not spike |
| 8 | Goals (head-chosen) | achieve/fail rate sane over 200 yrs |
| **8b** | **`house_politics_pass`** — suffer / depose / venture | dynamics bounded; deposition frequency sane (a house should not change head every few years) |
| 9 | Schism | `econ_` Gini in 0.60–0.85; dissolutions must not spike |

Two invariants to test explicitly:

- **`power_shares_always_sum_to_100`** — after any pass, for every house.
- **`a_house_with_no_kin_is_bit_identical`** — the whole layer is opt-in.
