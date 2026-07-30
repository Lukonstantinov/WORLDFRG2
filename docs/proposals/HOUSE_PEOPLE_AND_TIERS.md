# Houses as people — tiers, kin, character, goals, expeditions, schism

**Status: DESIGN ONLY. Nothing here is built.** Schematics are ASCII on purpose (§2.2
bans HTML mockups; the `dev/` harness renders real components once something exists).

Six commissions: culture dress + house tiers · goals + head character · expeditions in
the house panel · a holdings roster naming who holds what · internal friction and
schism · more AI strategies, story-shaped.

---

## 0. Audit — what already exists

| Ask | Already built | Gap |
|---|---|---|
| Culture dress | **`cultureFigure.ts` (298 lines)** — 18 kits, m/f, skin/hair/build, `occasion` register (everyday working dress ↔ ceremonial finery), creole two-parent blending. `CultureFigures.tsx` renders it. | Used in **PeoplesPanel only**. No house ever shows a figure; no house symbol on the garment. |
| House tiers | Nothing. `archetype` (4) is SPECIALIZATION, not rank. But `wealth`, `volume`, `influence[]`, `political_power`, `prestige`, `bailos`, `charters`, `dominant_seat` all exist. | No tier field, no ranking, no grouping in the list. |
| House goals | Nothing. `War.goal: u8` exists for wars only. | New state. |
| Head character | Nothing — `head_name: String`. Archetype pivots at succession, but that describes the FIRM, not the person. | New state. |
| Expeditions | Rich `Expedition` — `leader`, `house` (backer!), `origin`/`dest`, `ox,oy → dx,dy`, `caravans`/`ships`, `good`, `cargo_qty`, `cost`, `revenue`, `arrived_frac`, `status` 0–4, `hazards: Vec<HazardEvent>`. `campaign_get_expeditions` exists. | Shown in **ColonialPanel + MapCanvas only**. Never in the house panel, despite `house` already being on the struct. `dest` is a HUB, so there is no province link. No expedition layer on the province plate. |
| Holdings + who holds it | Scattered across `hubs[].is_estate`+`owner_house`, `warehouses`, `offices`, `office_leases`, `bailos`, `banks`, `charters`. `HouseBrief` exposes `estates`, `offices`, `active[]`. | No unified roster. **No person on any holding.** `Official.kin: bool` is the only family-member bit in the codebase. |
| Internal friction / schism | `found_branch` + `branch_name_for` are fully written and `ENABLE_CADET_BRANCHES = false` disables them. | No tension state, no trigger, no record. The split MACHINERY exists. |
| More AI strategies | 4 archetypes with standing bonuses; `decide_*`/`apply_*` split started (polis, coinage, fleets). | Strategy is a static archetype, not a pursued intent. |

**The unifying observation:** items 2, 4 and 5 are the same feature. Character lives on
a person; "who holds this, family or not" needs a person on the holding; a schism is a
person leaving with their holdings. One `Kin` roster serves all three, and it is also
what makes the AI *story-shaped* rather than parameter-shaped.

---

## 1. Tiers — one standing score, ranked bands

Your spec: Tier 1 highest and most prominent, Tier 4 negligible.

Derived from ONE score so it is not another hand-tuned knob, and every input already
exists:

```
standing = 0.30·rank_norm(wealth)      // rank-normalised over LIVE houses, not absolute
         + 0.25·rank_norm(volume)      // trade actually moved, not money sitting still
         + 0.20·reach                  // Σ influence[] capped at 1
         + 0.15·seats                  // captured governments + bailos + charters
         + 0.10·rank_norm(prestige)
```

Tier by **rank band**, not by absolute score — a tier means "where this family stands
among its peers", which is what makes it readable as the world grows:

| Tier | Name | Band | Reads as |
|---|---|---|---|
| **1** | Great house | top 8% **and** standing ≥ 0.55 | dominates trade and regions |
| **2** | Major house | next 22% | a power in its own quarter |
| **3** | Lesser house | next 40% | trades, matters locally |
| **4** | Marginal house | the rest | negligible influence |

Three rules:

- **Tier 1 may be EMPTY.** The `standing ≥ 0.55` floor means a young world has no great
  houses. A tier that is always occupied carries no information.
- **Hysteresis is mandatory.** Recompute monthly with a ±0.04 dead band, or a house
  flickers between 2 and 3 forever and the chronicle fills with noise.
- **Tier changes are CHRONICLED** — "House Vareni is counted among the great houses"
  is one of the few genuinely satisfying beats available for free.

---

## 2. `Kin` — the roster that unlocks three asks

```rust
pub struct Kin {
    pub name: String,
    pub born_tick: u32,
    pub dies_tick: u32,
    /// 0 head · 1 heir · 2 factor (runs a holding) · 3 idle kin · 4 married out · 5 dead
    pub role: u8,
    /// Hub of the holding this person runs (−1 = at the seat, unposted).
    pub posted: i32,
    /// Character, −2..+2 per axis (§3). The head's drives the house's AI.
    pub character: [i8; 4],
    /// 0..1 loyalty to the head. Low + able + posted far = schism risk (§5).
    pub loyalty: f32,
    /// 0..1 competence. Multiplies the margin of whatever they run.
    pub skill: f32,
    /// Parent's kin index, or −1 — the line of descent a schism splits along.
    pub parent: i32,
}
```

Serves at once:
- **§3 character** — the head is `kin[head]`; succession promotes an heir with their
  OWN character, so a generational turn genuinely changes how the house plays.
- **§4 holdings** — a holding either has a `posted` kin (family) or a hired factor
  (not family). That distinction is exactly what you asked to see, and it *matters*:
  kin are loyal and often less able; hired factors are able and skim.
- **§5 schism** — a capable, disloyal, distantly-posted kin is the one who leaves.

Roster stays small (~4–9 live kin) so cost is trivial; capped and serde-defaulted, so a
house with no roster behaves exactly as today.

---

## 3. Character — four axes, each wired to an EXISTING knob

A character axis that doesn't move a real decision is horoscope text. Each of these
maps to a knob already in the code:

| Axis | − pole | + pole | Existing knob it moves |
|---|---|---|---|
| **Caution ↔ Boldness** | hoards, short terms | buys hulls, long voyages, expeditions | `decide_fleets`, `EXP_*` launch gate, contract `term_years` |
| **Honour ↔ Greed** | keeps contracts, accepts arbitration | undercuts, escalates, debases | `FEUD_HEAT`, `arbitrate_feuds` acceptance, `BRIBE_COST` spend |
| **Private ↔ Civic** | hoards to the family | funds works, subsidises the city | `fund_public_works`, `HOUSE_CONSUMPTION_RATE` |
| **Rooted ↔ Expansive** | deepens the seat | offices, bailos, colonies | `update_guilds_and_offices`, `lease_office`, colonisation |

Presented as a phrase, never four numbers: *"Bold, grasping, and set on the far
shore"* — the same discipline as the stability gauges.

**Gate:** with every character at 0 the dynamics run must be **bit-identical**. That is
what proves character is a modifier and not a rewrite.

---

## 4. Goals — a checkable ambition with a deadline

A goal must be able to SUCCEED or FAIL and be recorded, or it is decoration.

```rust
pub struct Goal {
    pub kind: u8, pub target_good: i32, pub target_hub: i32,
    pub target_house: i32, pub target_province: i32,
    pub set_tick: u32, pub deadline_tick: u32,
    pub progress: f32,
    pub state: u8,   // 0 pursuing · 1 achieved · 2 failed · 3 abandoned
}
```

| Goal | Success test | Chosen by |
|---|---|---|
| Corner the ⟨good⟩ trade | monopoly ≥ 60% held 5 yrs | Specialty · Greed |
| Seat the council of ⟨city⟩ | `captor_house == self` | Political · Civic |
| Raise a bailo at ⟨city⟩ | city in `bailos` | Fleet · Expansive |
| Charter a bank | owns a solvent bank 10 yrs | Banking |
| Reach ⟨province⟩ | an expedition returns from it | Bold · Expansive |
| Outlast House ⟨Y⟩ | Y defunct while we live | Greed + a hot feud |
| Restore the house | wealth back over its historic peak | after ruin or scandal |

One active goal per house (two for Tier 1). A goal biases the *weights* of decisions the
AI already makes — it never adds a new action. Achieved/failed goals stay on the
dossier: a family with three failed ambitions reads very differently from one with three
achieved.

---

## 5. Schism — the split you asked to be recorded

Trigger, all from state above:

```
tension = 0.35·(1 − cohesion)            // §2 of the shipped dossier
        + 0.25·(1 − mean kin loyalty)
        + 0.20·stretch (distant posts)
        + 0.10·(feuds running)
        + 0.10·(a passed-over able heir)
```

Above a threshold, one of three outcomes — never a silent number:

1. **Quarrel** (common) — cohesion drops, one kin's loyalty craters, chronicled.
2. **Departure** — the disloyal kin leaves with the holdings they were *posted to*,
   founding a new house (`found_branch` already does this) that starts as a **rival**.
3. **Rupture** (rare, Tier 1–2 only) — capital splits by line of descent; both halves
   drop a tier; a `FEUD_SUCCESSION` feud opens between them.

This is the honest reason to re-enable `ENABLE_CADET_BRANCHES`: not "more houses", but
*a consequence*. Every outcome writes a `HouseEvent` and a journal beat, so the schism
is in the family's permanent record.

---

## 6. Schematics

### 6a. Houses panel — grouped by tier

```
┌ ⚜️ Trading Families ─────────────────────────────────── ✕ ┐
│ 👑 Houses (14)  🏛 Guilds (3)  ⚔ Feuds  🎯 Ambitions      │
├───────────────────────────────────────────────────────────┤
│ ● Trade is flowing                              year 297  │
│ [shipped 41][by houses 68%][lost 2][in transit 9]         │
├───────────────────────────────────────────────────────────┤
│ ── TIER 1 · GREAT HOUSES (2) ────────── dominate trade ── │
│ ⬤[arms] House Vareni          🏦 ⚖ 👑        ████████ 42k │
│    Ilvar Vareni · Ostrahn · gen 4 · Ashkar                │
│    🎯 corner the silk trade          ██████░░░░ 61%       │
│    bold · grasping · set on the far shore                 │
│    ⚔ 3 quarrels · 🚢 8 · 🏢 4 offices · 🏛 2 bailos       │
│ ⬤[arms] House Okkath          🏦    👑       ██████░░ 31k │
│    …                                                      │
│ ── TIER 2 · MAJOR HOUSES (3) ───────────────────────────── │
│ ◐[arms] House Sedhri                          ███░░░░ 12k │
│ ── TIER 3 · LESSER HOUSES (6) ─────────────── collapsed ▸ │
│ ── TIER 4 · MARGINAL (3) ──────────────────── collapsed ▸ │
│ ── FALLEN HOUSES (9) ──────────────────────── collapsed ▸ │
└───────────────────────────────────────────────────────────┘
```
Tier glyph: ⬤ ◐ ◔ ○. Tier 3/4 collapse by default — that IS the "see who has the
highest influence" you asked for.

### 6b. Dossier header — the figure, in culture dress

```
┌ ⚜️ House Vareni ─────────────────────────────── ✕ ┐
│  ╭─────────╮   TIER 1 · GREAT HOUSE                │
│  │  ╭───╮  │   Ilvar Vareni, head since 256        │
│  │  │ o │  │   Ashkar (Punic kit) · gen 4          │
│  │ ╭┴───┴╮ │   ⬤ standing 0.81 ▲ rose from Tier 2 │
│  │ │▞▞ ⬥ │ │      in 271                           │
│  │ │▞▞   │ │                                       │
│  │ ╱     ╲ │   CHARACTER                           │
│  ╰─────────╯   bold · grasping · civic-minded ·    │
│   ceremonial   set on the far shore                │
│   [everyday]                                       │
│                                                    │
│  🎯 corner the silk trade    ██████░░░░ 61%  by 302│
│  ✓ raised a bailo at Kelmar (284)                  │
│  ✗ failed to seat the council of Ostrahn (279)     │
├────────────────────────────────────────────────────┤
│ Summary ⚖ Standing ⚔ Feuds 👪 Kin 🗝 Holdings      │
│ 🧭 Expeditions 🏦 Bank 📒 Accountant                │
└────────────────────────────────────────────────────┘
```

The figure is `cultureFigureSVG({ kit, sex, seed, occasion })` — **already written** —
seeded on the head's name, wearing the seat culture's kit. House identity is added as
**three** marks, so a house reads as *of its culture, but distinct*:

1. the garment's accent band recoloured to the house's `color`
2. the coat-of-arms charge as a small badge at the shoulder (reuses `CoatOfArms`)
3. a tier-dependent register — **Tier 1 defaults to `ceremonial`** (finery), Tier 3–4 to
   `everyday` (working dress). Rank becomes visible before you read a number.

### 6c. Kin & Holdings — the roster, and who holds what

```
┌ 👪 Kin ───────────────────────────────────────────┐
│ ★ Ilvar Vareni      head      Ostrahn   62y  ●●●●○│
│   bold · grasping                     loyal ●●●●● │
│ ◆ Sura Vareni       heir      Ostrahn   34y  ●●○○○│
│   cautious · honourable               loyal ●●●●○ │
│ ◇ Tanmo Vareni      factor    Kelmar    41y  ●●●●●│
│   bold · grasping        loyal ●○○○○  ⚠ able and  │
│                                    resentful      │
│ · Melqa Vareni      idle      Ostrahn   28y  ●●○○○│
│ ✕ Hanno Vareni      married out → House Tavric    │
│                                                   │
│ TENSION  ████████░░ 0.71  ⚠ a schism is possible  │
│   stretched across 6 cities · Tanmo passed over    │
└───────────────────────────────────────────────────┘

┌ 🗝 Holdings (11) ─────────────────────────────────┐
│ ▣ silk estate         Vaskeld   👪 Tanmo    ●●●●● │
│ ▲ Ostrahn dyeworks    Ostrahn   👪 Sura     ●●○○○ │
│ ◈ Banco Vareni        Ostrahn   👪 Ilvar    ●●●●○ │
│ ▢ Kelmar depot        Kelmar    💼 hired    ●●●●● │
│      Bodo Ashken — skimmed 42 over 6 years  ⚠     │
│ 🏛 bailo              Kelmar    👪 Tanmo          │
│ 🏢 office (leased→309) Vist     💼 hired          │
│ 📜 charter · salt     Ostrahn   —                 │
│ ── 4 more ─────────────────────────────────────── │
│ 👪 family = loyal, often less able                 │
│ 💼 hired  = able, and skims                        │
└───────────────────────────────────────────────────┘
```

### 6d. Expeditions in the house panel + on the plate

```
┌ 🧭 Expeditions ───────────────────────────────────┐
│ ACTIVE (2)                                        │
│ ⛵ Doran of House Vareni      → Sarkoth Reach      │
│    outbound  ●━━━━━━━━○──────  61% · 40d left     │
│    3 ships · 1 caravan · silk 240 · risked 180    │
│    ⚠ storm off the Kelmar bank (−1 ship)          │
│    ┌──────────────┐  the province they are        │
│    │   ,-~"~-.    │  reaching for, highlighted    │
│    │  (  ███  )   │  on the world map too         │
│    │   `-.,.-'    │                               │
│    └──────────────┘                               │
│ 🐫 Melqa of House Vareni     → Ashen Waste   12%  │
│                                                   │
│ RETURNED (4)   ✓3  ✗1                             │
│ ✓ 289 Sarkoth Reach  +410  (goal: reach it ✓)     │
│ ✗ 281 the Ashen Waste  −180  lost to raiders      │
└───────────────────────────────────────────────────┘
```

Needs one backend change: `Expedition` carries a hub `dest`, so add
`dest_province: i32` (resolved through `province_at` at launch) to make the province
highlight and the "reach ⟨province⟩" goal checkable.

### 6e. The schism, recorded

```
┌ 📜 House Vareni · chronicle ──────────────────────┐
│ 297 ⚡ SCHISM — Tanmo Vareni departs with the      │
│        Kelmar bailo and the Vaskeld estate,       │
│        founding House Vareni of Kelmar            │
│        · standing 0.81 → 0.62 · Tier 1 → 2        │
│        · a feud opens over the inheritance        │
│ 293 ⚔ House Okkath forces our counting-house in   │
│        Kelmar to close                            │
│ 284 ✓ raised a bailo at Kelmar                    │
└───────────────────────────────────────────────────┘
```

---

## 7. Sequencing (cheapest genuine win first)

1. **Tiers** — pure derivation of existing state + list grouping. No new sim state.
2. **The figure in culture dress** — `cultureFigure.ts` already exists; wire it, add the
   three house marks, tie register to tier.
3. **Expeditions tab** — `Expedition.house` already exists; one new field for the
   province link.
4. **`Kin` roster** → holdings authorship (§4) and the head's character phrase.
5. **Character wired to the four knobs** — gate: all-zero character ⇒ bit-identical.
6. **Goals**.
7. **Schism** — last, because it needs cohesion + kin loyalty + tiers to all be real.

Every step after (3) needs the dynamics + econ gates re-run, and (5) needs the
bit-identical proof.
