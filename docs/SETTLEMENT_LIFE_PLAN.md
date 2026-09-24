# Settlement Life Plan — how people live, earn, eat, die and make trouble in a campaign city

**Status: L0-L7 SHIPPED (all at dose 0 where dosed); L13 PARTIALLY SHIPPED.**
L0-L2 shipped 2026-09-23, L3 (annals + the Life tab), L4 (vital rates + age
bands), L5 (welfare into behaviour), L6 (housing & crowding) and L7 (the
settlement year's seasonal mortality) in follow-up sessions — see CLAUDE.md
§5.7 for the full account. One session shipped the two L13 ingredients that
already had real data behind them (the age pyramid and causes of death, both
from L4) into the Life tab, rather than wait on L13's other ingredients.
Housing/crowding (L6) is now real data too but not yet drawn in the Life tab
— a UI gap, not a missing mechanism. Church, watch and notables genuinely
have no mechanism behind them yet (L9/L10/L12) and are unstarted. L7 shipped
only its FIRST of four named parts (seasonal mortality) plus the already-
existing grain temporal CV print — the lean-months hoarding, harvest-labour
dip and feast-day calendar remain queued, named below. L8-L12 and the rest of
L13 remain queued per the build order below. Originally written from a read
of the campaign tick
(`sim/campaign/tick/`), not from memory; every claim in §1 names the code it
was read from. Coordinates with — and shares one slice with —
`MONEY_AND_COINAGE_PLAN.md` (see §2 D1).

The question this plan answers: *what does it mean to live in a settlement in this
simulation, and what is missing?* Today a city is an excellent **economic** object
(market, estates, manufactories, houses, banks, coin, trade) sitting on a thin
**social** one: a single `population` float, four class shares, and a mood. The
one place life already bites back on the world is unrest (§1 F11), and it is fed
almost entirely by sentiment formulas rather than by anything a household earned,
ate or suffered.

The governing rule is the one `INSTITUTIONS_BUILD_ORDER.md` set for an
observation-only game, restated because every slice here must obey it:

> **Every mechanism must produce a legible STORY, not a decision.** A slice names
> what it writes to the chronicle and what the Life tab (§3.12) shows, or it is not
> done. The player watches this world; a mechanism nobody can see is dead code with
> a dose.

---

## 0. Where this sits among the other plans

| Plan | Overlap | Rule here |
|---|---|---|
| `MONEY_AND_COINAGE_PLAN.md` | **M7** (the affordability fix) is this plan's **L1** — the same change. **M8** (wages + paid consumption through purses) supersedes L2's grain-equivalent incomes when it lands. | One implementation, whichever plan reaches it first; the other marks it shipped. L2 is designed so M8 can re-source it without changing its readers. |
| `CONSUMPTION_REBUILD_PLAN.md` | S7 `HOUSEHOLD_MONETIZATION_DOSE` was reverted for exactly the bug L1 fixes. | L1 is S7's stated prerequisite. Resuming S7's dose walk belongs to that plan, not this one. |
| `PLACES_DEMAND_AND_GROWTH_PLAN.md` | Slice 4 (`CAPACITY_LAND_WEIGHT`) moves the growth ceiling. | L4 (vital rates) keeps the capacity term as its envelope and must be walked with slice 4's dose PINNED, never together. |
| `SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` | N5 made routes seasonal; N6 elasticity is at zero. | L7's calendar is the settlement half of N5. It cannot move grain price CV alone (§1 F5). |
| `CITY_PROVINCE_WAR_PLAN.md` / §5.4 (levy) | Levy deaths already reduce `population`. | L4 routes those deaths into the adult-male band instead of the whole population. |
| `INSTITUTIONS_BUILD_ORDER.md` | Guild monopoly, grain law, hospices. | L9 (church) and L10 (watch) are new institutions under that plan's story rule. |

---

## 1. What is true today (read from the code)

**F1 — Grain the poor cannot buy reads as the city being fed.**
`update_food_and_starvation(&needs_struct)` runs at `tick/mod.rs:9578`, AFTER the
eating loop (`mod.rs:~9380`), and computes `food_have = stock + production`
(`disease.rs:~360`). Anything left uneaten — including grain a priced-out household
could not afford — counts as food on hand. That is why S7's household budget dose
(`HOUSEHOLD_MONETIZATION_DOSE`, `mod.rs:~2955`) was walked 0.1 → 0.02 → 0.005 and
reverted: a poorer populace read as a BETTER-fed city, silencing
`unrest_topples_councils`. The eating loop already knows the answer —
`tier_unmet[0]` feeds `lack_basic` — it is simply not what the famine check reads.
This is **entitlement failure** (Sen): the commonest pre-modern famine, where grain
is present and people starve anyway. Today the model cannot express it at all.

**F2 — There is no wage, and the "real wage" metric is not one.**
`household_income_pass` (`mod.rs:9656`) pays `trade_wealth × HOUSEHOLD_WAGE_SHARE
(0.02)`, inert at dose 0. The scorecard's `real_wage_index`
(`economy_validation.rs:602`) is `commoner_wealth / grain_price`, where
`commoner_wealth = civic_pc·0.5 + grain_wealth·0.3 + food_aff·0.4 − tax`
(`cities.rs::society_metrics`) — a blend of sentiment inputs with no unit. It has
read 72.3, 146.3 and 162.5 on different dates in `SCOREBOARD.md`; none of those
numbers can be compared to Allen's welfare ratios, because nothing in it is an
income. So the oldest question in pre-modern economic history — *could a labourer
here afford to live?* — has no answer in this world.

**F3 — Population is one number, and nobody is born or dies.**
`population` is an `f32` moved by (`disease.rs:~570-610`): a logistic step toward a
capacity (`POP_GROWTH_RATE` 0.0003/day up, `POP_DECLINE_RATE` 0.0006 down), a net
drift `birth_rate·food_sec − DEATH_RATE_BASE` (0.00009 / 0.00002 per day), a famine
multiplier `1 − 0.0016·(starving − 0.5)`, multiplicative plague culls, levy deaths
and migration. There are no birth or death COUNTS, no ages and no causes of death.
Consequences: the chronicle cannot say "3,200 died of the flux this summer"; a famine
leaves no missing generation; a war leaves no shortage of young men; the
city-kills-its-people effect (the urban graveyard) exists only as a capacity term
in `province_demography_pass` for big cities.

**F4 — Society has no memory; Pops are a rescaled copy of it.**
`update_society` (`cities.rs:1337`) eases four shares toward `target_shares`, dragged
toward the underclass by hardship. `derive_pops` (`cities.rs:1389`) then REBUILDS
nine professions every year from fixed splits (`farmers = commoner × 0.60`, …), with
`money = cw × 6 + 5` for elites and so on. Nothing persists between years, so there
is no mobility — only a target the shares drift toward. Pops do feed `update_unrest`
(militancy), the levy and the craftsmen labour cap (§5.4 of CLAUDE.md), but
`campaign_get_pops` has **no caller in `src/`** except its bridge wrapper.

**F5 — The harvest has a calendar; the people do not.**
`seasonal_mult` (`mod.rs:~8765`) gives food a harvest peak at ~day 230, amplitude
`0.10 + 0.32·|lat|`, shifted half a year in the south, riding a hemisphere-wide
fertile/lean cycle of 0.86–1.16. Yet the scorecard measures grain price CV within a
city at **0.010** against a historical band of 0.30–0.50 (`SCOREBOARD.md` 2026-08-19).
The seasonality is swallowed by stocks: `CONSUMPTION_AND_GOODS_REVIEW.md` measured
314 days of grain held in year one and 29.5 YEARS by year 100, on top of
`FOOD_RESERVE_DAYS` = 45. Mortality, work, festivals and hunger have no season at
all. **Consequence for this plan:** a settlement calendar can give people a year,
but it cannot restore price seasonality on its own; that needs the stock glut fixed
(CONSUMPTION_REBUILD S2/S3). L7 says so rather than promising the CV.

**F6 — The built city is entirely economic.**
Five structures (`mod.rs:2570`): granary, warehouse, shipyard, guildhall,
workshop — every one an output or freight bonus. Civic wonders (lighthouse → market
hall → cathedral, `houses.rs::run_civic_wonders`) are prestige flavour. Public health
exists as hospices, capped at `HOSPICE_MAX_LEVEL` 0.6 (`polis.rs:149`). There is no
HOUSING, so crowding cannot exist; "fire" exists only as a production/warehouse event
(`production.rs:2594`, "Fire ravages the warehouses of …") and an estate-damage roll
(`houses.rs:1865`) — it never burns a home or kills anyone. Flood is estate damage
only. There is no water supply or sanitation.

**F7 — There is no religion.** `religion` does not occur in `tick/`; `faith` occurs
once (`PILGRIM_STABILITY`). Clergy are a Pop profession (index 5) with no
institution behind them. `HolySite`/`run_pilgrimages` is flavour
(`ACTORS_AND_CARRIAGE_PLAN.md` §2). No alms, no poor relief other than the civic
granary.

**F8 — There is no public order.** No watch, no garrison, no crime. Unrest has
exactly two outlets — riot and revolt (`update_unrest`). The levy (`war_manpower`)
exists but only for war.

**F9 — Cities have no people with names.** `FIGURE_KINDS` is five world-wide roles
(Admiral, Demagogue, Master Craftsman, Great Banker, Explorer) with at most
`FIGURE_LIVING_CAP` = 6 alive in the WHOLE WORLD (`mod.rs:6388`). A given city
almost never has one. The only per-city person is the leader (`kin[0]` of the
council/captor house, `read_hubs.rs`).

**F10 — The economy oracle has no rural population.** The scorecard's own fixture
seeds no provinces, so urban share drifts 0.100 → 0.997 (`SCOREBOARD.md`). Any
demographic gate here must run on a PROVINCED fixture — `realm_reference_world`
(`economy_validation.rs:1492`, 24 provinces, neighbour graph) or `tests::dense_world`
— never on the plain `reference_world`, or it measures an empty countryside.

**F11 — Unrest is already rich, and is the right place to plug in.**
`update_unrest` (`cities.rs:1446`) reads mood, inequality, `lack_basic`, starvation,
war, minorities × stress × majority disposition, cultural cravings, pop militancy,
welfare, prosperity, plus grievance memory scaled by consciousness. The plan FEEDS
this function better inputs rather than adding a parallel unrest model.

---

## 2. Decisions (proposed defaults — maintainer to confirm)

| # | Decision | Proposed default | Why |
|---|---|---|---|
| **D1** | Who owns the affordability fix? | **One implementation, shared with MONEY M7**; whichever plan starts first ships it. This plan's L1 is the natural first. | Two copies of the famine check is exactly the rule-11 duplication hazard. |
| **D2** | Wages before real money exists? | **Yes — in grain-equivalents**, sourced from real production value. MONEY M8 later routes the SAME incomes through purses. | The welfare ratio is the single most useful historical measure and should not wait for the full money ledger. |
| **D3** | Age granularity | **Three bands** (children 0–14, adults 15–49, elders 50+) **plus an adult-male share**. | Enough for famine echoes, war sex ratios and dependency; five-year cohorts are 10× the state for little legible gain. |
| **D4** | Religion | **Institution only**: a church with endowment, clergy, tithe, alms and buildings. No doctrine, no heresy, no religious war (queued, §7). | The economic/social role of the medieval church (charity, credit, landholding, education) is well documented and fits the tick; doctrine needs a culture-level belief model that does not exist. |
| **D5** | Player agency | **Observation-only**, consistent with the houses plan. No new player verbs. | Every slice is therefore a SPECTACLE problem — it must show on the Life tab and in the chronicle. |
| **D6** | Pops and Society | **Pops become the state; Society becomes derived from them** — but LATE (L11), after a shadow phase proves the two agree at dose 0. | Today the dependency runs the wrong way (F4). Inverting it first would put the riskiest change before any instrument exists. |
| **D7** | Housing consumes real goods | **Yes**, timber + stone (+ clay/bricks where the world has them) as a construction demand, dosed from zero. | A new, real sink for bulk goods — which also works against the warehouse glut F5 names. |
| **D8** | Dose discipline | **Every behavioural slice dosed from zero**, walked one dose at a time with the others pinned; **three doses per session maximum**. | `ACTORS_AND_CARRIAGE_PLAN.md` §5.2 and `HOUSES_GUILDS_AND_MARKET_PLAN.md` §9 — the inheritance gate has flipped inside its own noise band five times. |

---

## 3. The model

### 3.1 Entitlement: a famine is a failure to EAT, not a lack of grain (L1)

Record, per hub per day, what the eating loop actually delivered:
`TickHub.food_eaten` and `TickHub.food_need` (the `needs_struct` food sum),
serde-defaulted. `update_food_and_starvation` computes TWO balances and blends them:

```
bal_stock = (stock_after + production − need) / need     // today's formula, unchanged
bal_eaten = (food_eaten − need) / need                    // ≤ 0 by construction
bal       = lerp(bal_stock, min(bal_stock, bal_eaten + margin), ENTITLEMENT_DOSE)
```

`margin` keeps a city that ate its full ration at the same positive balance it reads
today, so at full dose the only change is that a household priced out (S7) or a city
whose grain is locked in a merchant's warehouse now reads as underfed.
`ENTITLEMENT_DOSE = 0.0` ⇒ bit-identical (`sim_fingerprint`). The structural ration
(`needs_struct`) is not touched (N6's rule).

**Story:** the chronicle distinguishes *"famine in X: the granaries are empty"* from
*"famine in X amid full warehouses: bread beyond the reach of the poor"*.

### 3.2 Incomes and the welfare ratio (L2)

Each Pop gets an INCOME in grain-equivalents per head per year, sourced from money
the city actually earned — never a formula of sentiment:

| Profession | Income source (existing state) |
|---|---|
| farmers (0) | the hub's own food production value + its province's delivered surplus (`prov_surplus`), less dues/tithe |
| labourers (1) | a share of trade THROUGHPUT at the hub (porters, carters, dockers — `trade_last_year`) plus estate labour |
| craftsmen (2) | manufacture VALUE ADDED at the hub (output value − input value, `apply_manufacturing`) |
| clerks (3) | a share of the treasury's spending + house ledger overheads |
| merchants (4) | `export_earn` retained locally, net of house profits |
| clergy (5) | church income (L9); until then a fixed share of civic spending |
| capitalists (6) / aristocrats (7) | house dividends and rents resident at the hub; province dues held by resident houses |
| soldiers (8) | levy pay from the treasury while mobilised, else labourer rate |

Each pool is divided among that profession's heads. The **welfare ratio** is
income ÷ the cost of a bare-bones subsistence basket at LOCAL prices (the basket is
`needs_struct`'s basic tier × `price`, per head per year) — Allen's measure, so it
is directly comparable to published series: ~1 is bare subsistence; historically
labourers ran ~1–1.5 in Florence and Delhi and ~2–3 in 17th-century London and
Amsterdam (Allen 2001 — `economic-history` confirms the band before any assertion).

**L2 is OBSERVE ONLY.** `Pop.income` and `TickHub.welfare_ratio` are written and
read by nothing in the tick, so `sim_fingerprint` is unchanged. The scorecard gains
`labourer_welfare_ratio` (printed, not asserted, §2.5) beside the old
`real_wage_index`, which is kept (and relabelled "commoner wealth index") until the
new figure has earned trust. MONEY M8 later re-sources these same numbers from
purses; readers do not change.

### 3.3 Vital rates and the age pyramid (L4)

`TickHub.ages: [f32; 3]` (children, adults, elders — shares summing to 1) and
`male_adult_frac`, seeded from a stationary pre-modern pyramid (≈ 35 / 50 / 15).
Each year (with a daily accumulator so the chronicle can date an epidemic):

```
births  = adults × ½ × fertility(food_sec, welfare, starving_last_year, crowding)
deaths  = Σ band × mortality(band, food, disease, crowding, sanitation, season)
          + plague deaths (existing cull, now attributed by band)
          + levy deaths (existing, now taken from adult males)
          + fire/flood deaths (L8)
ageing  = fixed transfer rates between bands
```

Every death carries a CAUSE (`famine · plague · fever · war · fire · flood · old
age · infancy`), accumulated into `TickHub.deaths_by_cause` for the annals (§3.11).

**The envelope stays.** The existing capacity (`capacity = base_pop × cap_mult`)
is not deleted: crowding past capacity RAISES mortality and LOWERS fertility — the
Malthusian positive and preventive checks — instead of the logistic formula clamping
growth by fiat. `VITAL_RATES_DOSE` blends the old net step with the new one, so
0.0 is bit-identical and the walk can stop at any point that measures healthy.

Three effects this makes visible that nothing today can:
- **The famine echo.** A famine suppresses births for two or three years; fifteen
  years later the adult band is thin and the city's labour and levy are short.
- **War widows.** Levy deaths fall on adult men; `male_adult_frac` drops, fertility
  drops with it, and the chronicle can say so.
- **The urban graveyard.** City mortality exceeds rural by a crowding/sanitation
  term, so large cities shrink without migration — the de Vries pattern
  (`province_demography_pass` already supplies the migration that replaces them).

Historical bands, printed on the provinced fixture (§1 F10): crude birth rate
~35–45‰, crude death rate ~30–40‰ in ordinary years, crisis years 2–3× that,
life expectancy at birth ~25–35, infant/child mortality the largest single term
(Wrigley & Schofield; `historical-society` confirms before any is asserted).

### 3.4 Welfare into behaviour (L5)

Once L2 has a real number, three existing readers switch (blended by
`WELFARE_BEHAVIOUR_DOSE`) from sentiment proxies to it:
- `derive_pops`' militancy term reads each profession's OWN welfare ratio below 1.0,
  so starving labourers riot while comfortable burghers do not, in the same city;
- `urban_exodus_pass` and `province_demography_pass` compare welfare ratios between
  origin and destination rather than `sent_prosperity` — people move toward wages;
- `update_society`'s hardship drain reads the welfare ratio of commoners.

Nothing new is added to `update_unrest` itself; it simply receives truer pop inputs
(F11).

### 3.5 Housing and crowding (L6)

`TickHub.housing` — dwellings, measured in persons housed — seeded at
`population × 1.10`. Built monthly by the city (treasury + a share of `civic_pool`)
and by resident houses' civic spending, CONSUMING real timber and stone from the
hub's stock through the same supply-pick pattern `construction_pass` already uses for
satellites (`pick_build_supply_good`); decays at ~1%/yr; destroyed by fire, flood,
war damage and plague abandonment.

`crowding = population / housing`. Above 1.0, and dosed by `HOUSING_DOSE`:
- mortality rises (L4's crowding term) and plague spreads faster in that city;
- unrest rises (a new `update_unrest` input, bounded);
- rent: a slice of household income goes to housing owners — the resident houses and
  the church (L9), which is how a landlord class earns in a city at all;
- the growth ceiling reads housing as one more capacity term.

**Story:** *"Builders cannot keep pace; X's poor sleep ten to a room"*, *"A great
rebuilding in X — timber from Y fills the yards"*.

### 3.6 The settlement year (L7)

A per-hub calendar derived from latitude + `koppen` (both on `TickHub`):
- **Seasonal mortality**: a summer fever/flux peak in hot or wet climates, a winter
  peak in cold ones (respiratory, cold), feeding L4's mortality by season;
- **The lean months**: `lack_basic` is expected to peak before harvest; L7 adds
  pre-harvest HOARDING by houses (holding grain in depots for the spring price, where
  a depot exists — see `YARDS_VESSELS_AND_DEPOTS_PLAN.md` W-slices) and the grain law
  (§5.4, `LAW_GRAIN`) calling dearth earlier in spring;
- **Harvest labour**: at harvest, labourers leave the city for the fields — a
  seasonal dip in manufacturing labour cap, a seasonal wage bump;
- **Feast days**: a small, bounded calendar of holidays (culture-seeded dates) that
  lower output for a day and raise mood — the existing fair/pilgrimage seasons join
  this calendar instead of running independently.

L7 does NOT promise to fix grain price CV (F5). It is measured and printed; the CV
moves only when the stock glut does.

### 3.7 Urban hazards: fire, flood, water (L8)

- **Fire** becomes a settlement event, not only a warehouse event: probability from
  crowding × timber share of housing × dry season; burns a fraction of housing and
  stock, kills some (L4 cause `fire`), and produces REBUILDING demand (L6). A
  city that has burned may enact a building law (stone rebuilding, a new `Law`
  kind), lowering future risk and raising building cost — London 1666.
- **Flood** for riverine hubs (`hub.river`) in the wet season: housing and stock
  loss, fever afterwards.
- **Water and sanitation**: a new structure (wells → conduit → aqueduct), built by
  the council, raising `public_health` past `HOSPICE_MAX_LEVEL` and lowering L4's
  urban-mortality term. The first real civic building whose payoff is people, not
  output.

### 3.8 The church and charity (L9)

`TickHub.church: Church { endowment, income, clergy, alms_rate, hall_tier }`:
- **Income**: a tithe on the hub's food production (competing with — never stacked
  blindly on — the realm's harvest tithe: the crown and the church split what the
  peasants can pay, and a high combined rate adds rural unrest); BEQUESTS when a
  house head dies (a hook in `close_head_record` — pious houses leave more, read from
  the civic character axis); rents from housing it owns (L6).
- **Spending**: clergy upkeep (the clergy Pop is sized by it, not by a fixed split);
  building (chapel → church → cathedral — `run_civic_wonders`' cathedral tier is
  REPLACED by the church actually building one out of its endowment);
  **alms** in dearth, bought at market for the poor — a real purchase that, after L1,
  genuinely lowers `lack_basic` for the underclass, the parish counterpart to the
  civic granary.
- **Credit**: a rich church lends to the council (a small, bounded extension of the
  public-debt holder classes).

No doctrine, heresy or religious war (D4, queued §7). Faith is flavour text only:
culture-seeded names for the church and its feasts.

**Story:** *"In the famine the abbey fed four thousand at its gate"*, *"the
merchant X leaves a third of his fortune to the cathedral works"*.

### 3.9 Order and crime (L10)

`TickHub.watch` (strength, funded by the treasury) and a derived `crime` rate:
`crime = f(underclass share, inequality, crowding, labourer welfare < 1) − watch`.
Effects, all bounded and dosed:
- **theft**: a small drain on resident house stock/wealth, booked as its own ledger
  line (the §3.4e `war_levy` pattern), so it shows in the Accountant tab;
- **unrest**: crime feeds `update_unrest`; a strong watch dampens riots (and makes a
  revolt, when it comes, bloodier — the watch is a party to it);
- **banditry**: high crime and a weak watch raise CARAVAN loss on house legs leaving
  the hub. (Ownerless legs do not roll loss — `N1B_OWNERLESS_LOSS_RATE` is 0 — which
  is recorded, not worked around.)

### 3.10 Persistent pops and mobility (L11)

The inversion (D6). Pops persist year to year and move:
- **births** enter the profession of their parents (L4 per-pop, not per-hub);
- **mobility flows** driven by income RANK within the city: apprenticeship lifts
  labourers into craftsmen when the workshop labour cap binds; ruin drops craftsmen
  and merchants into labourers after a crash, a fire or a guild dissolution; famine
  pushes the poorest into the underclass/soldier pools; the elite are fed by house
  wealth, not by a share;
- **migrants arrive as pops** of the profession they had (exodus/rural pull carry a
  profession mix);
- `Society`'s four shares become an AGGREGATE of pops.

**Shadow phase first:** pops run persistent in parallel for one slice with the
existing derived pops still driving every reader; an `#[ignore]`d diagnostic reports
how far the two diverge. Only when the divergence is understood does
`PERSISTENT_POPS_DOSE` switch readers over.

### 3.11 The townspeople (L12)

Per-city NOTABLES, separate from the world-wide `FIGURE_KINDS` (which stay as they
are): the bishop/abbot (L9), a guildmaster (per `CraftGuild` at the hub), the
physician (L8's public health), the captain of the watch (L10), an alderman (the
council house's second kin), and the agitator (the existing Demagogue, localised).
Each is **anchored** to an institution it leads — the physician's quality nudges
public-health ease, the captain's the watch's effectiveness, within ±10% — so a death
or a bad appointment is felt, not merely announced. Count capped by city tier (1–6);
named from the city's culture kit; lifecycle and chronicle reuse `raise_notable_figures`.

### 3.12 The annals and the Life tab (L3, then L13)

`TickHub.annals: Vec<CityYear>` — one record per year, capped (rule 29: tail-aligned
if parallel series are ever added): population by band, births, deaths by cause,
bread price, labourer welfare ratio, crowding, riots/revolts, fires, notable events.

`campaign_city_life(hub)` serves it. HubPanel gains a **Life** tab:
1. **Headline**: one sentence — *"A crowded, hungry port: labourers earn 0.8 of
   subsistence and the fever took 1 in 30 last summer."* Built from the unusual
   figures only (the CityMarketView rule: quiet when ordinary).
2. **Bread against wages**: labourer welfare ratio over the annals, with famines
   and fires marked.
3. **The people**: the three-band pyramid (L4) and pops by profession with income
   (finally calling `campaign_get_pops`).
4. **Who died, and of what**: stacked deaths-by-cause per year.
5. **Why the city is angry**: `update_unrest`'s own terms ranked, the
   `SpecDriver` why-chain pattern — an explanation, never a raw 0..1.
6. **The townspeople** (after L12).

L3 ships the tab over what exists after L0–L2 (population, welfare, lack_basic,
unrest drivers). A later session folded the pyramid and causes-of-death — both
already computed by L4, just unread — into the same tab ahead of the rest of
L13, since they needed no new mechanism, only surfacing (`CityYear` widened
additively per rule 29's own tail-alignment discipline, `#[serde(default)]`
so an old annal reads as "not recorded" rather than corrupt). Housing and
notables still wait on L6/L12 — there is no data to fill them with.

---

## 4. Slices (build order)

Routing per CLAUDE.md §2.8: every `tick/` slice runs `cargo test --lib tick::tests`
+ `cargo test --lib econ_ -- --nocapture` (multi-seed inheritance gate included) +
`simulate_decades_reports_dynamics`; every frontend slice runs `npx tsc --noEmit`.
Each slice also names the gate that is NOT its own target (§2.4).

| # | Slice | Dose / effect | Gates |
|---|---|---|---|
| **L0** | ✅ **SHIPPED** — Instrument `econ_measure_settlement_life` (`#[ignore]`d, on `realm_reference_world` + `dense_world`): implied natural growth by city size; famine episodes and length; riots/revolts/plague-strikes per century (journal-capped, lower bound); distribution of `lack_basic`; how often `lack_basic > 0.2` while grain stock > 30 days of need (**the F1 count**). Plague RECOVERY TIME (not just strike frequency) needs per-tick population history not kept here — left unmeasured, named in the test's own doc comment, not approximated. **Baseline measured**: F1 count reads 0 of ~4,000-4,300 hub-year checks on both fixtures — `lack_basic` already sits near its ceiling almost everywhere (mean 0.886/0.743), a pre-existing near-permanent shortfall this instrument surfaces for the first time. | none — diagnostic | ran; baseline row in `SCOREBOARD.md` 2026-09-23 |
| **L1** | ✅ **SHIPPED** — Entitlement (= MONEY M7): `TickHub.food_eaten`/`food_need_today` recorded in the eating loop, blended into `update_food_and_starvation` via `entitlement_bal_e` (§3.1). Chronicle distinction (empty-granary vs. full-warehouse famine) NOT yet wired — that needs L3's tab/journal text, unstarted. | `ENTITLEMENT_DOSE = 0.0` (true no-op); the dose WALK is unstarted, separate work | `entitlement_dose_zero_is_a_noop`, `a_priced_out_city_reads_as_hungry` (`tick::tests`); `cargo test --lib econ_` 6/6 bit-identical incl. multi-seed inheritance gate. Walking the dose above 0 still needs: `unrest_topples_councils` + every famine test firing MORE not less (read the L0 count first); `econ_expenditure_shares_resemble_a_household`. |
| **L2** | ✅ **SHIPPED** — Incomes + welfare ratio (§3.2), OBSERVE ONLY. `Pop.income`/`TickHub.welfare_ratio` computed yearly in `derive_pops`, read by nothing else. Scorecard prints `labourer_welfare_ratio` beside `real_wage_index` (relabelled "commoner wealth index" in the printout, not renamed as a field). A simplified reading of §3.2's income-source table (no province-surplus or house-ledger detail yet — see CLAUDE.md §5.7). | none | `incomes_sum_to_what_the_city_earned`, `welfare_ratio_is_finite_and_positive` (`tick::tests`); `sim_fingerprint`/dynamics unchanged (verified via full `econ_` + `tick::tests` runs, not a dedicated fingerprint diff test). |
| **L3** | ✅ **SHIPPED** — Annals + Life tab v1 (§3.12): `TickHub.annals: Vec<CityYear>` (capped at `ANNALS_CAP`=300, one record/year — population, welfare ratio, lack_basic, grain price, mood, unrest), recorded yearly by `record_city_annals` right after `update_unrest`; served by `campaign_city_life(hub)`. `HubPanel.tsx` gains a Life tab: a headline built from unusual figures only, at-a-glance stats, and a welfare-ratio bar chart over the last dozen years. Explicitly names what it does NOT show yet (age pyramid, causes of death, housing, church, notables) rather than rendering empty. `campaign_get_pops` is still NOT wired anywhere in `src/` — that remains queued for Life tab v2 (L13), since Pop-level per-profession income has no reader in this cut. | UI + observe-only state | `sim_fingerprint` unchanged (verified via the full `tick::tests`/`econ_` runs); `city_annals_fill_yearly_and_stay_capped` (`tick::tests`, a cheap fixture — the first cut used `dense_world()` and cost 437s on its own, fixed to run in <1s); `npx tsc --noEmit` clean; `npx vite build` clean (181 modules) — the S10 caveat (no display to actually open the tab in) still applies. |
| **—** | **STOP MARKER** — L0–L3 is a coherent landing: the hidden famine is measured and (at dose 0) ready, the welfare ratio exists, and the player can see how a city lives. Simulation bit-identical. | | |
| **L4** | ✅ **SHIPPED (dose 0)** — Vital rates + age bands (§3.3): `TickHub.ages`/`male_adult_frac`/`deaths_by_cause` (indices `CAUSE_*`, `DEATH_CAUSE_COUNT`=8). `update_vital_rates` (yearly, `cities.rs`, before `update_society`) is UNCONDITIONAL bookkeeping — ageing transfer between bands, births into the children band, deaths drawn down by a real CBR/CDR calculation and tagged by cause (famine above the starvation floor, old-age/infancy otherwise) — and never itself writes `population`. War deaths are tagged separately, at the moment they happen, by `spend_levy_casualties` (`war.rs`): a real casualty now thins `male_adult_frac` and tags `CAUSE_WAR`, the "war widows" effect. The ONE path from vital rates into `population` is `vital_net_rate_e`, blended into the existing daily net-growth term at `VITAL_RATES_DOSE` — a true no-op at 0.0 (old formula returned untouched). Fire/flood (`CAUSE_FIRE`/`CAUSE_FLOOD`) stay at 0.0 until L8 exists to write them. | `VITAL_RATES_DOSE = 0.0` (true no-op); the age-band bookkeeping itself is unconditional/observational. Raising the dose (with `CAPACITY_LAND_WEIGHT` pinned) is unstarted, separate work. | `vital_rates_dose_zero_is_a_noop`, `a_famine_leaves_a_missing_generation`, `ages_move_over_time`, `war_deaths_fall_on_adult_men` (`tick::tests`, 283/283); `econ_` re-verified bit-identical incl. the multi-seed inheritance gate. Walking the dose still needs: CBR/CDR/life-expectancy printed on the provinced fixture (§1 F10); world population bounded and finite; Zipf/urban share not regressed; `big_cities_die_faster_than_they_breed` (unwritten — queued with the dose walk itself). |
| **L5** | ✅ **SHIPPED (dose 0)** — Welfare into behaviour (§3.4): three sentiment-only readers blend toward the real welfare ratio via one shared lever. `derive_pops`' per-pop militancy blends toward `welfare_militancy_e` (a labourer under bare subsistence reads more militant than a comfortable burgher in the SAME city); `update_society`'s hardship drain blends toward `welfare_hardship_e`; `province_demography_pass`'s rural pull and `urban_exodus_pass`'s destination ranking both blend their prosperity term toward `welfare_opportunity_e` — people move toward wages, not vibes. | `WELFARE_BEHAVIOUR_DOSE = 0.0` (true no-op — every helper early-returns the old value at `dose <= 0.0`) | `welfare_behaviour_dose_zero_is_a_noop`, `labourers_riot_before_burghers`, `welfare_opportunity_prefers_higher_wages_and_stays_bounded` (`tick::tests`, 286/286); `econ_` 6/6 bit-identical incl. the multi-seed inheritance gate. Raising the dose is unstarted, separate work — needs `unrest_topples_councils` + the `EXODUS_*` migration gates re-verified per step. |
| **L6** | ✅ **SHIPPED (dose 0)** — Housing + crowding (§3.5). `TickHub.housing`/`crowding`; `update_housing` (monthly) seeds/decays housing (unconditional — pure number bookkeeping, touches no stock) and, when crowded, builds more through a pure dosed helper (`housing_build_persons_e`) that consumes a real construction good picked via the existing `pick_build_supply_good` (non-food category — timber/stone/iron/brick) straight from the hub's own stock. **Construction is dosed, NOT unconditional** — unlike L4's age pyramid, spending real stock is a genuine economic action; a first cut left it unconditional and it moved `econ_inheritance_rules_fragment_differently` (a real, measured regression caught by the gate, fixed before shipping — see the constant's own doc comment). Two consequences (excess mortality via `housing_crowding_net_adjust_e`, composed AFTER `vital_net_rate_e`; unrest via `housing_crowding_unrest_e` in `update_unrest`) share the same `HOUSING_DOSE`, kept separate from `VITAL_RATES_DOSE`/`WELFARE_BEHAVIOUR_DOSE`. Rent (needs L9's church as a payee) and the growth-ceiling read (would tangle with `CAPACITY_LAND_WEIGHT`'s own walk) are explicitly NOT wired this pass — queued. | `HOUSING_DOSE = 0.0` (true no-op for construction AND both consequences); only seed/decay/the `crowding` read are unconditional | `housing_dose_zero_is_a_noop`, `crowding_above_one_costs_more_than_housed_comfortably`, `housing_seeds_from_population_once`, `housing_build_persons_e_spends_real_stock_it_has` (`tick::tests`, 299/299); `econ_` 6/6 bit-identical incl. multi-seed inheritance gate — RE-VERIFIED after the construction-gating fix. Raising the dose, wiring rent, and wiring the growth ceiling are each unstarted, separate work. |
| **L7** | ✅ **SHIPPED, PARTIAL (dose 0)** — The settlement year (§3.6), first of its four named parts: seasonal mortality only. `seasonal_mortality_mult_e` (mod.rs) reads a hub's own hemisphere (`hub_lat_frac`) and climate (`koppen >= 14` ⇒ cold) and returns a multiplier peaking at a warm/wet hub's OWN summer (fever) or a cold hub's OWN winter (respiratory) — both tag `CAUSE_FEVER`, since `DEATH_CAUSE_COUNT`'s own doc comment leaves no slot for a dedicated respiratory cause. `update_seasonal_mortality` (monthly, `cities.rs`) is gated on the WHOLE pass, not just a downstream reader — unlike L4's age-pyramid bookkeeping, this changes real `population`, the exact lesson L6's construction-gating regression already paid for. The monthly extra is SIGNED (both above and below the year's average), so the 12 monthly calls net close to zero over a year — a redistribution of `update_vital_rates`'s own yearly total onto its true season, never an added death rate on top of it; only the POSITIVE (peak-season) excess is tagged into `deaths_by_cause[CAUSE_FEVER]`, since that tally is descriptive only. Grain temporal CV was ALREADY printed (`economy_validation.rs`'s `temporal_cv`, "grain price CV within a city", band 0.30–0.50) before this slice — nothing new needed there; unmoved by this change since it touches mortality, not price. The other three named L7 parts — pre-harvest hoarding + `LAW_GRAIN` calling dearth earlier, a harvest-labour dip in the manufacturing cap with a seasonal wage bump, and a bounded feast-day calendar (folding the existing fair/pilgrimage seasons in) — are NOT built, queued below. | `CALENDAR_DOSE = 0.0` (true no-op — `seasonal_mortality_mult_e` returns exactly 1.0, `update_seasonal_mortality` returns before touching a hub) | `calendar_dose_zero_is_a_noop`, `summer_fever_in_the_south_winter_deaths_in_the_north`, `seasonal_mortality_redistributes_rather_than_adds` (`tick::tests`, 302/302); `econ_` 6/6 bit-identical incl. multi-seed inheritance gate. Raising the dose, and the three unbuilt parts (hoarding/`LAW_GRAIN`, harvest labour, feast days), are each unstarted, separate work — the feast-day calendar additionally needs the existing fair/pilgrimage passes folded in rather than duplicated. |
| **L8** | **Fire, flood, water** (§3.7). | `URBAN_HAZARD_DOSE` | fires/century printed; a burned city rebuilds within bounds; no death spiral (`simulate_decades`). |
| **L9** | **Church + charity** (§3.8). | `CHURCH_DOSE` | tithe + crown tithe never exceed rural capacity; alms lower `lack_basic` only where grain exists (post-L1); multi-seed inheritance gate (bequests move house wealth). |
| **L10** | **Watch + crime** (§3.9). | `ORDER_DOSE` | theft ledger line balances; crime bounded; `unrest_topples_councils` still fires (a watch may delay a revolt, never abolish it). |
| **L11** | **Persistent pops + mobility** (§3.10): shadow slice, then switch. | shadow (bit-identical), then `PERSISTENT_POPS_DOSE` | `pops_sum_to_population`; Society aggregate within tolerance of old shares at dose 0; full `econ_` + inheritance gate per dose step. |
| **L12** | **Townspeople** (§3.11). | effects anchored ±10%, dosed | notable count bounded per tier; save size bounded; chronicle milestones survive pruning (rule 20). |
| **L13** | 🟡 **PARTIALLY SHIPPED** — Life tab v2: pyramid + causes of death (✅, see below), housing/crowding (real data since L6, but not yet drawn — a UI gap), church/watch/notables (❌, still queued — no mechanism exists; L9/L10/L12). | UI + `CityYear` widened additively (`ages`/`deaths_by_cause`, `#[serde(default)]`) | `tsc` clean; full `tick::tests` (286/286) + `econ_` (6/6 incl. multi-seed inheritance gate) bit-identical — pure bookkeeping onto data L4 already computed, no dose needed. Housing/crowding chart still owed (data exists); church/watch/notables still owed a mechanism before they can be shown at all. |

**Build rules.** One dose at a time, the others pinned at zero. Three doses per
session at most. A dose that regresses an aggregate gate is REVERTED and the walk
recorded at the constant's own doc comment (§2.4 — the negative result is a
deliverable). A `SCOREBOARD.md` row whenever a measured number moves (§2.6), and
CLAUDE.md §5 updated in the same commit as each slice (§2.7).

---

## 5. Risks

1. **L1 is the same change that broke S7.** The difference is that L1 changes what
   the famine check READS, with no household budget attached; S7's dose then becomes
   safe to resume. Expect `unrest_topples_councils` to be the sensitive test — it
   must keep firing, and if it fires MORE, that is the fix working (hidden hunger now
   counts), not a regression. Read the L0 count before judging.
2. **Vital rates (L4) touch the growth rate every other system is tuned against.**
   `WORLD_AGE_DEV_CAP` is documented as chaotically sensitive to per-hub capacity
   nudges. Walk small, re-run the sustained-runaway-wealth guard at every step, keep
   the capacity envelope.
3. **Housing (L6) is a new demand sink for bulk goods.** It could fix part of the
   warehouse glut or starve timber-poor cities of shipbuilding wood
   (`YARDS_VESSELS_AND_DEPOTS_PLAN.md`). Watch timber price at the yard cities.
4. **Two tithes.** Crown and church both take from the harvest; stacking them
   without a combined ceiling would collapse rural pools. L9 must split, never add.
5. **Pop persistence (L11) is the largest refactor in the plan** and inverts a
   dependency every unrest/levy/labour reader relies on. That is why it is last and
   shadowed first.
6. **Legibility debt.** Each slice adds numbers; without L3/L13 they are invisible
   in an observation-only game. Do not ship three sim slices ahead of the tab.
7. **The fixture problem (F10).** A demographic gate on the province-less
   `reference_world` measures an empty countryside and will pass or fail for the
   wrong reason.

---

## 6. What is expected to move, and what is not

- **Should move:** labourer welfare ratio exists and lands in a historical band;
  famines kill by cause and leave echoes; big cities need migrants; riots follow
  wages; the chronicle gains a social register.
- **Should NOT move** (and is a regression if it does): bounded, finite house wealth;
  house turnover; wars/century; the inheritance gate's direction; expenditure shares.
- **Will not move by this plan alone:** grain price CV within a city (needs the stock
  glut fixed — F5), price/distance gradient (a trade problem, not a settlement one).

---

## 7. Queue (rule 36 — owed, not waived)

1. **Doctrine and religious difference** — a per-culture faith, conversion,
   minorities of another faith, heresy, religious war. Waits on L9 (the institution)
   and a culture-level belief field; gate: minority unrest must not double-count
   `MINORITY_UNREST`.
2. **Education and literacy** — the church school and the guild apprenticeship as
   routes of mobility. Waits on L9 + L11.
3. **Household structure** — nuclear vs stem vs joint households per culture,
   feeding fertility and inheritance of small property. Waits on L4 + L11; relates
   to §8.15's inheritance rules.
4. **Slavery and bound labour** — present in many historical settings, absent here.
   Waits on L11 (a pop that cannot move) and a decision from the maintainer on
   whether the setting includes it.
5. **Gender and work** — women's labour in textiles, brewing, retail; widows as
   heads of craft households (the house layer already has the widow regent). Waits
   on L4's sex split extending beyond adult men.
6. **Guild life for the craftsman pop** — journeymen, masters, closed guilds
   restricting mobility. Waits on L11 + `HOUSES_GUILDS_AND_MARKET_PLAN.md` S3.
7. **Rural village life** — the province rural pool as more than a number (the
   manor, the commune, peasant revolts distinct from urban riots). Waits on L4
   applied to `prov_rural`; gate: `province_land_pass` tests.
8. **Player-facing verbs** (build housing, endow a church, fund a watch) — explicitly
   excluded by D5; queued so that if D5 ever changes, the decide/apply split each
   slice uses (the `decide_polis_policy` pattern) is ready to accept a player choice.

---

## 8. What this plan does not change

The world pipeline (no tile, no generation phase), the market price formula, trade
dispatch, the money model (owned by `MONEY_AND_COINAGE_PLAN.md`), and the house /
realm politics layers — except where a slice names a specific existing reader it
switches, behind its own dose.
