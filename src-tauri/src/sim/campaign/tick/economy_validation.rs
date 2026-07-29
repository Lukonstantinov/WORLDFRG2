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
/// houses, free land to the south, and a seeded province layer behind the cities
/// so urbanisation is measurable.
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
    let nprov = 5usize;
    let urban_seed: f32 = s.hubs.iter().map(|h| h.population).sum();
    let rural_each = (urban_seed * 9.0 / nprov as f32).max(1.0);
    s.prov_cap = vec![rural_each * 1.6; nprov];
    s.prov_rural = vec![rural_each; nprov];
    s.prov_culture = (0..nprov).map(|i| format!("Culture{i}")).collect();
    s.prov_seat = (0..nprov).map(|i| [(i as f32) * 11.0, 20.0]).collect();
    s.prov_net_mig = vec![0.0; nprov];
    s.hub_province = (0..30).map(|i| (i % nprov) as i32).collect();
    s.hub_culture = (0..30).map(|i| format!("Culture{}", i % nprov)).collect();
    s.hub_minorities = vec![Vec::new(); 30];

    s.rebuild_routes();
    s
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

    // ── Asserted structure ──────────────────────────────────────────────────
    assert!(
        card.integration_gradient >= ECON_INTEGRATION_FLOOR,
        "distance stopped mattering to grain prices (r = {:.3}, floor {:.3}) — \
         the market has collapsed into a single warehouse",
        card.integration_gradient, ECON_INTEGRATION_FLOOR
    );
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

/// The scorecard must be reproducible. A fidelity gate that returns a different
/// number each run cannot guard anything — this is the economy's equivalent of
/// the phase-3 field checksums.
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
