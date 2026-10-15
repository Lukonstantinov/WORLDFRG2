# WorldForge 2 — Docs Catalogue (`docs/`)

> Extracted from CLAUDE.md to keep the always-loaded file lean. Index of the
> plan/design docs in `docs/`; read the one you need. Linked from CLAUDE.md §9.

## 9. Docs (`docs/`)

**START HERE — the current plan**
```
living_world/00_INDEX.md          ← ⭐ THE BUILD QUEUE (2026-09-25). Ten numbered rows —
                                    01 feeds & pruning · 02 people · 03 development
                                    tracks · 04 government & edicts · 05 culture
                                    acceptance · 06 ideology & scholars · 07 artisans &
                                    masterworks · 08 leisure & games · 09 realms, war &
                                    barbarians · 10 settlement overview. A session takes
                                    the FIRST row that is NOT STARTED with its
                                    dependencies DONE, builds its slices in order, runs
                                    the economy gates once at the row's end (§2.9), and
                                    marks the row in the index in the same commit
FIX_PLAN.md                       ← ⭐ Measured Earth-fidelity baseline + the prioritised
                                    fix plan (climate · one-simulator · economy · society),
                                    with a regression gate per item. Read before planning work.
SCOREBOARD.md                     ← ⭐ The project held as ~12 NUMBERS instead of 89k lines:
                                    both fidelity scorecards, test counts, perf, and an
                                    explicit list of what is still UNMEASURED. See §2.6.
```

**Live operational docs** (these describe the project as it is)
```
SETTLEMENT_LIFE_PLAN.md           ← ⭐ L0-L3 SHIPPED (§5.7, the STOP MARKER
                                    reached), L4-L13 queued. How
                                    people live, earn, eat, die and make trouble in
                                    a campaign city. Eleven measured findings,
                                    headed by F1:
                                    the famine check reads grain LEFT IN STOCK
                                    after eating (`update_food_and_starvation`
                                    runs after the eating loop), so grain the poor
                                    cannot buy reads as the city being fed — the
                                    bug that reverted S7, and entitlement failure
                                    the model cannot express. Also: no wage exists
                                    and `real_wage_index` is a sentiment blend, not
                                    an income; population is one float with no
                                    births, deaths, ages or causes; Pops are
                                    re-derived yearly with no memory; no housing,
                                    religion, watch or per-city notables. Fourteen
                                    slices L0-L13 (instrument · entitlement = MONEY
                                    M7, one shared change · incomes + Allen welfare
                                    ratio · annals + a Life tab · STOP MARKER · vital
                                    rates + age bands · welfare into behaviour ·
                                    housing/crowding consuming real timber+stone ·
                                    the settlement year · fire/flood/water · church
                                    + alms · watch + crime · persistent pops
                                    (shadowed first) · townspeople), every
                                    behavioural one dosed from zero, demographic
                                    gates on a PROVINCED fixture only
MONEY_AND_COINAGE_PLAN.md         ← ⭐ AGREED IN SCOPE, NOTHING BUILT. Real,
                                    CONSERVED money: coins are struck at a mint
                                    from mined bullion, held in purses per holder
                                    per city, carried in chests on real legs, and
                                    spent — never `+=`'d into a vault. Measured
                                    premise (F1): every sale today credits the
                                    carrier (`production.rs:2297`) with no payer,
                                    so money is created on every trade. Barter is
                                    ALWAYS available and settles with real stock
                                    moving both ways (the payment goods leave the
                                    seller's stockpile); money just beats it.
                                    Currencies with 2-3 denominations and dated
                                    issues, culture-based units of account and
                                    regional gold:silver ratios, mints that CLOSE
                                    when their coin goes unused, a Victorian coin
                                    catalogue with follow-a-coin, a market money
                                    band, and banks on real reserves with bills
                                    of exchange settled by specie shipments.
                                    Slices M0-M11: a PARALLEL LEDGER observes
                                    first (M0-M4 bit-identical), then barter,
                                    affordability, wages and the switch-over are
                                    each dosed from zero
HOUSES_GUILDS_AND_MARKET_PLAN.md  ← ⭐ S2 (annona carrier class) + S3 (craft
                                    guild roster unfreeze) + S4 (craft
                                    signatures served) + S5 (transit demand,
                                    shipped inert at zero) + S7 (eight
                                    orphan-raw recipes) + S8/S9 (the house/
                                    craft atlas queries) + S10 (the four-
                                    window split) BUILT AND GATED — see
                                    CLAUDE.md §5.6. S1 is BLOCKED (a
                                    pre-existing, already-measured negative
                                    result, not merely undosed — see queue
                                    item Q14). S3's own 1→3 dose walk measured
                                    UNTESTABLE by the standing gates (they
                                    carry zero manufactured goods). **S6
                                    dose-walked to 0.3 and REVERTED** — two
                                    real gate failures (a limited-liability
                                    breach and a relay-fixture assumption the
                                    dose invalidates), recorded at
                                    `BLOCKADE_STAGING_DOSE`'s own doc comment.
                                    S10/S11(text+map, incl. Q19)/S12a/S12b/
                                    S12c(3 of 5) — everything this plan built
                                    in follow-up sessions after the first —
                                    could only be verified by `tsc`/`vite
                                    build` — no display to actually open the
                                    windows in, said plainly rather than
                                    claimed as tested (queue item Q17, and
                                    Q19's on-map lane rendering is named as
                                    the single highest-risk piece to check
                                    first). Q20 (full cross-tab year-travel on
                                    the timeline plate), Q21 (Kin as a real
                                    tree — needs new persisted sim state) and
                                    Q22 (Lineage as a drawn SVG diagram) remain
                                    QUEUED. The first session's own THREE-DOSE
                                    ceiling (S1 investigated/blocked, S3
                                    walked/untestable, S6 walked/reverted) is
                                    unchanged — every slice shipped since is a
                                    read-only query or pure frontend work, not
                                    a dose. The one-session build
                                    plan for houses, guilds and the settlement
                                    market, after four decisions: the era is a
                                    Roman/medieval MIX (no `EraProfile` switch built
                                    yet — every Roman-flavoured slice ships
                                    era-NEUTRAL so the axis can be added later, queue
                                    Q1); the *annona* reframe applies to BIG CITIES
                                    ONLY; the player stays OBSERVATION-ONLY (so every
                                    item is a SPECTACLE problem, which is why half the
                                    plan is atlas and UI); and every behavioural change
                                    ships dosed from zero. Its cheapness comes from one
                                    finding: **four mechanisms in it are already fully
                                    wired and dead at a zero dose** —
                                    `N1B_OWNERLESS_LOSS_RATE` (the ownerless-cargo loss
                                    roll, its consequence and its `diag_lost` counter
                                    all built, constant `0.0`), `BLOCKADE_STAGING_DOSE`,
                                    `CAPACITY_BIND_DOSE`, and the house trading web on
                                    the map (`OverlayManager.ts:1602` already routes
                                    seat→city lanes through the real corridor
                                    resolver — undirected, unlabelled, no goods in
                                    it). Twelve slices: signatures (no sim change at
                                    all) · eight new manufactured goods wiring the
                                    five ORPHAN RAWS that today have zero downstream
                                    consumers (clay, coal, alum, hemp, pitch) · the
                                    annona carrier-class split · three dose walks
                                    (ownerless loss, a per-city guild cap, the routed
                                    blockade) · transit demand built inert · two
                                    atlas queries · the four-window split · the
                                    House/Craft Atlas with real on-map lane labelling
                                    · the Houses redesign (bump chart, sparkline
                                    strips, event ticker, dossier timeline). Carries a
                                    STOP MARKER (§7) so a short session still lands
                                    something coherent, a 13-item numbered QUEUE
                                    (§8, rule 36 — each naming what it waits for),
                                    and a risk register whose first line is that three
                                    doses in one session is the ceiling against an
                                    instrument that has flipped inside its own noise
                                    band five times
ACTORS_AND_CARRIAGE_PLAN.md       ← ⭐ MEASURED + PLANNED, one diagnostic built,
                                    no proposal implemented. WHO MOVES THE CARGO:
                                    `econ_measure_carrier_mix` finds **95.7% of all
                                    shipments are carried by nobody** (`owner = -1`)
                                    against 4.3% by the entire merchant-house layer.
                                    The ownerless branch needs no vessel slot, is not
                                    clamped by capital, and NEVER SINKS (`let lost =
                                    if owner >= 0 {..} else { false }`); the transfer
                                    itself sits outside the carrier resolution, so
                                    fleet/capital/risk govern who PROFITS, never what
                                    MOVES. House carriage cannot scale (5.5× the
                                    fleet on the large world → a LOWER house share).
                                    Names the frictionless residual as the likely
                                    cause of F2's −0.064 price/distance gradient.
                                    Eight gated proposals (N1 make LOCAL_HAUL_DAYS
                                    bind · N2 ban the lane+good not the carrier ·
                                    N3 the Company's chartered staple vs
                                    opportunistic venture + the guild→Company rename ·
                                    N4 kill `house_for`'s `.position()` incumbency
                                    bias · N5 the sailing window · N6 price-elastic
                                    demand · N7 the League · N8 make the market book
                                    honest). Carries its own recorded WRONG TURN
                                    (§5.1) and the note (§5.2) that
                                    `econ_inheritance_rules_fragment_differently`
                                    went RED then GREEN inside one day's commits —
                                    re-run it per DOSE STEP, not per phase
SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md
                                  ← ⭐ N5/N6/N7.1-7.2 BUILT, gated, zero/low dose.
                                    The design for the three proposals
                                    ACTORS_AND_CARRIAGE_PLAN left as sketches —
                                    N5 sailing window · N6 price-elastic demand ·
                                    N7 the League — each with data structures,
                                    hook sites, a zero-dose setting, gates by
                                    name and a not-built list.
                                    **Shipped this session**: N5 — `CampaignSim.
                                    base_days_season`/`season_slices` (a
                                    quantised per-lane u8 multiplier,
                                    `SEASON_SLICES = 4`, `SEASON_MULT_STEP =
                                    1/64`, capped at `SEASON_MAX_MULT = 3.0`),
                                    built at campaign start via
                                    `compute_route_days_matrix_for_season` and
                                    read through one `lane_days(a,b)` accessor
                                    wired into dispatch, the return leg and
                                    contract delivery — real live dose (not
                                    zero), since it carries no wealth-
                                    concentration risk per §5's build order.
                                    N6 — `DEMAND_ELASTICITY = [0,0,0]` (true
                                    no-op, `elastic_aggregate_mult`/`_e` split
                                    pure/parametrized for testing), applied to
                                    the category AGGREGATE outside `base_need`;
                                    a twin `needs_struct` buffer carries the
                                    STRUCTURAL need and `update_food_and_
                                    starvation` now reads it instead of the
                                    (elastic) `needs` — the one real behaviour
                                    change at zero dose, since food/starvation
                                    used to read the same buffer prices clear
                                    on. N7.1/7.2 — `League`/`Boycott` structs,
                                    `TickHub.league`, yearly formation
                                    (`maybe_form_leagues`, gated on tier +
                                    a shared trade tie + a shared threat) and
                                    the diet (`run_league_diet`, dues/drift/
                                    annexation-exit/seat-succession), a lane-
                                    scoped `Boycott` enforced in `dispatch`
                                    beside `quarantined`/`export_ban_until` —
                                    `LEAGUE_BOYCOTT_MAX = 0` (N7.3, the diet
                                    voting one, stays unbuilt/zero-dosed).
                                    Gates: `n5_season_multipliers_at_unity_
                                    are_a_noop`, `n5_a_lane_is_dearer_in_its_
                                    stormy_season`, `n6_elasticity_at_zero_
                                    is_a_noop`, `n6_a_dearer_good_is_bought_
                                    less`, `n6_the_ration_is_not_elastic`,
                                    `n7_a_world_with_no_leagues_is_bit_
                                    identical`, `n7_a_league_is_not_a_realm`,
                                    `n7_leagues_form_and_dissolve`, `n7_
                                    boycott_is_inert_at_zero`, `n7_a_
                                    boycotted_city_reroutes` — all in `tick::
                                    tests`, plus `simulate_decades_reports_
                                    dynamics` re-verified (wealth bounded,
                                    richest 278,201 over 50y). `econ_
                                    measure_league_formation` (§4.3's own
                                    named instrument) is NOT yet built — the
                                    unit-level formation/dissolution gate
                                    above stands in for it this session.
                                    **Three findings from writing it are worth
                                    more than the designs.** (1) N5 is mostly
                                    ALREADY BUILT world-side: `build_coarse_cost`
                                    takes `season`/`months` and already closes
                                    snow-shut passes and stormy sailing windows
                                    off real `storm_base`/`reef_risk`/elevation,
                                    and the campaign's own
                                    `compute_route_days_matrix` calls it with
                                    `season = -1` — "no seasonal closure". N5 is
                                    calling that per season, storing a quantised
                                    u8 per-lane multiplier (v=0 ⇒ exactly 1.0 ⇒
                                    the bit-identical gate), read through one
                                    `lane_days` accessor. Its real target is the
                                    scorecard's within-city grain price CV of
                                    **0.000** (band 0.30–0.50), not the gradient.
                                    (2) Demand is NOT "perfectly price-inelastic"
                                    — category substitution already weights by
                                    `pref / rel` where `rel = price/base_value`,
                                    so cross-price elasticity is live; only the
                                    category AGGREGATE is fixed. The design's own
                                    sharpest rule: **elasticity belongs to the
                                    market, not to the ration** — it applies
                                    OUTSIDE `base_need` (whose other callers are
                                    `need_scale`'s calibration, the starvation
                                    sums and civic provisioning), and every
                                    welfare signal keeps reading a parallel
                                    STRUCTURAL need, or a council stops
                                    provisioning exactly when prices spike.
                                    (3) N7's stated dependency is WRONG: N2 as
                                    built bans a hub × GOOD, but a boycott is
                                    lane-scoped ("we bar trade with that city"),
                                    which nothing expresses — N7 needs an N2
                                    EXTENSION, not N2 dosed. Build order inside
                                    N7 is the institution first and the weapon
                                    last, because a boycott is N2's market
                                    closure × members and N2's single-city
                                    version broke the hard wealth bound twice
MONEY_MINES_AND_GOODS_PLAN.md     ← ⭐ SLICES 1-7 BUILT AND GATED (slice 0's pure
                                    file-split refactor deferred — a hygiene move,
                                    not a behaviour change, and lower priority than
                                    landing the fixes themselves). Shipped this
                                    session: **slice 1** — a note-funded loan now
                                    RETIRES its notes on default
                                    (`BANK_FORECLOSURE_RECOVERY`), closing F1a's
                                    permanent equity leak. **Slice 2** — `Loan.
                                    arrears_months` + a solvency-based affordability
                                    test (not `wealth > pay*1.2`): a borrower who
                                    misses a payment capitalizes the shortfall and
                                    accrues arrears, defaulting only past
                                    `LOAN_ARREARS_LIMIT` (6 months). **Slice 3** —
                                    `BANK_MAX_BORROWER_SHARE`/`BANK_MAX_LOAN_FRAC`/
                                    `BANK_BORROWER_POOL_K`: no borrower may hold more
                                    than 35% of a bank's book, loans are capped at
                                    20% of reserves, and the borrower draw is a
                                    top-track-record POOL (`stable_growth_years`),
                                    never wealth-weighted (N4's own lesson: weighting
                                    by wealth/political_power measurably inverted the
                                    inheritance gate). **Slice 4** — `bank_maybe_
                                    invest` now stakes a Mine/Quarry too, via the
                                    already-typed OFFTAKE share kind (`payout: 0`)
                                    instead of dividend. **Slice 5a/5d (F2)** —
                                    `deposits::WorkingKind` (Shaft/Open/Placer/Pan) +
                                    `default_working_for(id)` replace the substring
                                    cascade for mine-vs-quarry, per-district (a
                                    `Deposit`/`MineSite` each carry their own
                                    `working`); the self-sealing `workable`/`served`
                                    checks in `province.rs`/`colonies.rs` now compare
                                    against `d.working.estate_kind()` instead of a
                                    hard-coded Mine (2), so a flooded QUARRY body can
                                    finally become workable. **Slice 5b/5c** — new
                                    `maybe_found_extraction_estate` (yearly) sites a
                                    Mine/Quarry ON its real ore body (scored by
                                    extent × depth-workability × local price ÷ a
                                    distance-scaled haulage cost, so a rich remote
                                    body can lose to a poorer reachable one), founder
                                    = a house or the seat city (D2); a shallow/
                                    moderate body self-funds, a deep/flooded one
                                    DEMANDS a real bank loan (`purpose: "mine"`,
                                    skipped entirely with no bank reachable — D3).
                                    **Slice 6a/6b** — district counts re-authored
                                    against the plan's own historical table (copper
                                    3×, diamond 3×, silver/garnet/amethyst/topaz/
                                    carnelian lowered, iron/gold/tin/lapis/ambergris
                                    unchanged), gated by
                                    `deposit_counts_span_an_order_of_magnitude`.
                                    **Slice 7** — a `Distribution::Deposits` good now
                                    bypasses the belt-mean floor wherever this
                                    province holds a real working (both
                                    `campaign_province_goods` and `campaign_province_
                                    potential`, F4); `province_good_potential_base`
                                    reads Σ(extent × depth-workability) over the
                                    province's own workings for a deposit good
                                    instead of `belt × prov_cap × land_share`; the
                                    manufactured-goods filter now runs ONCE inside
                                    `generate_and_persist_provinces` before
                                    persisting (F5), with `get_provinces`/`get_
                                    province_layer` kept only as the fix for a world
                                    saved before that line landed. NOT done: the
                                    long-run `econ_measure_finance` before/after
                                    lifespan comparison and the mines-founded-per-
                                    century measurement §7 of the plan asks for —
                                    both are `#[ignore]`d, multi-decade diagnostics
                                    outside this session's time budget; the unit
                                    gates above (verified failing on the pre-fix
                                    code by inspection) stand in for them. One causal
                                    chain: banks survive → mines get financed → ore
                                    becomes an industry → deposit goods show up in
                                    the province view. Six measured findings, of
                                    which three are outright bugs. **F1a — a
                                    DEFAULTED note-funded loan never retires its
                                    notes** (`money.rs:612-626` writes off the asset
                                    but `notes_issued` is reduced only on the
                                    REPAYMENT path at `:624`), so every default
                                    destroys 0.6 × principal of equity permanently:
                                    the same hole the comment at `money.rs:604`
                                    records closing for repayment, never closed for
                                    default. That, plus one missed payment killing
                                    the whole loan (no arrears state at all, while
                                    the borrowing house gets a one-year grace),
                                    every loan going to the single richest resident,
                                    and half of reserves in one loan at 3× leverage
                                    against a 0.6%/month spread, is why banks fail in
                                    ~5 years. **F2 — every gemstone is a QUARRY**:
                                    `estate_kind_for_good` (`mod.rs:2112`) still
                                    decides mine-vs-quarry by SUBSTRING MATCH on the
                                    good's name even inside the post-S4 DIST_DEPOSITS
                                    branch, so diamond/ruby/sapphire/emerald/jade/
                                    lapis/turquoise/marble/salt all fall to kind 8 and
                                    the ENTIRE mining mechanic (`mine_depth`,
                                    drainage, flooded-body unlocking) is structurally
                                    unreachable for them — self-sealingly, since both
                                    `workable` (`province.rs:767`) and `served`
                                    (`colonies.rs:1524`) hard-code `estate_kind == 2`.
                                    **F3 — an estate is never sited on ore**:
                                    `maybe_found_estate` maximises `base_per_capita ×
                                    demand_pressure` (so a deposit good never wins)
                                    and places at a hash offset from the parent
                                    (`houses.rs:792`), never consulting
                                    `mine_deposits` — so §8.16's eleven deposit
                                    models are generated and then read by almost
                                    nothing. **F4 — deposit goods are FILTERED OUT of
                                    the province list**: `Province.good_belt` is a
                                    coverage MEAN, so a 1-3 cell diamond district in
                                    a 3,000-cell province lands at ≈0.0318 against a
                                    `PROV_GOOD_ABSENT_BELT` of 0.0314 — a LARGER
                                    province hides a RICHER mine, while
                                    `ProvincePotential.deposits` draws the working
                                    anyway, so the plate and the list disagree.
                                    **F5 — the manufactured filter runs in 1 of 4
                                    exits** (`get_province_layer` only), which is why
                                    manufactured goods appear right after generation
                                    and vanish after a reload. **F6 — deposit counts**
                                    put diamond/jade at 2 districts and lapis/
                                    ambergris at 1 on a default world, while amethyst,
                                    topaz, garnet and carnelian each get TWICE copper's
                                    6. Eight slices (constant-split refactor · notes
                                    bug · arrears · book diversification · offtake
                                    stakes in extraction works, reusing the already-
                                    typed `Share.payout == 0` · ore-sited financed
                                    mines with shallow self-funding and deep credit ·
                                    historical + area-scaled district counts · the
                                    three province-view fixes), each with its gate and
                                    its dose. Decisions recorded in §2; §5 is a
                                    NUMBERED QUEUE of the work that follows slices
                                    0-7, each item naming what it waits for — a
                                    schedule, never a refusal
PLACES_DEMAND_AND_GROWTH_PLAN.md  ← ⭐ ALL SEVEN SLICES BUILT AND GATED.
                                    Slices 1-3 shipped as a SURGICAL variant of
                                    the plan's literal design (see below for
                                    why); 4/5/6/7 ship as dosed no-ops or
                                    small live preferences, per their own
                                    entries further down. The companion:
                                    where towns are · how big they get · why anyone
                                    trades. **Slice 6 (F9, the cheap keystone) is
                                    live in code**: `LOCAL_SATIETY`
                                    (`tick/mod.rs`) is a hub's own jading toward a
                                    comfort/luxury good it largely makes itself —
                                    `self_supply_of` compares per-capita production
                                    against the per-capita need `base_need` itself
                                    uses, and `local_satiety_mult`/its pure,
                                    testable twin `local_satiety_mult_e` (the same
                                    split N6's `elastic_aggregate_mult`/`_e` uses)
                                    shave the MARKET-FACING `needs[h][g]` by up to
                                    the dose as self-supply approaches 1.0 — never
                                    `needs_struct` (the structural ration
                                    `lack_basic`/starvation/crisis relief read is
                                    untouched, applied at the wiring site in
                                    `mod.rs` alongside fashion/cultural-taste, not
                                    inside `base_need`), and never a basic good
                                    (`need_tier < 1` is a hard no-op). Ships at
                                    `LOCAL_SATIETY = 0.0` — a true no-op, gated by
                                    `local_satiety_is_a_noop_at_zero`,
                                    `a_producing_city_wants_less_of_its_own_
                                    luxury`, `a_basic_good_is_never_subject_to_
                                    satiety`, `the_structural_ration_is_not_
                                    affected_by_satiety` — and is the one term to
                                    dose-walk before the continuous `foreign_lux`
                                    rewrite and the `stock_origin` provenance
                                    accumulator now compound on the SAME expression
                                    — see slice 7 below.
                                    **Settlements were ONE flat greedy pass**
                                    (`settlements.rs:522`, `generate_settlements`)
                                    with a single spacing radius and tier assigned
                                    afterwards, so a "capital" was just a town
                                    that scored well; and provinces (phase 7b,
                                    seeded FROM settlements at `provinces.rs:1013`)
                                    fell back to jittered FILLER seeds, so a
                                    province could hold zero settlements or five
                                    of equal rank.
                                    **Slices 1-3 are BUILT, as a SURGICAL variant
                                    of the plan's literal design rather than the
                                    literal pipeline reorder (primary pass →
                                    provinces → secondary pass as three separate
                                    phase-7/7b/7c steps).** The literal reorder
                                    would change the WorkflowPanel step sequence,
                                    phase numbering and `ColumnSet` wiring — a
                                    much larger blast radius, and the plan's own
                                    risk register calls for empirically tuning
                                    `PRIMARY_MIN_DIST` against rendered worlds,
                                    which a text-only session cannot do. Instead:
                                    **Slice 1 (F1)** — `mark_primary_settlements`
                                    (`settlements.rs`) runs as a POST-HOC
                                    classification pass immediately after
                                    `generate_settlements`' existing greedy
                                    selection completes, over the SAME final site
                                    list — it changes not one settlement's
                                    position, population, tier or count (gated by
                                    the pre-existing `the_base_settlement_set_is_
                                    unchanged`), only which ones gain the new
                                    `Settlement.primary: bool`. Ranks by `0.65 *
                                    access + 0.35 * habitability score`
                                    (`access` — coast/river-mouth/nav/crossroads
                                    — already computed inline per site, reused
                                    rather than duplicated) under a WIDE spacing
                                    radius (`PRIMARY_SPACING_MULT` = 5×
                                    `min_dist`, never halved on a river cell —
                                    stringing capitals down one valley is
                                    explicitly a SECONDARY behaviour). Always
                                    marks at least one settlement on a non-empty
                                    list. Outposts and step-7a junction sites are
                                    never primary. **Slice 2 (F2, D1)** —
                                    `generate_provinces` (`provinces.rs`) now (a)
                                    seeds from PRIMARIES ONLY when any settlement
                                    in the list carries `.primary` (`any_primary`
                                    — falls back to the WHOLE list, today's exact
                                    pre-slice behaviour, the instant nothing is
                                    marked, so an old world's settlement data or
                                    any caller that never ran the marking pass is
                                    untouched); (b) every FILLER province (no
                                    settlement fell inside it) gets a REAL
                                    settlement FOUNDED at its own best-scoring
                                    land cell (`best_cell_of_province`, one
                                    linear pass over `buf.habitability` — the
                                    SAME field the base settlement pass ranks
                                    candidates by) rather than the meaningless
                                    jittered seed cell, and never at the
                                    centroid (D4). `generate_provinces` widened
                                    its return type to also carry `Vec<Settlement>`
                                    (the founded seats); every caller
                                    (`sim_generate_provinces`, both run-alls,
                                    `ProvincePanel.tsx`/`StepSettlements.tsx`)
                                    folds them into its own settlement list —
                                    rule 34's "generating data is not loading it"
                                    discipline applied to a new artefact, not
                                    just an old one. **Slice 3 (D3)** —
                                    `place_settlement_at` takes a new `is_capital`
                                    parameter (`Option<bool>` at the Tauri command
                                    boundary, defaulting to `false` — an older
                                    frontend build stays compatible) and sets
                                    `.primary` from it; the UI is an explicit
                                    "Place as province capital" checkbox next to
                                    the existing Place Settlement tool
                                    (`uiStore.placeSettlementIsCapital`), never an
                                    implicit "manual ⇒ capital" rule. Slice 3c's
                                    route re-verification was not separately
                                    exercised (no new route code was written —
                                    Stage A's existing fall-through shortlist and
                                    `TRADE_COMPONENT_HORIZON_KM` already handle an
                                    arbitrary settlement count, and this slice
                                    adds settlements through the exact same
                                    `Settlement` list every route command already
                                    consumes). Gates: `every_province_has_
                                    exactly_one_seat`, `a_province_seat_is_not_
                                    its_centroid`, `an_old_settlement_list_seeds_
                                    from_everyone`, `primary_settlements_are_
                                    regional_not_merely_fertile`, `mark_primary_
                                    always_marks_at_least_one` (all in
                                    `provinces::tests`/`settlements.rs`), plus
                                    the pre-existing `provinces::tests` (16/16)
                                    and `goods_` (17/17, `sim::step8_
                                    biological_goods::goods_validation`) suites
                                    re-verified clean — settlement positions are
                                    unchanged so goods catchments cannot have
                                    moved. **Deliberately not measured here**:
                                    province count/mean-area/settlements-per-
                                    province before/after on a real generated
                                    world (the plan's own scoreboard ask) — that
                                    is visual cartography work a future session
                                    with the ability to render and inspect a
                                    generated world should do before tuning
                                    `PRIMARY_SPACING_MULT` further.
                                    **Slice 4 (D5/D6, F4) is BUILT.** `CAPACITY_
                                    LAND_WEIGHT` (`tick/mod.rs`) replaces a city's
                                    `founding_pop`-only growth ceiling with a blend
                                    toward `land_capacity_blend(founding_pop,
                                    land_capacity, weight)`, where `land_capacity`
                                    is the hub's own province's `prov_cap` shared
                                    among the province's live hubs by trade weight
                                    (`disease.rs::update_food_and_starvation`,
                                    precomputed `prov_trade_weight` map, skipped
                                    entirely without a province layer). D6's
                                    `works_dev_mult` (a hub's own estates +
                                    structures, capped at `WORKS_DEV_CAP`) is
                                    gated behind the SAME dose flag rather than
                                    shipped live, so the whole slice moves as one
                                    unit. Ships at `CAPACITY_LAND_WEIGHT = 0.0` —
                                    a true no-op — gated by
                                    `capacity_land_weight_is_a_noop_at_zero`,
                                    `a_well_sited_small_town_can_overtake_a_badly_
                                    sited_large_one`, `works_capacity_is_bounded`.
                                    **Raising the dose is real future work, NOT
                                    done here**: CLAUDE.md's own record for the
                                    adjacent `WORLD_AGE_DEV_CAP` shows this growth
                                    pass is CHAOTIC-SENSITIVE to uniform per-hub
                                    capacity nudges — re-run
                                    `simulate_decades_reports_dynamics`'s
                                    sustained-runaway-wealth guard per dose step.
                                    **Slice 5 (F5) was already PARTLY built**
                                    (`EMPTY_PROVINCE_FOUND_BONUS` existed for
                                    `maybe_found_settlement_colony` alone) — now
                                    extended to `maybe_found_food_colony` and
                                    `maybe_found_house_outpost` too, through one
                                    shared pure helper (`province_founding_bonus`,
                                    a PREFERENCE never a rule — an unknown
                                    province, -1, or an already-settled one is an
                                    exact 0.0). `province_is_settled` widened to
                                    `pub(crate)` for the cross-module call. Gated
                                    by `colonisation_prefers_an_empty_province_
                                    but_is_not_forced_into_one` and
                                    `province_is_settled_reads_hub_province`.
                                    `maybe_found_mining_colony` (scored off real ore
                                    deposits, not the fertile-land list) and
                                    `maybe_found_estate`/`maybe_found_route_post`
                                    (hinterland estates, not new hubs) are
                                    deliberately left untouched — the bonus is
                                    about where a new HUB forms, not an estate.
                                    **Growth: a city's ceiling is a multiple of
                                    `founding_pop`** (`disease.rs:509`) is now
                                    FIXED by slice 4 above (at a live dose). **Demand:
                                    the foreign-craving term is a BINARY SWITCH**
                                    (`production.rs:895` — a city producing ANY amount
                                    of a luxury gets ZERO import craving) with no
                                    gradient, no distance and no provenance —
                                    **`foreign_lux` in `base_need` is STILL the
                                    binary switch** (continuity was judged too
                                    high-risk to rewrite blind — it's the one
                                    other term `COMFORT_IMPORT_FRAC`'s own
                                    five-time-broken-gate history already warns is
                                    fragile — left QUEUED, not attempted). **Slice
                                    6 (`LOCAL_SATIETY`) and slice 7 (provenance +
                                    distance prestige) are BOTH BUILT.** Slice 7:
                                    `InTransit.origin_km` (real routed distance,
                                    accumulated across relay hops in the arrivals
                                    pass, `mod.rs`) feeds `TickHub.stock_origin[g]`
                                    (a decaying EMA, `STOCK_ORIGIN_DECAY`, per
                                    arrival — option (i) from the plan, not the
                                    heavier grade×provenance matrix); `FOREIGN_
                                    PRESTIGE`/`PRESTIGE_REF_KM` fold
                                    `foreign_prestige_mult_e` into the same
                                    MARKET-FACING `needs` term slice 6 uses (never
                                    `needs_struct`; never a basic good). Ships at
                                    `FOREIGN_PRESTIGE = 0.0`, gated by
                                    `foreign_prestige_is_a_noop_at_zero`,
                                    `a_far_travelled_luxury_is_wanted_more_than_a_
                                    near_one`, `distance_prestige_never_touches_a_
                                    basic_good`, `an_old_save_with_no_origin_data_
                                    is_bit_identical`. Every demand term ships at
                                    dose 0 and is walked ONE AT A TIME (they
                                    multiply the same expression), gated by
                                    `econ_expenditure_shares_resemble_a_household`
                                    — NEITHER has been dose-walked yet; that is the
                                    next session's job, one dose at a time with
                                    the other pinned at zero (ACTORS_AND_CARRIAGE_
                                    PLAN.md §5.2's lesson). Per-culture demand
                                    profiles are §5's item 8 — the largest
                                    remaining demand item, QUEUED rather than
                                    dropped: all three terms multiply the same
                                    expression, so 8 begins once 6 and 7 are
                                    dose-walked live
PROVINCE_SYSTEM_PLAN.md           ← The province layer's design + status (see FIX_PLAN B1);
                                    the shipped algorithm itself is §8.10 above
DEPOSITS_AND_MINING_PLAN.md       ← ⭐ BUILT — all five slices, gated. Ore
                                    geology (§8.16) + grade→quality rewire + txt
                                    goods import & 8 new minerals (1-3). Slice 4:
                                    MINE (`estate_kind` 2) vs QUARRY (8, split
                                    off) as genuinely different mechanics — a
                                    mine's `mine_depth` taxes its OWN upgrade cost
                                    when deep/flooded (`MINE_UPGRADE_COST_MULT`,
                                    never the baseline output, already baked in
                                    at worldgen); a quarry is gated by TRANSPORT
                                    instead (`QUARRY_INLAND_UPGRADE_COST_MULT`);
                                    mercury → silver amalgamation is a real
                                    consumable extraction input
                                    (`apply_mercury_amalgamation`); a body KNOWN
                                    `EXTENT_WEAK` now declines to a floor under
                                    pressure (D3), everything else still
                                    persists. Slice 5: the quarry/mine window
                                    (districts → workings, `ProvinceInspector.
                                    tsx`); mining SETTLEMENTS (the Potosí case,
                                    `maybe_found_mining_colony`) found on a real
                                    GREAT/WORLD-CLASS strike, boom, and DECLINE
                                    rather than die when their food lifeline
                                    fails; a settlement's trade catchment radius
                                    grows slowly with age as a pure derived read
                                    (`catchment_radius_km`, never stored per-hub
                                    state). Not built: a drawn growing-catchment
                                    DISC on the survey plate (the value is real
                                    and served; no new canvas layer renders it)
                                    and the Mons Claudianus treasury-quarry edge
                                    case. Carries the measured findings that
                                    motivated it and its own list of the work
                                    still owed
WORLD_AND_TRADE_MASTER_PLAN.md    ← ⭐ THE ACTIVE PLAN. Three parts, one
                                    dependency chain (the land decides where
                                    trade can go). Part I tectonics/rivers/
                                    provinces/shelves — slices 1,2,3,5,7,8 BUILT
                                    and gated, 4 shipped scoped-down (no
                                    per-plate Euler-pole identity), 6 not
                                    attempted. Part II outpost connectivity &
                                    the entrepôt — measured, NOT built. Part III
                                    exploration / the known world / transport
                                    modes — decided, NOT built. Read its build
                                    order first: transport modes lead, because
                                    they are the biggest lever on the measured
                                    −0.064 price/distance gradient, have an
                                    instrument that already exists
                                    (`econ_fidelity_scorecard`), and may make
                                    the entrepôt and siting rules unnecessary
ITCZ_AND_LAND_TOOLS_PLAN.md       ← ⭐ COMMITS 1-2 BUILT, 3a ATTEMPTED AND
                                    REVERTED (negative result), 3b-c NOT
                                    ATTEMPTED. Stage-1 LASSO AREA TOOLS shipped
                                    (smooth↔roughen the coast, fjords, island/
                                    volcanic-arc chains, bulk fill — each one
                                    `buf.save`d so undo and re-roll come free,
                                    §8.25) and FOUR NEW ELEVATION GENERATORS
                                    shipped (rift/horst-graben · glaciated
                                    fjordland · plateau & mesa · volcanic hotspot,
                                    all through `apply_elevation_model` so both
                                    run-alls honour them, and the >25%-disagreement
                                    gate extended from 4 models to 8 — `glaciated`
                                    vs `shape`, the pair named as most likely to
                                    fail since glaciated starts FROM shape, passes).
                                    Carries the measured ITCZ finding that
                                    motivated the climate work: **there are TWO
                                    ITCZs**, `seasonal.rs::itcz_latitude` (8°, summer
                                    hemisphere 5-35°) for the WIND and
                                    `precipitation.rs::compute_itcz_shift_zonal`
                                    (±12° plus a ±10° migration, both hemispheres
                                    0-30°) for the RAIN — different amplitudes,
                                    different land measures, never reconciled, and
                                    the overlay draws both. **Unifying them (3a) was
                                    tried both directions and both regress a
                                    hard-asserted Earth-gate floor** — wind-adopts-
                                    rain costs exact-zone (39.0%→38.1%, floor 38.8%),
                                    rain-adopts-wind costs BOTH main-class (70.2%→
                                    68.4%, floor 70.2%) and exact-zone (→38.5%) —
                                    because both formulas were independently already
                                    tuned to the Earth gate, so a straight swap only
                                    discards whichever side's tuning gets overridden.
                                    Both reverted; the two-formula status quo ships
                                    unchanged. Full table in `docs/FIX_PLAN.md` A4.
                                    FIX_PLAN A4's pressure field (3b-c) was never
                                    attempted, since the plan built it to depend on
                                    3a landing first. Baseline measured at `7786da8`:
                                    Mumbai 161 mm (real ~2200), Bangladesh 84 mm at a
                                    **25% summer fraction** — it rains more in winter
                                    there, i.e. the monsoon runs backwards, not
                                    merely weak — STILL UNFIXED, since the
                                    prerequisite unification did not land.
CITY_PROVINCE_WAR_PLAN.md         ← ⭐ APPROVED, NOT YET BUILT. The next three workstreams:
                                    the settlement panel rework · provinces (enclave fix,
                                    sizing, real-terrain view, goods & exploitation) ·
                                    the political layer (city leader as a house office,
                                    city tiers, the city-as-state, and war). Carries its
                                    own caveat list (§5) — incl. that it REVERSES
                                    PROVINCE_SYSTEM_PLAN's "enclaves survive" decision —
                                    and its own §6 list of the work still owed
TRADE_STAGING_AND_POSTS_PLAN.md   ← ⭐ SLICES 3-4 BUILT AND DOSED LIVE (cargo no
                                    longer teleports: the entrepôt composition +
                                    the range-based staging relay, both real,
                                    both chain through `InTransit.via`/`hops`);
                                    slices 1, 2, 5, 6, 7 not built. This entry
                                    was stale (said "nothing built") until a
                                    2026-09-20 session corrected it while
                                    building `PORT_COMPETITION_PLAN.md` — see
                                    that doc's §0 for exactly what was
                                    mis-diagnosed and how it was checked
                                    against the real dispatch/arrivals code.
                                    Seven slices that
                                    make a long lane a RELAY instead of a teleport.
                                    Premise: a 7,000 km lane is historically ordinary;
                                    one with no stops, no middleman and no city grown
                                    along it is not. Carries the measured findings that
                                    motivated it — the trade HORIZON is 0.24×world_w
                                    (**9,617 km** on a 3600 grid, so the reported
                                    7,000 km lanes are legal by design), the three
                                    horizon-bypassing rescue passes price their lanes at
                                    FLAT 55 km/day with no terrain multiplier (so an
                                    absurd long lane is the CHEAPEST on the map), travel
                                    mode is `coastal_a && coastal_b` alone (a river or
                                    lake city is never "sea", so all its trade reads as
                                    overland), the campaign route matrix WAS built with
                                    `rivers_json = ""` so `is_river` was ALL FALSE and the
                                    river cost rungs never reached — **that one is now
                                    FIXED** (`metadata["rivers"]` is kept in sync by
                                    `persist_rivers` and read by
                                    `compute_route_days_matrix_for_season`; the finding
                                    survives only for hubs founded DURING a campaign,
                                    which still fall back to a straight line — see
                                    `ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` §2),
                                    `hub_pull` is
                                    applied TWICE on the same axis, and loss is a flat
                                    per-shipment roll independent of distance. Also the
                                    three things that already exist and make the plan
                                    cheap: `house_barred` + `pay_to_regain_markets` (the
                                    embargo weapon, 70% built), the full
                                    outpost→colony→free-city ladder incl. a shipped
                                    `age>=50 && pop>=15_000` independence gate, and
                                    `neighbor_path` — the sim ALREADY moves people in
                                    legs; cargo is the only thing that teleports. Its
                                    own risk register names the one likely to bite:
                                    embargo as a WEALTH-CONCENTRATION SPIRAL against the
                                    hard-won top-10% share. Companion UI schematics
                                    ("Break of Bulk", 8 plates) are an artifact, not a
                                    repo file (§2.2)
PORT_COMPETITION_PLAN.md          ← ⭐ SLICE 1 BUILT AND GATED (dosed at zero — a true
                                    no-op); slices 2-3 QUEUED. Grew out of a user
                                    report that the map showed no port where cargo
                                    breaks bulk. Corrects this session's own first,
                                    wrong diagnosis (§0): the map's relay MARKER was
                                    wired to a redundant, independently re-guessed
                                    `route_outlet` lookup instead of the real
                                    `InTransit.via` already sitting on every in-flight
                                    shipment — fixed, no guessing left in
                                    `campaign_merchant_routes()`. Also records that
                                    "big ports act as trade hubs" already ships
                                    (`hub_class`/`relay_count`/`is_transit_knot`, real
                                    map markers — blue square/red triangle/teal
                                    hourglass — since before this session; nothing
                                    built here for that ask). New: a small teal ring
                                    at every GEOMETRIC land↔sea/river medium change
                                    along a routed corridor
                                    (`OverlayManager.drawMediumTransitionRings`),
                                    distinct from the economic relay ring — cargo
                                    cannot sail on land, so this is real regardless of
                                    whether dispatch treats the voyage as one leg or
                                    several. And `TickHub.transit_toll_mult` — a
                                    port's own anchorage/staple due, biasing which
                                    outlet wins the entrepôt role in `rebuild_routes`
                                    so two nearby ports can genuinely compete on price
                                    for the same relay trade — shipped at
                                    `PORT_TOLL_COMPETITION_DOSE = 0.0`, proven inert by
                                    `port_toll_competition_is_a_noop_at_zero_dose`.
                                    Raising the dose needs its own `#[ignore]`d
                                    firing-rate diagnostic first (the `maybe_grant_
                                    provinces`/R4 precedent) then a proper dose walk
                                    against `econ_` + the multi-seed inheritance gate —
                                    neither done, both queued by name, not silently
                                    assumed safe
MERCHANT_VESSELS_AND_INFORMATION_PLAN.md
                                  ← ⭐ DESIGN, NOT APPROVED, NOTHING BUILT. Six
                                    staged changes to the trade mechanism, built on
                                    one finding: a VESSEL IS NOT A THING. `fleet_sea`/
                                    `_river`/`_caravan` are three counters on `House`
                                    with no identity, location or cargo; `dispatch`
                                    decrements one slot per shipment REGARDLESS of
                                    quantity, and `SHIP/BOAT/CARAVAN_CAPACITY` are
                                    read only by futures-contract delivery. So one
                                    shipment carries exactly one good, and "which
                                    vessels are in port" is missing STATE, not a
                                    missing query. Stage 1 makes a `Vessel` real
                                    (manifest, capacity that binds, port dwell);
                                    stage 4 is the substantive one — a house trades on
                                    the price it BELIEVES, with a spread set by how
                                    fresh its knowledge is (never been → surveyed →
                                    office → controls the seat), which is the
                                    information-decay mechanism Persson/Federico
                                    credit for real market integration and the most
                                    plausible fix for the measured −0.026 gradient.
                                    Stages 5-6 generalise the EXISTING `envoys.rs`
                                    into survey agents and finally wire the EXISTING
                                    `RouteProspect`/`establish_corridor` loop into
                                    trade (today a corridor feeds only the overlay).
                                    Stage 7 is the STAPLE RIGHT — a house holding a
                                    seat sets the price on its chartered goods within
                                    a band, and rivals learn to route elsewhere; it
                                    REQUIRES stage 4, because price-setting with no
                                    way for rivals to learn the price is bad is upside
                                    with no downside. Carries its own gates per stage
                                    — including the companion gate that matters most,
                                    that long-haul trade VOLUME must not collapse —
                                    and its own list of the work still owed. Four
                                    design decisions are recorded at the top
                                    (individual vessels · houses+guilds only ·
                                    privilege staged as terms-then-price · no code yet)
REALM_AND_GOVERNMENT_PLAN.md      ← ⭐ R1-R5 BUILT, each partially. THE FIRST
                                    COUNTRIES: a merchant house takes a city
                                    (`captor_house`), PROCLAIMS sovereignty after a
                                    hard year-50 floor + a decade holding it
                                    (`maybe_proclaim_realms`), and is ELEVATED —
                                    its wealth and trade assets become the crown's,
                                    the house leaves the merchant world (`crowned`,
                                    never `defunct` — §5.1) and becomes a dynasty
                                    with a real GENEALOGY (`Realm.family` — persons,
                                    births, child mortality, aging, succession by
                                    the culture's LineRule, regency for a minor
                                    heir; `tick/realms.rs`). Realms hold provinces
                                    (`prov_realm`, a THIRD authority layer above
                                    rule 24/25), and `compute_states` now reads
                                    this real persisted state rather than deriving
                                    one — a realm outlives its capital's tier later
                                    dropping (the Karakorum rule). TAXATION is real:
                                    the harvest tithe redirects to the crown scaled
                                    by COLLECTION EFFICIENCY (cohesion × distance
                                    from the capital — "pre-modern states were
                                    limited by what they could collect, not what
                                    they charged"), plus two crown-set levies (poll,
                                    customs) and TAX FARMING (a house buys N years
                                    of tithe collection for cash now — `publicani`/
                                    *iltizam*). Realm coin deliberately deferred
                                    (§7 of the plan) rather than rushed into the
                                    tuned `money.rs` coinage system. WAR gained three
                                    priced goals — Humiliate/Enthrone/Vassalize —
                                    and realm-aware resolution: a sovereign hub's
                                    TRUE ruler is its crown, and a ceded province or
                                    annexed member city now correctly carries its
                                    sovereignty with it (`WAR_GOAL_PROVINCE`/
                                    `WAR_GOAL_ANNEX`, previously silent corruptions
                                    of rule 25 the moment either touched sovereign
                                    territory). `war_affordable_treasury` fixes an
                                    R3 side effect that would have made every realm
                                    systematically too poor to declare war. Surfaced
                                    in the frontend Realms panel (relabelled from
                                    States). STILL UNBUILT: realm coin; the "one
                                    war, one score, many cities" MULTI-CITY pooling
                                    §1.4 itself names (R4 ships the war GOALS, not
                                    this); separate peace; the two-war penalty;
                                    free-city war participation; annexing/
                                    vassalizing a realm's own CAPITAL (a full
                                    foreign-crown conquest, explicitly guarded off
                                    in `apply_war_goal` pending its own design).
                                    **THREE FORMATION PATHS now, not one.** Every
                                    realm used to be a merchant republic wearing
                                    the word "Kingdom": both eligibility paths ran
                                    through a house. `Realm.founding_path` adds
                                    PATH B (`maybe_proclaim_city_realms` — a tier-1
                                    city proclaims for itself, the FIRST reader
                                    `hub.tier`/`hub.standing` has ever had) and
                                    PATH C (`maybe_proclaim_culture_realms` — a
                                    contiguous single-culture bloc of at least
                                    `REALM_CULTURE_MIN_PROVINCES` provinces
                                    unifies under its largest city, over
                                    `prov_culture` + `prov_neighbors`; cite the
                                    constant, never a number — this line read
                                    "≥4" against a shipped value of 2 until a
                                    v2.0 audit caught it).
                                    `Realm.government` splits DYNASTIC from CIVIC:
                                    a republic (`found_civic_realm`) has no
                                    `family`, no succession by birth, and
                                    `ruling_house = u32::MAX` — every reader must
                                    resolve it through `houses.get`, never index it
                                    raw (war.rs did, and the first republic to win
                                    a war would have panicked the tick).
                                    **THREE DEAD FIELDS REVIVED**, together because
                                    they are one mechanism: `update_realm_cohesion`
                                    (yearly) drifts cohesion toward the founding
                                    path's target, dragged down per culturally-
                                    FOREIGN province held — the brake on unlimited
                                    expansion — and nudged by `legitimacy`, which
                                    now finally has a reader; `assign_realm_ranks`
                                    (yearly) is the percentile ladder + top-rank
                                    absolute floor + hysteresis that `Realm.rank`'s
                                    own doc already described and nobody had
                                    written, with COHESION as one of its four axes;
                                    and `realm_title_for(rank, government)` replaces
                                    the flat four-name list that styled a house
                                    holding one town "King". **The measured result
                                    is a NEGATIVE one and matters more than the
                                    code**: a matched before/after of
                                    `econ_measure_realm_formation` gives 8 realms
                                    by year 170 BOTH before and after, because the
                                    reference world cannot express either new path
                                    (it seeds `prov_culture` as `Culture{i}`, a
                                    different culture per province, and never seeds
                                    `prov_neighbors`, so Path C early-returns; and
                                    its 30 undifferentiated cities never clear tier
                                    1's absolute standing floor). Both paths are
                                    gated by unit tests instead. That null result
                                    was then FIXED by building an instrument that
                                    works: `realm_reference_world` +
                                    `econ_measure_realm_paths` (72 cities · 24
                                    provinces · six peoples in contiguous blocs · a
                                    real neighbour graph · a rank-size city
                                    spread), kept SEPARATE from the scorecard's
                                    world because `prov_culture` feeds migration.
                                    Measured there: merchant-only 8.0 realms/
                                    century leaving 9 of 24 provinces permanently
                                    STATELESS, versus 11.5/century and 22 of 24
                                    under a crown with all three paths — the
                                    ablation is the evidence the paths matter.
                                    Three bugs it exposed that review had not:
                                    sovereignty was DOUBLE-ASSIGNED (a coronation
                                    collected by `prov_holder` without checking
                                    `prov_realm`, so two realms could list one
                                    province — rule 27's layers are independent and
                                    taking an owned province needs a war); Path C
                                    could never fire (it required the WHOLE culture
                                    bloc free, so one proclamation anywhere
                                    foreclosed a people's nationhood forever — it
                                    now unifies whatever of itself is still free
                                    and runs FIRST, since unification happens
                                    against existing statelets); and LANDLESS
                                    realms (a city proclaiming over a province
                                    another crown held — `has_free_province_at`
                                    gates both seat paths now). `REALM_RAMP_YEARS`
                                    removes the year-50 cliff so crowns fade in
                                    over a generation rather than all appearing in
                                    one year. **`suppress_realms`** exists for ONE
                                    caller — `econ_inheritance_rules_fragment_
                                    differently`, whose 60-year window overlaps the
                                    realm era; a coronation moves a whole house's
                                    fortune out of the merchant pool at once (§5.2,
                                    "crowns drain the merchant pool") and that
                                    swamped the gate's wealth signal and INVERTED
                                    it. Isolating it is the same discipline as
                                    fixing the seed and the world; realm formation
                                    keeps its own instrument. See
                                    `docs/WORLD_REALISM_REVIEW.md` §3.5-§3.6 —
                                    including §3.6's historical verdict (Tilly's
                                    ~500 European polities c.1500 says MORE realms
                                    is right, and that nothing here CONSOLIDATES is
                                    now the binding gap).
                                    **CONSOLIDATION** closes the half of Tilly's
                                    curve the model lacked (it only ever
                                    fragmented): `realm_expansion_pass` annexes an
                                    ADJACENT free province preferring its own
                                    culture, `realm_vassalage_pass` lets a 2.5×
                                    stronger neighbour impose vassalage and, after
                                    80 years, integrate outright (the first writer
                                    `Realm.vassals` has ever had), and
                                    `realm_secession_pass` lets a culturally
                                    foreign province break away from a collapsed
                                    crown — a realm that only grows converges on
                                    one colour as surely as one that only splits
                                    (§5.6). All three are CONTIGUITY-driven over
                                    `prov_neighbors`, which is what makes a realm
                                    read as a country. The RATES are the
                                    deliverable as much as the mechanism: shipped
                                    naive, consolidation ran away (19 founded, only
                                    5 standing, 16 integrations); slowed, it holds
                                    31 founded / 21 standing with all three paths,
                                    both governments and the whole rank ladder
                                    occupied. Partition is now CONTIGUOUS
                                    (`province_hops` seeds heirs far apart and
                                    grows connected shares) instead of round-robin
                                    by index, which produced checkerboard realms;
                                    and a realm founded by a PEOPLE is named for
                                    that people (France is not "the Kingdom of
                                    Paris"). Personal union, inherited claims and
                                    conquering a foreign CAPITAL remain unbuilt.
                                    FRAGMENTATION is real (R5) but only Path A — a
                                    Partible culture's realm divides among eligible
                                    sons at EVERY succession (`partition_realm`,
                                    the same `partible_heirs` distribution
                                    `divide_estate` already uses for a merchant
                                    house), which is what finally makes the shipped
                                    `InheritanceRule` decide whether a people can
                                    hold an empire at all. Path B (contested-
                                    succession civil war) and overseas merchant-
                                    gated holdings (§3.5) remain unbuilt. The
                                    autonomy axis (`Realm.autonomy`, dormant since
                                    R1) got its first real readers — a centralized
                                    crown collects more but loses more to distance,
                                    an autonomous one the reverse. Capital moves are
                                    a real, tested mechanism triggered ONLY by a
                                    capital going abandoned (a defensive case, not
                                    a speculative "chase prosperity" AI). REVERSES
                                    CITY_PROVINCE_WAR_PLAN §6's deferral of a realm
                                    entity above cities. Carries its own caveats (§5)
                                    and §6 list of the work still owed; §7's
                                    order table records exactly what each phase shipped
TWO_APPS_AND_FILE_UPLOAD_PLAN.md  ← ⭐ SLICES 1-2 BUILT + the campaign LIBRARY and
                                    START OVER; slices 3-5 unbuilt. The audit found
                                    `save_world_as` doing `DELETE FROM campaign` after
                                    the backup while `settlements`/`economy` are CAMPAIGN
                                    keys — so no `.worldforge` file the app had ever
                                    written could start a campaign. `CAMPAIGN_KEYS` is
                                    now split into `WORLD_HUMAN_KEYS` (ships in the world
                                    file) and `CAMPAIGN_RUN_KEYS` (does not) — see §10
                                    rule 28. Implementation found a SECOND site of the
                                    same bug the audit missed: `new_campaign` wiped the
                                    table too, so Finalize → New Campaign → Begin
                                    Campaign failed on a world that had never been
                                    saved at all. Recommends NOT splitting into two
                                    binaries (the campaign seeds itself from the whole
                                    world pipeline; FIX_PLAN B1 wants that edge tighter,
                                    not process-separated). §8 of the doc records what
                                    shipped, including what it does NOT fix — old world
                                    files still carry no economy, and nothing can
                                    retro-fit data a file never contained
TECTONICS_AND_ISOLATION_PLAN.md   ← ⭐ AGREED, NOTHING BUILT. Two subjects.
                                    PART A · an ocean is a barrier: the measured
                                    cause of trans-oceanic lanes is
                                    `rescue_tiny_components`, which folds any
                                    <3-hub component into the nearest big one with
                                    NO DISTANCE CAP — so a mid-ocean island is
                                    relabelled part of a far continent and `#6`'s
                                    same-component guard (which exists precisely to
                                    stop "dishonest trans-oceanic arrows") is handed
                                    a lie about what is connected. Cap it in km
                                    (rule 25); beyond the cap a component trades
                                    internally and a city that cannot get what it
                                    needs starves and is abandoned — already a
                                    modelled outcome (`abandon_hub` sets
                                    `died_cause`), not something to invent. Also
                                    unifies the Flows highlight onto the SAME coarse
                                    grid + crossing rule the Dynamic Trade Flow
                                    layer already uses, so the dashed direct-line
                                    fallback (rule 35) becomes rare rather than
                                    routine. PART B · tectonic character, appearance
                                    level but DERIVED FROM the Euler-pole motion
                                    field: power-law plate SIZES (today's jittered
                                    grid makes them all alike; Earth spans three
                                    orders of magnitude), a persisted MOTION layer
                                    (plates are transient today, so the velocity
                                    field that drives boundary classification cannot
                                    be drawn), collision STYLE (`geology.rs` already
                                    computes the setting and nothing uses it to shape
                                    the belt PROFILE — continent-continent should be
                                    broad and MULTI-RIDGE, ocean-continent narrow and
                                    volcanic), and RELICT SUTURES. That last is the
                                    believability item and exists in no form today:
                                    `age` is `fbm_noise`, pure noise uncorrelated
                                    with whether a boundary is active, and is only
                                    ever assigned ON a belt that exists NOW — so
                                    there can be no Ural, no Appalachian, no Scottish
                                    Highland, all of which are former sutures inside
                                    a plate. The plan's recommendation is to GENERATE
                                    A PAST, not run a simulation: bake 2-4 former
                                    sutures with ages, height falling with age. It
                                    says plainly that this fakes the HISTORY rather
                                    than the physics
CONSUMPTION_REBUILD_PLAN.md       ← ⭐ APPROVED IN SCOPE, NOTHING BUILT. The
                                    build order for the review below, after
                                    three decisions on the maximal path: add a
                                    price→production loop (not just tuning), the
                                    population PAYS for what it consumes, and a
                                    mine's output comes from the ore body's
                                    `extent` rather than the city's population.
                                    Eight slices, each with its own gate, its own
                                    push and a gate-that-is-not-the-target; the
                                    three behavioural ones (price loop, ore
                                    ceiling, households) each dosed from zero on
                                    the N1/N6 pattern, re-running `econ_` PER
                                    DOSE STEP. **S1 is the cheap keystone**: the
                                    demand table becomes a BUDGET SHARE rather
                                    than a quantity — one division by
                                    `base_value`, no table rewritten — which
                                    alone moves food 12.4%→46.5% of consumption
                                    spend, and two constant nudges carry it to
                                    61.0% food / 8.8% luxury / gems at 0.05×
                                    bread. Names the part of S1 that is easy to
                                    miss and impossible to see (`need_scale`'s
                                    own `sum_tw_desire` must gain the same
                                    terms), and the two things NOT in scope so
                                    they are not assumed done (N1's ownerless
                                    residual; `tech_factor` pinned at its 0.85
                                    floor)
CONSUMPTION_AND_GOODS_REVIEW.md   ← ⭐ MEASURED ANALYSIS, NOTHING BUILT (one
                                    #[ignore]d diagnostic added). WHY THE
                                    WAREHOUSES ARE FULL AND THE GEMS MOVE IN
                                    BULK. Consumption is `eat = need.min(stock)`
                                    with **no counterparty** — the population is
                                    a sink, not an agent, so **there are no
                                    buyers in this economy** and no market panel
                                    can honestly show any (`supply_accum`'s five
                                    classes are all SELLERS). Measured from the
                                    shipped demand tables: food & drink is
                                    **12.4%** of a city's consumption spend
                                    (history: 60-80%), luxury tier **69.7%**, and
                                    a city spends **13.2× more on gemstones than
                                    on wheat**. Measured in the reference world
                                    (`econ_measure_goods_stock_and_price`): 314
                                    days of grain and 1,499 days of silk held in
                                    YEAR ONE, rising to 29.5 years and 197 years
                                    by year 100, with `price/base` pinned at
                                    0.14-0.52 against a `PRICE_FLOOR_MULT` of
                                    0.15 (one excursion, iron at 0.94). Four causes, each sufficient:
                                    consumption capped at the ration so 100% of
                                    surplus is retained; production reads no
                                    price ANYWHERE (`made = by_inputs.min(
                                    labor_cap)`); **31 of 45 goods ship
                                    `perishable = 0.0`** so spoilage
                                    early-returns and `wh_capacity` — whose only
                                    effect is a multiplier ON the spoil rate — is
                                    provably inert for them; and `need_scale` is
                                    ONE aggregate scalar over all 45 goods, so at
                                    most the AVERAGE is right and each good's
                                    level is an accident of two tables nobody
                                    compared. **The manufactory shortage is the
                                    same bug**: `maybe_found_guild_workshop`
                                    gates on `demand_pressure_at >= 1.08` while
                                    that function is `(price/base).clamp(0.6,
                                    3.0)` whose measured MAXIMUM across every
                                    good and date is 0.94, so the gate is never
                                    once satisfied anywhere — unsatisfiable, not
                                    merely rare; `maybe_found_estate` uses the
                                    same figure as a SCORE not a GATE, which is
                                    the whole asymmetry. Also: `estate_kind` is
                                    guessed by **substring match on the good's
                                    name** rather than read from `distribution`
                                    (the icon bug's root, and a stale-table
                                    failure its own doc comment already records);
                                    the strata are a single clamped 0.4-1.8 tilt
                                    with the comfort tier literally neutral, and
                                    `Pop` is inert, so a Vic3-style pops WINDOW
                                    would imply causation the sim does not have.
                                    Ends with 8 gated proposals in build order
                                    and 6 questions that need a decision before
                                    any of it starts
INSTITUTIONS_BUILD_ORDER.md       ← ⭐ PHASES 0, 1, 2, 3.1, 4.1, 4.2, 4.3, 4.4
                                    AND 5 BUILT AND GATED; 4.5 BUILT BUT DOSED
                                    AT ZERO. The sequenced order for the brainstorm below,
                                    after four maintainer decisions: the player
                                    stays OBSERVATION-ONLY (so "play the bank" is
                                    REJECTED, not deferred) · all three chains
                                    sequenced in one plan · MEASURE the craft
                                    spread before choosing its dose · Murano
                                    scoped as ONE named feature, not a guild
                                    rework. Its governing rule follows from the
                                    first decision and is harder than a player
                                    verb: **every mechanism must produce a legible
                                    STORY, not a decision** — a slice names what it
                                    writes to the chronicle or it is not done.
                                    Phase 0 = three instruments, no production
                                    code: `econ_measure_finance` and `real_world_
                                    craft_spread` (see SCOREBOARD 2026-09-13 for
                                    what they measured — the second now shows 0%
                                    "leader is simply the biggest city", down from
                                    100% before Phase 2); `econ_measure_war_trade`
                                    still isn't built. **Phase 1 (shipped)**:
                                    `resolve_bank_failure` tries WOUND DOWN
                                    (liquidation covers deposits+notes — deposits
                                    alone let almost every failure "succeed"
                                    quietly and drove crashes toward 0/century)
                                    then ABSORBED (a solvent same-component rival
                                    buys the book) before the old COLLAPSE path,
                                    which alone still ignites
                                    `trigger_regional_crash`; a bank may now hold
                                    the Monte (`kind == 1` in every coupon/
                                    deleverage/haircut/default loop, plus a fresh
                                    bank-subscribes-headroom path); `League.purse`
                                    funds a season's convoy escort
                                    (`LEAGUE_ESCORT_COST`/`_LOSS_MULT`) and
                                    chronicles an unaffordable renewal too, not
                                    just a funded one. **Phase 2 (shipped)** — the
                                    Murano feature: `TickHub.tradition` (years of
                                    practice, faster under a guild, decaying when
                                    idle) fully replaces the old population
                                    `size_bonus` in the quality cap; a guildhall
                                    accrues `CraftGuild.secrecy` that subtracts
                                    from `maybe_steal_quality`'s roll; `maybe_
                                    poach_master` (new) lets a rival bribe away a
                                    master, carrying `MASTER_POACH_TRADITION_FRAC`
                                    of the source's tradition to the destination
                                    at a real cost to the source guild's strength
                                    +secrecy; a signature (`brand_name`) is earned
                                    once tradition clears `SIGNATURE_TRADITION_
                                    YEARS` AND quality clears `SIGNATURE_QUALITY_
                                    FLOOR`; a high-throughput, import-dependent
                                    hub (`ENTREPOT_THROUGHPUT_FLOOR`/`_IMPORT_
                                    DEP_FLOOR`) gets a manufacture efficiency
                                    bonus on the input TAKE only, chronicled once
                                    via the new `LAW_ENTREPOT`. **Phase 3.1
                                    (shipped)**: a hub at war raises its own debt
                                    target (`WAR_DEBT_TARGET_MULT`, still bounded
                                    by the pre-existing `DEBT_MAX_RATIO`/
                                    serviceability gate) and the Monte-opening/
                                    civic-default chronicle lines now name the war
                                    and the banks a default hits — closing the
                                    war→debt→bank-holds-it→haircut→fails→crash
                                    chain from four systems that already existed.
                                    **Phase 4.1 (shipped)**: `CONTRABAND_GOODS`
                                    (metalware/iron/timber/pitch/hemp) is checked
                                    live in `dispatch` against `TickHub.war_with`
                                    — lane-scoped to the actual enemy, unlike
                                    `export_ban_until`'s blanket ban — and every
                                    declare-war chronicle line names which of the
                                    goods this world carries are barred. **Phase
                                    4.2 (shipped)**: a lane between two hubs
                                    actually `war_with` each other is diverted
                                    through the SAME neutral-stop `staging_hop`
                                    relay N1/N1c already use (`BLOCKADE_STAGING_
                                    DOSE`, shipped at 0.0 — a true no-op, since
                                    a bare refusal at this shape of dose has
                                    twice collapsed the inheritance gate's world
                                    per N1/N2's own history), chronicled once per
                                    war on the first shipment actually diverted
                                    ("the war between X and Y forces cargo to go
                                    by way of Z"), never once per shipment.
                                    **Phase 4.3 (shipped LIVE, not dosed)**: a
                                    member-to-member League lane pays cheaper
                                    freight (`LEAGUE_FREIGHT_DISCOUNT` 0.85, the
                                    same shape `GUILDHALL_FREIGHT` already uses)
                                    and a reduced tariff on both ends
                                    (`LEAGUE_TARIFF_MULT` 0.5) — the first thing
                                    beyond Phase 1.3's convoy escort that makes
                                    League membership matter, shipped live rather
                                    than dose-walked because a discount on an
                                    already-taxed, narrow lane class (96% of
                                    shipments move ownerless and never reach
                                    either check) carries none of the
                                    concentration risk a routing or exclusion
                                    mechanism does. **Phase 4.4 (shipped)**: THE
                                    KONTOR — a league with a purse that clears
                                    `KONTOR_COST` establishes one shared depot at
                                    the non-member hub it trades with most, once
                                    that tie clears the same `LEAGUE_FLOW_MIN`
                                    threshold formation itself uses; members
                                    trading through that host get the SAME 4.3
                                    privilege as a fellow member would
                                    (`lane_league_privileged`, the one pure
                                    decision `dispatch` reuses across all three
                                    call sites); the host may expel it
                                    (`maybe_expel_kontors`, likelier while at war
                                    with a member or under high unrest),
                                    chronicled on both establishment and
                                    expulsion. **Phase 4.5 built, dosed at zero**:
                                    the diet's target-selection rule
                                    (`choose_boycott_target` — prefers a real war
                                    a member is fighting, the Denmark 1361-70
                                    case, else the seat of the strongest
                                    threatening rank-≥2 realm) is real code,
                                    tested, and wired into `run_league_diet`, but
                                    `LEAGUE_BOYCOTT_MAX` remains 0 — the plan's
                                    own text calls dosing this "last, and expect
                                    trouble" (N2's related cargo-ban mechanism
                                    broke the hard wealth bound twice at low
                                    doses), and walking it above zero needs its
                                    own session with room for the full
                                    multi-seed-inheritance + dense-world gate
                                    sweep per dose step, which this one did not
                                    have left. **Phase 5 (shipped)**: a fifth
                                    `ProvWork` kind
                                    (`WORK_CADASTRE`, crown-funded, state-
                                    infrastructure-tier) permanently multiplies a
                                    surveyed province's tithe collection by
                                    `PROV_CADASTRE_EFFICIENCY_MULT`; an active tax
                                    farm now costs real rural unrest + crown
                                    cohesion every year it stands, which is what
                                    makes the cadastre matter rather than merely
                                    exist. §6 lists what is deliberately out.
                                    **`econ_inheritance_rules_fragment_
                                    differently` is confirmed pre-existing broken,
                                    independent of any of this** — it fails
                                    identically on the commit before this whole
                                    plan's work began; not a regression this work
                                    introduced, and left unchased per §8.15's own
                                    caution against tuning a dose against a stale
                                    or already-distorted measurement.
INSTITUTIONS_BRAINSTORM.md        ← ⭐ BRAINSTORM, NOTHING APPROVED OR BUILT.
                                    Variants for six institutions the maintainer
                                    asked about — banks as a player · war's real
                                    trade impact · craft guilds/the Murano case ·
                                    a bureaucratic apparatus · settlements with
                                    unique production · trade leagues. Its §0 is
                                    the finding that unifies them: **nothing in
                                    this world can forbid anything.** `dispatch`
                                    honours exactly five prohibitions (plague
                                    quarantine · famine food lock · `export_ban_
                                    until` at INFINITY · league boycott at 0 ·
                                    `house_barred`, authored only by colonies) and
                                    **WAR IS NOT ONE OF THEM** — `dispatch` never
                                    reads `self.wars`, so two cities at war trade
                                    at full volume every day; the "blockade" only
                                    scales `export_earn` (real for a city's
                                    prosperity, zero effect on cargo). Every one of
                                    the six is a different answer to "who may
                                    exclude whom from what". Other measured items:
                                    `League.purse` has ONE writer and ZERO readers
                                    (dues accumulate forever, unspent); glassware
                                    and ceramics DO exist (belt goods converted to
                                    recipes, glassware ← bay_salt + timber) and
                                    `maybe_steal_quality` already implements
                                    industrial espionage, so the Murano THEFT half
                                    is built and the SECRECY half is not; and the
                                    quality ceiling is `0.62 + size + structures`,
                                    where the size term SATURATES at 60,000 pop —
                                    and `real_world_craft_spread` then FALSIFIED
                                    this file's own first prediction: the finest
                                    maker is the biggest city **0 times in 14
                                    goods**, because every real city converges on
                                    the SAME ceiling (0.960) and there is no leader
                                    at all, only a tie. `LAW_GUILD_MONOPOLY` (0.97)
                                    is the one thing that lifts anyone above it, so
                                    a guild charter is already the only mechanism
                                    producing a distinctive maker. Three
                                    interlocking chains proposed rather than six
                                    projects: money-and-consequence (banks stop
                                    dying → war paid by borrowing → a bank holds
                                    the bond → a lost war breaks it, the 1345
                                    chain, four systems that each work and are not
                                    connected) · exclusion (contraband → a routed
                                    blockade → league privileges → the staple
                                    right, every one routing through a neutral
                                    rather than refusing, per N1c/N2's measured
                                    lesson) · why-a-place-is-itself (untie quality
                                    from city size → a craft can be kept and lost →
                                    the signature → the entrepôt). §8 carries seven
                                    open questions that gate the work
BANKS_MONEY_AND_CRAFT_PLAN.md     ← ⭐ REVIEW + PLAN, NOTHING BUILT. Banks, coinage
                                    and the craft guilds, measured against the
                                    standing dynamics run rather than against the
                                    code. Headline: all three are modelled well
                                    INTERNALLY and are terminal — `price_level` is
                                    computed by a real quantity-theory loop and read
                                    by NOBODY (`live_price` is `base·(need/stock)^k`,
                                    no money term), `coin_exchange`'s bimetallic
                                    ratio prices only a bank's own bill income (a
                                    merchant crossing currencies pays no spread), and
                                    a craft guild's entire economic power is one
                                    quality ceiling. Money's ONE channel into trade is
                                    `coin_discount`'s ≤10% freight shave above
                                    `RESERVE_TRUST_MIN` 0.55 — and the measured run
                                    puts average coin trust at 43% from year 30 on, so
                                    that channel is CLOSED for half the campaign,
                                    because 12 crashes in 50 years (every `fail_bank`
                                    calls `trigger_regional_crash`) keep hammering
                                    trust down. Two subsystems in a loop neither was
                                    designed against; invisible without the digest.
                                    Also: nobody BORROWS (`bank_maybe_lend` rolls a
                                    die and pushes cash at the RICHEST resident, no
                                    project attached), `BANK_CREDIT_MULT`'s 1.6×
                                    "trades on credit" is an archetype perk connected
                                    to no bank and never repaid, no bank may hold
                                    public debt though CLAUDE.md claimed otherwise
                                    (B-F4, now corrected above), and the world holds
                                    **12 craft guilds total, seeded once at tick 0 and
                                    never founded or dissolved again**, against 23
                                    manufactured goods — Florence alone had 21 arti.
                                    Eight slices, gate each: 0 the missing instrument
                                    (`econ_measure_finance` — there is no `econ_*`
                                    diagnostic for finance at all) · 1 split bank
                                    failure from contagion (cheapest, re-opens the
                                    coin channel) · 2 let a bank hold the Monte ·
                                    3 credit demanded not pushed · 4 make
                                    `price_level` spendable (dosed from zero; touches
                                    `live_price`, may not survive) · 5 make someone
                                    pay the bill · 6 A6/Fugger, loan→arrears→offtake ·
                                    7 crafts as a live population. §5 lists what is
                                    deliberately NOT proposed (wages/labour is
                                    FIX_PLAN Part C, not a finance change); §6 maps
                                    what SYSTEMS_21_PROPOSALS, SOCIAL_ECONOMIC_WEALTH_
                                    PROPOSAL (whose "banking is dormant, lower the
                                    founding bar" reading is now measurably STALE and
                                    backwards) and ESTATES_SHARES A5/A6 already said
CITY_TRADERS_PANEL_PLAN.md        ← ⭐ AGREED, BACKEND GROUNDWORK BUILT AND INERT,
                                    UI NOT BUILT. A third tab beside Market/Flows:
                                    WHO TRADES HERE (carriers by volume/standing/
                                    route length/carriage, with an import-export
                                    filter) over WHO IS ESTABLISHED HERE (offices,
                                    bailos, the council seat, capture) — two lists
                                    because they routinely disagree, a house can
                                    seat a council and carry nothing. Carries the
                                    measured finding that shapes every decision in
                                    it: `econ_measure_carrier_mix` reports **96% of
                                    shipments move on no house's account**, so the
                                    panel will read "local merchants 96%" on nearly
                                    every city — which is the finding, not a defect,
                                    and must never be suppressed to flatter the
                                    house list. Also records why the transit column
                                    is called RE-EXPORT: the sim has no multi-leg
                                    voyage (cargo teleports A→B), so what is
                                    measurable is a trader landing and re-shipping
                                    the same good, and the column must not claim
                                    more than that
ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md
                                  ← ⭐ STAGES A + B BUILT (§9's own plan — dedup/
                                    parallelise the routing Dijkstras, sample the cost
                                    grid per tile, price mountain passes by minimum,
                                    build trade components from ROUTED reachability
                                    instead of Euclidean distance, guarantee every
                                    settlement a route with fall-through); §10 Q1's own
                                    PREREQUISITE for Stage C — make
                                    `econ_inheritance_rules_fragment_differently`
                                    multi-seed before touching a single dose — is now
                                    BUILT too (see §8.15's gate description); STAGE C
                                    (the carriage/economy dose walk itself) NOT STARTED.
                                    The route/connectivity/carriage counterpart to
                                    TRADE_AND_MARKET_REVIEW (which reviews the PRICE mechanism).
                                    Its headline is that the economy's problem is not
                                    missing mechanism: **ELEVEN built, wired, gated
                                    mechanisms ship at a dose of exactly zero**
                                    (N1/N1b/N2 · CAPACITY_BIND_DOSE · DEMAND_/PROD_
                                    ELASTICITY · ORE_CEILING · HOUSEHOLD_MONETIZATION ·
                                    WH_RELEASE · LEAGUE_BOYCOTT_MAX · GUILD_CHARTER_
                                    RANGE), so production ignores price, demand
                                    ignores price, carriage ignores capacity and
                                    storage ignores price — which IS
                                    TRADE_AND_MARKET_REVIEW F1's measured flatness
                                    restated as a cause. Names
                                    `econ_inheritance_rules_fragment_differently`
                                    (single-seed, 60-year) as the one instrument
                                    blocking all eleven doses.
                                    **Five measured bugs, three now FIXED (B1-B3).**
                                    (1) FIXED — mountain passes ARE priced (a ×0.45
                                    saddle discount) but the coarse cell WAS 55.7 km
                                    with elevation POINT-SAMPLED from the block centre,
                                    so a 1-10 km real pass was invisible and route
                                    quality through mountains was luck. `min_land_elev`
                                    (the block's own minimum land elevation, sampled in
                                    the same per-tile pass that also dropped the six
                                    full-grid fine arrays, §8.9-style) now prices both
                                    the base relief term and the saddle discount; the
                                    centre-sampled `elev` is kept for every OTHER reader
                                    (`path_metrics`, `flow.rs`'s land-leg kind). Gated
                                    by `a_narrow_pass_off_centre_is_still_found`.
                                    (2) FIXED — components WERE built from EUCLIDEAN
                                    distance, land and sea never consulted (`max_link =
                                    0.30 * world_w` = 12,022 km at the default grid,
                                    wider than EVERY ocean on Earth), so the world
                                    collapsed to ONE component and every
                                    `component == component` guard downstream was a
                                    no-op. `compute_routed_components`
                                    (query_commands/mod.rs) now builds components from
                                    AT MOST *k* single-source Dijkstra searches (*k* =
                                    number of components) over the real coarse cost
                                    grid, bounded by a new `TRADE_COMPONENT_HORIZON_KM`
                                    (3,000 km) via `path_allowed`'s reach-1 rule. The
                                    old "rescue tiny/lone components... any distance"
                                    fuse is RETIRED, not capped — real reachability
                                    already unions anything that can legally reach a
                                    bigger market, so a hub that cannot is genuinely
                                    isolated and may go poor. Gated by two
                                    `component_tests` fixtures (an ocean wider than the
                                    horizon splits; narrower merges).
                                    (3) FIXED — a non-hub settlement got exactly ONE
                                    candidate link by straight-line distance, dropped
                                    silently on a Dijkstra miss, a reach violation, or a
                                    shared coarse cell. `compute_trade_routes` now tries
                                    a ranked shortlist of its 5 nearest hubs, folded
                                    into the same batched-Dijkstra call, falling through
                                    to the next candidate instead of dropping the town;
                                    a town whose whole shortlist fails is logged, though
                                    surfacing that to the UI is a separate, unbuilt
                                    frontend/bridge change. Gated by
                                    `a_lesser_town_falls_through_to_its_second_hub_
                                    when_the_first_is_unreachable`.
                                    (4) freight — **the doc CORRECTS ITSELF here and
                                    the correction is the finding**; read §2's
                                    correction block, not the claim above it. First cut
                                    said `good_freight` has no mode term so distance is
                                    mispriced (30% of grain's value over 300 km, "as
                                    cheap by sea"). Literally true about the missing
                                    term, WRONG in conclusion: it used the NOMINAL
                                    `days_per_cell` (55 km/day) where `days_per_cell`
                                    is only a REFERENCE speed pinned to cost 2.2
                                    (`cost_to_days = days_per_cell·f/(OPEN_SEA_COST·
                                    100)`), never the speed anything travels. Real
                                    routed speeds: calm coastal sea **242 km/day**,
                                    nav river 60.5, flat land 30.2, hills 14.8 — so
                                    300 km overland costs **54.5%** of grain's value
                                    (not 30%) against 6.8% by coastal sea, an 8:1
                                    ratio that is exactly Masschaele. LAND speeds match
                                    history; **calm coastal sea is 2.4-4.8× too fast**
                                    against a ~50-100 km/day real effective average, so
                                    the lever is the coastal-sea cost rung, NOT
                                    `freight_per_day` and not a new mode term (C1 is
                                    rewritten). What survives: the mode penalty is
                                    PROPORTIONAL, never DIFFERENTIAL — road should be
                                    disqualifying for grain and merely dear for silk
                                    (now C1b). And it REMOVES a charge: a 4,000 km sea
                                    haul at 91% beating 1,000 km overland at 182% is
                                    historically CORRECT (Baltic grain reached
                                    Amsterdam; inland Polish grain did not), so
                                    trans-oceanic trade is §5's component bug alone,
                                    not a freight mispricing. Freight does still reach
                                    only the founding hubs — every colony founded
                                    during a campaign routes by straight line. (5) Caravan/boat
                                    capacity is declared and unwired: `CAPACITY_BIND_
                                    DOSE = 0`, so any shipment takes one slot; 96% of
                                    cargo is ownerless and never reaches the check;
                                    `cap_land` pools boats WITH caravans; and 120:40 is
                                    3:1 against a historical ~10-13:1 (a cog 150-200 t
                                    vs a hundred-camel caravan 15-20 t).
                                    **Perf** (§8 of the doc; P1/P2/P3/P5 now FIXED by
                                    Stage A, P4 improved as a consequence): ZERO rayon
                                    in all seven `query_commands/` files → rayon now
                                    covers the batched Dijkstra and the route-days
                                    matrix build; one Dijkstra per PAIR at four call
                                    sites (~500 whole-grid searches, ~2 GB of allocation
                                    churn per route refresh) → deduped to one per SOURCE
                                    via a shared `coarse_dijkstra_batch`, proven
                                    byte-identical to the old per-pair paths;
                                    `build_coarse_cost` loading 97 MB of fine fields to
                                    read 259k centre cells (390 MB on Large) → sampled
                                    per tile as it loads instead; `charter_owner`
                                    allocating n nested Vecs PER DAY → one flat
                                    `Vec<i32>`. Campaign start's ~1,000 sequential
                                    Dijkstras (`SEASON_SLICES`) are the same batching
                                    win applied at scale, now parallel rather than
                                    sequential. Four-stage plan (A free speed → B cost
                                    grid can see terrain → C carriage sorts cargo by
                                    mode → D siting) — A and B are BUILT (see the
                                    bugs list above for B1-B3's own status); **C1
                                    BUILT** — `COASTAL_SEA_COST` 0.5 → 1.6
                                    (`query_commands/mod.rs`), which the shared
                                    `cost_to_days` conversion resolves to ~75.6 km/day
                                    (was 242, 2.4-4.8× the real ~50-100 km/day
                                    pre-modern effective average), leaving `freight_
                                    per_day` and every land/river cost untouched —
                                    gated by `coastal_sea_speed_matches_the_
                                    historical_effective_average`. **Measured on a
                                    real generated world, same seed before/after**
                                    (`real_world_price_distance_gradient`,
                                    `commands/real_world_diagnostics.rs`, `#[ignore]`d
                                    — built for exactly this gap, since every `tick::
                                    tests`/`econ_` fixture is a synthetic in-memory
                                    `CampaignSim` that never touches `build_coarse_
                                    cost` at all, so neither of the two gates this
                                    dose's own doc names could ever move): grain price/
                                    distance gradient **r = 0.092 → 0.185**, nearly
                                    doubled, both positive — the historically correct
                                    sign, and the metric `WORLD_AND_TRADE_MASTER_PLAN.
                                    md` §4/F2 names as this whole area's headline
                                    failure. See that doc's "UPDATE 2" for the full
                                    pair and its caveats (one seed, one world size, one
                                    20-year run). This NECESSARILY
                                    retires the old static "sea:river:road ≈ 1:4:8"
                                    cost-grid ratio the non-findings list below once
                                    called settled (`coastal_sea_river_and_open_sea_
                                    price_in_a_sane_order` now asserts ordering, not a
                                    fixed ratio) — see §8.15-adjacent doc comment at
                                    `COASTAL_SEA_COST` for why: that ratio is a
                                    citation of Masschaele's FREIGHT-cost figure, and
                                    the cost grid conflates freight cost with travel
                                    SPEED through one shared conversion. **C1b also
                                    WIRED, at zero dose** — `good_freight` (`tick/
                                    production.rs`) gained a `sea: bool` param (every
                                    call site already computed `hubs[a].coastal &&
                                    hubs[b].coastal` nearby for display purposes,
                                    reused not duplicated); a LAND leg's `bulk` term
                                    scales by `1.0 + LAND_BULK_PENALTY * (bulk -
                                    1.0).max(0.0)`, sea untouched, a good at or below
                                    bulk 1.0 untouched on either mode at any dose.
                                    `LAND_BULK_PENALTY = 0.0` ships as a true no-op —
                                    the `CAPACITY_BIND_DOSE`/`ORE_CEILING_DOSE`
                                    pattern — proven by `c1b_land_bulk_penalty_is_a_
                                    noop_at_zero` plus `tick::tests` 216/217 (the one
                                    failure pre-existing, unrelated) and `econ_` 6/6
                                    bit-identical. Dosing it is separate, unstarted
                                    work. **C2 re-assessed, not built** — its "8×
                                    too fast" numeric complaint was pre-C1; post-C1
                                    the same formula gives coastal sea 75.6 km/day
                                    against land's unchanged 30.3 (2.5:1, inside the
                                    doc's own "~4×" citation), so the specific
                                    contradiction is gone — the architectural split
                                    (`days` still serves both cost and time) is
                                    deferred as a standing invariant to re-check, not
                                    built, given its blast radius. **C3 deliberately
                                    skipped** — `WORLD_AND_TRADE_MASTER_PLAN.md` §4
                                    already tried this exact `cap_land` split and hit
                                    an unexplained regression (a 90-year house went
                                    bankrupt split, arithmetic verified correct);
                                    re-attempting with no new information would very
                                    likely re-discover the same failure. C4 remains
                                    last by design. **D1 BUILT** —
                                    `try_found_house_outpost` (`houses.rs`) now caps
                                    distance from the METROPOLIS specifically (not
                                    just the nearest network node, which still
                                    governs scoring), adds a 150 km minimum gap, and
                                    checks mode-legal range via the SAME
                                    `leg_exceeds_range` ordinary trade already uses
                                    (a no-op in every test fixture, live only in a
                                    real campaign). `econ_diagnose_outpost_founding`
                                    now founds 1 outpost (was 2) on the reference
                                    fixture — not a regression: the reduction is the
                                    home-distance check doing exactly its job (it can
                                    only ADD a rejection relative to the old check),
                                    and founding did not silently stall (the gate's
                                    own named failure mode). `econ_fidelity_scorecard`/
                                    `_large_world` diffed line-for-line against a
                                    stashed pre-D1 run of the same commit — every
                                    printed figure bit-identical, confirmed rather
                                    than assumed (see `docs/SCOREBOARD.md`
                                    2026-09-11d). **D3 BUILT** — `hub.river`
                                    now reads three real river LANDMARKS off
                                    `sim::rivers::River` (a mouth's
                                    `mouth_kind != 0`, a tributary's own
                                    confluence point, a navigable trunk's
                                    head-of-navigation) within 25 km, replacing
                                    a blanket ~400 km "near any navigable
                                    point" test. Measured on a real 1800×900
                                    world: riverine share 10.9% → 2.0%, a real
                                    ~5.5× drop (the gate's own "must fall
                                    substantially" bar), diagnostic run at
                                    production-scale km/cell since the usual
                                    300-wide test world is too coarse for a
                                    25 km radius to mean anything (rule 21/25).
                                    **D2's RIVER half now BUILT** (the
                                    COMPONENT half remains deferred — its own
                                    entry records why): `compute_colonizable_
                                    sites` stamps each candidate site's real
                                    `river` flag via the SAME landmark test
                                    D3 uses (`sim::rivers::river_landmarks`,
                                    factored out for this reuse), wired
                                    through every founding path
                                    (`create_estate`, `create_market_colony`,
                                    the mid-campaign pool recompute) in place
                                    of the hardcoded `river: false` every
                                    campaign-founded hub previously shipped
                                    with. `econ_` bit-identical (the change
                                    only writes a field no economic pass
                                    conditions on differently yet).
                                    **C4 BUILT** — `N1_LOCAL_HAUL_BIND_DAYS`
                                    dosed to 90 days, completing Stage C.
                                    Routes an over-threshold ownerless leg
                                    through N1c's own `staging_hop` relay
                                    rather than refusing it outright (N1c's own
                                    history already proved a bare refusal on
                                    an ownerless-only bind collapses the
                                    inheritance gate), made a per-sim field so
                                    abstract fixtures opt out exactly like
                                    `ship_leg_max_km`/`caravan_leg_max_km`. A
                                    30-day first trial staged everything with
                                    zero refusals and still collapsed
                                    `dense_world` trade volume to ~10% — proof
                                    routing alone isn't automatically safe once
                                    it fires on most of a world's trade. 90
                                    days measures healthy on `tests::
                                    dense_world` over 40 years and leaves the
                                    multi-seed inheritance gate + full `econ_`
                                    bit-identical (both opt out via the field).
                                    Plus 7 open questions
                                    (§10, all answered — A→B→C order, isolated markets
                                    may starve, a ~3,000 km trade horizon, the
                                    inheritance gate goes multi-seed before Stage C,
                                    now built — see §8.15) and a NON-FINDINGS list
                                    recording that the tick is NOT the perf problem
                                    and worldgen river siting is one of the
                                    better-modelled things in the tree
IN_APP_VERIFICATION_CHECKLIST.md  ← Manual in-app verification checklist
PORTING_REFERENCE.md              ← Porting reference
```

**`docs/proposals/` — A MENU, NOT COMMITMENTS**

Twenty-five documents (feature catalogs, trade/cartography specs, Victoria-2 UI
direction, settlement and population analyses, finance/heraldry variants, roadmap
batches, and the eight-part HOUSE design series — province view · tiers/kin/goals ·
body politic · succession crisis · power struggle · faction naming ·
`HOUSE_MASTER_PLAN.md`, which critiques and sequences the rest ·
`HOUSE_INHERITANCE_AND_TERRITORY.md`, which amends it) live in `docs/proposals/`.

> If you read only one of the house documents, read `HOUSE_MASTER_PLAN.md` and then
> `HOUSE_INHERITANCE_AND_TERRITORY.md` (which amends it). Part 0 of the master plan
> records a **blocking measured finding**: a house currently lives ~12 years against a
> historical 30–90, so the politics layer has no substrate until turnover is fixed. The
> amendment adds the open risk that the too-rich and too-short-lived anomalies may be
> **one bug (overextension) or in tension** — and if they are in tension, fixing lifespan
> raises peak wealth and the phase boundary is wrong. They were being read by fresh sessions as a backlog to work through,
and they are not one: they are far more good ideas than anyone can build.

> **Do not start work from `docs/proposals/`.** Check `FIX_PLAN.md` for what is actually
> prioritised and `SCOREBOARD.md` for what is actually measured. Reach into `proposals/`
> only when a specific design question needs the original rationale.

Historical HTML/SVG mockups are archived under `docs/mockups/_archive/`; a stray
reference image lives in `docs/reference/`. The repo root holds only `README.md`
and `CLAUDE.md`.

> **Older `*_PLAN.md` docs are gone.** Twelve superseded planning documents were deleted —
> they described work that has long since shipped and were being read as commitments.
> **The systems themselves are still in the code**; the code and this file are now the
> record. For the original rationale of one:
> `git log --diff-filter=D --name-only -- docs/` finds the deleting commit, then
> `git show <sha>^:docs/<file>`.

---

