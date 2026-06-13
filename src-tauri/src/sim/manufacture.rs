//! Shared production-chain resolver.
//!
//! Pre-modern wealth was made as much by *transforming* imported raws into
//! finished exports (wool→cloth, ore→arms, raw→refined sugar) as by sitting on a
//! resource. `Distribution::Manufactured` goods carry a recipe (`GoodSpec.inputs`)
//! and have NO per-cell belt; this pass turns a hub's input stocks into finished
//! output, concentrated in big cities (labor ∝ population).
//!
//! It runs in two places on the SAME `GoodSpec` definitions:
//!   • worldgen — `compute_economy` calls it on the static per-hub production
//!     vectors before the market solves, so manufactured goods flow as exports.
//!   • campaign — `tick.rs` runs the same logic per tick for manufactory estates.
//!
//! Pure & deterministic (no DB / RNG). Topologically orders manufactured goods by
//! recipe depth so multi-stage chains (raw→intermediate→finished) resolve, and
//! refuses cycles / missing inputs (those recipes are disabled with a warning).

use super::goods_spec::{Distribution, GoodSpec};
use std::collections::HashMap;

/// Transform per-hub production in place: consume recipe inputs and add finished
/// `Manufactured` goods, scaled by each hub's labor capacity (∝ population).
///
/// `prod[hub][good]` is the production vector (indexed parallel to `specs`).
/// `hub_pop[hub]` is the hub population. Returns human-readable warnings for any
/// recipe that was disabled (cycle or missing/unknown input).
pub fn apply_manufacturing(
    prod: &mut [Vec<f32>],
    specs: &[GoodSpec],
    hub_pop: &[f32],
) -> Vec<String> {
    let ng = specs.len();
    if ng == 0 || prod.is_empty() {
        return Vec::new();
    }
    let id_index: HashMap<&str, usize> =
        specs.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();

    // Resolve each manufactured good's input indices once; drop recipes whose
    // inputs reference an unknown good.
    let mut warnings = Vec::new();
    let mut recipes: HashMap<usize, Vec<(usize, f32)>> = HashMap::new();
    for (g, spec) in specs.iter().enumerate() {
        if !matches!(spec.distribution, Distribution::Manufactured) || !spec.enabled {
            continue;
        }
        if spec.inputs.is_empty() {
            // Manufactured but no recipe → nothing to make; leave at zero.
            continue;
        }
        let mut resolved = Vec::with_capacity(spec.inputs.len());
        let mut ok = true;
        for inp in &spec.inputs {
            match id_index.get(inp.good.as_str()) {
                Some(&idx) => resolved.push((idx, inp.qty.max(0.0))),
                None => {
                    ok = false;
                    warnings.push(format!(
                        "{}: recipe input '{}' is unknown — recipe disabled",
                        spec.id, inp.good
                    ));
                    break;
                }
            }
        }
        if ok {
            recipes.insert(g, resolved);
        }
    }
    if recipes.is_empty() {
        return warnings;
    }

    // Topologically order the manufactured goods by recipe depth (raws first) so a
    // chain that feeds another chain (e.g. refined good → liqueur) resolves in one
    // pass. Kahn-style over the manufactured-good subgraph; a leftover set = cycle.
    let order = match topo_order(&recipes) {
        Ok(o) => o,
        Err(cyclic) => {
            for g in &cyclic {
                warnings.push(format!("{}: recipe is part of a cycle — disabled", specs[*g].id));
                recipes.remove(g);
            }
            // Re-order the acyclic remainder.
            topo_order(&recipes).unwrap_or_default()
        }
    };

    // Median population → labor scale, so big cities out-manufacture villages.
    let median_pop = median(hub_pop).max(1.0);

    for (h, pv) in prod.iter_mut().enumerate() {
        if pv.len() < ng {
            pv.resize(ng, 0.0);
        }
        let pop = hub_pop.get(h).copied().unwrap_or(0.0).max(0.0);
        for &g in &order {
            let inputs = match recipes.get(&g) {
                Some(v) => v,
                None => continue,
            };
            // Labor capacity caps output; raw availability caps it too.
            let labor_cap = (pop / median_pop) * specs[g].labor.max(0.0);
            if labor_cap <= 0.0 {
                continue;
            }
            // How many output units the on-hand inputs can support.
            let mut by_inputs = f32::INFINITY;
            for &(idx, qty) in inputs {
                if qty <= 0.0 {
                    continue;
                }
                by_inputs = by_inputs.min(pv[idx] / qty);
            }
            if !by_inputs.is_finite() || by_inputs <= 0.0 {
                continue;
            }
            let made = by_inputs.min(labor_cap);
            if made <= 0.0 {
                continue;
            }
            for &(idx, qty) in inputs {
                pv[idx] = (pv[idx] - made * qty).max(0.0);
            }
            pv[g] += made;
        }
    }

    warnings
}

/// Kahn topological sort over the manufactured-good dependency subgraph. An edge
/// input→output exists when an input is itself a manufactured good in the set.
/// Returns the order (deps first) or `Err(cyclic_nodes)` if a cycle remains.
fn topo_order(recipes: &HashMap<usize, Vec<(usize, f32)>>) -> Result<Vec<usize>, Vec<usize>> {
    // In-degree = how many of a good's inputs are themselves manufactured goods in
    // the set (edges from raws don't count — raws are always available first).
    let mut indeg: HashMap<usize, usize> = HashMap::new();
    for (&g, inputs) in recipes {
        let d = inputs.iter().filter(|(idx, _)| recipes.contains_key(idx)).count();
        indeg.insert(g, d);
    }

    let mut queue: Vec<usize> = indeg.iter().filter(|(_, &d)| d == 0).map(|(&g, _)| g).collect();
    queue.sort_unstable(); // deterministic
    let mut order = Vec::with_capacity(recipes.len());
    while let Some(g) = queue.pop() {
        order.push(g);
        // Any good that consumes g loses one in-degree.
        let mut newly: Vec<usize> = Vec::new();
        for (&h, inputs) in recipes {
            if inputs.iter().any(|(idx, _)| *idx == g) {
                if let Some(d) = indeg.get_mut(&h) {
                    if *d > 0 {
                        *d -= 1;
                        if *d == 0 {
                            newly.push(h);
                        }
                    }
                }
            }
        }
        newly.sort_unstable();
        queue.extend(newly);
        queue.sort_unstable();
    }
    if order.len() == recipes.len() {
        Ok(order)
    } else {
        // The nodes never reaching in-degree 0 are the cycle.
        let cyclic: Vec<usize> = recipes
            .keys()
            .filter(|g| !order.contains(g))
            .copied()
            .collect();
        Err(cyclic)
    }
}

fn median(v: &[f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s: Vec<f32> = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    s[s.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::goods_spec::{Distribution, Domain, GoodSpec, RecipeInput};

    fn raw(id: &str) -> GoodSpec {
        GoodSpec {
            id: id.into(), name: id.into(), icon: "".into(), color: "#fff".into(),
            enabled: true, domain: Domain::Continental, distribution: Distribution::Local,
            rarity: 0.5, desire: 0.5, network_luxury: false, builtin: false, deposit: None,
            scoring: None, category: "x".into(), need_tier: 1, base_value: 1.0,
            bulk: 1.0, perishable: 0.0, inputs: vec![], labor: 1.0, consumption_interval: 30.0,
        }
    }
    fn made(id: &str, inputs: Vec<(&str, f32)>) -> GoodSpec {
        let mut g = raw(id);
        g.distribution = Distribution::Manufactured;
        g.inputs = inputs.into_iter().map(|(s, q)| RecipeInput { good: s.into(), qty: q }).collect();
        g
    }

    #[test]
    fn wool_becomes_cloth_where_wool_is_present() {
        let specs = vec![raw("wool"), made("cloth", vec![("wool", 1.0)])];
        // Two hubs of equal (median) population: one has wool, one does not.
        let mut prod = vec![vec![10.0, 0.0], vec![0.0, 0.0]];
        let pops = vec![1000.0, 1000.0];
        apply_manufacturing(&mut prod, &specs, &pops);
        assert!(prod[0][1] > 0.0, "wool hub manufactured cloth");
        assert!(prod[0][0] < 10.0, "wool was consumed");
        assert_eq!(prod[1][1], 0.0, "no wool → no cloth");
    }

    #[test]
    fn multi_stage_chain_resolves_in_one_pass() {
        // raw cane → refined sugar → liqueur, all at one big hub.
        let specs = vec![
            raw("cane"),
            made("sugar", vec![("cane", 1.0)]),
            made("liqueur", vec![("sugar", 0.5)]),
        ];
        let mut prod = vec![vec![20.0, 0.0, 0.0]];
        let pops = vec![1000.0];
        apply_manufacturing(&mut prod, &specs, &pops);
        assert!(prod[0][1] > 0.0, "sugar made from cane");
        assert!(prod[0][2] > 0.0, "liqueur made from sugar in the SAME pass (topo order)");
    }

    #[test]
    fn a_cycle_is_rejected_without_panicking() {
        // a ← b, b ← a : both manufactured, mutual dependency.
        let specs = vec![
            made("a", vec![("b", 1.0)]),
            made("b", vec![("a", 1.0)]),
        ];
        let mut prod = vec![vec![5.0, 5.0]];
        let pops = vec![1000.0];
        let warnings = apply_manufacturing(&mut prod, &specs, &pops);
        assert!(warnings.iter().any(|w| w.contains("cycle")), "cycle reported: {warnings:?}");
    }

    #[test]
    fn big_cities_out_manufacture_villages() {
        let specs = vec![raw("wool"), made("cloth", vec![("wool", 1.0)])];
        // Same wool, very different populations; the big city should make more.
        let mut prod = vec![vec![100.0, 0.0], vec![100.0, 0.0]];
        let pops = vec![10_000.0, 200.0];
        apply_manufacturing(&mut prod, &specs, &pops);
        assert!(prod[0][1] > prod[1][1], "the larger city manufactures more cloth");
    }
}
