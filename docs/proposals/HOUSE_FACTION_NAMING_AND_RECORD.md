# Factions — names that mean something, and a permanent record

**Status: DESIGN. Nothing built.** Fifth in the series. ASCII schematics only.
Predecessors: `HOUSE_PEOPLE_PLAN.md` · `HOUSE_POWER_AND_POLITICS.md` ·
`HOUSE_SUCCESSION_CRISIS.md` · `HOUSE_POWER_STRUGGLE_VIEW.md`.

Five more decisions:

| # | Decision | Binds |
|---|---|---|
| 1 | Faction names **evocative and unique** (Brotherhood of the Black Axe, Red Lion party), and the two parties in **different colours** | a naming generator + faction tinctures |
| 2 | A **summary historical record** of every crisis | `CrisisRecord`, kept after the crisis ends |
| 3 | When the cause or the stake **changes**, record that it changed and why | `CauseShift` entries inside the crisis |
| 4 | A head who survives gets a **grace period** | `crisis_immune_until` |
| 5 | The undecided bloc is **contestable** each round | a pull roll per camp, per round |

---

## 1. Faction names — built from the heraldry that already exists

The best find in this design: `CoatOfArms.tsx` already carries the exact vocabulary. Named
**tinctures** — gules, azure, vert, purpure, sable, or — and sixteen **charges** —
`lion · eagle · tower · fleur-de-lis · crown · mullet · sun · crescent · rose · garb ·
escallop · sword · key · anchor · boar · trefoil`.

That is precisely how real factions were named: Red Lion, White Rose, Black Eagle, the
Blues and Greens of the hippodrome. So the generator draws from the same palette that
draws the shields, and:

> **The faction's colour IS the tincture in its name.** "the Black Boar" is drawn in
> sable; "the Red Lion" in gules. Name and colour agree by construction, so a player who
> learns the colour has learned the name and vice-versa.

### Patterns

Six patterns, weighted by the faction's role, cause and culture. Each produces a *kind* of
name, so the two parties in a crisis never read alike.

| # | Pattern | Form | Example | Weighted toward |
|---|---|---|---|---|
| 1 | **Tincture + charge** | "the ⟨Red Lion⟩" | the Black Boar · the Azure Key | plots; martial cultures |
| 2 | **Sworn brotherhood** | "the ⟨Brotherhood⟩ of the ⟨Black Axe⟩" | the Company of the Sable Sword | plots; clannish/devout |
| 3 | **Legitimist** | "the ⟨Old Council⟩" | the Elder Bench · the Seat · House and Hearth | loyalists |
| 4 | **Leader's men** | "⟨Ilvar⟩'s men" | the Vareni bloc · Tanmo's hand | either; cheapest fallback |
| 5 | **Place** | "the ⟨Kelmar⟩ party" | the Arsenal men · the Kelmar bench | either; where the crisis is local |
| 6 | **Grievance** | "the ⟨Dispossessed⟩" | the Wronged of Kelmar · the Passed-Over | plots only |

**Culture-specific collective nouns** for patterns 2 and 3, so a Punic plot and a Norse
plot don't sound the same. Drawn from the `KITS` table already indexed by culture:

| Culture | Brotherhood word | Legitimist word |
|---|---|---|
| Roman | Sodalitas · Collegium | the Elders · the Curia |
| Hellene | Hetairia · Synomosia | the Boule |
| Punic | the Compact · the Circle | the Elder Bench |
| Norse | the Félag · the Oathmen | the Thing |
| Celtic | the Fian · the Sworn | the Clanhold |
| Arab | the Bayt · the Pact | the Majlis |
| Persian | the Anjoman | the Divan |
| Slavic | the Druzhina | the Veche |
| Turkic / Mongol | the Nöker | the Kurultai |
| Sinitic | the Guan · the Society | the Censorate |
| Indic / Yamato | the Sangha · the Kō | the Elder House |
| default | the Brotherhood · the Company | the Old Council |

### Two rules

- **Determinism.** Name and tincture hash on `(seed, house, crisis.opened_tick, camp)`, so
  a replayed campaign produces the same factions — and the same crisis referred to in the
  chronicle a century later still has its name.
- **Contrast is mandatory.** The plot's tincture must clear a luminance/hue distance from
  the loyalists'. Loyalists default to the **house's own colour** (they *are* the house);
  the plot picks the furthest tincture from it. Without this a crisis can render as two
  near-identical reds and the whole faction-colour idea collapses.

---

## 2. The undecided are contested (decision 5)

Each round, **both camps pull** at the undecided bloc — so the middle is the battleground,
which is historically how these were won.

```
pull = 0.30·camp_share                      // momentum: winners attract
     + 0.25·persuasion(leader.character)    // Honour+Civic persuade; Greed BUYS
     + 0.20·leader.skill
     + 0.15·money_spent / undecided_weight  // only a Greedy leader spends
     + 0.10·kinship_ties into the bloc
```

Resolved as one roll per camp per round; the winner takes a slice of the undecided
proportional to the gap. A cordial member who is won over **changes camp on the panel** and
gets a line in the round log — "the cordial cousins went to the plot for a promise of the
Kelmar lease."

Money spent this way is real: it leaves house wealth, which can *worsen* the very falling
funds that opened the crisis. A Greedy head buying support while the books bleed is one of
the better tragedies this design can produce, and it comes free from the interaction.

---

## 3. The cause and the stake can change (decision 3)

A crisis is not fixed at its opening. Recorded explicitly, because a shift is a story beat:

```rust
pub struct CauseShift {
    pub round: u8, pub tick: u32,
    pub from_cause: u8, pub to_cause: u8,
    /// Why it moved — always concrete.
    pub text: String,
}
```

| Trigger | Recorded as |
|---|---|
| The head's crisis action backfires badly | "no longer about the funds — about the Sarkoth ships" |
| A foreign hand is discovered mid-crisis | "House Okkath's hand is seen; the quarrel turns on the Kelmar lease" |
| The plot leader dies | "the plot outlived its leader; Zaro takes it up" |
| The heir switches camp | "the heir's turn made it a war of succession, not of accounts" |
| A holding is conceded | "the stake shrank when the dyeworks were given away" |

The **stake** shifts the same way and is recorded the same way: a crisis heading for a
simple deposition, once the plot passes 70%, becomes a crisis that may split the house —
so the footer changes and the change itself is logged. (Answering the earlier open point:
"AT STAKE" is only literally at risk in the split case, so it is labelled by the *likely*
outcome and re-labelled, with a note, when that likelihood moves.)

---

## 4. Grace after survival (decision 4)

A head who survives a crisis gets `crisis_immune_until = tick + 5 years`:

- The plot leader's share is cut hard and their regard floored — they are a marked man.
- The head gains a `PowerModifier`: "+9 survived the rising of 297".
- No new crisis may open in the window. Without this a weak head sits in permanent crisis
  and the mechanic stops meaning anything.

The grace is **visible** on the dossier ("secure until 302"), because a player watching a
bad head should be able to see *why* nothing is happening yet.

---

## 5. The permanent record (decision 2)

Kept per house after the crisis closes, capped at the last ~8, full list behind a
disclosure — the same discipline as the family chronicle.

```rust
pub struct CrisisRecord {
    pub opened_year: u32, pub closed_year: u32,
    pub loyalist_name: String, pub loyalist_tint: String,
    pub plot_name: String, pub plot_tint: String,
    pub ruler: String, pub plot_leader: String,
    pub cause_opened: u8, pub cause_closed: u8,
    pub shifts: Vec<CauseShift>,
    pub rounds: u8,
    /// Peak share each camp reached — how close it actually was.
    pub peak_loyalist: f32, pub peak_plot: f32,
    pub outcome: u8, pub successor: String,
    pub died: Vec<(String, String)>,   // name, how
    pub changed_hands: Vec<String>,    // holdings that moved
    pub foreign: Vec<String>,          // rivals whose hand was in it
}
```

---

## 6. Schematics

### 6a. The struggle, with named factions in their own colours

```
┌ ⚔ House Vareni · POWER STRUGGLE · round 3 of 4 · opened 297 ────── ✕ ┐
│  now about: the Sarkoth ships   (was: three years of falling funds)   │
├────────────────────────────────────┬──────────────────────────────────┤
│  THE ELDER BENCH            ⬤gold │  THE BLACK BOAR         ⬤sable   │
│  Ilvar's legitimists         49%   │  Tanmo's sworn men        39%    │
│  ████████████████████░░░░░░        │  ███████████████░░░░░░░          │
├────────────────────────────────────┼──────────────────────────────────┤
│ ★ Ilvar Vareni           RULER 41% │ ◆ Tanmo Vareni   PLOT LEADER 19% │
│   ▸ to hold what he built          │   ▸ passed over for the seat     │
│ ◆ Sura Vareni             HEIR 8%  │ ◇ Melqa Vareni              11%  │
│   ▐ the heir wavered, then stood   │   ⚠ FOREIGN HAND — House Okkath  │
│     with the ruler (cordial: 50/50)│     293: leases their Kelmar     │
│                                    │     bailo, and they hold it      │
│                                    │ ◇ Zaro Vareni                9%  │
├────────────────────────────────────┴──────────────────────────────────┤
│  UNDECIDED  ▒▒▒▒▒ 12% → 8%   ⚔ contested this round                   │
│    both parties courted the cordial cousins                           │
│    🎲 the Black Boar won them — promised the Kelmar lease   +4%        │
├───────────────────────────────────────────────────────────────────────┤
│  ROUND 1 · spring 297  Ilvar launched a venture to the Sarkoth Reach   │
│    🎲 ✗ BACKFIRED — two ships lost          bench 61% → 52%           │
│    ▐ THE QUARREL CHANGED — no longer about the funds, but about        │
│      the ships                                                        │
│  ROUND 2 · summer 297  Ilvar stood firm, named no concession           │
│    🎲 ○ no effect — hostility hardened      boar 31% → 36%             │
│  ROUND 3 · autumn 297  Ilvar bought support among the cousins          │
│    🎲 ✗ the money was seen — funds fell further, and they took it      │
│      anyway                                  bench 52% → 49%          │
├───────────────────────────────────────────────────────────────────────┤
│  AT STAKE  the seat only — for now                                    │
│    ▐ CHANGED r2: a split is now possible (the Boar passed 35%),        │
│      so 6 holdings and Banco Vareni are at risk too                   │
└───────────────────────────────────────────────────────────────────────┘
```

### 6b. The permanent record

```
┌ ⚔ House Vareni · past risings (4) ────────────────────────────────── ┐
│ 297–298  THE ELDER BENCH ⬤  vs  THE BLACK BOAR ⬤                    │
│   opened over three years of falling funds                          │
│   turned on the Sarkoth ships (r1) · then on the Kelmar lease (r2)   │
│   4 rounds · closest: bench 49% – boar 44%                           │
│   ✕ DEPOSED — Tanmo Vareni seized the seat                           │
│      Ilvar Vareni deposed 298, did not survive the year               │
│      the Kelmar bailo and the Vaskeld estate changed hands            │
│      House Okkath's hand was in it                                    │
│                                                                     │
│ 264–265  THE OATHMEN ⬤  vs  THE AZURE KEY ⬤                         │
│   opened over the head's vice (lavish)                              │
│   2 rounds · ✓ THE RULER PREVAILED — Zaro's party broken            │
│      Zaro Vareni marked; secure until 270                           │
│ ▸ 2 earlier risings                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

### 6c. Grace, visible on the dossier

```
│ ★ RULER  Tanmo Vareni                            52% ▲               │
│   bold · grasping                    head since 298                  │
│   ✓ survived nothing yet — took the seat by the rising of 297         │
│   🛡 secure until 303 — the house will not rise again so soon         │
```

---

## 7. Sequence position

Amends `HOUSE_POWER_STRUGGLE_VIEW.md` §7:

| # | Step | Gate |
|---|---|---|
| 8b | Crisis: open · **named factions + tints** · heir choice · rounds · resolve | `every_crisis_terminates`; **faction tints must clear a contrast floor** (a unit test over many seeds, exactly like the biome-contrast test in §8.12) |
| 8c | **Contested undecided** + cause/stake shifts + grace period | deposition rate sane; **money spent courting must not become a wealth sink large enough to move the econ scorecard** |
| 8d | **`CrisisRecord`** (permanent, capped) | save-size growth bounded over 500 yrs |
| 9 | Schism | `econ_` Gini in 0.60–0.85; dissolutions must not spike |

Invariants, now five:

- `power_shares_always_sum_to_100`
- `a_house_with_no_kin_is_bit_identical`
- `every_crisis_terminates`
- `allegiance_partitions_the_house`
- **`faction_names_and_tints_are_distinct`** — over a large seed sweep, the two camps in a
  crisis never share a name or a tint within the contrast floor. This is the same class of
  test as `every_biome_pattern_tiles_seamlessly`: cheap, and it catches the one failure
  that would silently ruin the feature.
