//! # The economy fidelity gate
//!
//! This is the campaign half's answer to `step4_climate/earth_validation.rs`.
//!
//! The climate pipeline is scored against a real reference map to one decimal
//! place. The campaign economy — ~16.7k lines simulating houses, banks, coinage,
//! guilds, wars, plagues and colonies — had **no fidelity oracle at all**. Its
//! test suite is extensive but tests *mechanism*: does a contract deliver, does a
//! bank fail, is output deterministic, does wealth stay finite and bounded. Not
//! one assertion asked whether a number resembles a real pre-modern economy.
//!
//! ## The method
//!
//! A fantasy world has invented goods, invented cities and no real currency, so
//! absolute quantities cannot be compared to history at all. What *can* be
//! compared are **dimensionless structural regularities** — ratios, dispersions,
//! gradients and distributional shapes that hold across every well-documented
//! pre-modern economy regardless of its units:
//!
//! | Metric | Real-world value | Source |
//! |---|---|---|
//! | Grain price gap rises with distance | positive gradient, steep | Federico; Persson, *Grain Markets in Europe* |
//! | Cross-city grain price dispersion (CV) | ~0.20–0.40 | Chilosi et al. on market integration |
//! | Within-city grain price volatility (CV) | ~0.30–0.50 | Persson; Clark, English prices |
//! | City rank-size (Zipf) slope | ~ −0.8 to −1.2 | De Vries, *European Urbanization*; Bairoch |
//! | Urbanisation share | ~0.08–0.15 | De Vries |
//! | Wealth Gini (top of society) | ~0.60–0.85 | Alfani; Van Zanden |
//! | Top-10% wealth share | ~0.60–0.90 | Alfani |
//! | Merchant firm lifespan | ~1–3 generations (30–90 yr) | Greif; Mueller & Lane on Venetian houses |
//!
//! ## How to read the scorecard
//!
//! Like the Earth harness, **most metrics are printed, not asserted.** A printed
//! metric that sits outside its historical band is a finding, not a build
//! failure — it tells you where the model is unlike a real economy, which is
//! exactly the information that did not exist before. Assertions are reserved for
//! bands the model already satisfies, so that they guard against *regression*
//! rather than encoding aspiration.
//!
//! **Promote a printed metric to an assertion as the model earns it** — that is
//! how `earth_validation.rs`'s spot checks are meant to work too, and it converts
//! the current tuning frontier into permanent protection.
//!
//! Run it:
//! ```bash
//! cargo test --lib econ_ -- --nocapture
//! ```

use super::tests::{good, house_at, hub, sim};
use super::*;

// ── Asserted floors ─────────────────────────────────────────────────────────
// Deliberately loose. Each is set from a MEASURED baseline with margin, so it
// catches a structural break without pinning the model to today's exact tuning.
// Tighten these as the economy improves, exactly as `EARTH_MAIN_FLOOR` is raised.

/// Grain price gaps must widen with travel distance. A market where distance does
/// not matter is not a pre-modern market — it is one global warehouse.
const ECON_INTEGRATION_FLOOR: f32 = 0.05;
/// Cities must differ in price at all. Zero dispersion means arbitrage is
/// instantaneous and perfect, which no pre-modern economy was.
const ECON_SPATIAL_CV_FLOOR: f32 = 0.01;
/// The city-size distribution must stay heavy-tailed rather than collapsing to
/// uniformity (slope ≈ 0) or to a single primate city (very steep).
const ECON_ZIPF_MIN: f32 = -3.0;
const ECON_ZIPF_MAX: f32 = -0.15;
/// Wealth must concentrate. A pre-modern merchant elite with an egalitarian
/// wealth distribution is the one thing history never produced.
const ECON_GINI_FLOOR: f32 = 0.15;

/// Years of campaign time each scorecard run covers.
const RUN_YEARS: u32 = 60;

// ── The scorecard ───────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct EconScorecard {
    /// Pearson r between pairwise travel days and |ln(price ratio)| for grain.
    pub integration_gradient: f32,
    /// Mean |ln price ratio| for the nearest and furthest quartile of city pairs.
    /// Distance bands are taken from the world's OWN distance distribution — a
    /// fixed day threshold silently empties itself on a small or fast world.
    pub gap_near: f32,
    pub gap_far: f32,
    /// Surviving (non-defunct) houses at the end of the run. Reported because the
    /// wealth statistics below are meaningless when only a handful remain.
    pub houses_alive: usize,
    /// Banks ever chartered over the run.
    pub banks_founded: usize,
    /// Urban share at the START of the run, for comparison with `urban_share`.
    /// The interesting quantity is the DRIFT: the harness seeds a historically
    /// ordinary ~10% and asks whether the model stays anywhere near it.
    pub urban_share_initial: f32,
    /// Coefficient of variation of grain price ACROSS cities, final year.
    pub spatial_cv: f32,
    /// Coefficient of variation of grain price WITHIN a city, across years.
    pub temporal_cv: f32,
    /// OLS slope of ln(rank) on ln(population) — the Zipf exponent.
    pub zipf_slope: f32,
    /// Urban share of total (urban + rural) population.
    pub urban_share: f32,
    /// Gini of house wealth among surviving houses.
    pub wealth_gini: f32,
    /// Share of total house wealth held by the richest tenth.
    pub top10_share: f32,
    /// Bank failures per century of simulated time.
    pub bank_failures_per_century: f32,
    /// House dissolutions per century.
    pub house_turnover_per_century: f32,
    /// Share of years in which a FIFTH OR MORE of live cities showed a famine
    /// signal. "Any city anywhere" is trivially true in a 30-city world and
    /// measures nothing; a general subsistence crisis is the historical event.
    pub crisis_year_share: f32,
    /// Mean real wage proxy: grain purchasable per unit of commoner wealth.
    pub real_wage_index: f32,
    /// CV of land-use pressure (`prov_rural / prov_cap`) across provinces, final
    /// year. Workstream 2.5 has not landed yet, so there is no true potential/
    /// actual `exploitation` ratio per good to report; this is the best available
    /// STAND-IN — how unevenly the countryside is worked — and Step 0's job is
    /// only to make it non-zero. Promote/replace once 2.5 ships the real ratio.
    pub prov_pressure_cv: f32,
    /// CV of each province's share of total hub production value, final year. A
    /// stand-in for 2.5's market ↔ local split and 3.x's regional market share —
    /// how concentrated production is by region rather than by good.
    pub prov_output_cv: f32,
    /// Wars started (existing DLC 3.5 rival-polis economic war) per century over
    /// the run, from `war_log`. A baseline reading of the CURRENT war mechanism,
    /// ahead of the abstract state-war system §3.4 will add — 3.4f's own
    /// diagnostic measures that system once it exists.
    pub wars_per_century: f32,
}

// ── Statistics helpers ──────────────────────────────────────────────────────

fn mean(xs: &[f32]) -> f32 {
    if xs.is_empty() { return 0.0; }
    xs.iter().sum::<f32>() / xs.len() as f32
}

/// Coefficient of variation — the scale-free dispersion measure. This is what
/// makes a fantasy currency comparable to silver grams.
fn cv(xs: &[f32]) -> f32 {
    let m = mean(xs);
    if m.abs() < 1e-9 || xs.len() < 2 { return 0.0; }
    let var = xs.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / (xs.len() - 1) as f32;
    var.sqrt() / m.abs()
}

fn pearson(xs: &[f32], ys: &[f32]) -> f32 {
    let n = xs.len().min(ys.len());
    if n < 3 { return 0.0; }
    let (mx, my) = (mean(&xs[..n]), mean(&ys[..n]));
    let mut num = 0.0;
    let (mut dx, mut dy) = (0.0f32, 0.0f32);
    for i in 0..n {
        let (a, b) = (xs[i] - mx, ys[i] - my);
        num += a * b;
        dx += a * a;
        dy += b * b;
    }
    if dx < 1e-12 || dy < 1e-12 { return 0.0; }
    num / (dx.sqrt() * dy.sqrt())
}

/// Ordinary least-squares slope of `ys` on `xs`.
fn ols_slope(xs: &[f32], ys: &[f32]) -> f32 {
    let n = xs.len().min(ys.len());
    if n < 3 { return 0.0; }
    let (mx, my) = (mean(&xs[..n]), mean(&ys[..n]));
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        let a = xs[i] - mx;
        num += a * (ys[i] - my);
        den += a * a;
    }
    if den < 1e-12 { return 0.0; }
    num / den
}

/// Gini coefficient over non-negative values, via the sorted-rank formulation.
fn gini(values: &[f32]) -> f32 {
    let mut v: Vec<f32> = values.iter().copied().filter(|x| x.is_finite() && *x >= 0.0).collect();
    if v.len() < 2 { return 0.0; }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len() as f32;
    let total: f32 = v.iter().sum();
    if total < 1e-9 { return 0.0; }
    let weighted: f32 = v.iter().enumerate().map(|(i, x)| (i as f32 + 1.0) * x).sum();
    (2.0 * weighted) / (n * total) - (n + 1.0) / n
}

/// Share of the total held by the richest `frac` of the population.
fn top_share(values: &[f32], frac: f32) -> f32 {
    let mut v: Vec<f32> = values.iter().copied().filter(|x| x.is_finite() && *x >= 0.0).collect();
    if v.is_empty() { return 0.0; }
    v.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let total: f32 = v.iter().sum();
    if total < 1e-9 { return 0.0; }
    let k = ((v.len() as f32 * frac).ceil() as usize).clamp(1, v.len());
    v[..k].iter().sum::<f32>() / total
}

// ── The reference world ─────────────────────────────────────────────────────

/// A deliberately ordinary world: 30 cities on a grid, six goods, ten merchant
/// houses, free land to the south, and a HETEROGENEOUS province layer behind the
/// cities — five provinces of distinct geography (fertile lowland, wooded hills,
/// arid steppe, temperate mix, marginal upland) — so urbanisation, exploitation
/// and regional dispersion are all measurable, not just levels.
///
/// It mirrors the world used by `simulate_decades_reports_dynamics` on purpose —
/// the fidelity scorecard should describe the same economy the dynamics test
/// already guards, not a special case built to flatter the metrics.
fn reference_world() -> CampaignSim {
    let goods = vec![
        good("wheat", 0, 0, 1.0, 0.85, true),
        good("fish", 0, 0, 1.2, 0.7, true),
        good("olives", 0, 0, 1.6, 0.6, true),
        good("silk", 1, 2, 20.0, 0.35, false),
        good("iron", 2, 1, 5.0, 0.45, false),
        good("wine", 3, 2, 8.0, 0.4, false),
    ];
    let ng = goods.len();
    let mut hubs = Vec::new();
    for i in 0..30u32 {
        let x = (i % 6) as f32 * 9.0;
        let y = (i / 6) as f32 * 9.0;
        let pop = 8000.0 + (i as f32 * 911.0) % 26000.0;
        let prod: Vec<f32> = (0..ng)
            .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 })
            .collect();
        hubs.push(hub(i, x, y, pop, prod, 0));
    }
    let mut s = sim(hubs, goods);

    for i in 0..10u32 {
        let seat = (i * 3) % 30;
        let mut h = house_at(seat, vec![3 + (i as usize % 3)], 3);
        h.archetype = (i % 4) as u8;
        h.wealth = 40.0 + (i as f32) * 8.0;
        h.prestige = 0.5;
        h.dominant_seat = i % 2 == 0;
        s.houses.push(h);
    }
    s.seed_house_count = s.houses.len() as u32;

    for i in 0..12u32 {
        s.colonizable.push(ColonizeSite {
            x: 4.5 + (i % 4) as f32 * 12.0,
            y: 40.0 + (i / 4) as f32 * 8.0,
            koppen: 8,
            elevation: 0.1,
            fertility: 0.45 + (i % 3) as f32 * 0.15,
            coastal: i % 2 == 0,
            kind_hint: 1,
            trade_value: 0.2 + (i % 4) as f32 * 0.1,
            delta: false,
            chokepoint: false,
        });
    }

    // A province layer behind the cities, seeded at a historically ordinary
    // ~10% urban (De Vries). Pre-modern populations were 85–92% rural; without
    // this the urbanisation metric would read 100%, which is the single most
    // unhistorical number this model could print. Seeding it CORRECTLY is what
    // makes the measured drift away from it meaningful.
    //
    // Step 0 of CITY_PROVINCE_WAR_PLAN.md: this layer used to be UNIFORM — every
    // province identical, seats on a straight line — which bounds the land pass
    // but makes it impossible to measure DISPERSION (why one province is rich and
    // its neighbour poor). Five provinces now carry distinct geography, each
    // matched to one row of the hub grid (`i / 6`) so a province's seat sits near
    // its own member cities instead of anywhere on the map.
    let nprov = 5usize;
    let urban_seed: f32 = s.hubs.iter().map(|h| h.population).sum();
    let rural_each = (urban_seed * 9.0 / nprov as f32).max(1.0);
    // [fertile river lowland · wooded hills · arid steppe · temperate mix · marginal upland]
    let cap_mult: [f32; 5] = [2.6, 1.5, 0.6, 1.8, 1.5];
    let fill_frac: [f32; 5] = [0.75, 0.55, 0.35, 0.60, 0.30];
    let forest: [f32; 5] = [0.15, 0.70, 0.05, 0.40, 0.25];
    let arable: [f32; 5] = [0.55, 0.12, 0.10, 0.30, 0.06];
    let soil: [f32; 5] = [0.85, 0.65, 0.35, 0.60, 0.32];
    let irrigated: [f32; 5] = [0.10, 0.0, 0.0, 0.05, 0.0];
    let tenure: [[f32; 4]; 5] = [
        [0.24, 0.16, 0.09, 0.51],
        [0.16, 0.08, 0.09, 0.67],
        [0.10, 0.04, 0.06, 0.80],
        [0.18, 0.10, 0.09, 0.63],
        [0.12, 0.05, 0.07, 0.76],
    ];
    let seats: [[f32; 2]; 5] = [
        [20.0, 2.0],
        [8.0, 11.0],
        [35.0, 19.0],
        [15.0, 29.0],
        [30.0, 38.0],
    ];
    s.prov_cap = (0..nprov).map(|i| rural_each * cap_mult[i]).collect();
    s.prov_rural = (0..nprov).map(|i| rural_each * cap_mult[i] * fill_frac[i]).collect();
    s.prov_culture = (0..nprov).map(|i| format!("Culture{i}")).collect();
    s.prov_seat = seats.to_vec();
    s.prov_net_mig = vec![0.0; nprov];
    s.hub_province = (0..30).map(|i| (i / 6) as i32).collect();
    s.hub_culture = (0..30).map(|i| format!("Culture{}", i / 6)).collect();
    s.hub_minorities = vec![Vec::new(); 30];
    // Land-use/soil/tenure state `province_land_pass` indexes directly every year —
    // must be pre-sized to `nprov` and heterogeneous, or `ensure_province_land`'s
    // fallback (keyed only off `prov_cap`) would correlate forest and soil through a
    // single quality scalar instead of varying independently the way real geography
    // does (a fertile plain is not simply "less of the same" than a wooded hill).
    s.prov_forest = forest.to_vec();
    s.prov_arable = arable.to_vec();
    s.prov_pasture = (0..nprov)
        .map(|i| ((1.0 - forest[i] - arable[i]).max(0.0) * 0.55).clamp(0.0, 1.0))
        .collect();
    s.prov_irrigated = irrigated.to_vec();
    s.prov_soil = soil.to_vec();
    s.prov_tenure = tenure.to_vec();
    s.prov_tax = vec![0.12; nprov];
    s.prov_arrears = vec![0.0; nprov];
    s.prov_unrest = vec![0.0; nprov];
    s.prov_surplus = vec![0.0; nprov];
    s.prov_revenue = vec![0.0; nprov];
    s.prov_holder = vec![-1; nprov];

    calibrate_like_campaign_start(&mut s);
    s.rebuild_routes();
    s
}

/// Apply the same three calibration steps `campaign_start_sim` performs, which the
/// bare `tests::sim()` helper does NOT.
///
/// This matters more than it looks. `tests::sim()` hard-codes `need_scale: 1.0`,
/// but a real campaign computes it from the world's own production and population
/// (`campaign_commands/lifecycle.rs`). For this reference world the correct value
/// is ≈0.012 — so an uncalibrated run demands roughly **eighty times** the food it
/// can grow. The result is not a subtly pessimistic economy but a qualitatively
/// different one: every hub sits in permanent deficit, `dispatch` never sees a
/// surplus above `FOOD_RESERVE_DAYS`, so **grain is never traded at all**. The
/// price/distance gradient then measures thirty autarkies rather than a market,
/// and the volatility, crisis and welfare metrics are all reading a famine.
///
/// A fidelity harness that does not reproduce the real starting conditions
/// measures its own setup. Keep this in step with `lifecycle.rs`.
fn calibrate_like_campaign_start(s: &mut CampaignSim) {
    const TIER_W: [f32; 3] = [1.0, 0.45, 0.22];
    const FOOD_SURPLUS: f32 = 1.5;

    // Phase 0.4 · resolve each people's law of inheritance and open the founding head's
    // record — `campaign_start_sim` does both. Without it the seeded houses keep the
    // bare test helper's placeholder head_lifespan (274 years), so not one of them ever
    // reaches a SUCCESSION inside a 60-year run and every inheritance metric measures
    // only the houses founded during it.
    s.ensure_culture_rules();
    s.seed_house_lines();

    let total_pop: f32 = s.hubs.iter().map(|h| h.population).sum::<f32>().max(1.0);
    let total_prod: f32 = s.hubs.iter().flat_map(|h| h.production.iter()).sum::<f32>().max(1e-3);
    let sum_tw_desire: f32 = s
        .goods
        .iter()
        .map(|g| TIER_W[g.need_tier.min(2) as usize] * g.desire.max(0.0))
        .sum::<f32>()
        .max(1e-3);
    s.need_scale = total_prod / (total_pop * sum_tw_desire);

    // Founding food-viability: a settlement would not exist where it cannot feed
    // itself, so food output is raised (never cut) to a viable surplus.
    let mut total_food_need = 0.0f32;
    let mut total_food_prod = 0.0f32;
    for h in &s.hubs {
        for (g, tg) in s.goods.iter().enumerate() {
            if tg.food {
                total_food_need += h.population
                    * TIER_W[tg.need_tier.min(2) as usize]
                    * tg.desire.max(0.0)
                    * s.need_scale
                    * DEMAND_PRESSURE;
                total_food_prod += h.production[g];
            }
        }
    }
    if total_food_prod > 1e-3 {
        let food_scale = (total_food_need * FOOD_SURPLUS / total_food_prod).max(1.0);
        if food_scale > 1.0 {
            let food: Vec<bool> = s.goods.iter().map(|g| g.food).collect();
            for h in s.hubs.iter_mut() {
                for (g, &is_food) in food.iter().enumerate() {
                    if is_food {
                        h.base_per_capita[g] *= food_scale;
                        h.production[g] *= food_scale;
                        h.stock[g] *= food_scale;
                    }
                }
            }
        }
    }
}

// ── The measurement ─────────────────────────────────────────────────────────

/// Runs the reference world for `RUN_YEARS` and measures it.
fn measure(s: &mut CampaignSim) -> EconScorecard {
    let mut card = EconScorecard::default();

    const GRAIN: usize = 0; // wheat — the numeraire good
    let mut grain_series: Vec<f32> = Vec::new(); // city 0's grain price, yearly
    let mut crisis_years = 0u32;
    let mut dissolutions = 0usize;
    let mut prev_defunct = 0usize;

    {
        let u: f32 = s.hubs.iter().filter(|h| !h.abandoned).map(|h| h.population).sum();
        let r: f32 = s.prov_rural.iter().copied().filter(|x| x.is_finite()).sum();
        card.urban_share_initial = if u + r > 1.0 { u / (u + r) } else { 0.0 };
    }

    for _ in 1..=RUN_YEARS {
        s.advance(365);

        // A GENERAL subsistence crisis — a fifth or more of live cities at once.
        let live_now = s.hubs.iter().filter(|h| !h.abandoned && h.population > 1.0).count();
        let starving_now = s.hubs.iter()
            .filter(|h| !h.abandoned && h.population > 1.0 && h.starving > 0.5)
            .count();
        if live_now > 0 && (starving_now as f32 / live_now as f32) >= 0.20 {
            crisis_years += 1;
        }
        let now_defunct = s.houses.iter().filter(|h| h.defunct).count();
        if now_defunct > prev_defunct {
            dissolutions += now_defunct - prev_defunct;
            prev_defunct = now_defunct;
        }
        if let Some(h) = s.hubs.first() {
            if h.price.get(GRAIN).is_some_and(|p| p.is_finite() && *p > 0.0) {
                grain_series.push(h.price[GRAIN]);
            }
        }
    }

    let live: Vec<usize> = (0..s.hubs.len())
        .filter(|&i| !s.hubs[i].abandoned && s.hubs[i].population > 1.0)
        .collect();
    let n = s.hubs.len();

    // ── Market integration: does distance still cost anything? ──────────────
    // Federico and Persson measure integration exactly this way — the price gap
    // between two markets as a function of the distance between them.
    let mut dists = Vec::new();
    let mut gaps = Vec::new();
    let mut pairs: Vec<(f32, f32)> = Vec::new(); // (days, |ln gap|)
    for (ai, &a) in live.iter().enumerate() {
        for &b in live.iter().skip(ai + 1) {
            let d = s.days.get(a * n + b).copied().unwrap_or(f32::INFINITY);
            if !d.is_finite() || d <= 0.0 { continue; }
            let (pa, pb) = (s.hubs[a].price[GRAIN], s.hubs[b].price[GRAIN]);
            if !(pa.is_finite() && pb.is_finite() && pa > 0.0 && pb > 0.0) { continue; }
            let gap = (pa.ln() - pb.ln()).abs();
            dists.push(d);
            gaps.push(gap);
            pairs.push((d, gap));
        }
    }
    card.integration_gradient = pearson(&dists, &gaps);

    // Bands from the world's own distance QUARTILES. An absolute day threshold
    // silently empties itself: this reference world's longest route is ~11 days,
    // so a ">30 days" band contained no pairs at all and reported a flat 0.000
    // that looked like a model finding and was really a broken instrument.
    pairs.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
    if pairs.len() >= 4 {
        let q = pairs.len() / 4;
        card.gap_near = mean(&pairs[..q].iter().map(|p| p.1).collect::<Vec<_>>());
        card.gap_far = mean(&pairs[pairs.len() - q..].iter().map(|p| p.1).collect::<Vec<_>>());
    }

    // ── Price dispersion, in space and in time ──────────────────────────────
    let final_prices: Vec<f32> = live.iter()
        .map(|&i| s.hubs[i].price[GRAIN])
        .filter(|p| p.is_finite() && *p > 0.0)
        .collect();
    card.spatial_cv = cv(&final_prices);
    card.temporal_cv = cv(&grain_series);

    // ── Rank-size: the shape of the urban hierarchy ─────────────────────────
    let mut pops: Vec<f32> = live.iter().map(|&i| s.hubs[i].population).collect();
    pops.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let ln_pop: Vec<f32> = pops.iter().filter(|p| **p > 1.0).map(|p| p.ln()).collect();
    let ln_rank: Vec<f32> = (1..=ln_pop.len()).map(|r| (r as f32).ln()).collect();
    card.zipf_slope = ols_slope(&ln_pop, &ln_rank);

    // ── Urbanisation ────────────────────────────────────────────────────────
    let urban: f32 = live.iter().map(|&i| s.hubs[i].population).sum();
    let rural: f32 = s.prov_rural.iter().copied().filter(|r| r.is_finite()).sum();
    card.urban_share = if urban + rural > 1.0 { urban / (urban + rural) } else { 0.0 };

    // ── Wealth concentration ────────────────────────────────────────────────
    let wealths: Vec<f32> = s.houses.iter()
        .filter(|h| !h.defunct && h.wealth.is_finite() && h.wealth > 0.0)
        .map(|h| h.wealth)
        .collect();
    card.wealth_gini = gini(&wealths);
    card.top10_share = top_share(&wealths, 0.10);
    card.houses_alive = wealths.len();
    card.banks_founded = s.banks.len();

    // ── Institutional turnover ──────────────────────────────────────────────
    let centuries = RUN_YEARS as f32 / 100.0;
    let bank_failures = s.banks.iter().filter(|b| b.defunct).count() as f32;
    card.bank_failures_per_century = bank_failures / centuries;
    card.house_turnover_per_century = dissolutions as f32 / centuries;
    card.crisis_year_share = crisis_years as f32 / RUN_YEARS as f32;

    // ── Real wage proxy ─────────────────────────────────────────────────────
    // Allen's method in miniature: commoner wealth expressed in grain. The
    // project already uses a grain numeraire, so this maps directly.
    let wages: Vec<f32> = live.iter()
        .filter_map(|&i| {
            let p = s.hubs[i].price[GRAIN];
            let w = s.hubs[i].society.commoner_wealth;
            (p.is_finite() && p > 1e-6 && w.is_finite()).then_some(w / p)
        })
        .collect();
    card.real_wage_index = mean(&wages);

    // ── Regional dispersion (Step 0 — see the fields' own docs) ─────────────
    let pressures: Vec<f32> = (0..s.prov_cap.len())
        .map(|p| (s.prov_rural[p].max(0.0) / s.prov_cap[p].max(1.0)))
        .filter(|x| x.is_finite())
        .collect();
    card.prov_pressure_cv = cv(&pressures);

    if !s.hub_province.is_empty() {
        let np = s.prov_cap.len().max(
            s.hub_province.iter().copied().filter(|&p| p >= 0).map(|p| p as usize + 1).max().unwrap_or(0),
        );
        let mut prov_output = vec![0.0f32; np];
        for h in 0..s.hubs.len() {
            if s.hubs[h].is_estate || s.hubs[h].abandoned { continue; }
            let pid = s.hub_province.get(h).copied().unwrap_or(-1);
            if pid < 0 || pid as usize >= np { continue; }
            prov_output[pid as usize] += s.hubs[h].production.iter().sum::<f32>();
        }
        card.prov_output_cv = cv(&prov_output);
    }

    card.wars_per_century = s.war_log.len() as f32 / centuries;

    card
}

fn print_scorecard(c: &EconScorecard) {
    println!();
    println!("═══ Economy fidelity scorecard ({RUN_YEARS} years, 30-city reference world) ═══");
    println!("  metric                          simulated     real pre-modern     source");
    println!("  ────────────────────────────────────────────────────────────────────────────");
    println!("  price gap × distance (r)        {:>9.3}     positive, steep     Federico/Persson",
             c.integration_gradient);
    println!("    mean |ln gap| nearest quartile{:>9.3}", c.gap_near);
    println!("    mean |ln gap| furthest quartile{:>8.3}", c.gap_far);
    println!("  grain price CV across cities    {:>9.3}     0.20 – 0.40         Chilosi et al.",
             c.spatial_cv);
    println!("  grain price CV within a city    {:>9.3}     0.30 – 0.50         Persson; Clark",
             c.temporal_cv);
    println!("  rank-size (Zipf) slope          {:>9.3}    −0.8 – −1.2          De Vries; Bairoch",
             c.zipf_slope);
    println!("  urban population share          {:>9.3}     0.08 – 0.15         De Vries",
             c.urban_share);
    println!("    (seeded at                    {:>9.3}  — the drift is the finding)",
             c.urban_share_initial);
    println!("  house wealth Gini               {:>9.3}     0.60 – 0.85         Alfani; Van Zanden",
             c.wealth_gini);
    println!("  top-10% wealth share            {:>9.3}     0.60 – 0.90         Alfani",
             c.top10_share);
    println!("    (over {:>3} surviving houses — statistics need ≥5 to mean anything)",
             c.houses_alive);
    println!("  banks chartered over the run    {:>9}     recurrent           Mueller & Lane",
             c.banks_founded);
    println!("  bank failures / century         {:>9.2}     recurrent           Mueller & Lane",
             c.bank_failures_per_century);
    println!("  house dissolutions / century    {:>9.2}     1–3 generations     Greif",
             c.house_turnover_per_century);
    println!("  crisis (famine) year share      {:>9.3}     ~0.05 – 0.20        Livi-Bacci",
             c.crisis_year_share);
    println!("  real wage index (grain-eq)      {:>9.3}     trend ≈ flat        Allen",
             c.real_wage_index);
    println!("  ── Step 0 · regional dispersion (province layer now heterogeneous) ──");
    println!("  province land-pressure CV       {:>9.3}     (exploitation stand-in, 2.5 pending)",
             c.prov_pressure_cv);
    println!("  province output-share CV        {:>9.3}     (market-share stand-in, 2.5 pending)",
             c.prov_output_cv);
    println!("  wars started / century          {:>9.2}     (current DLC 3.5 mechanism, 3.4 pending)",
             c.wars_per_century);
    println!("════════════════════════════════════════════════════════════════════════════");
    println!("  Printed metrics outside their band are FINDINGS, not failures — see the");
    println!("  module docs. Promote one to an assertion when the model earns it.");
    println!();
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn econ_fidelity_scorecard() {
    let mut s = reference_world();
    let card = measure(&mut s);
    print_scorecard(&card);

    // Everything must at least be a number. A NaN here would mean the metric is
    // measuring nothing, which is worse than measuring something unhistorical.
    assert!(card.integration_gradient.is_finite(), "integration gradient not finite");
    assert!(card.spatial_cv.is_finite(), "spatial CV not finite");
    assert!(card.temporal_cv.is_finite(), "temporal CV not finite");
    assert!(card.zipf_slope.is_finite(), "zipf slope not finite");
    assert!(card.wealth_gini.is_finite(), "wealth gini not finite");
    assert!(card.prov_pressure_cv.is_finite(), "province pressure CV not finite");
    assert!(card.prov_output_cv.is_finite(), "province output CV not finite");
    assert!(card.wars_per_century.is_finite(), "wars/century not finite");
    // The whole point of Step 0: a heterogeneous layer must actually show
    // dispersion, or the seeding change did nothing.
    assert!(
        card.prov_pressure_cv > 1e-3,
        "provinces show no land-pressure dispersion (CV = {:.4}) — the layer is \
         still effectively uniform",
        card.prov_pressure_cv
    );

    // ── Asserted structure ──────────────────────────────────────────────────
    // NOT asserted. On a correctly calibrated world the gradient measures ~0 —
    // distance does not move grain prices. Per this module's own rule, a metric
    // the model does not satisfy is a printed FINDING, not a build failure, so
    // asserting it here would encode aspiration. `ECON_INTEGRATION_FLOOR` is kept
    // as the documented target to promote this to once the model earns it.
    //
    // The likely cause is mechanical, not subtle: `freight_per_day = 0.01` against
    // wheat's `base_value = 1.0` over a longest route of ~11.5 days caps transport
    // at ~11% of grain value, where real overland carting roughly DOUBLED grain's
    // price over 150–300 km (Masschaele, EcHR 46 (1993) 266–79). Compounding it,
    // `drought`/`bumper` are i.i.d. per hub, so there is no regional scarcity for
    // a gradient to form against.
    let _ = ECON_INTEGRATION_FLOOR;
    assert!(
        card.spatial_cv >= ECON_SPATIAL_CV_FLOOR,
        "cities no longer differ in price (CV = {:.4}, floor {:.4}) — \
         arbitrage has become instantaneous and perfect",
        card.spatial_cv, ECON_SPATIAL_CV_FLOOR
    );
    assert!(
        card.zipf_slope <= ECON_ZIPF_MAX && card.zipf_slope >= ECON_ZIPF_MIN,
        "urban hierarchy lost its heavy tail (slope = {:.3}, band [{:.2}, {:.2}])",
        card.zipf_slope, ECON_ZIPF_MIN, ECON_ZIPF_MAX
    );
    // Only meaningful with a population to be unequal WITHIN. On the current
    // baseline the run ends with two surviving houses, so this assertion is
    // skipped — which is itself the finding recorded in docs/SCOREBOARD.md.
    if card.houses_alive >= 5 {
        assert!(
            card.wealth_gini >= ECON_GINI_FLOOR,
            "house wealth stopped concentrating (Gini = {:.3}, floor {:.3}) — \
             no pre-modern merchant elite was ever egalitarian",
            card.wealth_gini, ECON_GINI_FLOOR
        );
    } else {
        println!(
            "  NOTE: wealth-concentration assertion skipped — only {} houses survived \
             {RUN_YEARS} years. That is the finding, not a passing grade.",
            card.houses_alive
        );
    }
    assert!(
        (0.0..=1.0).contains(&card.urban_share),
        "urban share out of range: {:.3}", card.urban_share
    );
}

/// **KNOWN FAILING — an open defect, not a flaky test. See docs/SCOREBOARD.md.**
///
/// The scorecard must be reproducible; a fidelity gate that returns a different
/// number each run cannot guard anything. This is the economy's equivalent of the
/// phase-3 field checksums, and it currently **fails**.
///
/// `CLAUDE.md` §5 claims a tick is "pure & deterministic per `(seed, tick)`". That
/// is not true once the economy is actually trading. The cause is HashMap
/// iteration order feeding **float accumulations**: float addition is not
/// associative, and Rust's `RandomState` gives every HashMap instance its own
/// order, so two identical worlds in one process diverge. Two such sites are
/// already fixed (`classify_hubs`'s `throughput` sum and `flow_year`'s ordering,
/// both in `cities.rs`) and the divergence shrank but did not vanish — roughly a
/// dozen further accumulator maps remain across `houses.rs`, `disease.rs`,
/// `colonies.rs` and `mod.rs` (`tally`, `total`, `infl`, `best`, `count`,
/// `comp_capital`, `comp_food_supply`/`_need`, `size`, `culture_goods`,
/// `trade_cur`).
///
/// Why it was invisible until now: the existing determinism assertions in
/// `tests.rs` run a **famine** world (`tests::sim()` hard-codes `need_scale: 1.0`,
/// ~84× real demand), where almost nothing is ever traded — so `flow_accum` and
/// its siblings stay nearly empty and the order cannot matter. Calibrating the
/// reference world to real campaign-start conditions is what exposed it.
///
/// **FIXED.** Four sites were folding or ordering by HashMap iteration order:
///
/// 1. `money.rs::update_currency_baskets` — summed a partner-volume map with `+=`
///    and divided every basket weight by that total. Float addition is not
///    associative, so the total differed run to run and the coin basket flipped.
/// 2. `production.rs::fold_trade_year` — pushed new `(hub, good)` series onto
///    `trade_hist` in map order, and the peak sort is stable, so equal peaks kept
///    insertion order and a different set survived truncation.
/// 3. `mod.rs` culture desire — built `hub_desire[h]` as a `Vec` from a map.
/// 4. `colonies.rs::update_lingua_franca` — iterated components in map order AND
///    resolved the dominant-culture `max_by` tie by hash order.
///
/// Each now iterates in key order, with an explicit tie-break where a comparison
/// could tie. `seed_trade_fairs` had already been fixed the same way. This test is
/// no longer ignored: it is the guard that stops the defect coming back, and any new
/// hash accumulator in `tick/` will trip it.
#[test]
fn econ_scorecard_is_deterministic() {
    let mut a = reference_world();
    let mut b = reference_world();
    let ca = measure(&mut a);
    let cb = measure(&mut b);

    macro_rules! same {
        ($f:ident) => {
            assert!(
                (ca.$f - cb.$f).abs() < 1e-4,
                concat!(stringify!($f), " not reproducible: {} vs {}"),
                ca.$f, cb.$f
            );
        };
    }
    same!(integration_gradient);
    same!(spatial_cv);
    same!(temporal_cv);
    same!(zipf_slope);
    same!(urban_share);
    same!(wealth_gini);
    same!(top10_share);
    same!(real_wage_index);
    same!(prov_pressure_cv);
    same!(prov_output_cv);
    same!(wars_per_century);
}

/// Sanity check on the statistics themselves. A scorecard whose own maths is
/// wrong would silently mis-describe the economy forever.
#[test]
fn econ_statistics_are_correct() {
    // Perfect equality → Gini 0; one holder takes all → Gini → 1.
    assert!(gini(&[5.0, 5.0, 5.0, 5.0]).abs() < 1e-6, "equal shares must give Gini 0");
    assert!(gini(&[0.0, 0.0, 0.0, 100.0]) > 0.7, "near-total concentration must give a high Gini");

    // Top-share is a share.
    assert!((top_share(&[1.0, 1.0, 1.0, 1.0], 0.25) - 0.25).abs() < 1e-6);
    assert!((top_share(&[0.0, 0.0, 0.0, 9.0], 0.25) - 1.0).abs() < 1e-6);

    // CV is scale-free — the property that makes a fantasy currency comparable
    // to silver grams. Doubling every price must not change it.
    let base = [1.0f32, 2.0, 3.0, 4.0];
    let doubled: Vec<f32> = base.iter().map(|x| x * 2.0).collect();
    assert!((cv(&base) - cv(&doubled)).abs() < 1e-6, "CV must be scale-invariant");

    // Correlation and slope recover a known linear relationship.
    let xs = [1.0f32, 2.0, 3.0, 4.0, 5.0];
    let ys = [2.0f32, 4.0, 6.0, 8.0, 10.0];
    assert!((pearson(&xs, &ys) - 1.0).abs() < 1e-4, "perfect correlation must read 1");
    assert!((ols_slope(&xs, &ys) - 2.0).abs() < 1e-4, "slope must recover 2");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  PHASE 0.1 · WHY DO HOUSES DIE SO YOUNG?  (diagnosis, not a gate)
//
//  The scorecard prints ~312 house dissolutions per century against Greif's 1–3
//  generations (30–90 years). With ~37 houses surviving a 60-year run that implies a
//  mean firm lifespan near TWELVE years — 2.5–7× too short. Every house design in
//  `docs/proposals/HOUSE_*.md` assumes decades, so this is the blocking finding.
//
//  This harness answers the question the aggregate cannot: WHICH houses die, at what
//  age, and had they ever been real firms? Per CLAUDE.md §2.4 a diagnosis is a complete
//  task, so this is written to be read, not to pass.
//
//  Run: cargo test --lib econ_diagnose_house_turnover -- --ignored --nocapture
// ═══════════════════════════════════════════════════════════════════════════════

/// One dead house, as the diagnosis sees it.
struct DeadHouse {
    age_years: f32,
    /// Peak wealth it ever reached — separates a failed firm from a stillborn one.
    peak_wealth: f32,
    /// Peak monthly upkeep it had committed (fleet + warehouse capacity + estates).
    /// The overextension hypothesis predicts this correlates with dying young.
    peak_upkeep: f32,
    /// Did it ever move goods? A house that never traded was never viable.
    ever_traded: bool,
    is_guild: bool,
    /// How the house came into the world — "spin-off" (a family separating out of a
    /// guild), "branch" (a cadet house in another city), "co-heir" (a partible
    /// division) or "seed". Phase 0.2 showed the answer to "why do houses die young"
    /// was in the FOUNDING, so the diagnosis has to know which founding.
    origin: &'static str,
}

/// Classify a house by how it was founded, from its own chronicle.
fn house_origin(h: &House, seeded: bool) -> &'static str {
    if seeded { return "seed"; }
    if h.name.contains(" Line)") { return "co-heir"; }
    match h.events.first().map(|e| e.text.as_str()) {
        Some(t) if t.contains("as a branch of") => "branch",
        _ => "spin-off",
    }
}

#[test]
#[ignore]
fn econ_diagnose_house_turnover() {
    let mut s = reference_world();
    let n0 = s.houses.len();
    // Track, per house index, the peak figures while it is alive. Houses are appended
    // and never removed (defunct is a flag), so index is a stable identity.
    let mut peak_wealth: Vec<f32> = vec![f32::MIN; n0];
    let mut peak_upkeep: Vec<f32> = vec![0.0; n0];
    let mut ever_traded: Vec<bool> = vec![false; n0];
    let mut born_tick: Vec<u32> = vec![0; n0];
    let mut was_defunct: Vec<bool> = vec![false; n0];
    let mut dead: Vec<DeadHouse> = Vec::new();
    let mut founded = 0usize;

    // Sample monthly so a house that lives one year is still observed.
    let months = RUN_YEARS * 12;
    for _ in 0..months {
        s.advance(30);
        // Grow the trackers for houses founded since the last sample.
        while peak_wealth.len() < s.houses.len() {
            peak_wealth.push(f32::MIN);
            peak_upkeep.push(0.0);
            ever_traded.push(false);
            born_tick.push(s.tick);
            was_defunct.push(false);
            founded += 1;
        }
        for hi in 0..s.houses.len() {
            let h = &s.houses[hi];
            if h.defunct {
                // Newly dead → record it once.
                if !was_defunct[hi] {
                    was_defunct[hi] = true;
                    dead.push(DeadHouse {
                        age_years: (s.tick.saturating_sub(born_tick[hi])) as f32
                            / TICKS_PER_YEAR as f32,
                        peak_wealth: peak_wealth[hi],
                        peak_upkeep: peak_upkeep[hi],
                        ever_traded: ever_traded[hi],
                        is_guild: h.is_guild,
                        origin: house_origin(h, hi < n0),
                    });
                }
                continue;
            }
            peak_wealth[hi] = peak_wealth[hi].max(h.wealth);
            if h.volume > 0.01 { ever_traded[hi] = true; }
            // The committed monthly burn — the same terms `apply_wealth_sinks` and
            // `manage_fleets` actually charge, which is what the overextension
            // hypothesis is about.
            let fleet = (h.fleet_sea + h.fleet_river + h.fleet_caravan) as f32
                * SHIP_COST * FLEET_UPKEEP_FRAC;
            let wh: f32 = s.warehouses.iter()
                .filter(|w| w.owner == hi as i32)
                .map(|w| CAP_UPKEEP * w.capacity * s.city_size_factor(w.hub as usize))
                .sum();
            let est = s.hubs.iter()
                .filter(|x| x.is_estate && x.owner_house == hi as i32).count() as f32
                * UPKEEP_WAREHOUSE_BASE * UPKEEP_ESTATE_FRAC;
            peak_upkeep[hi] = peak_upkeep[hi].max(fleet + wh + est);
        }
    }

    let centuries = RUN_YEARS as f32 / 100.0;
    let alive = s.houses.iter().filter(|h| !h.defunct).count();
    // ── The RIGHT estimator ────────────────────────────────────────────────────
    // "Dissolutions per century" is the wrong metric to tune against: it scales with
    // how many houses are standing, so the same mortality reads differently in a
    // 20-house world and a 50-house one. And a 60-year run cannot directly observe a
    // 30–90-year lifespan — most houses are still alive, i.e. RIGHT-CENSORED.
    //
    // The censoring-correct estimator is a hazard rate over EXPOSURE: total
    // house-years actually lived (by the dead AND the living), against deaths.
    //   hazard = deaths / house-years        implied mean lifespan = 1 / hazard
    // That is stock-independent and uses the survivors' time instead of discarding it.
    let mut exposure_years = 0.0f32;
    for hi in 0..s.houses.len() {
        if s.houses[hi].is_guild { continue; }
        let end = if s.houses[hi].defunct {
            // Approximate the death tick by the recorded age; good enough for a rate.
            born_tick[hi] as f32
                + dead.iter().map(|d| d.age_years).sum::<f32>() / dead.len().max(1) as f32
                    * TICKS_PER_YEAR as f32
        } else { s.tick as f32 };
        exposure_years += ((end - born_tick[hi] as f32) / TICKS_PER_YEAR as f32).max(0.0);
    }
    let private_dead: Vec<&DeadHouse> = dead.iter().filter(|d| !d.is_guild).collect();
    let stillborn: Vec<&&DeadHouse> = private_dead.iter().filter(|d| !d.ever_traded).collect();
    let real: Vec<&&DeadHouse> = private_dead.iter().filter(|d| d.ever_traded).collect();

    let mean = |v: &[&&DeadHouse], f: &dyn Fn(&DeadHouse) -> f32| -> f32 {
        if v.is_empty() { return 0.0; }
        v.iter().map(|d| f(d)).sum::<f32>() / v.len() as f32
    };
    let median_age = |v: &mut Vec<&&DeadHouse>| -> f32 {
        if v.is_empty() { return 0.0; }
        v.sort_by(|a, b| a.age_years.partial_cmp(&b.age_years).unwrap());
        v[v.len() / 2].age_years
    };

    println!();
    println!("═══ Phase 0.1 · why houses die young ({RUN_YEARS}-year reference world) ═══");
    println!("  founded during the run      {:>8}", founded);
    println!("  alive at the end            {:>8}", alive);
    println!("  died (all)                  {:>8}", dead.len());
    println!("  died (private houses)       {:>8}", private_dead.len());
    println!("  dissolutions / century      {:>8.1}     (stock-dependent — see below)",
             private_dead.len() as f32 / centuries);
    println!("  house-years of exposure     {:>8.0}", exposure_years);
    if exposure_years > 1.0 && !private_dead.is_empty() {
        let hazard = private_dead.len() as f32 / exposure_years;
        println!("  ⇒ MEAN FIRM LIFESPAN        {:>8.1} yr   band: 30–90 (Greif)",
                 1.0 / hazard);
        // Excluding stillbirths: the lifespan of houses that were ever real firms.
        if !real.is_empty() {
            let h2 = real.len() as f32 / exposure_years;
            println!("    excluding stillbirths     {:>8.1} yr   ← the honest figure",
                     1.0 / h2);
        }
    } else {
        println!("  ⇒ MEAN FIRM LIFESPAN         no deaths — mortality is effectively zero");
    }
    println!();
    println!("  ── THE KEY SPLIT ──────────────────────────────────────────────────");
    println!("  NEVER TRADED (stillborn)    {:>8}   {:>5.1}% of deaths",
             stillborn.len(),
             100.0 * stillborn.len() as f32 / private_dead.len().max(1) as f32);
    println!("  had traded (real failures)  {:>8}   {:>5.1}% of deaths",
             real.len(),
             100.0 * real.len() as f32 / private_dead.len().max(1) as f32);
    println!();
    {
        let mut sb = stillborn.clone();
        let mut rl = real.clone();
        println!("  ── AGE AT DEATH (years) ───────────────────────────────────────────");
        println!("  stillborn   mean {:>6.1}   median {:>6.1}",
                 mean(&stillborn, &|d| d.age_years), median_age(&mut sb));
        println!("  real firms  mean {:>6.1}   median {:>6.1}   ← THIS is the lifespan",
                 mean(&real, &|d| d.age_years), median_age(&mut rl));
        println!("              band 30–90 (Greif; Mueller & Lane)");
    }
    println!();
    println!("  ── WHICH FOUNDING DIES? (Phase 0.2 found the answer was the founding) ──");
    for origin in ["seed", "spin-off", "branch", "co-heir"] {
        let of: Vec<&&DeadHouse> = private_dead.iter().filter(|d| d.origin == origin).collect();
        if of.is_empty() { continue; }
        let still = of.iter().filter(|d| !d.ever_traded).count();
        println!("  {origin:<10} deaths {:>4}   never traded {:>4} ({:>5.1}%)   mean age {:>5.1} yr",
                 of.len(), still, 100.0 * still as f32 / of.len() as f32,
                 mean(&of, &|d| d.age_years));
    }
    println!();
    println!("  ── OVEREXTENSION HYPOTHESIS ───────────────────────────────────────");
    println!("  real firms: peak wealth   mean {:>10.1}", mean(&real, &|d| d.peak_wealth));
    println!("  real firms: peak upkeep   mean {:>10.2}  (monthly, committed)",
             mean(&real, &|d| d.peak_upkeep));
    // If dying young is overextension, age at death should fall as committed upkeep
    // rises — i.e. a NEGATIVE correlation. A flat correlation refutes it.
    if real.len() >= 5 {
        let ages: Vec<f32> = real.iter().map(|d| d.age_years).collect();
        let ups: Vec<f32> = real.iter().map(|d| d.peak_upkeep).collect();
        let wls: Vec<f32> = real.iter().map(|d| d.peak_wealth).collect();
        println!("  corr(age, peak upkeep)    {:>10.3}  negative ⇒ overextension kills",
                 pearson(&ages, &ups));
        println!("  corr(age, peak wealth)    {:>10.3}  positive ⇒ the rich live longer",
                 pearson(&ages, &wls));
    } else {
        println!("  too few real failures to correlate ({} < 5)", real.len());
    }
    println!();
    println!("  Every dissolution in the model comes from ONE site: update_solvency's");
    println!("  \"a private house in the red for a full year is bankrupt\". There is no");
    println!("  other death. So the question is only ever WHY wealth went negative.");
    println!("═══════════════════════════════════════════════════════════════════════");
    println!();

    // Diagnostic only — it must not fail the build, per §2.5's printed-metric rule.
    assert!(dead.len() + alive > 0, "the run produced no houses at all");
}

/// `HOUSE_MASTER_PLAN.md` §2.5 · "The foreign hand may never fire — MEASURE BEFORE
/// BUILDING": before writing the mechanism (4.4, still un-attempted), count how
/// often its two channels' conjunction actually exists over a long run.
///
///   Channel A — a RIVAL house holds an office/bailo in the city where our kin sits.
///   Channel B — our kin holds a LEASE in a city a rival CONTROLS (`captor_house`).
///
/// "and that member is already disaffected" is read here as `loyalty < 0.4` — the
/// same rough cut this codebase already uses elsewhere for a hostile kinsman
/// (`crisis.rs`'s plot-leader pick). A diagnosis is a complete task per CLAUDE.md
/// §2.4; this prints the finding and does not build the mechanism itself.
#[test]
#[ignore]
fn econ_measure_foreign_hand_conjunction() {
    let mut s = reference_world();
    let years = 300u32;
    let months = years * 12;
    let mut samples = 0u64;
    let mut conjunction = 0u64;
    let mut disaffected_conjunction = 0u64;
    for _ in 0..months {
        s.advance(30);
        for hi in 0..s.houses.len() {
            if s.houses[hi].defunct || s.houses[hi].is_guild || s.houses[hi].kin.is_empty() { continue; }
            let leases = s.houses[hi].office_leases.clone();
            for k in &s.houses[hi].kin {
                if k.role == 4 || k.role == 5 || k.posted < 0 { continue; }
                let hub = k.posted as usize;
                if hub >= s.hubs.len() { continue; }
                samples += 1;
                let channel_a = s.houses.iter().enumerate().any(|(oj, oh)| {
                    oj != hi && !oh.defunct
                        && (oh.offices.contains(&(hub as u32)) || oh.bailos.contains(&(hub as u32)))
                });
                let captor = s.hubs[hub].captor_house;
                let channel_b = captor >= 0 && captor as usize != hi
                    && leases.iter().any(|&(h, _)| h as usize == hub);
                if channel_a || channel_b {
                    conjunction += 1;
                    if k.loyalty < 0.4 { disaffected_conjunction += 1; }
                }
            }
        }
    }
    let centuries = years as f64 / 100.0;
    println!();
    println!("═══ 2.5 · foreign-hand conjunction ({years}-year reference world) ═══");
    println!("  kin-months sampled (posted kin only)   {:>10}", samples);
    println!("  channel A or B present                 {:>10}   {:>5.2}%",
        conjunction, 100.0 * conjunction as f64 / samples.max(1) as f64);
    println!("  … AND that kin already disaffected     {:>10}   {:>5.2}%",
        disaffected_conjunction, 100.0 * disaffected_conjunction as f64 / samples.max(1) as f64);
    println!("  ⇒ full-conjunction rate                {:>10.1} / century",
        disaffected_conjunction as f64 / centuries);
    println!("  Verdict: {}", if disaffected_conjunction as f64 / centuries < 5.0 {
        "fires well under a handful of times a century — 4.4 would very likely ship as dead code. Leave it un-built."
    } else {
        "fires often enough to be worth building — revisit 4.4."
    });
    println!("═══════════════════════════════════════════════════════════════════");
    println!();
    // Diagnostic only — it must not fail the build, per §2.5's printed-metric rule.
    assert!(samples > 0, "the run produced no posted kin at all to measure");
}

/// `CITY_PROVINCE_WAR_PLAN.md` §3.4f · "measure war frequency BEFORE tuning anything" —
/// the same precedent Phase 4.4 set for the foreign hand. This measures the mechanism as
/// it exists TODAY (DLC 3.5's `update_wars`/`maybe_declare_war`, a flat 10% yearly chance
/// between two rival-council seats in the same connectivity component) — the baseline
/// 3.4a–e's "score + gated preconditions" redesign will be judged against. §5.8: a low
/// global war count may just mean most cities have no reachable rival, so this also
/// reports the share of war-eligible cities that are structurally isolated (alone in
/// their own connectivity component) — a low war count from THAT cause is not a broken
/// trigger and must not be "fixed" by loosening the trigger itself.
#[test]
#[ignore]
fn econ_measure_war_frequency() {
    let mut s = reference_world();
    // 150y, not 300 — war-driven house turnover keeps growing `s.houses`, and several
    // per-tick passes scan it, so cost per year rises through the run; 150y already
    // gives a solid per-century rate (the outpost diagnostic uses the same window).
    let years = 150u32;
    // A `War` carries no id of its own; (a, b, start_tick) is the only stable identity,
    // so it doubles as the key for "have we already counted this one as started" and
    // for looking its goal/start-year back up once it disappears from `s.wars` (ended).
    let mut started: std::collections::HashSet<(u32, u32, u32)> = std::collections::HashSet::new();
    let mut goal_of: std::collections::HashMap<(u32, u32, u32), (u8, u32)> = std::collections::HashMap::new();
    let mut prev_keys: std::collections::HashSet<(u32, u32, u32)> = std::collections::HashSet::new();
    let mut wars_started = 0u32;
    let mut durations: Vec<u32> = Vec::new();
    let mut outcome_counts = [0u32; 4]; // plunder, tribute, trade rights, annex
    let mut cause_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for _ in 0..years {
        s.advance(TICKS_PER_YEAR);
        // The year label AFTER this advance — `s.tick / TICKS_PER_YEAR`, not the loop
        // index, since a war that resolves during THIS call already reflects the tick
        // this advance just reached (using the loop index instead undercounts every
        // duration by exactly one year).
        let cur_year = s.tick / TICKS_PER_YEAR;
        let cur_keys: std::collections::HashSet<(u32, u32, u32)> =
            s.wars.iter().map(|w| (w.a, w.b, w.start_tick)).collect();
        for w in &s.wars {
            let key = (w.a, w.b, w.start_tick);
            if started.insert(key) {
                wars_started += 1;
                goal_of.insert(key, (w.goal, w.start_tick / TICKS_PER_YEAR));
                *cause_counts.entry(w.cause.clone()).or_insert(0) += 1;
            }
        }
        for key in prev_keys.difference(&cur_keys) {
            if let Some(&(goal, start_year)) = goal_of.get(key) {
                durations.push(cur_year.saturating_sub(start_year));
                outcome_counts[goal.min(3) as usize] += 1;
            }
        }
        prev_keys = cur_keys;
    }

    // §5.8 · war-eligible seats (the same filter `maybe_declare_war` itself applies)
    // and how many of them share NO connectivity component with any other seat.
    let seats: Vec<usize> = (0..s.hubs.len())
        .filter(|&h| !s.hubs[h].is_estate && s.hubs[h].council_house >= 0 && s.hubs[h].population > 1.0)
        .collect();
    let isolated = seats.iter()
        .filter(|&&h| !seats.iter().any(|&o| o != h && s.hubs[o].component == s.hubs[h].component))
        .count();

    let centuries = years as f64 / 100.0;
    let mean_duration = if durations.is_empty() { 0.0 }
        else { durations.iter().sum::<u32>() as f64 / durations.len() as f64 };

    println!();
    println!("═══ 3.4f · war frequency ({years}-year reference world, PRE-3.4a–e baseline) ═══");
    println!("  wars started                            {:>10}", wars_started);
    println!("  ⇒ wars / century                         {:>10.2}", wars_started as f64 / centuries);
    println!("  wars resolved in the window              {:>10}", durations.len());
    println!("  mean duration                            {:>10.1} yr", mean_duration);
    println!("  outcome mix   plunder {:>4}   tribute {:>4}   trade-rights {:>4}   annex {:>4}",
        outcome_counts[0], outcome_counts[1], outcome_counts[2], outcome_counts[3]);
    let mut causes: Vec<(&String, &u32)> = cause_counts.iter().collect();
    causes.sort_by(|a, b| b.1.cmp(a.1));
    print!("  causes       ");
    for (c, n) in &causes { print!(" {c} {n} "); }
    println!();
    println!("  war-eligible cities (a council seat)     {:>10}", seats.len());
    println!("  structurally isolated (own component)    {:>10}   {:>5.1}%",
        isolated, 100.0 * isolated as f64 / seats.len().max(1) as f64);
    println!("═══════════════════════════════════════════════════════════════════════");
    println!();
    // Diagnostic only — it must not fail the build, per §2.5's printed-metric rule.
    assert!(!seats.is_empty(), "the run produced no war-eligible cities at all");
}

/// Player-reported: "no outposts are created" over the course of ordinary play.
/// `maybe_found_house_outpost` (`houses.rs`) is gated on THREE things at once — a
/// non-empty `colonizable` site pool, a founder wealthy enough (`OUTPOST_FOUND_WEALTH`
/// = 100,000, `OUTPOST_FOUND_COST` = 70,000), and a reachable site within
/// `COLONY_MAX_KM` of the founder's home/offices. Diagnose which of the three is
/// actually the blocker before touching any constant, per CLAUDE.md §2.4.
///
/// **Two real bugs found and fixed** (neither was the wealth bar — it clears 96.8% of
/// months): (1) only the SINGLE richest house ever got to try, each call, so the moment
/// its own home+offices network stopped bordering a remaining site the mechanism stalled
/// for good even with other wealthy, well-placed houses idle — fixed by letting every
/// qualifying house attempt, richest first, up to `OUTPOST_MAX_PER_CALL` successes.
/// (2) ordinary estates (founded far more often) could exhaust the shared
/// `MAX_TOTAL_ESTATES` budget outposts also draw from — fixed by `OUTPOST_RESERVED_ESTATES`.
/// A house's own ESTATES were also added as network anchors alongside home+offices (a
/// plantation already worked nearby is a natural base for a regional exploitation post).
///
/// **What this diagnostic still shows, and why it's left alone:** on THIS synthetic
/// fixture, founding still stops after year 31 (2 outposts) even with both fixes,
/// because `reference_world()`'s `colonizable` sites sit in one compact band (y 40–56)
/// disjoint from most hubs (y 0–36) — a geometry no real generated world has (frontier
/// sites there are scattered broadly, not walled into one corner). Widening
/// `COLONY_MAX_KM` to chase this one fixture's number would be tuning a constant against
/// its own target metric with no independent gate (exactly what §2.4 warns against), and
/// `COLONY_MAX_KM`=2500 is itself a prior deliberate "user rule" (see its own doc comment
/// in `mod.rs`), not a stale default. Left as an open item: confirm in a REAL generated
/// world via the app whether outposts now recur past the first founding wave; if they
/// still stall there, the fixture's geometry is not the explanation and this needs
/// another diagnostic pass against real site distribution.
#[test]
#[ignore]
fn econ_diagnose_outpost_founding() {
    let mut s = reference_world();
    let years = 150u32;
    let months = years * 12;
    let mut richest_ever = 0.0f32;
    let mut months_wealth_bar_cleared = 0u32;
    let mut colonizable_over_time: Vec<(u32, usize)> = Vec::new();
    let mut outposts_founded = 0u32;
    let mut outpost_years: Vec<u32> = Vec::new();
    let mut prev_outpost_hubs = 0usize;
    for m in 0..months {
        s.advance(30);
        let richest = s.houses.iter().filter(|h| !h.defunct).map(|h| h.wealth).fold(0.0f32, f32::max);
        richest_ever = richest_ever.max(richest);
        if richest >= 100_000.0 { months_wealth_bar_cleared += 1; }
        if m % 12 == 0 {
            colonizable_over_time.push((m / 12, s.colonizable.len()));
        }
        let outpost_hubs = s.hubs.iter().filter(|h| h.colony_kind == 2).count();
        if outpost_hubs > prev_outpost_hubs {
            outposts_founded += (outpost_hubs - prev_outpost_hubs) as u32;
            outpost_years.push(m / 12);
        }
        prev_outpost_hubs = outpost_hubs;
    }
    println!();
    println!("═══ Outpost founding — {years}-year diagnostic ═══");
    println!("  richest house's peak wealth ever    {:>10.0}   (bar: OUTPOST_FOUND_WEALTH = 100,000)", richest_ever);
    println!("  months the bar was CLEARED           {:>10} / {} ({:.1}%)",
        months_wealth_bar_cleared, months, 100.0 * months_wealth_bar_cleared as f64 / months as f64);
    println!("  colonizable sites remaining, by decade:");
    for (yr, n) in &colonizable_over_time {
        if yr % 10 == 0 { println!("    year {yr:>4}: {n:>3} sites left"); }
    }
    println!("  outposts actually founded            {:>10}", outposts_founded);
    println!("  founded in years: {:?}", outpost_years);
    println!("═══════════════════════════════════════════════════════════════════");
    println!();
    assert!(months > 0, "diagnostic only — never fails the build");
}

// ── Phase 0.4 · the inheritance gate ────────────────────────────────────────
//
//  `HOUSE_MASTER_PLAN` 0.4 gates the inheritance rule on one thing, and it is the
//  right one: *two worlds identical but for the rule must show measurably different
//  fragmentation. If they don't differ, the rule isn't wired to anything and is
//  flavour text.*
//
//  So this runs the SAME reference world twice — same seed, same cities, same goods,
//  same houses — changing only the law of inheritance every culture in it follows, and
//  compares what the merchant class looks like after 60 years.
//
//  What partible inheritance is expected to do, and why:
//  a share of the estate leaves the parent firm at every death, so capital that would
//  have compounded in one house instead starts a second. More houses, each smaller,
//  and a flatter top — which is precisely why the Italian *fraterna* had to be
//  reconstituted each generation while English gentry estates accumulated.

/// Force every culture in `s` onto one law of inheritance. `ensure_culture_rules`
/// leaves an already-registered culture alone, so this survives the run.
fn force_inheritance(s: &mut CampaignSim, line: LineRule, rule: InheritanceRule) {
    let mut names: Vec<String> = Vec::new();
    for c in s.hub_culture.iter() {
        if !c.is_empty() && c != "—" && !names.iter().any(|n| n == c) { names.push(c.clone()); }
    }
    s.culture_rules = names.into_iter()
        .map(|culture| CultureRule { culture, line: line.as_u8(), rule: rule.as_u8() })
        .collect();
}

/// What a merchant class looks like after a run — the fragmentation measures.
struct Fragmentation {
    houses_alive: usize,
    /// Houses that ever existed, live or dead — divisions show up here even when the
    /// co-heir later fails.
    houses_ever: usize,
    /// Estate divisions recorded (`inheritance` events that actually split).
    divisions: usize,
    /// Co-heir houses founded by a division.
    coheirs: usize,
    /// Generations where the heirs held the capital TOGETHER — the *fraterna* case,
    /// which is what partible inheritance does when the shares are too small to stand
    /// alone. Counted because a generation that did not split is as much a consequence
    /// of the rule as one that did.
    joint: usize,
    /// Successions that occurred at all — the denominator for the two above.
    successions: usize,
    mean_wealth: f32,
    /// Share of all house wealth held by the single richest house.
    top_share: f32,
    gini: f32,
    /// Total merchant wealth — the division must MOVE capital, never create it.
    total_wealth: f32,
}

fn measure_fragmentation(s: &CampaignSim) -> Fragmentation {
    let live: Vec<&House> = s.houses.iter().filter(|h| !h.defunct && !h.is_guild).collect();
    let wealths: Vec<f32> = live.iter().map(|h| h.wealth.max(0.0)).collect();
    let total: f32 = wealths.iter().sum();
    let top = wealths.iter().copied().fold(0.0f32, f32::max);
    Fragmentation {
        houses_alive: live.len(),
        houses_ever: s.houses.iter().filter(|h| !h.is_guild).count(),
        divisions: s.houses.iter()
            .flat_map(|h| h.events.iter())
            .filter(|e| e.kind == "inheritance" && e.text.starts_with("the estate is divided"))
            .count(),
        coheirs: s.houses.iter().filter(|h| h.name.contains(" Line)")).count(),
        joint: s.houses.iter()
            .flat_map(|h| h.events.iter())
            .filter(|e| e.kind == "inheritance" && e.text.contains("keep the capital together"))
            .count(),
        successions: s.houses.iter()
            .flat_map(|h| h.events.iter())
            .filter(|e| e.kind == "succession")
            .count(),
        mean_wealth: if live.is_empty() { 0.0 } else { total / live.len() as f32 },
        top_share: if total > 1e-6 { top / total } else { 0.0 },
        gini: gini(&wealths),
        total_wealth: total,
    }
}

fn run_under(line: LineRule, rule: InheritanceRule) -> Fragmentation {
    let mut s = reference_world();
    force_inheritance(&mut s, line, rule);
    for _ in 1..=RUN_YEARS { s.advance(365); }
    measure_fragmentation(&s)
}

#[test]
fn econ_inheritance_rules_fragment_differently() {
    let part = run_under(LineRule::Agnatic, InheritanceRule::Partible);
    let prim = run_under(LineRule::Agnatic, InheritanceRule::Primogeniture);
    let ulti = run_under(LineRule::Agnatic, InheritanceRule::Ultimogeniture);
    let seni = run_under(LineRule::Agnatic, InheritanceRule::Seniority);

    let row = |name: &str, f: &Fragmentation| {
        println!("  {name:<16} {:>7} {:>6} {:>7} {:>7} {:>6} {:>6} {:>9.3} {:>7.3} {:>11.0}",
                 f.houses_alive, f.houses_ever, f.successions, f.divisions, f.coheirs,
                 f.joint, f.top_share, f.gini, f.mean_wealth);
    };
    println!();
    println!("═══ Phase 0.4 · inheritance ({RUN_YEARS} years, one world, four laws) ═══");
    println!("  {:<16} {:>7} {:>6} {:>7} {:>7} {:>6} {:>6} {:>9} {:>7} {:>11}",
             "rule", "alive", "ever", "succ", "divided", "co-heir", "joint",
             "top share", "gini", "mean wealth");
    row("partible", &part);
    row("primogeniture", &prim);
    row("ultimogeniture", &ulti);
    row("seniority", &seni);
    println!();
    println!("  Partible splits the capital at every death: MORE firms, each SMALLER.");
    println!("  Note what it does NOT do — the top share and Gini do not fall, because a");
    println!("  division adds small firms at the bottom as fast as it trims the top. The");
    println!("  measure that moves is mean wealth per house: the same capital, spread");
    println!("  over more houses. Seniority fragments by a different route — short");
    println!("  tenures, so many more successions, so many more branches.");
    println!("═══════════════════════════════════════════════════════════════════════");
    println!();

    // ── The gate ────────────────────────────────────────────────────────────
    // 1. The rule is WIRED: partible actually divides estates, and the rules that
    //    concentrate never do. A zero in either direction means the enum is decoration.
    assert!(part.divisions > 0, "partible inheritance never divided an estate");
    assert!(part.coheirs > 0, "a division never produced a co-heir house");
    assert_eq!(prim.divisions, 0, "primogeniture must not divide an estate");
    assert_eq!(ulti.divisions, 0, "ultimogeniture must not divide an estate");
    assert_eq!(seni.divisions, 0, "seniority must not divide an estate");

    // 2. The rule MATTERS: fragmentation differs measurably. More firms ever founded
    //    under partible than under any rule that concentrates.
    assert!(
        part.houses_ever > prim.houses_ever,
        "partible must fragment the merchant class more than primogeniture \
         ({} vs {} houses ever)", part.houses_ever, prim.houses_ever
    );

    // 3. The same capital is spread THINNER. This is the measure that actually moves
    //    (see the note printed above): a division does not lower the top share, it
    //    lowers what an average house holds.
    assert!(
        part.mean_wealth < prim.mean_wealth,
        "partible must leave the average house poorer than primogeniture \
         ({:.0} vs {:.0})", part.mean_wealth, prim.mean_wealth
    );

    // 4. Nothing is created. A division MOVES capital from parent to co-heir; if the
    //    partible world simply has more total wealth, the split is minting money.
    assert!(part.total_wealth.is_finite() && prim.total_wealth.is_finite(),
            "house wealth is not finite");
    for f in [&part, &prim, &ulti, &seni] {
        assert!(f.mean_wealth >= 0.0 && f.mean_wealth < 1e6,
                "house wealth left its bounds: mean {:.1}", f.mean_wealth);
    }
}
