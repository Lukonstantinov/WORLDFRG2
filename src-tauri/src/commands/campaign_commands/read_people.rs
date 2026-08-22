//! read_people commands — split from the former monolithic campaign_commands.rs.
//! `use super::*` inherits the shared imports, structs and helpers kept in mod.rs.
use super::*;


/// Per-culture census: population, town count, top cities, houses, mobility. Sorted
/// by population (largest people first). Powers the Peoples panel.
#[tauri::command]
pub fn campaign_get_cultures(db: State<'_, WorldDb>) -> Result<Vec<CultureBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    use std::collections::HashMap;
    struct Acc { pop: f64, towns: u32, presence: u32, cities: Vec<(String, f64)> }
    let mut map: HashMap<String, Acc> = HashMap::new();
    let add = |map: &mut HashMap<String, Acc>, c: &str, name: &str, p: f64, major: bool, present: bool| {
        let e = map.entry(c.to_string()).or_insert_with(|| Acc { pop: 0.0, towns: 0, presence: 0, cities: vec![] });
        e.pop += p;
        if major { e.towns += 1; }
        if present { e.presence += 1; }
        e.cities.push((name.to_string(), p));
    };
    for i in 0..sim.hubs.len() {
        let h = &sim.hubs[i];
        if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
        let pop = h.population.max(0.0) as f64;
        let maj = sim.hub_culture.get(i).cloned().unwrap_or_default();
        if maj.is_empty() || maj == "—" { continue; }
        let mshare: f32 = sim.hub_minorities.get(i).map(|m| m.iter().fold(0.0f32, |a, (_, s)| a + *s)).unwrap_or(0.0);
        let maj_share = (1.0 - mshare).clamp(0.0, 1.0) as f64;
        add(&mut map, &maj, &h.name, pop * maj_share, true, true);
        if let Some(mins) = sim.hub_minorities.get(i) {
            for (c, s) in mins {
                if *s <= 0.005 { continue; }
                add(&mut map, c, &h.name, pop * (*s as f64), false, *s >= 0.05);
            }
        }
    }
    let mut houses_by: HashMap<String, Vec<String>> = HashMap::new();
    for hh in &sim.houses {
        if hh.defunct { continue; }
        if let Some(c) = sim.hub_culture.get(hh.hub as usize) {
            if !c.is_empty() && c != "—" {
                houses_by.entry(c.clone()).or_default().push(hh.name.clone());
            }
        }
    }

    // ── Wealth, co-habitation and house-rivalries per culture ──
    let mut wealth_by: HashMap<String, f64> = HashMap::new();
    let mut cohab: HashMap<String, HashMap<String, f32>> = HashMap::new();
    for i in 0..sim.hubs.len() {
        let h = &sim.hubs[i];
        if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
        let w = (h.grain_wealth as f64 + 1.5 * h.trade_wealth as f64).max(0.0);
        let maj = sim.hub_culture.get(i).cloned().unwrap_or_default();
        if maj.is_empty() || maj == "—" { continue; }
        let mins = sim.hub_minorities.get(i);
        let mshare: f32 = mins.map(|m| m.iter().fold(0.0f32, |a, (_, s)| a + *s)).unwrap_or(0.0);
        let maj_share = (1.0 - mshare).clamp(0.0, 1.0);
        *wealth_by.entry(maj.clone()).or_default() += w * maj_share as f64;
        let mut present: Vec<(String, f32)> = vec![(maj.clone(), maj_share)];
        if let Some(m) = mins {
            for (c, s) in m {
                if *s <= 0.005 { continue; }
                *wealth_by.entry(c.clone()).or_default() += w * (*s as f64);
                if *s >= 0.05 { present.push((c.clone(), *s)); }
            }
        }
        for a in 0..present.len() {
            for b in 0..present.len() {
                if a == b { continue; }
                *cohab.entry(present[a].0.clone()).or_default()
                    .entry(present[b].0.clone()).or_default() += present[a].1.min(present[b].1);
            }
        }
    }
    for hh in &sim.houses {
        if hh.defunct { continue; }
        if let Some(c) = sim.hub_culture.get(hh.hub as usize) {
            if !c.is_empty() && c != "—" { *wealth_by.entry(c.clone()).or_default() += hh.wealth.max(0.0) as f64; }
        }
    }
    // House rivalries → culture rivalries.
    let mut rival_by: HashMap<String, HashMap<String, u32>> = HashMap::new();
    for hh in &sim.houses {
        if hh.defunct { continue; }
        let ca = sim.hub_culture.get(hh.hub as usize).cloned().unwrap_or_default();
        if ca.is_empty() || ca == "—" { continue; }
        for &r in &hh.rivals {
            if let Some(rh) = sim.houses.get(r) {
                let cb = sim.hub_culture.get(rh.hub as usize).cloned().unwrap_or_default();
                if !cb.is_empty() && cb != "—" && cb != ca {
                    *rival_by.entry(ca.clone()).or_default().entry(cb).or_default() += 1;
                }
            }
        }
    }

    let living_names: std::collections::HashSet<String> = map.keys().cloned().collect();
    let family_of: HashMap<String, String> =
        living_names.iter().map(|n| (n.clone(), culture_lore(&sim, n).0)).collect();
    let traits_of: HashMap<String, Vec<usize>> =
        living_names.iter().map(|n| (n.clone(), culture_trait_ids(&sim, n))).collect();
    let is_frictional = |t: &[usize]| t.contains(&3) || t.contains(&13); // Martial / Xenophobic
    let is_mercantile = |t: &[usize]| t.contains(&0);

    // Relations for one living culture: kin (same family), then co-habitants sorted by
    // shared presence classed hostile / rival / friendly, plus house rivalries.
    let relations_for = |name: &str| -> Vec<CultureRelation> {
        let mut rels: Vec<CultureRelation> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let fam = family_of.get(name).cloned().unwrap_or_default();
        let my_t = traits_of.get(name).cloned().unwrap_or_default();
        // Kin — same language family.
        let mut kin: Vec<&String> = living_names.iter()
            .filter(|o| o.as_str() != name && !fam.is_empty() && family_of.get(*o) == Some(&fam))
            .collect();
        kin.sort();
        for k in kin.into_iter().take(3) {
            if seen.insert(k.clone()) { rels.push(CultureRelation { name: k.clone(), kind: "kin".into() }); }
        }
        // Co-habitants (share cities) → hostile / rival / friendly by trait clash.
        if let Some(neigh) = cohab.get(name) {
            let mut ns: Vec<(&String, f32)> = neigh.iter().map(|(k, v)| (k, *v)).collect();
            ns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (o, _) in ns.into_iter().take(5) {
                if o == name || seen.contains(o) { continue; }
                let ot = traits_of.get(o).cloned().unwrap_or_default();
                let same_fam = !fam.is_empty() && family_of.get(o) == Some(&fam);
                let kind = if !same_fam && (is_frictional(&my_t) || is_frictional(&ot)) { "hostile" }
                    else if is_mercantile(&my_t) && is_mercantile(&ot) { "rival" }
                    else { "friendly" };
                seen.insert(o.clone());
                rels.push(CultureRelation { name: o.clone(), kind: kind.into() });
            }
        }
        // Merchant-house rivalries.
        if let Some(rv) = rival_by.get(name) {
            let mut rs: Vec<(&String, u32)> = rv.iter().map(|(k, v)| (k, *v)).collect();
            rs.sort_by(|a, b| b.1.cmp(&a.1));
            for (o, _) in rs.into_iter().take(3) {
                if seen.insert(o.clone()) { rels.push(CultureRelation { name: o.clone(), kind: "rival".into() }); }
            }
        }
        rels.truncate(8);
        rels
    };

    let trait_briefs = |ids: &[usize]| -> Vec<CultureTraitBrief> {
        ids.iter().filter_map(|&i| crate::sim::cultures::TRAITS.get(i)).map(|t| CultureTraitBrief {
            name: t.name.to_string(), emoji: t.emoji.to_string(), blurb: t.blurb.to_string(),
        }).collect()
    };

    let mut out: Vec<CultureBrief> = map.into_iter().map(|(name, mut acc)| {
        acc.cities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top: Vec<(String, u32)> = acc.cities.into_iter().take(4).map(|(n, p)| (n, p as u32)).collect();
        let mut houses = houses_by.remove(&name).unwrap_or_default();
        houses.sort(); houses.dedup(); houses.truncate(8);
        let (family, origin, kit, kit2) = culture_lore(&sim, &name);
        // Desired goods: this people's kit (plus a creole's second parent), deduped.
        let mut desired_goods: Vec<String> = Vec::new();
        for k in [kit, kit2] {
            if k >= 0 {
                for g in crate::sim::cultures::kit_desired_goods(k as usize) {
                    if !desired_goods.iter().any(|x| x == g) { desired_goods.push((*g).to_string()); }
                }
            }
        }
        let trait_ids = traits_of.get(&name).cloned().unwrap_or_default();
        let lingua_regions = sim.lingua.iter().filter(|l| l.culture == name).count() as u32;
        CultureBrief {
            color: culture_color(&name),
            mobility: crate::sim::tick::CampaignSim::culture_mobility(&name),
            population: acc.pop as u32, towns: acc.towns, presence: acc.presence,
            top_cities: top, houses, family, origin, kit, kit2, desired_goods,
            traits: trait_briefs(&trait_ids),
            relations: relations_for(&name),
            wealth: *wealth_by.get(&name).unwrap_or(&0.0),
            alive: true, obituary: String::new(), lingua_regions,
            name,
        }
    }).collect();
    out.sort_by(|a, b| b.population.cmp(&a.population));

    // ── Extinct peoples: worldgen hearths (and creoles) that no living settlement
    // now holds. Filed separately by the panel, with an obituary. ──
    let mut dead_names: Vec<(String, String, i32)> = Vec::new(); // (name, origin, kit)
    if let Some(m) = crate::sim::cultures::active() {
        for h in &m.hearths {
            if !living_names.contains(&h.people) {
                dead_names.push((h.people.clone(), h.origin.clone(), h.kit as i32));
            }
        }
    }
    for cr in &sim.creoles {
        if !living_names.contains(&cr.name) && !dead_names.iter().any(|(n, _, _)| n == &cr.name) {
            dead_names.push((cr.name.clone(), cr.origin.clone(), cr.kit_a as i32));
        }
    }
    dead_names.sort(); dead_names.dedup_by(|a, b| a.0 == b.0);
    for (name, origin, kit) in dead_names {
        let (family, o2, k2, kit2) = culture_lore(&sim, &name);
        let origin = if origin.is_empty() { o2 } else { origin };
        let kit = if kit >= 0 { kit } else { k2 };
        let trait_ids = culture_trait_ids(&sim, &name);
        let home = origin.split(['.', ';', '—']).next().unwrap_or("").trim();
        let obituary = if home.is_empty() {
            format!("The {name} are no more — no living settlement now holds them; they survive only in memory and in the blood of neighbouring peoples.")
        } else {
            format!("The {name} have faded from the living world. {home}. No settlement now holds them as its own; their line endures only mingled into the peoples who absorbed them.")
        };
        out.push(CultureBrief {
            color: culture_color(&name),
            mobility: crate::sim::tick::CampaignSim::culture_mobility(&name),
            population: 0, towns: 0, presence: 0, top_cities: vec![], houses: vec![],
            family, origin, kit, kit2,
            desired_goods: vec![],
            traits: trait_briefs(&trait_ids),
            relations: vec![],
            wealth: 0.0, alive: false, obituary,
            lingua_regions: sim.lingua.iter().filter(|l| l.culture == name).count() as u32,
            name,
        });
    }
    Ok(out)
}


#[tauri::command]
pub fn campaign_get_culture_presence(name: String, db: State<'_, WorldDb>) -> Result<CulturePresenceGrid, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::sim::cultures::ensure_active(&conn);
    let map = match crate::sim::cultures::active() { Some(m) => m, None => return Ok(CulturePresenceGrid { width: 0, height: 0, data: vec![], land: vec![], dominant: vec![] }) };
    let (cw, ch) = (map.cw, map.ch);
    let n = (cw * ch) as usize;
    let mut data = vec![0u8; n];
    let mut land = vec![0u8; n];
    // 1 where this people is DOMINANT (homeland / local majority), 0 where it's only
    // a minority presence — drives the full-vs-30% opacity split on the minimap.
    let mut dominant = vec![0u8; n];
    // ── Land mask from the REAL terrain, downsampled to the coarse raster ──
    // MAJOR landmasses only: a coarse cell counts as land when a meaningful share
    // (≥28%) of its fine cells are land, so continents and sizeable islands show
    // while tiny specks drop out. This is what makes the minimap resemble the
    // uploaded world (the old mask came from culture assignment, so uninhabited
    // islands were invisible).
    if let Ok(buf) = crate::sim::world_buffer::WorldBuffer::load_with(
        &conn, crate::sim::world_buffer::ColumnSet::TERRAIN)
    {
        let (ww, wh) = (buf.width, buf.height);
        let mut land_cnt = vec![0u32; n];
        let mut tot_cnt = vec![0u32; n];
        for y in 0..wh {
            for x in 0..ww {
                let cx = (x as u64 * cw as u64 / ww.max(1) as u64).min(cw as u64 - 1) as u32;
                let cy = (y as u64 * ch as u64 / wh.max(1) as u64).min(ch as u64 - 1) as u32;
                let ci = (cy * cw + cx) as usize;
                tot_cnt[ci] += 1;
                if buf.terrain[buf.idx(x, y)] == 1 { land_cnt[ci] += 1; }
            }
        }
        for ci in 0..n {
            if tot_cnt[ci] > 0 && land_cnt[ci] as f32 / tot_cnt[ci] as f32 >= 0.28 {
                land[ci] = 1; data[ci] = 40;
            }
        }
    } else {
        // Fallback (terrain not loadable): the old culture-assignment land mask.
        for (ci, &id) in map.ids.iter().enumerate().take(n) {
            if id != 255 { land[ci] = 1; data[ci] = 40; }
        }
    }
    // Homeland raster: this people's hearth cells brightest (lit even on a minor
    // island the terrain threshold skipped, since a hearth means people live there).
    let home = map.hearths.iter().position(|h| h.people == name);
    if let Some(hi) = home {
        for (ci, &id) in map.ids.iter().enumerate().take(n) {
            if id as usize == hi { data[ci] = 255; land[ci] = 1; dominant[ci] = 1; } // homeland = dominant
        }
    }
    // Live spread: light up coarse cells around cities where the people lives now.
    if let Some(sim) = get_sim(&db, &conn)? {
        let (ww, wh) = (map.world_w.max(1) as f32, map.world_h.max(1) as f32);
        let coarse = |x: f32, y: f32| -> usize {
            let cx = ((x / ww * cw as f32) as u32).min(cw.saturating_sub(1));
            let cy = ((y / wh * ch as f32) as u32).min(ch.saturating_sub(1));
            (cy * cw + cx) as usize
        };
        for i in 0..sim.hubs.len() {
            let h = &sim.hubs[i];
            if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
            let ci = coarse(h.x, h.y);
            if ci >= n { continue; }
            if sim.hub_culture.get(i).map(|c| c == &name).unwrap_or(false) {
                land[ci] = 1; data[ci] = 255; dominant[ci] = 1; // local MAJORITY = dominant
            } else if let Some(mins) = sim.hub_minorities.get(i) {
                if let Some((_, s)) = mins.iter().find(|(c, _)| c == &name) {
                    // Minority quarter: present but NOT dominant (unless this cell is
                    // already the people's homeland/majority elsewhere).
                    if *s >= 0.05 { land[ci] = 1; data[ci] = data[ci].max((120.0 + *s * 130.0).min(240.0) as u8); }
                }
            }
        }
    }
    Ok(CulturePresenceGrid { width: cw, height: ch, data, land, dominant })
}


/// Notable people of a culture — its great merchant houses' heads: who they were,
/// when they lived, what they did (trade monopolies, banking, ruling councils), their
/// seat city and the far cities their agents reached (a form of migration/expansion).
/// Grounded entirely in the live house registry.
#[tauri::command]
pub fn campaign_get_culture_notables(name: String, db: State<'_, WorldDb>) -> Result<Vec<NotablePerson>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let year_of = |t: u32| (t / crate::sim::tick::TICKS_PER_YEAR) as i64;
    let cur_year = year_of(sim.tick);
    let good_name = |g: usize| sim.goods.get(g).map(|x| x.name.clone()).unwrap_or_default();
    let hub_of = |hi: u32| sim.hubs.get(hi as usize);

    // Private houses (not civic guilds) seated in a city of this culture.
    let mut cand: Vec<&crate::sim::tick::House> = sim.houses.iter().filter(|hh| {
        !hh.is_guild && sim.hub_culture.get(hh.hub as usize).map(|c| c == &name).unwrap_or(false)
    }).collect();
    // Magnates first: political power + prestige + (log) wealth.
    cand.sort_by(|a, b| {
        let s = |h: &crate::sim::tick::House| h.political_power + h.prestige * 0.5 + (h.wealth.max(0.0)).ln_1p() * 0.1;
        s(b).partial_cmp(&s(a)).unwrap_or(std::cmp::Ordering::Equal)
    });
    let richest = cand.iter().map(|h| h.wealth).fold(0.0f32, f32::max);

    let mut out = Vec::new();
    for hh in cand.into_iter().take(8) {
        let seat = hub_of(hh.hub);
        let city = seat.map(|s| s.name.clone()).unwrap_or_default();
        let head = if hh.head_name.is_empty() { hh.name.clone() } else { hh.head_name.clone() };
        let founded = year_of(hh.founded_tick);
        let era = if hh.defunct {
            format!("of old — the house rose about year {founded}, now passed into memory")
        } else {
            format!("flourishing by year {cur_year} — {} generation(s) of the line", hh.generation.max(1))
        };
        // Deeds — trade, banking, politics, dominion.
        let mut deeds: Vec<String> = Vec::new();
        if richest > 0.0 && hh.wealth >= richest * 0.999 {
            deeds.push(format!("the richest magnate of the {name}"));
        }
        deeds.push(match hh.archetype {
            1 => "master of a great merchant fleet".to_string(),
            2 => "a banking magnate whose credit financed whole cities".to_string(),
            3 => format!("a power behind the council of {city}"),
            _ => "head of a great trading dynasty".to_string(),
        });
        let monos: Vec<String> = hh.mono_ever.iter().take(3).map(|&g| good_name(g)).filter(|s| !s.is_empty()).collect();
        if !monos.is_empty() { deeds.push(format!("held the {} trade in his grip", monos.join(", "))); }
        let chs: Vec<String> = hh.charters.iter().take(2).map(|&g| good_name(g)).filter(|s| !s.is_empty()).collect();
        if !chs.is_empty() { deeds.push(format!("won a civic charter on {}", chs.join(", "))); }
        if hh.dominant_seat && !city.is_empty() { deeds.push(format!("ruled the commerce of {city}")); }
        let known_for = deeds.join("; ");

        // Cities: seat (home) + offices the house reached (its agents' migration/expansion).
        let mut cities: Vec<NotableCity> = Vec::new();
        if let Some(s) = seat { cities.push(NotableCity { name: s.name.clone(), x: s.x, y: s.y, role: "seat".into() }); }
        let mut office_hubs: Vec<u32> = hh.offices.clone();
        for &(o, _) in &hh.office_leases { if !office_hubs.contains(&o) { office_hubs.push(o); } }
        for &o in office_hubs.iter().take(6) {
            if o == hh.hub { continue; }
            if let Some(s) = hub_of(o) {
                cities.push(NotableCity { name: s.name.clone(), x: s.x, y: s.y, role: "office".into() });
            }
        }
        out.push(NotablePerson {
            name: head, house: hh.name.clone(), era, known_for, city,
            wealth: hh.wealth.max(0.0) as f64, alive: !hh.defunct, cities,
        });
    }
    Ok(out)
}


/// The 6-monthly population series for one people (for the Peoples-panel line chart).
/// Returns `[year, population]` points in time order.
#[tauri::command]
pub fn campaign_get_culture_history(name: String, db: State<'_, WorldDb>) -> Result<Vec<[f32; 2]>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    Ok(sim.culture_history.iter().map(|s| {
        let p = s.pops.iter().find(|(c, _)| c == &name).map(|(_, p)| *p).unwrap_or(0.0);
        [s.t, p]
    }).collect())
}


/// Per-hub share of ONE culture: `[x, y, share]` for every settlement where the
/// people is present (≥ 5%). Drives the map's culture-share overlay.
#[tauri::command]
pub fn campaign_culture_hubs(name: String, db: State<'_, WorldDb>) -> Result<Vec<[f32; 3]>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out = vec![];
    for i in 0..sim.hubs.len() {
        let h = &sim.hubs[i];
        if h.is_estate || h.abandoned || h.population < 1.0 { continue; }
        let share = sim.culture_share_at(i, &name);
        if share > 0.05 { out.push([h.x, h.y, share]); }
    }
    Ok(out)
}


/// DLC 4 · the derived Pops of one hub (read-only foundation of the Nations & POPs
/// layer). Empty when no campaign is active or the hub index is unknown.
#[tauri::command]
pub fn campaign_get_pops(db: State<'_, WorldDb>, hub: u32) -> Result<Vec<PopBrief>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let h = hub as usize;
    if h >= sim.hubs.len() { return Ok(vec![]); }
    Ok(sim.hubs[h].pops.iter().map(|p| PopBrief {
        profession: crate::sim::tick::POP_PROFESSIONS
            .get(p.profession as usize).copied().unwrap_or("?").to_string(),
        size: p.size,
        money: p.money,
        needs_life: p.needs_life,
        needs_everyday: p.needs_everyday,
        needs_luxury: p.needs_luxury,
        consciousness: p.consciousness,
        militancy: p.militancy,
    }).collect())
}


/// Phase 6 · the Plagues & Epidemics panel: outbreaks grouped from the strike log,
/// active first then deadliest. The panel re-sorts client-side.
#[tauri::command]
pub fn campaign_get_epidemics(db: State<'_, WorldDb>) -> Result<Vec<EpidemicBrief>, String> {
    use crate::sim::tick::TICKS_PER_YEAR;
    use std::collections::BTreeMap;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let name = |h: u32| sim.hubs.get(h as usize).map(|x| x.name.clone()).unwrap_or_default();
    let mut groups: BTreeMap<u32, Vec<&crate::sim::tick::PlagueStrike>> = BTreeMap::new();
    for s in &sim.epidemics { groups.entry(s.outbreak).or_default().push(s); }
    let mut out: Vec<EpidemicBrief> = groups.into_iter().map(|(id, mut strikes)| {
        let start = strikes.iter().map(|s| s.start_tick).min().unwrap_or(0);
        let end = strikes.iter().map(|s| s.until_tick).max().unwrap_or(0);
        let active = strikes.iter().any(|s| s.until_tick > sim.tick);
        let total_dead: f32 = strikes.iter().map(|s| s.deaths).sum();
        // SIR totals. Legacy strikes stored no `infected` (0) → fall back to deaths so
        // the ill count is never below the death toll.
        let ill_of = |s: &crate::sim::tick::PlagueStrike| s.infected.max(s.deaths);
        let total_ill: f32 = strikes.iter().map(|s| ill_of(s)).sum();
        let total_recovered: f32 = (total_ill - total_dead).max(0.0);
        // The outbreak's category = the (most severe) category recorded on its strikes
        // (legacy strikes stored 0 → treat as local cat-3).
        let category = strikes.iter().map(|s| if s.category == 0 { 3 } else { s.category })
            .min().unwrap_or(3);
        // Origin = the spontaneous strike (source < 0), else the earliest.
        let origin_hub = strikes.iter().find(|s| s.source < 0).map(|s| s.hub)
            .unwrap_or_else(|| strikes.iter().min_by_key(|s| s.start_tick).map(|s| s.hub).unwrap_or(0));
        // Chronological → the spread history (origin first, then each city as reached).
        strikes.sort_by_key(|s| s.start_tick);
        let cities: Vec<PlagueCityBrief> = strikes.iter().enumerate().map(|(i, s)| PlagueCityBrief {
            hub: s.hub,
            x: sim.hubs.get(s.hub as usize).map(|h| h.x).unwrap_or(0.0),
            y: sim.hubs.get(s.hub as usize).map(|h| h.y).unwrap_or(0.0),
            name: name(s.hub),
            deaths: s.deaths.round() as u32,
            ill: ill_of(s).round() as u32,
            recovered: (ill_of(s) - s.deaths).max(0.0).round() as u32,
            pop: s.pop_at.round() as u32,
            active: s.until_tick > sim.tick,
            from_name: if s.source >= 0 { name(s.source as u32) } else { String::new() },
            year: s.start_tick / TICKS_PER_YEAR,
            order: i as u32,
        }).collect();
        // The named disease = the disease of the spontaneous (origin) strike.
        let dz = strikes.iter().find(|s| s.source < 0).map(|s| s.disease)
            .unwrap_or_else(|| strikes.first().map(|s| s.disease).unwrap_or(0));
        let dspec = crate::sim::tick::DISEASES.get(dz as usize);
        let disease = dspec.map(|s| s.name.to_string()).unwrap_or_else(|| "Pestilence".into());
        let transmission = match dspec.map(|s| s.mode).unwrap_or(0) {
            1 => "water-borne", 2 => "airborne", 3 => "vector · locale", _ => "trade-borne",
        }.to_string();
        EpidemicBrief {
            id,
            name: format!("{} of {}", disease, name(origin_hub)),
            origin_name: name(origin_hub),
            start_year: start / TICKS_PER_YEAR,
            end_year: end / TICKS_PER_YEAR,
            active,
            total_dead: total_dead.round() as u32,
            total_ill: total_ill.round() as u32,
            total_recovered: total_recovered.round() as u32,
            category,
            disease,
            transmission,
            cities,
        }
    }).collect();
    out.sort_by(|a, b| b.active.cmp(&a.active)
        .then(b.total_dead.cmp(&a.total_dead))
        .then(b.end_year.cmp(&a.end_year)));
    Ok(out)
}


/// Phase 6 · the Notable Figures panel: the campaign's great lives, living first.
#[tauri::command]
pub fn campaign_get_figures(db: State<'_, WorldDb>) -> Result<Vec<FigureBrief>, String> {
    use crate::sim::tick::{TICKS_PER_YEAR, FIGURE_KINDS};
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(vec![]) };
    let mut out: Vec<FigureBrief> = sim.figures.iter().map(|f| {
        let h = sim.hubs.get(f.hub as usize);
        FigureBrief {
            name: f.name.clone(),
            role: FIGURE_KINDS.get(f.kind as usize).copied().unwrap_or("Figure").to_string(),
            hub: f.hub,
            x: h.map(|x| x.x).unwrap_or(0.0),
            y: h.map(|x| x.y).unwrap_or(0.0),
            city: h.map(|x| x.name.clone()).unwrap_or_default(),
            good_name: if f.good >= 0 {
                sim.goods.get(f.good as usize).map(|g| g.name.clone()).unwrap_or_default()
            } else { String::new() },
            born_year: f.born_tick / TICKS_PER_YEAR,
            died_year: if f.dead { f.dies_tick / TICKS_PER_YEAR } else { 0 },
            alive: !f.dead,
        }
    }).collect();
    out.sort_by(|a, b| b.alive.cmp(&a.alive).then(b.born_year.cmp(&a.born_year)));
    Ok(out)
}


/// Phase 7 · marriage alliances (from `sim.alliances`) + feuds (from `House.rivals`)
/// between living houses, with their seat cities for the map.
#[tauri::command]
pub fn campaign_get_dynasties(db: State<'_, WorldDb>) -> Result<DynastiesPayload, String> {
    use std::collections::HashSet;
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? {
        Some(s) => s, None => return Ok(DynastiesPayload { alliances: vec![], feuds: vec![] }),
    };
    let houses = &sim.houses;
    let link = |a: usize, b: usize| -> Option<HouseLink> {
        let (ha, hb) = (houses.get(a)?, houses.get(b)?);
        if ha.defunct || hb.defunct { return None; }
        let (ca, cb) = (sim.hubs.get(ha.hub as usize)?, sim.hubs.get(hb.hub as usize)?);
        Some(HouseLink {
            a_name: ha.name.clone(), b_name: hb.name.clone(),
            a_hub: ha.hub, b_hub: hb.hub,
            ax: ca.x, ay: ca.y, bx: cb.x, by: cb.y,
            a_city: ca.name.clone(), b_city: cb.name.clone(),
        })
    };
    let alliances: Vec<HouseLink> = sim.alliances.iter()
        .filter_map(|&(a, b)| link(a as usize, b as usize)).collect();
    // Feuds: unique (min,max) pairs from rivals lists among living houses.
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut feuds: Vec<HouseLink> = Vec::new();
    for (i, h) in houses.iter().enumerate() {
        if h.defunct { continue; }
        for &r in &h.rivals {
            let (lo, hi) = if i < r { (i, r) } else { (r, i) };
            if lo == hi || !seen.insert((lo, hi)) { continue; }
            if let Some(l) = link(lo, hi) { feuds.push(l); }
        }
    }
    Ok(DynastiesPayload { alliances, feuds })
}


// ═══════════════════════════════════════════════════════════════════════════════
//  SETTLEMENT PEOPLES & POWERS — per-settlement minority-power reading.
//  A pure DERIVED read: population share (hub_minorities/hub_culture), civic
//  influence and market/production control (house influence + trade + estates +
//  bailos), blended into a POWER score, with a FONDACO flag when a foreign trading
//  community keeps a bailo here. Adds no sim state, so it cannot touch the economy.
// ═══════════════════════════════════════════════════════════════════════════════

/// One people/community's standing in a single settlement.
#[derive(serde::Serialize)]
pub struct PeopleGroup {
    pub culture: String,
    pub is_majority: bool,
    pub pop_share: f32,      // 0..1 of the settlement's population
    pub civic: f32,          // 0..1 share of civic influence here
    pub market: f32,         // 0..1 share of trade + production here
    pub power: f32,          // blended 0..1 (pop · civic · market, +fondaco)
    pub fondaco: bool,       // a foreign-culture house keeps a bailo (fondaco) here
    pub council_seat: bool,  // this people holds the council / captor seat
    pub houses: u32,         // houses of this culture operating here
    pub works: u32,          // estates/manufactories at this hub they own
    pub traits: Vec<CultureTraitBrief>,
    pub note: String,        // one-line flavour of why they matter
}

/// A settlement's peoples, majority first then minorities/communities by power.
#[derive(serde::Serialize)]
pub struct SettlementPeoples {
    pub hub: u32,
    pub name: String,
    pub population: f32,
    pub majority_culture: String,
    pub groups: Vec<PeopleGroup>,
}

#[tauri::command]
pub fn campaign_settlement_peoples(hub: u32, db: State<'_, WorldDb>) -> Result<Option<SettlementPeoples>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let sim = match get_sim(&db, &conn)? { Some(s) => s, None => return Ok(None) };
    let h = hub as usize;
    if h >= sim.hubs.len() || sim.hubs[h].is_estate { return Ok(None); }
    let maj = sim.hub_culture.get(h).cloned().unwrap_or_default();
    if maj.is_empty() || maj == "—" { return Ok(None); }
    use std::collections::HashMap;

    #[derive(Default)]
    struct Acc { pop: f32, civic: f32, trade: f32, prod: f32, houses: u32, works: u32, fondaco: bool, seat: bool }
    let mut acc: HashMap<String, Acc> = HashMap::new();
    macro_rules! ent { ($c:expr) => { acc.entry($c).or_default() } }

    // ── population shares (majority = 1 − Σ minorities) ──
    let mins = sim.hub_minorities.get(h).cloned().unwrap_or_default();
    let mshare: f32 = mins.iter().map(|(_, s)| *s).sum();
    ent!(maj.clone()).pop += (1.0 - mshare).clamp(0.0, 1.0);
    for (c, s) in &mins {
        if c.is_empty() || c == "—" { continue; }
        ent!(c.clone()).pop += *s;
    }

    // ── who holds the seat (captor outranks a merely-dominant council) ──
    let seat_house = if sim.hubs[h].captor_house >= 0 { sim.hubs[h].captor_house } else { sim.hubs[h].council_house };

    let culture_of = |hi: usize| -> String {
        let home = sim.houses[hi].hub as usize;
        sim.hub_culture.get(home).cloned().unwrap_or_default()
    };

    // ── house presence: civic influence, market trade, bailo/fondaco, seat ──
    for hi in 0..sim.houses.len() {
        let hh = &sim.houses[hi];
        if hh.defunct { continue; }
        let c = culture_of(hi);
        if c.is_empty() || c == "—" { continue; }
        let inf = hh.influence.iter().find(|(hb, _)| *hb as usize == h).map(|(_, v)| *v).unwrap_or(0.0);
        let trd = hh.trade_at.iter().find(|(hb, _)| *hb as usize == h).map(|(_, v)| *v).unwrap_or(0.0);
        let bailo = hh.bailos.iter().any(|&c2| c2 as usize == h);
        if inf <= 0.0 && trd <= 0.0 && !bailo && hh.hub as usize != h { continue; }
        let a = ent!(c.clone());
        a.civic += inf;
        a.trade += trd;
        a.houses += 1;
        if bailo && c != maj { a.fondaco = true; }
        if seat_house == hi as i32 { a.seat = true; }
    }

    // ── production control: estates/manufactories here, by owner's culture ──
    for e in &sim.hubs {
        if !e.is_estate || e.parent != h as i32 || e.owner_house < 0 { continue; }
        let oc = culture_of(e.owner_house as usize);
        if oc.is_empty() || oc == "—" { continue; }
        let a = ent!(oc);
        a.works += 1;
        a.prod += e.production.iter().copied().fold(0.0f32, |s, v| s + v.max(0.0));
    }

    // ── normalise each axis to a within-city share ──
    let tot_civic: f32 = acc.values().map(|a| a.civic).sum();
    let tot_trade: f32 = acc.values().map(|a| a.trade).sum();
    let tot_prod: f32  = acc.values().map(|a| a.prod).sum();
    let has_prod = tot_prod > 1e-6;

    let trait_briefs = |ids: &[usize]| -> Vec<CultureTraitBrief> {
        ids.iter().filter_map(|&i| crate::sim::cultures::TRAITS.get(i)).map(|t| CultureTraitBrief {
            name: t.name.to_string(), emoji: t.emoji.to_string(), blurb: t.blurb.to_string(),
        }).collect()
    };

    let mut groups: Vec<PeopleGroup> = acc.into_iter().map(|(culture, a)| {
        let civic_share = if tot_civic > 1e-6 { a.civic / tot_civic } else { 0.0 };
        // A seat-holder is civically dominant by definition — floor its bar.
        let civic = if a.seat { civic_share.max(0.5) } else { civic_share };
        let trade_share = if tot_trade > 1e-6 { a.trade / tot_trade } else { 0.0 };
        let prod_share  = if has_prod { a.prod / tot_prod } else { 0.0 };
        let market = if has_prod { 0.6 * trade_share + 0.4 * prod_share } else { trade_share };
        // Blend: the user's three axes, weighted toward commercial/production power,
        // with a modest fondaco premium (a trading community punches above its numbers).
        let mut power = 0.30 * a.pop + 0.30 * civic + 0.40 * market;
        if a.fondaco { power = (power * 1.15).min(1.0); }
        let ids = sim.culture_trait_ids(&culture);
        // One-line note: strongest signal first.
        let mut note = String::new();
        if a.fondaco { note = "keeps a fondaco — a foreign trading quarter".into(); }
        else if a.seat { note = "holds the city's seat of council".into(); }
        else if a.works > 0 && ids.contains(&12) { note = format!("master craftsmen — {} works", a.works); }
        else if a.works > 0 { note = format!("owns {} works here", a.works); }
        else if a.houses > 0 { note = format!("{} merchant hous{} operating here", a.houses, if a.houses == 1 { "e" } else { "es" }); }
        PeopleGroup {
            is_majority: culture == maj,
            culture, pop_share: a.pop, civic, market, power,
            fondaco: a.fondaco, council_seat: a.seat, houses: a.houses, works: a.works,
            traits: trait_briefs(&ids), note,
        }
    }).collect();

    // Drop negligible communities (no people AND no commercial footing), keep the
    // majority and any real fondaco/house presence.
    groups.retain(|g| g.is_majority || g.pop_share > 0.005 || g.houses > 0 || g.works > 0 || g.fondaco);
    // Majority first, then minorities/communities by power.
    groups.sort_by(|a, b| b.is_majority.cmp(&a.is_majority)
        .then(b.power.partial_cmp(&a.power).unwrap_or(std::cmp::Ordering::Equal)));
    groups.truncate(10);

    Ok(Some(SettlementPeoples {
        hub, name: sim.hubs[h].name.clone(), population: sim.hubs[h].population.max(0.0),
        majority_culture: maj, groups,
    }))
}
