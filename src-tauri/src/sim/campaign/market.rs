//! Static market equilibrium solver — the Part III replacement for per-hop
//! markup compounding (which clamped every long route to the same terminal
//! cap, the "×9.6 plateau").
//!
//! Model: each hub holds STOCKS per good (production), fills a NEEDS ladder
//! (basic → comfort → luxury, with substitution between goods of the same
//! category), and quotes a LOCAL PRICE in the grain-equivalent numeraire:
//!
//! ```text
//! price[h][g] = base_value[g] · (need / stock)^K      (smoothed, clamped)
//! ```
//!
//! Merchants then arbitrage: a good moves from a to b only when the price gap
//! beats the freight (additive cost per travel day, so cheap bulky goods stay
//! local and silk crosses the world) plus tolls and a margin. Shipping evens
//! stocks out, prices relax over a few rounds, and the spatial price gradient
//! DECAYS with distance instead of slamming into a cap.
//!
//! Pure and deterministic: no DB, no RNG — unit-tested below.

/// Market-relevant facts about one good (mapped from `GoodSpec`).
#[derive(Clone, Debug)]
pub struct MarketGood {
    /// Substitution group id (goods sharing it satisfy the same need).
    /// Interned by the caller; usize::MAX = no substitution group.
    pub category: usize,
    /// 0 basic, 1 comfort, 2 luxury — fills in that order, weighted down.
    pub need_tier: u8,
    /// World-standard value in grain-equivalent (wheat = 1.0).
    pub base_value: f32,
    /// Base demand weight (the spec's `desire`).
    pub desire: f32,
    /// Freight weight/volume multiplier (1.0 = silk; 3-4 = bulky staple).
    pub bulk: f32,
    /// Extra freight per travel-day from spoilage (additive).
    pub perishable: f32,
}

/// Freight cost to move one unit of good `g` over `days` travel-days at the base
/// per-day rate: bulky goods cost more to haul, perishable goods accrue spoilage
/// on top. With `bulk = 1.0, perishable = 0.0` this is exactly `per_day · days`
/// (the old flat model), so a neutral good is unchanged.
#[inline]
pub fn freight_of(g: &MarketGood, per_day: f32, days: f32) -> f32 {
    per_day * days * g.bulk + g.perishable * days
}

/// One trading settlement-cluster.
#[derive(Clone, Debug)]
pub struct MarketHub {
    /// Population weight (absolute population works; only ratios matter).
    pub population: f32,
    /// Production per good, arbitrary units (consistent across hubs).
    pub production: Vec<f32>,
}

/// Symmetric route facts between hub pairs (from the coarse-grid Dijkstra).
pub struct RouteMatrix {
    pub n: usize,
    /// Travel days a→b; `f32::INFINITY` = unreachable under the chosen reach.
    pub days: Vec<f32>,
    /// Toll surcharge a→b in grain-eq per unit (straits, great-hub transit…).
    pub toll: Vec<f32>,
}

impl RouteMatrix {
    pub fn new(n: usize) -> Self {
        Self { n, days: vec![f32::INFINITY; n * n], toll: vec![0.0; n * n] }
    }
    #[inline]
    pub fn idx(&self, a: usize, b: usize) -> usize {
        a * self.n + b
    }
    pub fn set(&mut self, a: usize, b: usize, days: f32, toll: f32) {
        let (i, j) = (self.idx(a, b), self.idx(b, a));
        self.days[i] = days;
        self.days[j] = days;
        self.toll[i] = toll;
        self.toll[j] = toll;
    }
}

pub struct MarketParams {
    /// Relaxation rounds (price smoothing makes ~8 plenty).
    pub rounds: usize,
    /// Freight cost in grain-eq per unit per travel day. Flat per unit, so it
    /// is RELATIVELY heavier for cheap goods — timber stays local, silk sails.
    pub freight_per_day: f32,
    /// Minimum arbitrage gap (grain-eq per unit) before a shipment happens.
    pub margin: f32,
    /// Price-curve exponent in (need/stock)^K.
    pub k: f32,
    /// Price penalty multiplier when substituting within a category.
    pub substitution_penalty: f32,
}

impl Default for MarketParams {
    fn default() -> Self {
        Self { rounds: 8, freight_per_day: 0.012, margin: 0.05, k: 0.6, substitution_penalty: 1.15 }
    }
}

/// One aggregated shipment lane after relaxation.
#[derive(Clone, Debug)]
pub struct MarketFlow {
    pub from: usize,
    pub to: usize,
    pub good: usize,
    pub amount: f32,
    /// Value at the destination (amount × destination price), grain-eq.
    pub value: f32,
}

/// One emergent currency good at a hub, with the components that made it money:
/// liquidity (how many distinct trades it appears in), value density (its grain-
/// equivalent base value) and price stability. These drive the settlement
/// window's explained currency card.
#[derive(Clone, Debug, Default)]
pub struct CurrencyInfo {
    pub good: usize,
    pub liquidity: f32,
    pub value: f32,
    pub stability: f32,
    pub score: f32,
}

#[derive(Clone, Debug, Default)]
pub struct HubMetrics {
    /// Food-stock value per capita (food security). Drives growth/starvation.
    pub grain_wealth: f32,
    /// (export earnings − import spend) per capita (commercial prosperity).
    pub trade_wealth: f32,
    /// Top emergent currency goods at this hub, best first.
    pub currency_goods: Vec<CurrencyInfo>,
    pub export_value: f32,
    pub import_value: f32,
}

pub struct MarketResult {
    /// Local price per hub per good, grain-equivalent.
    pub prices: Vec<Vec<f32>>,
    /// Post-trade stocks.
    pub stocks: Vec<Vec<f32>>,
    /// Post-substitution needs (what each hub actually wanted).
    pub needs: Vec<Vec<f32>>,
    pub flows: Vec<MarketFlow>,
    pub hubs: Vec<HubMetrics>,
    /// Unmet basic+comfort need fraction per hub per good (0 = satisfied).
    pub unmet: Vec<Vec<f32>>,
}

const TIER_WEIGHT: [f32; 3] = [1.0, 0.45, 0.22];
const PRICE_FLOOR_MULT: f32 = 0.15;
const PRICE_CEIL_MULT: f32 = 12.0;
const EPS: f32 = 1e-4;

pub fn solve(
    hubs: &[MarketHub],
    goods: &[MarketGood],
    routes: &RouteMatrix,
    params: &MarketParams,
) -> MarketResult {
    let n = hubs.len();
    let ng = goods.len();
    assert_eq!(routes.n, n, "route matrix size must match hub count");

    // Population weights normalized so needs and production are comparable:
    // total need per good ≈ total production of an average good.
    let total_pop: f32 = hubs.iter().map(|h| h.population).sum::<f32>().max(1.0);
    let total_prod: f32 = hubs
        .iter()
        .flat_map(|h| h.production.iter())
        .sum::<f32>()
        .max(EPS);
    let prod_per_good = total_prod / ng.max(1) as f32;

    let mut stocks: Vec<Vec<f32>> = hubs.iter().map(|h| {
        let mut p = h.production.clone();
        p.resize(ng, 0.0);
        p
    }).collect();

    // Baseline (pre-substitution) need per hub/good: population share × tier
    // weight × desire, scaled into production units.
    let base_need: Vec<Vec<f32>> = hubs
        .iter()
        .map(|h| {
            let pop_share = h.population / total_pop;
            (0..ng)
                .map(|g| {
                    pop_share
                        * TIER_WEIGHT[goods[g].need_tier.min(2) as usize]
                        * goods[g].desire.max(0.0)
                        * prod_per_good
                        * 1.6 // demand pressure: a touch more want than supply
                })
                .collect()
        })
        .collect();

    // Category membership for substitution.
    let n_cats = goods
        .iter()
        .filter(|g| g.category != usize::MAX)
        .map(|g| g.category + 1)
        .max()
        .unwrap_or(0);
    let mut cat_goods: Vec<Vec<usize>> = vec![Vec::new(); n_cats];
    for (g, spec) in goods.iter().enumerate() {
        if spec.category != usize::MAX {
            cat_goods[spec.category].push(g);
        }
    }

    let mut prices: Vec<Vec<f32>> = (0..n)
        .map(|_| goods.iter().map(|g| g.base_value).collect())
        .collect();
    let mut needs = base_need.clone();
    use std::collections::HashMap;
    let mut flow_map: HashMap<(usize, usize, usize), f32> = HashMap::new();
    let mut prev_prices = prices.clone();

    for _round in 0..params.rounds {
        // 1) Substitution: each hub redistributes every category's total need
        //    toward its cheaper (relative to base value) alternatives.
        for h in 0..n {
            for members in &cat_goods {
                if members.len() < 2 {
                    continue;
                }
                let total: f32 = members.iter().map(|&g| base_need[h][g]).sum();
                if total <= EPS {
                    continue;
                }
                // Attractiveness ∝ desire / relative price (with a penalty for
                // straying from the hub's baseline preference mix).
                let weights: Vec<f32> = members
                    .iter()
                    .map(|&g| {
                        let rel = (prices[h][g] / goods[g].base_value.max(EPS))
                            .max(PRICE_FLOOR_MULT);
                        let pref = base_need[h][g] / total;
                        pref / (rel * params.substitution_penalty).max(EPS)
                            + pref * (1.0 - 1.0 / params.substitution_penalty)
                    })
                    .collect();
                let wsum: f32 = weights.iter().sum::<f32>().max(EPS);
                for (mi, &g) in members.iter().enumerate() {
                    needs[h][g] = total * weights[mi] / wsum;
                }
            }
        }

        // 2) Local price from scarcity, smoothed.
        prev_prices.clone_from(&prices);
        for h in 0..n {
            for g in 0..ng {
                let base = goods[g].base_value;
                let ratio = (needs[h][g] + EPS) / (stocks[h][g] + EPS);
                let target = (base * ratio.powf(params.k))
                    .clamp(base * PRICE_FLOOR_MULT, base * PRICE_CEIL_MULT);
                prices[h][g] = 0.5 * prices[h][g] + 0.5 * target;
            }
        }

        // 3) Arbitrage: ship where the destination price beats delivered cost.
        //    Trade decisions use LIVE prices (recomputed from current stocks as
        //    shipments land), so a producer's price rises as it exports and the
        //    gap closes at delivered-cost parity instead of overshooting.
        let live = |stock: f32, need: f32, base: f32| -> f32 {
            (base * ((need + EPS) / (stock + EPS)).powf(params.k))
                .clamp(base * PRICE_FLOOR_MULT, base * PRICE_CEIL_MULT)
        };
        for g in 0..ng {
            let base = goods[g].base_value;
            for a in 0..n {
                for b in 0..n {
                    if b == a {
                        continue;
                    }
                    let surplus = stocks[a][g] - needs[a][g];
                    if surplus <= EPS {
                        break; // sold out for this round
                    }
                    let days = routes.days[routes.idx(a, b)];
                    if !days.is_finite() {
                        continue; // sea-impassable under the chosen reach
                    }
                    let pa = live(stocks[a][g], needs[a][g], base);
                    let pb = live(stocks[b][g], needs[b][g], base);
                    let freight = freight_of(&goods[g], params.freight_per_day, days);
                    let toll = routes.toll[routes.idx(a, b)];
                    let delivered = pa + freight + toll;
                    let gap = pb - delivered - params.margin * base;
                    if gap <= 0.0 {
                        continue;
                    }
                    // Demand elasticity: never fill b past the stock level at
                    // which its local price would fall below the delivered cost
                    // — distant hubs import less and keep dearer prices, which
                    // is exactly the decaying spatial gradient we want.
                    let max_stock = needs[b][g] * (base / delivered.max(EPS)).powf(1.0 / params.k);
                    let room = max_stock - stocks[b][g];
                    if room <= EPS {
                        continue;
                    }
                    // Half-steps toward parity per round → geometric convergence.
                    let amount = surplus.min(room) * 0.5;
                    if amount <= EPS {
                        continue;
                    }
                    stocks[a][g] -= amount;
                    stocks[b][g] += amount;
                    *flow_map.entry((a, b, g)).or_insert(0.0) += amount;
                }
            }
        }
    }

    // Final price pass so quoted prices reflect post-trade stocks.
    for h in 0..n {
        for g in 0..ng {
            let base = goods[g].base_value;
            let ratio = (needs[h][g] + EPS) / (stocks[h][g] + EPS);
            prices[h][g] = (base * ratio.powf(params.k))
                .clamp(base * PRICE_FLOOR_MULT, base * PRICE_CEIL_MULT);
        }
    }

    // Aggregate flows deterministically (sorted by route then good).
    let mut flows: Vec<MarketFlow> = flow_map
        .into_iter()
        .map(|((from, to, good), amount)| MarketFlow { from, to, good, amount, value: 0.0 })
        .collect();
    flows.sort_by(|x, y| (x.from, x.to, x.good).cmp(&(y.from, y.to, y.good)));

    // An imported good can't be quoted below its cheapest actual delivered
    // cost (origin price + freight + toll) — this is what keeps the spatial
    // price gradient: a distant consumer pays the freight, a near one barely.
    for f in &flows {
        let delivered = prices[f.from][f.good]
            + freight_of(&goods[f.good], params.freight_per_day, routes.days[routes.idx(f.from, f.to)])
            + routes.toll[routes.idx(f.from, f.to)];
        let cap = goods[f.good].base_value * PRICE_CEIL_MULT;
        if prices[f.to][f.good] < delivered {
            prices[f.to][f.good] = delivered.min(cap);
        }
    }
    for f in flows.iter_mut() {
        f.value = f.amount * prices[f.to][f.good];
    }

    // Hub metrics.
    let mut metrics: Vec<HubMetrics> = vec![HubMetrics::default(); n];
    // Distinct counterparties per (hub, good) → currency liquidity.
    let mut partners: HashMap<(usize, usize), std::collections::HashSet<usize>> = HashMap::new();
    for f in &flows {
        let sell_value = f.amount * prices[f.from][f.good];
        metrics[f.from].export_value += sell_value + (f.value - sell_value).max(0.0) * 0.5;
        metrics[f.to].import_value += f.value;
        partners.entry((f.from, f.good)).or_default().insert(f.to);
        partners.entry((f.to, f.good)).or_default().insert(f.from);
    }
    let mut unmet: Vec<Vec<f32>> = vec![vec![0.0; ng]; n];
    for h in 0..n {
        let pop = hubs[h].population.max(1.0);
        // Food security: stock value of the food categories per capita.
        let mut food = 0.0;
        for g in 0..ng {
            let w = match goods[g].need_tier {
                0 => 1.0,
                1 => 0.4,
                _ => 0.0,
            };
            // Only edible categories count toward grain wealth; the caller maps
            // category ids — by convention 0..=3 are cereal/protein/oil/sweetener.
            if goods[g].category <= 3 {
                food += stocks[h][g] * goods[g].base_value * w;
            }
            let need = needs[h][g];
            if need > EPS && goods[g].need_tier < 2 {
                unmet[h][g] = ((need - stocks[h][g]) / need).clamp(0.0, 1.0);
            }
        }
        metrics[h].grain_wealth = food * total_pop / (pop * prod_per_good.max(EPS));
        metrics[h].trade_wealth =
            (metrics[h].export_value - metrics[h].import_value) * total_pop
                / (pop * prod_per_good.max(EPS));

        // Emergent currency goods: liquidity × value density × price stability.
        let mut scores: Vec<CurrencyInfo> = (0..ng)
            .filter_map(|g| {
                let liq = partners.get(&(h, g)).map(|s| s.len()).unwrap_or(0) as f32;
                if liq < 1.0 {
                    return None;
                }
                let stability = 1.0
                    / (1.0 + ((prices[h][g] - prev_prices[h][g]).abs()
                        / prices[h][g].max(EPS)));
                let value = goods[g].base_value;
                Some(CurrencyInfo {
                    good: g,
                    liquidity: liq,
                    value,
                    stability,
                    score: liq * value.sqrt() * stability,
                })
            })
            .collect();
        scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.good.cmp(&b.good)));
        scores.truncate(3);
        metrics[h].currency_goods = scores;
    }

    MarketResult { prices, stocks, needs, flows, hubs: metrics, unmet }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// goods: 0 wheat (cereal/basic/1.0), 1 rice (cereal/basic/1.1),
    ///        2 silk (fiber/luxury/20), 3 iron (metal/comfort/3)
    fn goods() -> Vec<MarketGood> {
        vec![
            MarketGood { category: 0, need_tier: 0, base_value: 1.0, desire: 0.85, bulk: 1.0, perishable: 0.0 },
            MarketGood { category: 0, need_tier: 0, base_value: 1.1, desire: 0.8, bulk: 1.0, perishable: 0.0 },
            MarketGood { category: 4, need_tier: 2, base_value: 20.0, desire: 0.35, bulk: 1.0, perishable: 0.0 },
            MarketGood { category: 7, need_tier: 1, base_value: 3.0, desire: 0.65, bulk: 1.0, perishable: 0.0 },
        ]
    }

    fn hub(pop: f32, production: Vec<f32>) -> MarketHub {
        MarketHub { population: pop, production }
    }

    #[test]
    fn converges_and_stays_finite() {
        let hubs = vec![
            hub(10_000.0, vec![50.0, 0.0, 0.0, 5.0]),
            hub(20_000.0, vec![0.0, 60.0, 2.0, 0.0]),
            hub(5_000.0, vec![10.0, 5.0, 0.0, 20.0]),
        ];
        let mut routes = RouteMatrix::new(3);
        routes.set(0, 1, 10.0, 0.0);
        routes.set(1, 2, 20.0, 0.1);
        routes.set(0, 2, 25.0, 0.0);
        let res = solve(&hubs, &goods(), &routes, &MarketParams::default());
        for h in 0..3 {
            for g in 0..4 {
                assert!(res.prices[h][g].is_finite() && res.prices[h][g] > 0.0);
                assert!(res.stocks[h][g].is_finite() && res.stocks[h][g] >= -1e-3);
            }
            assert!(res.hubs[h].grain_wealth.is_finite());
            assert!(res.hubs[h].trade_wealth.is_finite());
        }
        // Same input → same output (determinism).
        let res2 = solve(&hubs, &goods(), &routes, &MarketParams::default());
        assert_eq!(res.prices, res2.prices);
        assert_eq!(res.flows.len(), res2.flows.len());
    }

    #[test]
    fn scarcity_raises_price_and_freight_makes_a_decaying_gradient() {
        // Hub 0 produces silk; 1 (near) and 2 (far) produce none.
        let hubs = vec![
            hub(10_000.0, vec![40.0, 0.0, 10.0, 10.0]),
            hub(10_000.0, vec![40.0, 0.0, 0.0, 10.0]),
            hub(10_000.0, vec![40.0, 0.0, 0.0, 10.0]),
        ];
        let mut routes = RouteMatrix::new(3);
        routes.set(0, 1, 8.0, 0.0);
        routes.set(0, 2, 60.0, 0.0);
        routes.set(1, 2, 52.0, 0.0);
        let res = solve(&hubs, &goods(), &routes, &MarketParams::default());
        let silk = 2;
        // Producer cheapest, near importer next, far importer dearest…
        assert!(res.prices[0][silk] < res.prices[1][silk]);
        assert!(res.prices[1][silk] < res.prices[2][silk]);
        // …but the far price is a finite gradient, not a hard ×9.6-style wall
        // (it stays under the solver ceiling).
        assert!(res.prices[2][silk] < 20.0 * 12.0);
        // Silk actually moved to both importers.
        assert!(res.flows.iter().any(|f| f.good == silk && f.to == 1));
        assert!(res.flows.iter().any(|f| f.good == silk && f.to == 2));
    }

    #[test]
    fn category_substitution_covers_a_missing_staple() {
        // Hub 1 grows only rice, hub 0 only wheat; no route between them.
        let hubs = vec![
            hub(10_000.0, vec![60.0, 0.0, 0.0, 5.0]),
            hub(10_000.0, vec![0.0, 60.0, 0.0, 5.0]),
        ];
        let routes = RouteMatrix::new(2); // unreachable
        let res = solve(&hubs, &goods(), &routes, &MarketParams::default());
        // Each hub shifted its cereal need onto its local alternative…
        assert!(res.needs[0][0] > res.needs[0][1], "hub 0 wants wheat");
        assert!(res.needs[1][1] > res.needs[1][0], "hub 1 wants rice");
        // …and neither starves on the basic tier.
        assert!(res.unmet[0][0] < 0.6);
        assert!(res.unmet[1][1] < 0.6);
        // No flows: they're unreachable.
        assert!(res.flows.is_empty());
    }

    #[test]
    fn cutting_the_food_route_collapses_grain_wealth() {
        // Hub 1 produces NO food at all; hub 0 is the breadbasket.
        let hubs = vec![
            hub(10_000.0, vec![120.0, 0.0, 0.0, 0.0]),
            hub(10_000.0, vec![0.0, 0.0, 5.0, 30.0]),
        ];
        let mut connected = RouteMatrix::new(2);
        connected.set(0, 1, 6.0, 0.0);
        let fed = solve(&hubs, &goods(), &connected, &MarketParams::default());

        let cut = RouteMatrix::new(2); // route severed
        let starving = solve(&hubs, &goods(), &cut, &MarketParams::default());

        assert!(fed.hubs[1].grain_wealth > starving.hubs[1].grain_wealth * 2.0,
            "fed {} vs starving {}", fed.hubs[1].grain_wealth, starving.hubs[1].grain_wealth);
        // The cut-off hub's unmet basic cereal need is severe.
        assert!(starving.unmet[1][0].max(starving.unmet[1][1]) > 0.8);
    }

    #[test]
    fn bulk_makes_a_heavy_good_ship_less_and_stay_dearer_than_a_light_one() {
        // Two luxuries IDENTICAL except for bulk: good 0 is light (silk-like),
        // good 1 is heavy (marble-like). Both produced only at hub 0; hub 2 is far.
        let goods = vec![
            MarketGood { category: usize::MAX, need_tier: 2, base_value: 10.0, desire: 0.6, bulk: 1.0, perishable: 0.0 },
            MarketGood { category: usize::MAX, need_tier: 2, base_value: 10.0, desire: 0.6, bulk: 6.0, perishable: 0.0 },
        ];
        let hubs = vec![
            hub(10_000.0, vec![60.0, 60.0]),
            hub(10_000.0, vec![0.0, 0.0]),
            hub(10_000.0, vec![0.0, 0.0]),
        ];
        let mut routes = RouteMatrix::new(3);
        routes.set(0, 1, 10.0, 0.0);
        routes.set(0, 2, 45.0, 0.0);
        routes.set(1, 2, 38.0, 0.0);
        let res = solve(&hubs, &goods, &routes, &MarketParams::default());
        let (light, heavy) = (0usize, 1usize);
        // The heavy good costs more to haul, so the far hub pays MORE for it…
        assert!(res.prices[2][heavy] > res.prices[2][light],
            "heavy {} vs light {} at far hub", res.prices[2][heavy], res.prices[2][light]);
        // …and less of it actually reaches the far hub (freight eats the arbitrage).
        let shipped = |g: usize| -> f32 {
            res.flows.iter().filter(|f| f.good == g && f.to == 2).map(|f| f.amount).sum()
        };
        assert!(shipped(heavy) < shipped(light),
            "heavy shipped {} vs light {}", shipped(heavy), shipped(light));
    }
}
