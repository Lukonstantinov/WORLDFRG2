# The succession crisis — a struggle with rounds, not a threshold

**Status: DESIGN. Nothing built.** Third in the series, after `HOUSE_PEOPLE_PLAN.md` and
`HOUSE_POWER_AND_POLITICS.md`. ASCII schematics only.

Five more decisions taken:

| # | Decision | Binds |
|---|---|---|
| 1 | The relation ladder's top rung is **loyal**, not "postal" | four rungs: loyal · friendly · cordial · hostile |
| 2 | **Observation only** for now — no player verbs | the AI supplies every choice; no new mutating commands |
| 3 | Deposition is **not instant**: a crisis opens, a struggle runs over several rounds, the head's handling is rolled | new `HouseCrisis` object with rounds |
| 4 | A plague death names **which plague, its years, and the year he died** | epidemics already carry all of it |
| 5 | A deposed ruler **dies** (exile later, if ever) | no bitter ex-ruler faction |

---

## 1. The relation ladder (decision 1)

Stored as a continuous `regard: f32` (0..1) that drifts yearly, **bucketed** into four
labels for display. Continuous drift means a relation can worsen gradually; four labels
mean the player reads a word, not a decimal.

| Bucket | Label | In a struggle |
|---|---|---|
| ≥ 0.75 | **loyal** | backs the head with their full share |
| 0.50–0.74 | **friendly** | backs the head |
| 0.25–0.49 | **cordial** | abstains — their share sits in neither camp |
| < 0.25 | **hostile** | backs the plot; the largest hostile share becomes the **plot leader** |

Every relation carries the reason that moved it most (the table in
`HOUSE_POWER_AND_POLITICS.md` §3), so the label is never bare.

---

## 2. The crisis (decision 3)

Discontent crossing its threshold no longer deposes anyone. It **opens a crisis** — a
named, dated object that resolves over years, so a deposition is a story with a middle
rather than an event.

```rust
pub struct HouseCrisis {
    pub house: u32,
    pub opened_tick: u32,
    /// 0 falling funds · 1 failed ambitions · 2 the head's vice
    /// · 3 a hostile faction · 4 a lost venture · 5 a lost feud
    pub cause: u8,
    /// Kin index of the challenger, or −1 = leaderless discontent (which the head
    /// finds much easier to survive).
    pub plot_leader: i32,
    pub round: u8,
    /// Share backing each camp, recomputed every round from live regard.
    pub head_support: f32,
    pub plot_support: f32,
    pub rounds: Vec<CrisisRound>,
    /// 0 running · 1 the head prevailed · 2 deposed · 3 the house dissolved · 4 split
    pub outcome: u8,
    pub ended_tick: u32,
    /// Who took the seat (kin index) when the outcome is deposition.
    pub successor: i32,
}

pub struct CrisisRound {
    pub tick: u32,
    /// What the head ATTEMPTED — chosen by his character, not at random.
    pub action: u8,
    /// −1 backfired · 0 no effect · +1 worked
    pub result: i8,
    pub support_delta: f32,
    pub text: String,
}
```

### Cadence

A round every **quarter** (90 ticks). The crisis runs **3–5 rounds** (~1 year), then
resolves. Long enough that the player can watch it, short enough that it doesn't become
the permanent state of the house.

### The head's move each round — chosen by CHARACTER

This is what makes the struggle a story rather than a dice check: *how* a head fights is
determined by who he is, and each option uses machinery that already exists.

| Action | Chosen when | Effect if it works | If it backfires |
|---|---|---|---|
| **Concede a holding** to a rival | Honourable, or Cautious | plot leader's regard rises sharply | reads as weakness — cordial members drift hostile |
| **Buy off the plot** | Greedy, and the house is liquid | hostile shares soften | the money is seen; funds fall further |
| **Marry a rival's line in** | Clannish culture, Honourable | the plot leader becomes kin, regard jumps | the match is refused publicly |
| **Launch a venture** | Bold, Expansive | a win restores his standing outright | a loss is fatal — the classic overreach |
| **Press a feud to a win** | Rapacious, Martial culture | prestige and power both rise | the feud escalates and costs more |
| **Stand firm** | Reckless, or Miserly (won't pay) | discontent stalls for a round | hostility hardens |

### The roll

```
p(work) = 0.25 + 0.45·skill + 0.10·(head_support − plot_support)
        + character_fit_bonus(action)          // acting in character helps
        − 0.15·vice_severity
```

Deterministic per `(seed, house, crisis, round)` — the same campaign replays identically.
`skill` is the dominant term, which is the point: **an incompetent head fails his own
crisis.** Acting *in character* is rewarded, so a Bold head is genuinely better off
gambling on a venture than conceding, and a Cautious one the reverse.

### Resolution after the final round

| Condition | Outcome |
|---|---|
| `head_support > plot_support` | **The head prevails.** Plot leader's share cut hard, regard floors, hostility recorded. He is now a marked man. |
| plot wins, and a successor is available | **Deposed.** The old head **dies** (decision 5). |
| plot wins, `plot_support ≥ 0.7`, and the house is Tier 1–2 with a big hostile share | **Split** (rare) — holdings + wealth, wealth moves. Reason recorded on both halves. |
| plot wins but no viable successor **and** the house is insolvent | **Dissolved** (very rare) — the family ends in its own quarrel rather than in bankruptcy. |

### Who takes the seat

In order of likelihood, and each writes a different chronicle line:

1. **The plot leader** — "seized the seat"
2. **The heir**, if their regard is ≥ cordial and skill is not the worst — "the succession held"
3. **Another prominent figure** — a compromise candidate: "the house settled on Melqa"

---

## 3. Death causes, named (decision 4)

`Kin` carries a `death_cause: String` and `death_year: u32`, both written at death. Every
input already exists — `Epidemic` carries `name`, `disease`, `origin_name`, `start_year`,
`end_year`, `category`.

| Cause | Rendered |
|---|---|
| Age | "died 291, aged 68" |
| **Plague** | "died 288 of the **Bubonic Plague** — the Ostrahn pestilence of 286–291" |
| Lost voyage | "lost 296 with the Sarkoth venture" |
| War levy | "died 284 in the war with Kelmar" |
| **Deposed** | "deposed 297 and did not survive the year" |

The plague line names the disease, the epidemic, its span **and** the year of death, so a
figure's end is dated inside a real world event rather than being an unexplained
disappearance.

---

## 4. Schematics

### 4a. A crisis in progress

```
┌ ⚜️ House Vareni · 👥 Figures ─────────────────────── ✕ ┐
│ ⚠ SUCCESSION CRISIS · opened 297 · round 2 of 4        │
│   cause: three years of falling funds                  │
│   Tanmo Vareni leads the plot                          │
│                                                        │
│   HEAD  ████████████████████░░  52%   Ilvar            │
│   PLOT  ██████████████░░░░░░░░  36%   Tanmo            │
│   undecided (cordial + pool)     12%                   │
│                                                        │
│   ROUND 1 · spring 297                                 │
│    Ilvar launched a venture to the Sarkoth Reach       │
│    ✗ BACKFIRED — two ships lost, funds fell further    │
│      head 61% → 52%                                    │
│   ROUND 2 · summer 297                                 │
│    Ilvar stood firm and named no concession            │
│    ○ no effect — hostility hardened                    │
│      head 52% → 52%,  plot 31% → 36%                   │
│   ROUND 3 · autumn 297 …                               │
│                                                        │
│   ▐ Ilvar is reckless and unskilled. He gambles when   │
│     he should concede, and the odds are against him.    │
└────────────────────────────────────────────────────────┘
```

### 4b. Resolved, in the record

```
┌ 📜 House Vareni · chronicle ──────────────────────────┐
│ 298 ⚡ DEPOSED — after four rounds of struggle the      │
│       house turned on Ilvar Vareni. Tanmo Vareni       │
│       seized the seat. Ilvar did not survive the year. │
│       cause: three years of falling funds              │
│       · head 61% → 34% over the crisis                 │
│       · the Sarkoth venture (296) broke him            │
│ 297 ⚠ a succession crisis opened                       │
│ 296 ✗ the Sarkoth venture failed — two ships lost      │
└───────────────────────────────────────────────────────┘
```

### 4c. The roster after, with named deaths

```
│ ── GONE ─────────────────────────────────────────────── │
│  ✕ Ilvar Vareni    head 256–298                        │
│      deposed 298 and did not survive the year          │
│  † Bodo Vareni     steward, Kelmar                     │
│      died 288 of the Bubonic Plague — the Ostrahn       │
│      pestilence of 286–291                             │
│  † Melqa Vareni    lost 296 with the Sarkoth venture   │
│  → Odarra Vareni   married out 271 → House Sedhri      │
```

---

## 5. What decision 2 (observation only) simplifies

No new mutating commands. The whole layer is read-only IPC:
`campaign_house_figures(idx)` and `campaign_house_crisis(idx)`. Every choice — the head's
crisis action, who is promoted, who is elected — is made by the AI.

This also means the `decide_X`/`apply_X` split (FIX_PLAN B2) is **not** required yet, but
the crisis action SHOULD be written as `decide_crisis_action(&self, …) -> CrisisAction`
from the start, because that is the seam a player verb would later plug into, and writing
it pure costs nothing now.

---

## 6. Sequence position

Slots into `HOUSE_POWER_AND_POLITICS.md` §8 in place of step 8b:

| # | Step | Gate |
|---|---|---|
| 5b | Power shares + relations + modifiers (read-only) | shares sum to 100; no kin ⇒ bit-identical |
| 7b | Competence + vice | dynamics bounded; house death-rate must not spike |
| 8 | Goals (head-chosen) | achieve/fail rate sane over 200 yrs |
| **8b** | **Crisis: open, rounds, resolve** | dynamics bounded; **crisis→deposition rate sane over 300 yrs**; a crisis must always terminate (no crisis older than ~6 rounds) |
| 9 | Schism (via crisis outcome 4) | `econ_` Gini in 0.60–0.85; dissolutions must not spike |

Three invariants to test explicitly:

- **`power_shares_always_sum_to_100`**
- **`a_house_with_no_kin_is_bit_identical`**
- **`every_crisis_terminates`** — no crisis may run past its round cap, and a house may
  hold at most one open crisis. Without this an unresolved crisis becomes the permanent
  state of a house and the politics layer silently stops meaning anything.
