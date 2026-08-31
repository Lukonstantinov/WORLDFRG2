# Exploration, the known world, and transport modes

**Status: DESIGN COMPLETE, nothing built.** All open questions are decided (§6);
this is buildable as written. Companion to
`OUTPOST_CONNECTIVITY_AND_ENTREPOT_PLAN.md`, which measured the problems it
answers.

---

## 0 · What the code does today (measured)

### Three founders already exist, and the timing already fits

| Pass | Founder | Gate |
|---|---|---|
| `maybe_found_settlement_colony` | a **CITY** — large, food-secure, prosperous, under population pressure, with treasury | `COLONY_START_TICK` = **year 30** |
| `try_found_house_outpost` | a **HOUSE** — any clearing `OUTPOST_FOUND_WEALTH`, richest first, ≤3 per call | `OUTPOST_START_TICK` = **year 30** |
| `maybe_found_caravanserai` | a **CITY** — waystations on long land ties | `expansion_ok` |
| `maybe_graduate_outpost` | promotes a house outpost into a full colony in place | age + pop + wealth |

`expedition_launch_pass` is backed by **houses** (non-guild, wealth ≥
`EXP_MIN_HOUSE_WEALTH`), gated at `EXP_START_TICK` = **year 15**.

So the "exploration is the pre-stage for colonisation at year 30" structure the
brief asks for is **already the shape of the code** — expeditions run first, the
two founding passes open at year 30. What is missing is the *causal link*: today
the two are unrelated systems and expeditions gate nothing.

**Decision:** move `EXP_START_TICK` to **year 25**, per the brief. Five years of
exploration then feed the year-30 founding passes.

### The three defects this design fixes

Measured in the companion plan: houses are **omniscient** (no
`explored`/`discovered`/`surveyed` state exists; `colonizable` is a whole-world
list scanned in full every call), outposts are **sited without regard to whether
cargo can leave** (`delta`/`chokepoint` exist on the site struct; the house path
reads neither), and **water carriage costs the same as overland** (one global
`days_per_cell`; `fleet_river` pooled with ox-carts in `cap_land`).

---

## 1 · Knowledge: two kinds, and they behave differently

The central design decision, and the one that makes this more than a map filter:

> **Where a place is** is shareable. **What it costs there** is not.

- **MAP knowledge** — that a province and its towns exist, roughly where, roughly
  what it yields. Acquired by expedition; **freely exchanged** between houses and
  settlements that trade with each other. This is what the fog layer draws.
- **MARKET knowledge** — live prices, stocks, what sells. Acquired **only by
  direct presence**: an office or bailo there, or your own traders arriving. Not
  transferable by contact, and it goes **stale** when presence lapses.

That split is historically right (portolan charts and the *Periplus* circulated
widely; a Venetian house's Alexandria price book did not) and it is what makes
information an *asset* rather than a display toggle.

### 1.1 Encoding

Held **per province, per knower**. Provinces are already the world↔campaign join
(FIX_PLAN B1), already carry `prov_neighbors` for contiguity, and at 200-400 km
are about the resolution at which pre-modern knowledge actually existed. A
per-cell fog would be a large per-knower raster implying a precision nobody had —
and storage is the binding constraint: 40 houses × 300 provinces is trivial;
per-cell is not.

```rust
/// What one knower knows about one province. Serde-defaulted; an empty map
/// means "knows nothing", and §5's seeding is what keeps that from stranding
/// an existing campaign.
struct Known {
    level: u8,        // 0 unknown · 1 reported · 2 surveyed · 3 established
    since_tick: u32,  // when this level was reached — drives MARKET staleness
    source: i32,      // who told us (-1 = our own expedition)
}
```

**Knowers are both houses and cities** — forced by the code, not chosen: both
found things (§0), so both must know things. A city's knowledge lives on
`TickHub`, a house's on `House`.

### 1.2 How the levels move

- **0 → 1 (reported):** contact. A trading partner that knows a province at ≥2
  passes it on at 1. Also what a *failed* expedition leaves behind — a partial
  report, because a total loss teaches the player nothing and reads as arbitrary.
- **1 → 2 (surveyed):** an expedition returns. This is the gate for founding.
- **2 → 3 (established):** we hold or trade there — an office, an outpost, a
  colony, a bailo.

Map knowledge **never decays**: a coast once charted stays charted. Market
knowledge is not a level at all — it is `since_tick` on a level-3 entry, and it
ages the moment presence lapses.

---

## 2 · Expeditions become the acquisition mechanism

`expedition_launch_pass` and `route_prospects` already exist and gate nothing.
This gives them their job.

- **Backers:** houses (as today) **and** cities, since cities found colonies too.
- **Routing:** follow known trade routes where they exist; otherwise **coasts and
  rivers**. This is the historical pattern (the coastal crawl before open-water
  navigation; the Russian frontier moved along river systems; the American west
  along the Missouri and the Platte) and it is cheap once Slice D makes navigable
  water known to the campaign.
- **Range:** bounded, and deliberately short-ish. Distance is what makes the
  hazard real rather than decorative.
- **Return:** the expedition raises every province along its path to level 2 for
  its backer, and neighbours to 1.

### 2.1 Natives — the hazard

Per the brief: natives attack intruders (Cortés; the American frontier). This is
the mechanism that makes distance cost something.

**The honest constraint: the model has no native population.** Provinces carry
`prov_rural` but there is no notion of an unincorporated people, and inventing
one properly is a large design in its own right (arguably `historical-society`'s
question, not this plan's).

So build it as a **hazard field, not a polity** — explicitly a stand-in, in the
same documented-proxy tradition as `geology.rs`'s phase-2 climate term:

```
hostility(province) ≈ f(distance beyond the backer's known frontier,
                        province emptiness — no hub, low prov_rural,
                        terrain difficulty,
                        whether any prior expedition has been here)
```

An expedition rolls against it per leg. Outcomes: **returns** (full report),
**limps home** (level 1 only), **lost** (no report, backer eats the cost, a
chronicle entry naming the province). Hostility should **fall** once a province
is established — contact, or conquest, but the model need not say which.

This is deliberately not a native *faction*: no armies, no territory, no
diplomacy. If that is wanted later it replaces this field cleanly, because
nothing else reads it.

---

## 3 · What the fog gates — and what it must not

Per the brief: **trade mechanics are unchanged**. What changes is *reachability*
— a house or city cannot reach a city it has never heard of.

Concretely, knowledge gates:

- **founding** — an outpost/colony needs level ≥ 2 at that province;
- **goals** — a house goal cannot target an unknown province;
- **trade partners** — `rebuild_neighbors` prunes partners in provinces the
  knower does not know at ≥ 2.

It does **not** change price formation, dispatch, freight, or the market solver.
The `days` matrix, `good_freight`, and the needs ladder are untouched.

### 3.1 The economic risk, and the mitigation

**Pruning trade partners moves the economy** — that is unavoidable, and it will
shift the `econ_` bands. It is the one part of this design that is not additive,
and it should not be shipped claiming otherwise.

Mitigation, and it is a good one: **seed knowledge at campaign start from the
existing trade network.** Every province containing one of a knower's holdings or
current trade partners starts at level 3, its neighbours at 1. Then:

- the **founding economy is unchanged** — day-one partners are all known;
- the fog constrains **expansion only**, which is exactly the intent;
- an existing save is not stranded (§5).

**Gate:** `econ_fidelity_scorecard` before/after with fog on and seeding in
place. A small drift is acceptable and expected; a large one means the seeding is
wrong, not that the fog is.

### 3.2 The market-knowledge half

"Prices only where you have presence" is **already designed** as stage 4 of
`MERCHANT_VESSELS_AND_INFORMATION_PLAN.md` — a house trades on the price it
*believes*, with a spread set by how fresh its knowledge is (never been →
surveyed → office → controls the seat). That is the same mechanism the brief
describes.

**Do not build it twice.** This plan supplies the `Known` state and the
never-been / surveyed levels; that plan owns the belief-price and spread. They
should land together, and stage 4's own gate — long-haul trade volume must not
collapse — governs.

---

## 4 · Transport modes (build this FIRST)

Restated from the companion plan because everything above is easier with it.

Give a route a **mode** with its own per-day cost, capacity and risk — sea ≪
river < road < track — and split `cap_land` back into `fleet_river` and
`fleet_caravan` so a barge is not an ox-cart. Per the brief all three axes
differ, and water wins on all three, which is what makes it preferable with no
special case. Worldgen already knows which rivers are navigable
(`River.navigable`); that flag needs carrying into the campaign snapshot.

**Why first:** it is the biggest lever on the known market-integration defect
(basket price/distance gradient **−0.064**, 0 of 6 goods showing any gradient,
where the correct sign is positive); it is testable against an instrument that
already exists; and **it may make later work unnecessary** — if water carriage is
genuinely cheap, a site near navigable water becomes valuable through ordinary
route cost, with no bespoke entrepôt rule and no siting special case.

**Gate:** the `econ_fidelity_scorecard` gradient. If differentiating transport
cost does not move it off −0.064, the hypothesis is wrong and that is a **negative
result to record**, per §2.4 — not something to quietly keep.

---

## 5 · Migration — existing campaigns

Every field is serde-defaulted; an absent knowledge map means "knows nothing",
which would strand a year-200 campaign. So on load, if the map is empty, **seed
it from current holdings and trade partners** (§3.1) rather than starting blind.
Same seeding path as a new campaign, so there is one code path, not two.

---

## 6 · Decisions (all previously-open questions, now closed)

| # | Question | Decision |
|---|---|---|
| 1 | Who is the knower? | **Both houses and cities** — forced by §0: both found things. Exploration initiator = the colony founder. |
| 2 | Does knowledge decay? | **Map: never.** Market: not a level — `since_tick` ages once presence lapses. |
| 3 | Does the fog gate trade? | **Yes, reachability only** — unknown cities are not partners. Price formation untouched. Start-seeded (§3.1) so only expansion is constrained. |
| 4 | Player verb or AI? | **AI-driven**, matching every other campaign system. A player verb is a clean later addition. |
| 5 | What are "natives"? | A **hostility field**, explicitly a documented proxy — not a polity. No native faction, armies or diplomacy. |
| 6 | Existing campaigns? | **Seed from holdings + partners** (§5). |
| 7 | Expedition timing | `EXP_START_TICK` → **year 25**; colonisation stays **year 30**. |

---

## 7 · Build order

1. **Transport modes** (§4). Biggest lever, existing instrument, may subsume later work.
2. **Manufactured-goods filter + province detail** (§8). Small, self-contained, visible.
3. **Outpost siting reads `delta`/`chokepoint`** (companion plan E1). A few lines.
4. **Knowledge + expeditions + natives** (§1-§3). The large one — and only after
   re-measuring whether 1 and 3 already produced sensible siting.

---

## 8 · Province view: the manufactured-goods bug, and richer detail

### 8.1 The bug — measured

The Inspector lists **Books & Manuscripts** as a province good. It is manufactured
(`goods_spec.rs`: `mg("books", … inputs: paper 0.8, dyes 0.1)`, which sets
`Distribution::Manufactured`) and has neither belt nor deposit.

The chip row filters `!g.is_deposit` and nothing else; its source,
`campaign_province_potential`, filters only on magnitude:

```rust
for g in 0..ng {
    let belt = sim.prov_good_belt.get(idx).copied().unwrap_or(0.0);
    if belt <= PROV_GOOD_ABSENT_BELT { continue; }
    goods.push(ProvinceGoodPotential { … });
}
```

**Nothing anywhere filters `Distribution::Manufactured`.** The query already
builds an `is_deposit` map from the same specs and passes it through as a flag;
there is no manufactured equivalent.

The generation side is correct — `compute_trade_goods` writes
`buf.goods[slot] = vec![0u8; n]` for a manufactured good, and an all-zero belt
lands exactly on `PROV_GOOD_ABSENT_BELT` and is excluded. So the missing guard is
the whole defect.

**Honest limit:** that means something put a *non-zero* belt in the `books` slot
in this particular world, and which of two candidates it is cannot be told from
code alone — (a) a **stale column** at a reused index (the world predates `books`
in the spec, or the spec was edited after generation; CLAUDE.md §8.20's "fixed
indices in `TileData.goods`" is exactly this hazard), or (b) a length mismatch
between `Province.good_belt` and the campaign's `goods.len()` misaligning the flat
`prov_good_belt` row. **The fix does not depend on the answer** and should not
wait for it.

**Fix:** exclude manufactured goods **structurally**, as `is_deposit` already is,
and pass a belt-good mask into `generate_provinces` — which today receives no
goods spec at all and therefore *cannot* distinguish a manufactured column from a
belt column. That is the structural root, and fixing it makes the class of bug
impossible rather than repairing one world's data.

**Gate:** `a_province_never_lists_a_manufactured_good`.

### 8.2 Richer detail — all from state that already exists

- **Per good:** belt quality **and its grade word** (the served
  `deposits::grade_label` vocabulary — coarse/ordinary/good/fine/exquisite,
  §8.19), area covered, whether a named **locality** sits here with its grade
  (`GoodLocality` already carries `name`/`grade`/`extent`/`river_fed`), live
  `exploitation` vs `potential` (§2.5 computes both), and `market_share`.
- **Per deposit:** `ProvinceDepositDot` already carries `grade`, `extent` and
  `depth` per working — **none of which is shown**. Depth especially: "flooded"
  is a real economic fact (§8.16's "visible but largely LOCKED"), and nothing
  reads `depth` anywhere yet — `DEPOSITS_AND_MINING_PLAN.md` slice 4's own note.

Both are **read-only presentation of existing state** — no new sim, no new
persisted field — which makes this the cheapest item in the set.

---

## 9 · Deliberately NOT built

- **A native faction** — armies, territory, diplomacy. §2.1 is a hazard field and
  says so.
- **Per-cell fog.** Province granularity, for storage and honesty (§1.1).
- **Belief-priced trade.** Owned by `MERCHANT_VESSELS_AND_INFORMATION_PLAN.md`
  stage 4; this plan supplies the state it reads (§3.2).
- **A player expedition verb.** AI-driven first (§6.4).
- **Map-knowledge decay.** A charted coast stays charted (§6.2).
