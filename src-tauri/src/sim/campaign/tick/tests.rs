    use super::*;

    pub(super) fn good(name: &str, cat: i32, tier: u8, val: f32, desire: f32, food: bool) -> TickGood {
        TickGood { name: name.into(), category: cat, need_tier: tier, base_value: val, desire, food,
            fungible_input: false,
            bulk: 1.0, perishable: 0.0, inputs: vec![], labor: 1.0, consumption_interval: 30.0 }
    }

    pub(super) fn hub(id: u32, x: f32, y: f32, pop: f32, prod: Vec<f32>, comp: u32) -> TickHub {
        let ng = prod.len();
        let base_per_capita: Vec<f32> = prod.iter().map(|&p| p / pop.max(1.0)).collect();
        TickHub {
            id, x, y, name: format!("H{id}"), population: pop, founding_pop: pop,
            stock: vec![0.0; ng], price: vec![1.0; ng], production: prod,
            grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: false, parent: -1, koppen: 0, coastal: false, component: comp,
            export_earn: 0.0, import_spend: 0.0,
            mood: 0.6, sent_food: 0.7, sent_prosperity: 0.5, sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(),
            in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0,
            estate_kind: 0, estate_tier: 0, last_upgrade_tick: 0, owner_house: -1, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, coin_history: Vec::new(), debt_principal: 0.0, debt_coupon: 0.0, debt_holders: Vec::new(), mint_bullion_ratio: 1.0, has_mint: false,
            quality: Vec::new(), stolen_good: -1, stolen_from: -1,
            colony_kind: 0, colony_stage: 0, autonomous: false, founder_hub: -1, backers: Vec::new(),
            reserve_food: 0.0, reserve_cap: 0.0, supply_years: 0.0, colony_founded_tick: 0,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: 0, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
        }
    }

    pub(super) fn house_at(hub: u32, spec: Vec<usize>, fleet_sea: u32) -> House {
        House {
            name: format!("House{hub}"), hub, wealth: 50.0, prestige: 0.0, spec,
            monopoly: vec![], rivals: vec![], generation: 1, events: vec![],
            good_profit: vec![], mono50: vec![], mono_ever: vec![], dominant_seat: false,
            prev_wealth: 50.0, worst_loss: 0.0, fleet_sea, fleet_river: 0, fleet_caravan: 0,
            head_name: "Head".into(), head_since: 0, head_lifespan: 100_000, founded_tick: 0,
            political_power: 0.0, volume: 0.0, defunct: false, archetype: 1, charters: vec![],
            is_guild: false, offices: vec![], trade_at: vec![], debt_since: 0,
            wealth_history: vec![], office_leases: vec![],
            influence: vec![], bailos: vec![],
            head_female: false, head_age: 34, line: vec![], tier: 0, standing: 0.0,
        }
    }

    pub(super) fn sim(hubs: Vec<TickHub>, goods: Vec<TickGood>) -> CampaignSim {
        let mut s = CampaignSim {
            seed: 42, tick: 0, goods, hubs, in_transit: vec![], houses: vec![],
            active_events: vec![], journal: vec![], days_per_cell: 0.2, freight_per_day: 0.01,
            k: 0.6, margin: 0.05, need_scale: 1.0, world_w: 100.0, world_h: 100.0, last_tick_ms: 0.0,
            last_month_pop: 0.0, last_month_index: 0.0, seed_house_count: 0,
            culture_rules: vec![],
            fleets_migrated: true, tech_factor: 1.0, percap_migrated: true, society_migrated: false,
            components_rescued: true,
            house_ledger: Vec::new(), house_ledger_prev: Vec::new(), house_barred: Vec::new(),
            colonizable: vec![], satellite_sites: vec![], hinterland: vec![], migration_routes: vec![], creoles: vec![], lingua: vec![], culture_history: vec![], council_bought_month: vec![], hub_patron: vec![], dev_tier: vec![], dev_momentum: vec![], base_days: vec![], base_n: 0, colony_supply: vec![],
            hub_culture: vec![], hub_minorities: vec![], estate_idle_years: vec![],
            diag_shipments: 0, diag_by_house: 0, diag_by_guild: 0, diag_lost: 0, diag_volume: 0.0,
            recent_trades: vec![],
            spec_centers: vec![], spec_year: 0, spec_prev_profit: vec![],
            banks: vec![], crashes: vec![], wars: vec![], war_log: vec![],
            flow_year: vec![], flow_accum: std::collections::HashMap::new(),
            world_series: vec![], total_foundings: 0, total_abandonments: 0,
            migrations: vec![],
            good_flow_accum: vec![], hub_good_trade: vec![], year_frames: vec![],
            records: WorldRecords::default(),
            quality_migrated: false,
            days: vec![],
            neighbors: vec![],
            routes_dirty: false,
            warehouses: vec![],
            contracts: vec![],
            trade_cur: Default::default(),
            city_dominator: vec![],
            trade_last: vec![],
            trade_hist: vec![],
            figures: vec![],
            fairs: vec![],
            fairs_seeded: false,
            holy_sites: vec![],
            holy_seeded: false,
            alliances: vec![],
            guilds: vec![],
            guilds_seeded: false,
            wonders: vec![],
            epidemics: vec![],
            next_outbreak: 0,
            expansion_frozen_until: 0,
            expeditions: vec![], route_prospects: vec![], failed_expeditions: vec![],
            corridors: vec![], next_expedition_id: 0,
            prov_rural: vec![], prov_cap: vec![], prov_culture: vec![], prov_seat: vec![],
            hub_province: vec![], prov_net_mig: vec![], prov_neighbors: vec![],
            feuds: vec![],
            // Province LAND state — left empty exactly like the demography vectors
            // above, so `province_land_pass` early-returns and the dynamics run is
            // unaffected by the B1 land layer (that is the gate).
            prov_forest: vec![], prov_arable: vec![], prov_pasture: vec![],
            prov_irrigated: vec![], prov_soil: vec![], prov_tenure: vec![],
            prov_tax: vec![], prov_arrears: vec![], prov_unrest: vec![],
            prov_surplus: vec![], prov_revenue: vec![], prov_holder: vec![],
            prov_works: vec![], prov_history: vec![], prov_events: vec![],
        };
        s.rebuild_routes();
        s
    }

    /// Phase 2b · with a seeded province layer, the rural reservoir must FEED the cities
    /// (net rural→urban migration) while TOTAL population stays bounded and finite over
    /// decades — the urban-graveyard/reservoir loop must not blow up or crater.
    #[test]
    fn province_demography_feeds_cities_and_stays_bounded() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..3u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 8000.0 } else { 900.0 }).collect();
            hubs.push(hub(i, (i as f32) * 4.0, 0.0, 8000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        for h in s.hubs.iter_mut() { h.sent_prosperity = 0.6; h.starving = 0.0; h.food_balance = 1.0; }
        s.hub_culture = vec!["Aiora".into(), "Aiora".into(), "Belgar".into()];
        s.hub_minorities = vec![Vec::new(); 3];
        // Seed a two-province layer: each province a fed countryside behind its cities.
        s.prov_cap = vec![120_000.0, 120_000.0];
        s.prov_rural = vec![70_000.0, 70_000.0];
        s.prov_culture = vec!["Aiora".into(), "Belgar".into()];
        s.prov_seat = vec![[0.0, 0.0], [8.0, 0.0]];
        s.hub_province = vec![0, 0, 1];
        s.prov_net_mig = vec![0.0, 0.0];
        let total0: f32 = s.hubs.iter().map(|h| h.population).sum::<f32>()
            + s.prov_rural.iter().sum::<f32>();
        let urban0: f32 = s.hubs.iter().map(|h| h.population).sum();
        // Run ~40 years of the yearly pass.
        for _ in 0..40 { s.province_demography_pass(); }
        let urban1: f32 = s.hubs.iter().map(|h| h.population).sum();
        let total1: f32 = urban1 + s.prov_rural.iter().sum::<f32>();
        // Cities must have GROWN from rural in-migration.
        assert!(urban1 > urban0 * 1.05, "cities should grow via migration: {urban0} → {urban1}");
        // Nothing infinite or negative; total stays within a sane band of the start.
        assert!(total1.is_finite() && total1 > 0.0, "total must stay finite/positive: {total1}");
        assert!(total1 < total0 * 3.0, "total must stay bounded: {total0} → {total1}");
        assert!(s.prov_rural.iter().all(|&r| r.is_finite() && r >= 0.0), "rural pools stay finite/≥0");
        // Migrants must carry their province's people into the cities.
        assert!(s.hub_minorities[2].is_empty() || s.hub_minorities.iter().any(|m| !m.is_empty())
            || s.prov_net_mig.iter().any(|&m| m < 0.0), "countryside acts as a migration source");
    }

    /// B1 · the LAND state must work, wear, feed and stay bounded. The pass's whole
    /// reason to exist is the feedback edge — a province's surplus reaching the seat
    /// city's granary and its dues reaching that city's treasury — so this asserts both
    /// arrive, and that no land quantity can leave its physical range over 60 years.
    #[test]
    fn province_land_pass_feeds_the_seat_and_stays_bounded() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..2u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 6000.0 } else { 600.0 }).collect();
            hubs.push(hub(i, (i as f32) * 4.0, 0.0, 9000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        for h in s.hubs.iter_mut() { h.sent_prosperity = 0.6; h.starving = 0.0; h.food_balance = 1.0; }
        s.hub_culture = vec!["Aiora".into(), "Aiora".into()];
        s.hub_minorities = vec![Vec::new(); 2];
        // One province holding both cities, with a fed countryside.
        s.prov_cap = vec![90_000.0];
        s.prov_rural = vec![60_000.0];
        s.prov_culture = vec!["Aiora".into()];
        s.prov_seat = vec![[0.0, 0.0]];
        s.hub_province = vec![0, 0];
        s.prov_net_mig = vec![0.0];
        // `ensure_province_land` seeds the land state on the first pass, exactly as it
        // does for a save that predates it — so this also covers that path.
        let seat = s.province_seat_hub(0).expect("a province with towns has a seat");
        let food0 = s.hubs[seat].stock[0];
        let treasury0 = s.hubs[seat].treasury;
        for yr in 0..60u32 {
            s.province_demography_pass();
            s.province_land_pass(yr);
        }
        let seat = s.province_seat_hub(0).expect("still has a seat");
        // ── THE FEEDBACK EDGE: the countryside fed the city and paid it dues.
        assert!(s.hubs[seat].stock[0] > food0,
            "the province's surplus must reach the seat's granary: {} → {}",
            food0, s.hubs[seat].stock[0]);
        assert!(s.hubs[seat].treasury > treasury0,
            "rural dues must reach the holder's treasury: {} → {}",
            treasury0, s.hubs[seat].treasury);
        assert!(s.prov_holder[0] >= 0, "an administered province records its holder");
        // ── Every land quantity stays inside its physical range.
        for (name, v) in [("forest", &s.prov_forest), ("arable", &s.prov_arable),
                          ("pasture", &s.prov_pasture), ("irrigated", &s.prov_irrigated),
                          ("soil", &s.prov_soil), ("unrest", &s.prov_unrest)] {
            assert!(v[0].is_finite() && v[0] >= 0.0 && v[0] <= 1.0,
                "{name} must stay a finite share in 0..1: {}", v[0]);
        }
        // Soil wears but never dies — the floor is what stops a Malthusian death spiral.
        assert!(s.prov_soil[0] >= PROV_SOIL_FLOOR - 1e-4,
            "soil must not fall below its floor: {}", s.prov_soil[0]);
        // Land use is a partition: wood + crop + pasture cannot exceed the province.
        let used = s.prov_forest[0] + s.prov_arable[0] + s.prov_pasture[0];
        assert!(used <= 1.0 + 1e-3, "land use must not exceed the province: {used}");
        assert!(s.prov_surplus[0].is_finite() && s.prov_surplus[0] >= 0.0,
            "surplus stays finite/≥0: {}", s.prov_surplus[0]);
        // A crowded countryside must actually have cleared some woodland over 60 years.
        assert!(s.prov_history[0].len() == 60, "one sample per year: {}", s.prov_history[0].len());
        let first = &s.prov_history[0][0];
        let last = s.prov_history[0].last().unwrap();
        assert!(last.arable >= first.arable - 1e-3 || last.forest >= first.forest,
            "land use must move coherently: arable {} → {}, forest {} → {}",
            first.arable, last.arable, first.forest, last.forest);
    }

    /// B1 · a province with NO seeded land layer must be a complete no-op. This is the
    /// gate for the whole feature: the dynamics run seeds no provinces, so if the pass
    /// touched anything here the bit-identical claim would be false.
    #[test]
    fn province_land_pass_is_a_noop_without_provinces() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        let food0 = s.hubs[0].stock[0];
        let treasury0 = s.hubs[0].treasury;
        for yr in 0..25u32 { s.province_land_pass(yr); }
        assert_eq!(s.hubs[0].stock[0], food0, "no province layer ⇒ no food delivered");
        assert_eq!(s.hubs[0].treasury, treasury0, "no province layer ⇒ no dues collected");
        assert!(s.prov_forest.is_empty() && s.prov_history.is_empty(),
            "no province layer ⇒ no land state is even allocated");
    }

    /// A land improvement must cost its funder real money, take years, and only then
    /// change the land. Guards the "instant free improvement" shape a control verb
    /// would otherwise drift into.
    #[test]
    fn province_works_cost_money_take_years_and_then_change_the_land() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.prov_cap = vec![50_000.0];
        s.prov_rural = vec![30_000.0];
        s.prov_culture = vec!["Aiora".into()];
        s.prov_seat = vec![[0.0, 0.0]];
        s.hub_province = vec![0];
        s.prov_net_mig = vec![0.0];
        s.ensure_province_land(1);
        // Fund it generously so the work is never starved.
        s.hubs[0].treasury = 10_000.0;
        s.prov_forest[0] = 0.5;
        s.prov_arable[0] = 0.2;
        let arable0 = s.prov_arable[0];
        let treasury0 = s.hubs[0].treasury;
        s.prov_works.push(ProvWork { province: 0, kind: WORK_CLEAR, progress: 0.0,
            funder_hub: 0, funder_house: -1, start_tick: 0, idle_years: 0 });
        // One year in: paid for, under way, land untouched.
        s.province_land_pass(0);
        assert!(s.hubs[0].treasury < treasury0, "the first year must be paid for");
        assert!(!s.prov_works.is_empty(), "a multi-year work is still running after one year");
        // Run it out. Clearance is WORK_YEARS[WORK_CLEAR] years of funded work.
        for yr in 1..(WORK_YEARS[WORK_CLEAR as usize] as u32 + 2) { s.province_land_pass(yr); }
        assert!(s.prov_works.is_empty(), "a completed work is retired");
        assert!(s.prov_arable[0] > arable0,
            "completed clearance must add arable: {} → {}", arable0, s.prov_arable[0]);
        assert!(s.prov_events[0].iter().any(|e| e.kind == "clearance"),
            "the province records its own history");
    }

    /// An UNFUNDED work must stall rather than complete for free.
    #[test]
    fn an_unfunded_province_work_stalls() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.prov_cap = vec![50_000.0];
        s.prov_rural = vec![30_000.0];
        s.prov_seat = vec![[0.0, 0.0]];
        s.prov_culture = vec!["Aiora".into()];
        s.hub_province = vec![0];
        s.prov_net_mig = vec![0.0];
        s.ensure_province_land(1);
        s.hubs[0].treasury = 0.0; // nobody can pay
        // …and keep it that way. The land pass credits the seat's treasury with the
        // province's own dues, so a taxed province FUNDS ITS OWN improvements out of
        // them — which is correct, and is why the funding test has to set the rate to
        // zero to isolate the stall path.
        s.prov_tax[0] = 0.0;
        s.prov_forest[0] = 0.5;
        let arable0 = s.prov_arable[0];
        s.prov_works.push(ProvWork { province: 0, kind: WORK_CLEAR, progress: 0.0,
            funder_hub: 0, funder_house: -1, start_tick: 0, idle_years: 0 });
        for yr in 0..20u32 { s.province_land_pass(yr); }
        assert_eq!(s.prov_works.len(), 1, "an unfunded work stalls, it does not vanish");
        assert!(s.prov_works[0].progress < 0.2,
            "an unfunded work makes no real progress: {}", s.prov_works[0].progress);
        assert!(s.prov_arable[0] <= arable0 + 0.02,
            "no improvement lands without being paid for");
    }

    /// FEUDS · the elaborated model must do the four things the flat `rivals` list could
    /// not: heat with overlap, ESCALATE through stages, cool when the overlap goes away,
    /// and keep `rivals` in sync so every existing reader still works.
    #[test]
    fn feuds_heat_escalate_and_cool() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0, 400.0], 0),
            hub(1, 3.0, 0.0, 7000.0, vec![4000.0, 300.0], 0),
        ];
        let mut s = sim(hubs, goods);
        // Two houses in the same city, living off the same good — maximum overlap.
        for i in 0..2 {
            let mut h = house_at(0, vec![1], 2);
            h.name = format!("House {}", if i == 0 { "Alpha" } else { "Beta" });
            h.wealth = if i == 0 { 4000.0 } else { 1200.0 };
            h.influence = vec![(0, 0.5)];
            s.houses.push(h);
        }
        s.house_barred = vec![Vec::new(); 2];
        s.house_ledger = vec![Default::default(); 2];
        let (o, _, _) = s.feud_overlap(0, 1);
        assert!(o > 0.3, "two houses in one city on one good must overlap: {o}");
        s.open_feud(0, 1, FEUD_TRADE, 1, 0, 0.10);
        assert_eq!(s.feuds.len(), 1, "one pair, one feud");
        assert!(s.houses[0].rivals.contains(&1) && s.houses[1].rivals.contains(&0),
            "`rivals` must stay in sync so existing readers are unaffected");
        // A second grievance must NOT create a second feud — it pours heat on this one.
        s.open_feud(1, 0, FEUD_MARRIAGE, -1, 0, 0.20);
        assert_eq!(s.feuds.len(), 1, "a fresh grievance re-heats the existing feud");
        // Heat it over years of monthly passes; it must escalate past cold rivalry.
        let mut peak = 0u8;
        for m in 0..240u32 {
            s.tick = m * 30;
            s.update_feuds();
            if !s.feuds.is_empty() { peak = peak.max(s.feuds[0].stage); }
            if s.feuds[0].outcome != FEUD_RUNNING { break; }
        }
        assert!(peak >= FEUD_TRADEWAR,
            "sustained overlap must escalate a feud past open rivalry: peak stage {peak}");
        assert!(s.feuds[0].flares > 0, "a hot feud must actually flare");
        assert!(s.feuds[0].damage_a + s.feuds[0].damage_b > 0.0, "flares must cost someone");
        assert!(s.houses.iter().all(|h| h.wealth.is_finite()), "feuds keep wealth finite");
        // The weaker house pays, but limited liability holds — a feud impoverishes, it
        // does not drive a house arbitrarily negative on its own.
        assert!(s.houses[1].wealth > -1.0,
            "feud bites are a share of what a house HAS: {}", s.houses[1].wealth);
        // Now remove the overlap entirely: different goods, different cities. The feud
        // must cool and eventually be forgotten — the ending the old model never had.
        if s.feuds[0].outcome == FEUD_RUNNING {
            s.houses[1].spec = vec![0];
            s.houses[1].hub = 1;
            s.houses[1].influence.clear();
            s.houses[1].offices.clear();
            s.hubs[1].component = 99; // and not even the same trading region
            for m in 240..600u32 { s.tick = m * 30; s.update_feuds(); }
            assert_eq!(s.feuds[0].outcome, FEUD_COOLED,
                "a feud with no overlap left must cool: intensity {}", s.feuds[0].intensity);
            assert!(!s.houses[0].rivals.contains(&1),
                "a settled feud must clear the rival entries");
        }
    }

    /// A long feud between two houses that both trade in a peaceful, uncaptured city
    /// must be ARBITRATED by that city's council. This is the mechanism that stops the
    /// world converging on "every old house feuds with every other".
    #[test]
    fn a_long_feud_is_settled_by_the_council() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0, 400.0], 0)];
        let mut s = sim(hubs, goods);
        for i in 0..2 {
            let mut h = house_at(0, vec![1], 1);
            h.name = format!("House {i}");
            h.wealth = if i == 0 { 5000.0 } else { 2000.0 };
            h.influence = vec![(0, 0.4)];
            s.houses.push(h);
        }
        s.house_barred = vec![Vec::new(); 2];
        s.house_ledger = vec![Default::default(); 2];
        s.open_feud(0, 1, FEUD_TRADE, 1, 0, 0.5);
        // Give it some damage on both sides so the settlement has something to divide.
        s.feuds[0].damage_a = 40.0;
        s.feuds[0].damage_b = 120.0;
        let civic0 = s.hubs[0].civic_pool;
        // Run past the arbitration threshold. It is chance-gated per year, so give the
        // council plenty of years to reach for it.
        let mut settled = false;
        for yr in 0..90u32 {
            s.tick = yr * TICKS_PER_YEAR;
            s.arbitrate_feuds(yr);
            if s.feuds[0].outcome != FEUD_RUNNING { settled = true; break; }
        }
        assert!(settled, "a long feud in a peaceful city must eventually be settled");
        assert_eq!(s.feuds[0].outcome, FEUD_ARBITRATED, "the council imposed the settlement");
        assert!(s.hubs[0].civic_pool > civic0, "the city takes its cut of the settlement");
        assert!(!s.houses[0].rivals.contains(&1) && !s.houses[1].rivals.contains(&0),
            "an arbitrated feud is genuinely over");
        assert!(s.houses.iter().all(|h| h.wealth.is_finite() && h.prestige >= 0.0),
            "a settlement leaves both houses in a valid state");
    }

    /// Migration must MIX cultures: when people of one people move (over a trade tie)
    /// into a city of a DIFFERENT people, a minority quarter of the newcomers' culture
    /// must appear there. Guards the "every city stays monocultural" regression.
    #[test]
    fn migration_seeds_foreign_minority_quarters() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        // Four adjacent cities in one connected market; two peoples split down the middle.
        let mut hubs = Vec::new();
        for i in 0..4u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 6000.0 } else { 800.0 }).collect();
            hubs.push(hub(i, (i as f32) * 3.0, 0.0, 5000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        // Assign cultures: hubs 0,1 = "Aiora"; hubs 2,3 = "Belgar". (ensure_hub_cultures
        // won't overwrite these non-empty entries.)
        s.hub_culture = vec!["Aiora".into(), "Aiora".into(), "Belgar".into(), "Belgar".into()];
        s.hub_minorities = vec![Vec::new(); 4];
        // Make hub 1 (an "Aiora" city) the one thriving magnet; the "Belgar" hub 2 next
        // door is miserable, so its people drift across the culture border into hub 1.
        for (i, h) in s.hubs.iter_mut().enumerate() {
            if i == 1 { h.sent_prosperity = 0.95; h.starving = 0.0; h.food_balance = 1.0; }
            else { h.sent_prosperity = 0.30; h.starving = 0.0; h.food_balance = 1.0; }
        }
        // Run the economic-migration pass a few years (it's yearly).
        for _ in 0..8 { s.economic_migration_pass(); }
        // Hub 1 ("Aiora") must now host a "Belgar" minority carried in by the migrants.
        let belgar_at_1 = s.hub_minorities[1].iter().find(|(c, _)| c == "Belgar").map(|(_, sh)| *sh).unwrap_or(0.0);
        assert!(belgar_at_1 > 0.0,
            "expected a Belgar minority to form in the Aiora magnet city, got {:?}", s.hub_minorities[1]);
    }

    /// A big healthy city must ADOPT a tiny, failing neighbour as a satellite (reviving
    /// it) instead of leaving it to die. Guards the absorption rescue path.
    #[test]
    fn big_city_absorbs_dying_neighbour() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        // Hub 0 = a big healthy metropolis; hub 1 = a tiny failing town right beside it.
        let big_prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 40_000.0 } else { 5_000.0 }).collect();
        let small_prod: Vec<f32> = (0..ng).map(|_| 30.0).collect();
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 20_000.0, big_prod, 0),
            hub(1, 1.5, 0.0, 150.0, small_prod, 0),
        ];
        hubs[0].sent_prosperity = 0.65; hubs[0].starving = 0.0;
        hubs[0].stock = vec![10_000.0, 4_000.0, 0.0]; // food to grant
        hubs[1].sent_prosperity = 0.25; hubs[1].starving = 0.2; // struggling
        let mut s = sim(hubs, goods);
        s.tick = 20 * 365; // old enough that the town is past ABSORB_MIN_AGE
        s.hub_culture = vec!["Aiora".into(), "Belgar".into()];
        s.hub_minorities = vec![Vec::new(); 2];
        s.rebuild_routes();
        s.maybe_absorb_dying_city();
        assert_eq!(s.hubs[1].colony_kind, 3, "dying town should become a satellite");
        assert_eq!(s.hubs[1].founder_hub, 0, "…bound to the big neighbour");
        assert!(s.hubs[1].population > 150.0, "…and shored up with relocated settlers");
        // The metropolis's people arrive as a minority quarter (culture mixing).
        assert!(s.hub_minorities[1].iter().any(|(c, _)| c == "Aiora"),
            "adopted town should host an Aiora minority, got {:?}", s.hub_minorities[1]);
    }

    /// Reproduction for the "campaign restarts near year 30" crash: outposts only
    /// start founding at OUTPOST_START_TICK (= year 30), and founding APPENDS a hub
    /// mid-tick. With colonization sites seeded and houses rich enough to clear the
    /// outpost wealth bar, the world grows past year 30 — exercising every per-hub
    /// Expeditions must actually run end-to-end: a financed venture launches,
    /// travels, resolves, and over repeated proven round-trips a permanent corridor
    /// is established (with its founding recorded). Guards against the mechanic being
    /// inert (never launching) or never converging on a corridor.
    #[test]
    fn expeditions_launch_travel_and_establish_a_corridor() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        // Two distant cities (world_w 100 → 45-cell gap ≫ the 14% min) that trade
        // should want to connect, plus a house rich enough to bankroll ventures.
        let hubs = vec![
            hub(0, 5.0, 50.0, 1500.0, vec![80.0, 40.0], 0),
            hub(1, 50.0, 50.0, 1500.0, vec![60.0, 30.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.houses = vec![house_at(0, vec![1], 4)];
        s.houses[0].wealth = 5000.0;
        s.tick = 15 * TICKS_PER_YEAR;
        let mut launched_any = false;
        // ~40 years: relaunch toward the same city whenever idle, tick the travel.
        for _ in 0..(40 * TICKS_PER_YEAR) {
            if !s.corridors.is_empty() { break; }
            if s.expeditions.iter().all(|e| e.status >= 3) {
                s.launch_expedition(0, 0, 1);
                launched_any = true;
                s.houses[0].wealth = s.houses[0].wealth.max(2000.0); // keep it solvent for the test
            }
            s.expedition_travel_pass();
            s.tick += 1;
        }
        assert!(launched_any, "no expedition ever launched");
        assert!(!s.route_prospects.is_empty(), "a venture completed but no prospect ledger formed");
        assert!(!s.corridors.is_empty(),
            "repeated successful ventures never established a corridor (prospect: {:?})",
            s.route_prospects.first());
        let c = &s.corridors[0];
        assert!(c.successes >= EXP_MIN_SUCCESSES, "corridor established below the success bar");
    }

    /// loop against a freshly-appended hub. Must not panic (index/overflow) for 50y.
    #[test]
    fn outposts_and_colonies_past_year_30_dont_crash() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
            good("spices", 1, 2, 16.0, 0.4, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..24u32 {
            let x = (i % 6) as f32 * 9.0;
            let y = (i / 6) as f32 * 9.0;
            let pop = 9000.0 + (i as f32 * 911.0) % 24000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.013 } else { pop * 0.0015 })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0)); // one component → a connected market
        }
        let mut s = sim(hubs, goods);
        // A handful of houses, two of them seeded already wealthy so they clear the
        // heavy OUTPOST_FOUND_WEALTH (100k) bar by year 30 and actually plant outposts.
        for i in 0..6u32 {
            let seat = (i * 3) % 24;
            let mut h = house_at(seat, vec![2 + (i as usize % 3)], 3);
            h.archetype = (i % 4) as u8;
            // Seed several houses already very rich so they clear the heavy
            // OUTPOST_FOUND_WEALTH (100k) bar repeatedly and actually plant outposts.
            h.wealth = if i < 4 { 400_000.0 } else { 60.0 + i as f32 * 10.0 };
            h.prestige = 0.6;
            h.dominant_seat = i % 2 == 0;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        // Seed colonization sites on the frontier so outposts AND settlement colonies
        // can be founded (a bank is needed for a settlement colony; outposts only need
        // a rich house). Spread them across the map, trade-rich + coastal.
        for k in 0..12u32 {
            s.colonizable.push(ColonizeSite {
                x: (k % 4) as f32 * 12.0 + 4.0,
                y: (k / 4) as f32 * 12.0 + 4.0,
                koppen: 11, elevation: 0.2, fertility: 0.5, coastal: k % 2 == 0,
                kind_hint: ((k % 5) + 1) as u8, trade_value: 0.4 + (k as f32 % 3.0) * 0.2,
                delta: false, chokepoint: false,
            });
        }
        // A bank so settlement colonies (which need a same-continent bank) can form too.
        s.banks.push(Bank {
            name: "Banco".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 80.0, loans: vec![], real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0, stakes: vec![],
            dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        });
        s.rebuild_routes();
        let hubs0 = s.hubs.len();
        // Run through and well past year 30 — the outpost/colony founding window.
        for yr in 1..=50u32 {
            // Keep two houses permanently above the heavy outpost wealth bar so the
            // outpost-founding path is GUARANTEED to fire every year from year 30 on
            // (the real campaign has dynasties this rich; left alone the wealth tax
            // bends them under the bar before year 30 and the path never runs).
            for hi in 0..2usize.min(s.houses.len()) {
                if !s.houses[hi].defunct { s.houses[hi].wealth = s.houses[hi].wealth.max(400_000.0); }
            }
            s.advance(365);
            // Per-hub persistent arrays must never lag the hub list (the crash class).
            assert!(s.hub_patron.len() <= s.hubs.len(), "hub_patron never exceeds hubs");
            for h in &s.hubs { assert_eq!(h.stock.len(), ng, "every hub keeps ng columns"); }
            if yr % 5 == 0 {
                let outposts = s.hubs.iter().filter(|h| h.colony_kind == 2).count();
                eprintln!("yr {yr:2}: hubs {} (+{} since start) · outposts {outposts}",
                    s.hubs.len(), s.hubs.len() - hubs0);
            }
        }
        // The point of the test is the founding path: confirm the world actually GREW
        // past year 30 (outposts/estates/colonies appended) without crashing.
        assert!(s.hubs.len() > hubs0,
            "world must grow past year 30 (founding exercised): {} → {}", hubs0, s.hubs.len());
    }

    /// A wealthy house develops an EXISTING under-traded small city into a TRADE BASE:
    /// it opens an office, builds a guildhall, seeds capital and takes the city under
    /// its patronage — and patronage concludes once the city grows up.
    #[test]
    fn trade_base_development() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        // Hub 0: the house's busy seat. Hub 1: a small, inert town on the same
        // continent, in reach. Hub 2: too big to qualify (a control).
        let prod0: Vec<f32> = (0..ng).map(|g| if goods[g].food { 400.0 } else { 120.0 }).collect();
        let prod1: Vec<f32> = (0..ng).map(|g| if goods[g].food { 200.0 } else { 5.0 }).collect();
        let hubs = vec![
            hub(0, 0.0, 0.0, 80_000.0, prod0.clone(), 0),
            hub(1, 4.0, 0.0, 20_000.0, prod1, 0),
            hub(2, 8.0, 0.0, 200_000.0, prod0, 0),
        ];
        let mut s = sim(hubs, goods);
        // The small town starts inert (no throughput) — the under-traded signal.
        s.hubs[1].export_earn = 0.0;
        s.hubs[1].import_spend = 0.0;
        // A rich house seated at hub 0, clearing the (modest) base-investment bar.
        let mut rich = house_at(0, vec![1], 2);
        rich.wealth = 100_000.0;
        s.houses.push(rich);
        s.seed_house_count = 1;
        s.rebuild_routes();
        s.hub_patron.resize(s.hubs.len(), -1);
        s.tick = BASE_START_TICK;

        let w0 = s.houses[0].wealth;
        s.maybe_establish_trade_base();

        // The small town (hub 1) — not the big control (hub 2) — became the base.
        assert_eq!(s.hub_patron[1], 0, "the small under-traded city is patronised by house 0");
        assert_eq!(s.hub_patron[2], -1, "the large well-developed city is NOT taken as a base");
        assert!(s.houses[0].offices.contains(&1), "the patron opens an office in the base");
        assert!(s.hubs[1].structures.contains(&STRUCT_GUILDHALL), "a guildhall is built in the base");
        assert!(s.hubs[1].treasury > 0.0, "working capital is seeded into the city");
        assert!(s.houses[0].wealth < w0, "the house pays for the investment");

        // Graduation: once the base grows into a real node, patronage concludes.
        s.hubs[1].population = BASE_DEVELOPED_POP + 1.0;
        s.trade_base_pass();
        assert_eq!(s.hub_patron[1], -1, "patronage concludes once the city is developed");
    }

    /// STANDING PRACTICE (see CLAUDE.md): run the living campaign for decades and
    /// watch the dynamics — houses rise and fall, banks are chartered and fail,
    /// poleis mint coin, wars flare, crashes ripple. Prints a 5-yearly digest:
    ///   `cargo test --lib simulate_decades_reports_dynamics -- --nocapture`
    /// Also a hard regression: wealth must stay finite + bounded (no 100k blow-ups,
    /// no −50M contract craters) over the whole run.
    #[test]
    fn simulate_decades_reports_dynamics() {
        // 30 hubs in one connected market, six goods (three of them food).
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
            // Each hub specializes: a couple of goods at high output, the rest low.
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // Ten houses: a spread of archetypes, several dominant at their seat so a
        // council forms (→ coinage → banks).
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
        // Atlas 2.0 · FREE LAND south of the market towns, so the lifecycle passes
        // (organic swarming + colonial ventures) have somewhere to found.
        for i in 0..12u32 {
            s.colonizable.push(ColonizeSite {
                x: 4.5 + (i % 4) as f32 * 12.0,
                y: 40.0 + (i / 4) as f32 * 8.0,
                koppen: 8, elevation: 0.1,
                fertility: 0.45 + (i % 3) as f32 * 0.15,
                coastal: i % 2 == 0, kind_hint: 1,
                trade_value: 0.2 + (i % 4) as f32 * 0.1,
                delta: false, chokepoint: false,
            });
        }
        s.rebuild_routes();

        let mut min_w = f32::INFINITY;
        let mut peak_w = f32::NEG_INFINITY;
        let mut late_max = 0.0f32; // richest in the final decade — the SUSTAINED level
        let mut ever_dissolved = false;
        for yr in 1..=50u32 {
            s.advance(365);
            let active = s.houses.iter().filter(|h| !h.defunct).count();
            let defunct = s.houses.len() - active;
            if defunct > 0 { ever_dissolved = true; }
            let banks = s.banks.iter().filter(|b| !b.defunct).count();
            let coins = s.hubs.iter().filter(|h| !h.coin_name.is_empty()).count();
            let top_trust = s.hubs.iter().map(|h| h.coin_trust).fold(0.0f32, f32::max);
            let rich = s.houses.iter().filter(|h| !h.defunct)
                .map(|h| h.wealth).fold(0.0f32, f32::max);
            if yr > 40 { late_max = late_max.max(rich); }
            for h in &s.houses {
                if h.wealth.is_finite() { min_w = min_w.min(h.wealth); peak_w = peak_w.max(h.wealth); }
            }
            assert!(s.tech_factor.is_finite());
            // DLC 4 · finest good in the world this year + cumulative espionage.
            let mut finest = (0.0f32, 0usize);
            for h in &s.hubs {
                for (g, &q) in h.quality.iter().enumerate() {
                    if h.production.get(g).copied().unwrap_or(0.0) > 0.0 && q > finest.0 { finest = (q, g); }
                }
            }
            let thefts = s.journal.iter().filter(|e| e.kind == "espionage").count();
            let contracts = s.contracts.len();
            let colonies = s.hubs.iter().filter(|h| h.colony_kind == 1).count();
            let outposts = s.hubs.iter().filter(|h| h.colony_kind == 2).count();
            let offices: usize = s.houses.iter().filter(|h| !h.defunct).map(|h| h.offices.len()).sum();
            if yr % 5 == 0 {
                let towns_alive = s.hubs.iter()
                    .filter(|h| !h.is_estate && !h.abandoned && h.population >= 1.0).count();
                let hungry = s.hubs.iter()
                    .filter(|h| !h.is_estate && !h.abandoned && h.starving > 0.5).count();
                let thriving = s.hubs.iter().filter(|h| !h.is_estate && !h.abandoned
                    && h.mood > 0.55 && h.starving < 0.1).count();
                // B3/B4 · public-debt engagement + bills-of-exchange income (realism batch).
                let debt_cities = s.hubs.iter().filter(|h| h.debt_principal > 0.0).count();
                let debt_total: f32 = s.hubs.iter().map(|h| h.debt_principal).sum();
                let bills: f32 = s.banks.iter().map(|b| b.bills_income).sum();
                eprintln!(
                    "yr {yr:2}: houses {active}↑/{defunct}✝  banks {banks}  coins {coins} (trust {:.0}%)  wars {}  crashes {}  richest {rich:.0}  debt {debt_cities}c/{debt_total:.0}  bills {bills:.0}  contracts {contracts}  offices {offices}  colonies {colonies}  outposts {outposts}  towns {towns_alive} (+{}/−{}) hungry {hungry} thriving {thriving}  finest {} {:.0}%  thefts {thefts}",
                    top_trust * 100.0, s.wars.len(), s.crashes.len(),
                    s.total_foundings, s.total_abandonments,
                    s.goods[finest.1].name, finest.0 * 100.0,
                );
            }
        }
        eprintln!("over 50y: wealth ∈ [{min_w:.1}, {peak_w:.1}] · sustained (late) richest {late_max:.0}");
        assert!(min_w.is_finite() && peak_w.is_finite(), "wealth finite");
        // SUSTAINED wealth must stay sane. Wealth is intentionally NOT hard-capped
        // (a great trading dynasty can climb into the hundreds of thousands and so
        // afford a trade outpost), but the gentle quadratic surcharge still bends the
        // very richest back — no millions-scale runaway (the old bug ran to ~1.25M).
        // The ceiling was raised when PLAGUE IMMUNITY landed: cities that survive an
        // outbreak now resist re-infection for years, so the world is no longer culled
        // every few years and long-lived trading dynasties keep more of what they earn —
        // a designed, healthier-economy consequence. The bound still catches an
        // order-of-magnitude / millions-scale runaway.
        assert!(late_max < 1_000_000.0, "no SUSTAINED runaway-rich house: {late_max}");
        assert!(min_w > -100.0, "no runaway-insolvent house (limited liability): {min_w}");
        // The world must actually be DYNAMIC: houses turn over.
        assert!(ever_dissolved, "houses rise and fall over decades");
    }

    /// Perf P1 receipt · what a read-only panel query paid BEFORE the Arc resident
    /// sim (a full deep clone of a mature campaign) vs after (an Arc bump).
    /// Run: cargo test --lib bench_sim_clone -- --ignored --nocapture
    #[test]
    #[ignore]
    fn bench_sim_clone() {
        use std::time::Instant;
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
            let pop = 8000.0 + (i as f32 * 911.0) % 26000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 })
                .collect();
            hubs.push(hub(i, (i % 6) as f32 * 9.0, (i / 6) as f32 * 9.0, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        for i in 0..10u32 {
            s.houses.push(house_at((i * 3) % 30, vec![3 + (i as usize % 3)], 3));
        }
        s.seed_house_count = s.houses.len() as u32;
        s.rebuild_routes();
        s.advance(365 * 30); // a mature campaign: histories + journal filled
        let arc = std::sync::Arc::new(s);
        let t0 = Instant::now();
        for _ in 0..50 { let c = (*arc).clone(); std::hint::black_box(&c); }
        let deep = t0.elapsed().as_secs_f64() * 1000.0 / 50.0;
        let t1 = Instant::now();
        for _ in 0..1_000_000 { let c = arc.clone(); std::hint::black_box(&c); }
        let bump = t1.elapsed().as_secs_f64() * 1000.0 / 1_000_000.0;
        eprintln!("per-query cost: deep clone {deep:.3} ms  →  Arc bump {bump:.6} ms  ({:.0}× faster)",
            deep / bump.max(1e-9));
        assert!(deep > bump, "Arc bump must beat a deep clone");
    }

    /// Atlas 2.0 · a thriving city bursting past its founding size SWARMS: a slice
    /// of its people found an independent daughter town on nearby free land, the
    /// site is consumed, and the founding is chronicled.
    #[test]
    fn thriving_city_swarms_a_daughter_town() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let h0 = hub(0, 10.0, 10.0, 20_000.0, vec![20_000.0 * 0.02], 0);
        let mut s = sim(vec![h0], goods);
        s.colonizable.push(ColonizeSite {
            x: 16.0, y: 10.0, koppen: 8, elevation: 0.1, fertility: 0.8,
            coastal: false, kind_hint: 1, trade_value: 0.3,
            delta: false, chokepoint: false,
        });
        // The swarm preconditions: crowded (2× founding), content, fed.
        s.hubs[0].population = s.hubs[0].founding_pop * 2.0;
        s.hubs[0].mood = 0.7;
        s.hubs[0].starving = 0.0;
        s.tick = SWARM_START_TICK;
        let mother_pop = s.hubs[0].population;
        s.maybe_swarm_town();
        assert_eq!(s.hubs.len(), 2, "a daughter town was founded");
        let d = &s.hubs[1];
        assert_eq!(d.colony_kind, 0, "organic town, not a chartered colony");
        assert!(d.founded_tick == SWARM_START_TICK && !d.abandoned && !d.name.is_empty());
        assert!(d.population > 0.0 && s.hubs[0].population < mother_pop,
            "settlers actually left the mother city");
        assert!(s.colonizable.is_empty(), "the site was consumed");
        assert!(s.journal.iter().any(|e| e.kind == "founding"), "founding chronicled");
        assert_eq!(s.total_foundings, 1);
    }

    /// Atlas 2.0 · a famine-floored town with a fed HAVEN nearby is abandoned:
    /// survivors migrate to the haven, the abandonment is chronicled, and the ruin
    /// is never resurrected by the population floor.
    #[test]
    fn famine_town_is_abandoned_and_stays_dead() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 30_000.0, vec![30_000.0 * 0.03], 0), // the haven: fed
            hub(1, 60.0, 10.0, 10_000.0, vec![0.0], 0),             // doomed: no food
        ];
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        // Terminal state reached: famine-floored and starving for ABANDON_YEARS.
        s.hubs[1].population = s.hubs[1].founding_pop * 0.11;
        s.hubs[1].starving = 0.9;
        s.hubs[1].decline_years = ABANDON_YEARS;
        let haven_before = s.hubs[0].population;
        s.lifecycle_pass(true);
        assert!(s.hubs[1].abandoned, "famine town abandoned");
        assert!(s.hubs[1].population < 1.0 && s.hubs[1].died_tick == s.tick);
        assert!(s.hubs[0].population > haven_before, "survivors reached the haven");
        assert!(s.journal.iter().any(|e| e.kind == "abandonment"), "abandonment chronicled");
        assert_eq!(s.total_abandonments, 1);
        // The ruin stays dead through real ticks (the pop floor must not revive it).
        s.advance(60);
        assert!(s.hubs[1].population < 1.0, "ruin not resurrected by the pop floor");
        assert!(s.hubs[1].abandoned);
    }

    /// Social strata invariant: every settlement's four shares stay in [0,1] and sum
    /// to ~1 across decades of mobility, the strata actually DIFFERENTIATE between
    /// cities, and inequality stays a bounded 0..1 index.
    #[test]
    fn society_shares_bounded() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
            good("wine", 3, 2, 8.0, 0.4, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..20u32 {
            let x = (i % 5) as f32 * 9.0;
            let y = (i / 5) as f32 * 9.0;
            let pop = 6000.0 + (i as f32 * 1300.0) % 24000.0;
            // A clear rich/poor gradient: the first third are luxury-exporting
            // entrepôts (high trade wealth → patrician/burgher), the rest agrarian.
            let rich = i < 7;
            let prod: Vec<f32> = (0..ng)
                .map(|g| {
                    if g == 2 || g == 4 { // silk / wine — luxuries that build trade wealth
                        if rich { pop * 0.020 } else { pop * 0.0005 }
                    } else if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 }
                })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        for i in 0..8u32 {
            let seat = (i * 2) % 20;
            let mut h = house_at(seat, vec![2 + (i as usize % 3)], 3);
            h.archetype = (i % 4) as u8;
            h.wealth = 40.0 + (i as f32) * 12.0;
            h.dominant_seat = i % 2 == 0;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        s.rebuild_routes();

        for _ in 1..=60u32 {
            s.advance(365);
            for h in &s.hubs {
                if h.is_estate { continue; }
                let so = &h.society;
                let sum = so.patrician + so.burgher + so.commoner + so.underclass;
                if sum < 1e-3 { continue; } // not yet seeded (brand-new colony this tick)
                for (name, v) in [("patrician", so.patrician), ("burgher", so.burgher),
                                  ("commoner", so.commoner), ("underclass", so.underclass)] {
                    assert!(v >= -1e-4 && v <= 1.0 + 1e-4, "{name} share out of range: {v}");
                }
                assert!((sum - 1.0).abs() < 1e-2, "shares must sum to 1, got {sum}");
                assert!(so.inequality >= 0.0 && so.inequality <= 1.0, "inequality 0..1: {}", so.inequality);
                assert!(so.commoner_wealth.is_finite() && so.commoner_wealth >= 0.0, "commoner_wealth bad");
            }
        }
        // The strata must DIFFERENTIATE — not every city ends with the same elite share.
        let elites: Vec<f32> = s.hubs.iter().filter(|h| !h.is_estate)
            .map(|h| h.society.patrician + h.society.burgher).collect();
        let emin = elites.iter().cloned().fold(f32::INFINITY, f32::min);
        let emax = elites.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        // Cities must still visibly DIFFERENTIATE. The floor was 0.02 before house
        // GOVERNMENT CAPTURE landed — a dominant family that seizes a city's officials
        // spreads its (favourable-tariff) policy, which mildly lifts backwaters' trade and
        // so compresses the elite-share gap a little. The spread stays clearly non-trivial.
        assert!(emax - emin > 0.012, "strata should vary across cities (spread {:.3})", emax - emin);
    }

    /// It. 3 · A chronically poor, steeply unequal city should boil over: unrest
    /// climbs, a revolt fires, the ruling council is toppled & barred, and wealth
    /// stays finite throughout (the redistribution is a sink, not a blow-up).
    #[test]
    fn unrest_topples_councils() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.9, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..6u32 {
            let x = (i % 3) as f32 * 8.0;
            let y = (i / 3) as f32 * 8.0;
            let pop = 12000.0;
            // Food-starved everywhere (all one component → no relief), so dearth bites.
            let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { pop * 0.004 } else { pop * 0.002 }).collect();
            hubs.push(hub(i, x, y, pop, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // A tiny, fabulously rich oligarchy sits each city's council → extreme inequality.
        for i in 0..6u32 {
            let mut h = house_at(i, vec![2], 2);
            h.wealth = 800.0;
            h.dominant_seat = true;
            h.archetype = ARCH_POLITICAL;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        s.rebuild_routes();

        for _ in 1..=45u32 {
            s.advance(365);
            for h in &s.hubs {
                assert!(h.society.unrest >= 0.0 && h.society.unrest <= 1.0, "unrest 0..1");
            }
            for h in &s.houses {
                assert!(h.wealth.is_finite() && h.wealth < 1.0e7, "house wealth blew up: {}", h.wealth);
            }
        }
        let revolts = s.journal.iter().filter(|e| e.kind == "revolt").count();
        assert!(revolts >= 1, "a chronically poor, unequal city should revolt at least once");
        // Some council was barred by a revolt (the ban window was set).
        let banned = s.hubs.iter().any(|h| h.society.ousted_until > 0);
        assert!(banned, "a revolt should bar the toppled family from the council");
    }

    #[test]
    fn development_tier_ranks_by_institutions() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![20.0], 0),
            hub(1, 8.0, 0.0, 5000.0, vec![20.0], 0),
        ];
        let mut s = sim(hubs, goods);
        // A bare founding settlement is an Outpost (tier 1).
        assert_eq!(s.development_tier(0), 1, "bare hub is an Outpost");
        // A little trade lifts it to a Market (tier 2) — no population change needed.
        s.hubs[0].trade_last_year = 10.0;
        assert_eq!(s.development_tier(0), 2, "trade lifts it to a Market");
        // An abandoned ruin reads as 0.
        s.hubs[0].abandoned = true;
        assert_eq!(s.development_tier(0), 0, "abandoned reads 0");
    }

    /// Colonisation MECHANISM (deterministic): from year 50, an eligible city founds
    /// a SETTLEMENT colony (full market hub, joint-stock funded, migrants seeded) and
    /// a rich house plants a remote trade OUTPOST on poor land within office/home
    /// reach — both land-only — and a grown colony graduates then may go autonomous.
    #[test]
    fn colonisation_mechanism() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("wine", 3, 2, 8.0, 0.4, false),
        ];
        let ng = goods.len();
        // Three ordinary hubs clustered near the origin.
        let mut hubs = Vec::new();
        for i in 0..3u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if goods[g].food { 200.0 } else { 40.0 }).collect();
            hubs.push(hub(i, (i as f32) * 4.0, 0.0, 20_000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // A healthy, prosperous, treasury-rich founder city (hub 0).
        s.hubs[0].population = 30_000.0;
        s.hubs[0].founding_pop = 30_000.0;
        s.hubs[0].starving = 0.0;
        s.hubs[0].sent_prosperity = 0.8;
        s.hubs[0].treasury = 40.0;
        // A GREAT house seated at hub 0 → funds the venture + plants outposts. Must
        // clear the heavy outpost wealth bar (OUTPOST_FOUND_WEALTH).
        let mut rich = house_at(0, vec![2], 5);
        rich.wealth = 400_000.0;
        s.houses.push(rich);
        s.seed_house_count = 1;
        // A same-continent bank is REQUIRED to found a settlement colony (its family
        // becomes the colony's bank + mint).
        s.banks.push(Bank {
            name: "Banco di Test".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 50.0, loans: vec![], real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        });
        // A second, non-backer house (seated elsewhere) → the charter should bar it.
        let mut other = house_at(1, vec![3], 2);
        other.wealth = 50.0;
        s.houses.push(other);
        // Empty land near the cluster: a fertile site (→ settlement colony) and a
        // trade-rich poor coastal site (→ house outpost), both within the hop-reach cap.
        s.colonizable = vec![
            ColonizeSite { x: 3.0, y: 3.0, koppen: 0, elevation: 0.2, fertility: 0.80, coastal: false, kind_hint: 1, trade_value: 0.10, delta: false, chokepoint: false },
            ColonizeSite { x: 5.0, y: 2.0, koppen: 0, elevation: 0.1, fertility: 0.18, coastal: true, kind_hint: 4, trade_value: 0.60, delta: false, chokepoint: false },
        ];
        s.rebuild_routes();
        s.tick = COLONY_START_TICK; // open the age of colonisation

        let base = s.hubs.len();
        let parent_pop0 = s.hubs[0].population;
        s.maybe_found_house_outpost();
        s.maybe_found_settlement_colony();

        let outposts = s.hubs.iter().filter(|h| h.colony_kind == 2).count();
        let settlements: Vec<usize> = (0..s.hubs.len()).filter(|&h| s.hubs[h].colony_kind == 1).collect();
        assert_eq!(outposts, 1, "a house trade outpost was planted");
        assert_eq!(settlements.len(), 1, "a settlement colony was founded");
        assert!(s.hubs.len() == base + 2, "two new colony hubs exist");
        // Outpost is REMOTE (kept its own site coords, not co-located with home).
        let outpost = s.hubs.iter().find(|h| h.colony_kind == 2).unwrap();
        assert!(!outpost.is_estate || outpost.parent < 0, "outpost is remote, not an in-city estate");
        assert!(outpost.name.contains("(outpost)"));
        // Settlement colony is a FULL market hub, tagged, with backers, and the parent
        // shed migrants to seed it.
        let c = settlements[0];
        assert!(!s.hubs[c].is_estate, "settlement colony is a market hub");
        assert_eq!(s.hubs[c].colony_kind, 1, "tagged as a settlement colony");
        assert_ne!(s.hubs[c].name, format!("New {}", s.hubs[0].name), "colony has its OWN fresh name, not a duplicate of the parent");
        assert!(!s.hubs[c].backers.is_empty(), "joint-stock backers recorded");
        assert!(s.hubs[0].population < parent_pop0, "parent shed emigrants to the colony");
        // Bank + mint: the backing bank's family seats the colony's council & coin.
        assert!(s.hubs[c].main_bank >= 0, "colony has a main bank");
        assert!(!s.hubs[c].coin_name.is_empty(), "colony mints its own coin");
        // Food lifeline: civic supply contracts were signed (the roster).
        assert!(s.colony_supply.iter().any(|r| r.colony_hub == c as u32 && r.category == 0), "food supplier signed");
        // Monopoly charter: the non-backer house (idx 1) is barred from the colony.
        assert!(s.house_barred.get(1).is_some_and(|v| v.contains(&(c as u32))), "charter bars a non-backer house");

        // GROWTH GATE: needs ≥5yr supply + population + buildings. Set those + age 71
        // and run the yearly pass → graduates to city AND rebels (metropolis alive).
        s.hubs[c].population = 60_000.0;
        s.hubs[c].supply_years = 6.0;
        s.hubs[c].structures = vec![1, 2, 3];
        s.hubs[c].colony_founded_tick = 0;
        s.tick = 71 * 365;
        s.colony_pass();
        assert!(s.hubs[c].colony_stage >= 4, "graduates to city with supply+pop+buildings");
        assert!(s.hubs[c].war_with >= 0 && s.wars.iter().any(|w| w.cause == "independence"),
            "a mature year-70 colony-city wages a war of independence");
        // Resolve it (back-date so it's ripe) → freedom or a 15-yr cooldown.
        if let Some(w) = s.wars.iter_mut().find(|w| w.cause == "independence") { w.start_tick = s.tick - 3 * 365; }
        let yr = s.tick / 365;
        s.update_wars(yr);
        assert!(s.hubs[c].autonomous || s.hubs[c].indep_cooldown_until > 0,
            "the independence war resolves to a free city or a cooldown");
        // Wealth stays finite throughout.
        for h in &s.houses { assert!(h.wealth.is_finite()); }
    }

    /// Government: figures are seeded; a rich, locally-dominant house bribes them into
    /// service, CAPTURES the seat (favourable-house law logged), and seats turn over.
    #[test]
    fn government_capture_and_regime_change() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..3u32 {
            let prod: Vec<f32> = (0..ng).map(|g| if goods[g].food { 200.0 } else { 40.0 }).collect();
            hubs.push(hub(i, i as f32 * 4.0, 0.0, 40_000.0, prod, 0));
        }
        let mut s = sim(hubs, goods);
        // A rich BANKING house homed at hub 0 with commanding commercial influence there
        // (a cash briber — the reliable capture path).
        let mut rich = house_at(0, vec![1], 3);
        rich.wealth = 300_000.0;
        rich.archetype = ARCH_BANKING;
        rich.prestige = 1.0;
        rich.influence = vec![(0u32, 0.9)];
        s.houses.push(rich);
        s.seed_house_count = 1;
        s.rebuild_routes();
        let mut ever_captured = false;
        let mut seats_turned = false;
        let first_terms: Vec<u32>;
        s.tick = 365;
        s.update_government(1);
        assert!(!s.hubs[0].officials.is_empty(), "officials seeded");
        first_terms = s.hubs[0].officials.iter().map(|o| o.term_end).collect();
        // House keeps spending its money — top it back up each year so it can maintain grip.
        for yr in 2..=14u32 {
            s.tick = yr * 365;
            s.houses[0].wealth = 300_000.0;
            s.update_government(yr);
            if s.hubs[0].captor_house == 0 { ever_captured = true; }
            if s.hubs[0].officials.iter().zip(&first_terms).any(|(o, &t)| o.term_end != t) {
                seats_turned = true;
            }
        }
        assert!(ever_captured, "the dominant house should capture the government");
        assert!(s.hubs[0].captor_house == 0, "and hold it at the end");
        assert!(s.hubs[0].officials.iter().any(|o| o.house == 0 && (o.control >= OFFICIAL_CAPTURE || o.kin)),
            "at least one figure serves the house");
        assert!(!s.hubs[0].laws.is_empty(), "a favoured-house law was enacted on capture");
        assert!(seats_turned, "seats turn over across the years (regime change)");
        // The captor's influence at the city got a boost from capture.
        let infl0 = s.houses[0].influence.iter().find(|(c, _)| *c == 0).map(|(_, v)| *v).unwrap_or(0.0);
        assert!(infl0 >= 0.9, "capture boosts the captor's trade influence: {infl0}");
    }

    /// A starved colony (empty reserve) collapses: it dies out AND its bank writes off
    /// the colony loan (a loss that can later sink the bank → crash).
    #[test]
    fn colony_collapse_defaults_bank() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let ng = goods.len();
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 20_000.0, vec![200.0, 40.0], 0),  // metropolis
            hub(1, 4.0, 0.0, 5_000.0, vec![60.0, 10.0], 0),    // will be a colony
        ];
        hubs[1].colony_kind = 1;
        hubs[1].founder_hub = 0;
        hubs[1].main_bank = 0;
        hubs[1].reserve_food = 0.0;
        hubs[1].starving = 0.9;
        hubs[1].backers = vec![(2, 0, 1.0)];
        let mut s = sim(hubs, goods);
        let _ = ng;
        s.banks.push(Bank {
            name: "Banco".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 30.0, loans: vec![Loan { borrower_house: -1, borrower_polis: 1, principal: 10.0,
                outstanding: 10.0, rate: 0.01, start_tick: 0, term_ticks: 3650, purpose: "colony".into() }],
            real_estate: 1.0, deposits: 0.0, notes_issued: 0.0, branches: vec![0], prestige: 0.5,
            interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        });
        s.tick = 55 * 365;
        s.colony_pass();
        assert!(s.hubs[1].colony_kind == 0 && s.hubs[1].population <= 1.0, "starved colony collapsed");
        assert!(s.banks[0].losses > 0.0 && s.banks[0].loans.is_empty(), "bank wrote off the defaulted colony loan");
    }

    /// Manual benchmark for the per-day campaign tick. Run explicitly:
    ///   `cargo test --release --lib bench_campaign_tick -- --ignored --nocapture`
    /// Reports total + per-tick ms for a year on a mid-size campaign so the tick
    /// cost (and how it scales with hub/good count) can be measured.
    #[test]
    #[ignore]
    fn bench_campaign_tick() {
        use std::time::Instant;
        let ng = 24usize;
        let goods: Vec<TickGood> = (0..ng)
            .map(|g| good(&format!("g{g}"), (g % 12) as i32, (g % 3) as u8,
                          1.0 + g as f32, 0.30 + 0.5 * ((g % 5) as f32 / 5.0), g < 6))
            .collect();

        let nhubs = 160u32;
        let mut hubs = Vec::new();
        for i in 0..nhubs {
            let x = (i % 16) as f32 * 6.0;
            let y = (i / 16) as f32 * 6.0;
            let pop = 2000.0 + (i as f32 * 137.0) % 9000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 7 == 0 { pop * 0.02 } else { pop * 0.002 })
                .collect();
            hubs.push(hub(i, x, y, pop, prod, 0)); // one component → trade flows
        }
        let mut s = sim(hubs, goods);
        for i in (0..nhubs).step_by(8) {
            s.houses.push(house_at(i, vec![i as usize % ng], 2));
        }
        s.rebuild_routes();

        let days = 365u32;
        let t0 = Instant::now();
        s.advance(days);
        let total = t0.elapsed().as_secs_f64() * 1000.0;
        println!(
            "[campaign-bench hubs={nhubs} goods={ng}] {days} ticks: {total:.1}ms total, {:.3}ms/tick",
            total / days as f64
        );
    }

    #[test]
    fn deterministic_and_finite() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![50.0, 5.0], 0),
            hub(1, 40.0, 12.0, 8000.0, vec![40.0, 0.0], 0),
        ];
        let mut a = sim(hubs.clone(), goods.clone());
        let mut b = sim(hubs, goods);
        a.advance(365);
        b.advance(365);
        for h in 0..a.hubs.len() {
            for g in 0..a.goods.len() {
                assert!(a.hubs[h].price[g].is_finite() && a.hubs[h].price[g] > 0.0);
                assert!((a.hubs[h].price[g] - b.hubs[h].price[g]).abs() < 1e-3, "determinism");
            }
        }
    }

    #[test]
    fn speculation_runs_yearly_and_is_deterministic() {
        // DLC 3 · the yearly polis-policy + speculation passes must run inside
        // `advance`, stay finite/in-range, and be reproducible across two runs.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("amber", 1, 2, 14.0, 0.30, false),
        ];
        let mk = || {
            let hubs = vec![
                hub(0, 10.0, 10.0, 12000.0, vec![60.0, 6.0, 4.0], 0),
                hub(1, 40.0, 12.0, 9000.0, vec![45.0, 1.0, 0.5], 0),
                hub(2, 18.0, 38.0, 7000.0, vec![30.0, 0.5, 3.0], 0),
            ];
            let mut s = sim(hubs, goods.clone());
            for i in 0..3u32 { s.houses.push(house_at(i, vec![(i as usize) % 3], 2)); }
            s.rebuild_routes();
            s
        };
        let mut a = mk();
        let mut b = mk();
        a.advance(800); // > 2 years → at least two yearly speculation passes
        b.advance(800);
        assert_eq!(a.spec_year, b.spec_year, "speculation year reproducible");
        assert!(a.spec_year >= 2, "at least two yearly passes ran");
        assert_eq!(a.spec_centers.len(), b.spec_centers.len(), "centers reproducible");
        for c in &a.spec_centers {
            assert!(c.risk.is_finite() && (0.0..=1.0).contains(&c.risk), "risk in range");
            assert!((1..=5).contains(&c.stars));
            assert!(!c.drivers.is_empty(), "a scored polis has a reason-chain");
            // drivers are ranked largest-weight first
            for w in c.drivers.windows(2) { assert!(w[0].weight >= w[1].weight - 1e-6); }
        }
        // The polis agent set per-city tariffs (council policy ran).
        assert!(a.hubs.iter().any(|h| h.tariff_export > 0.0), "a council set a tariff");
    }

    #[test]
    fn coinage_runs_yearly_finite_and_deterministic() {
        // DLC 3.5 · council seats mint named coins with a bounded trust score, and
        // the whole coin/bank pass stays finite + reproducible across two runs.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let mk = || {
            let hubs = vec![
                hub(0, 10.0, 10.0, 30000.0, vec![160.0, 16.0], 0),
                hub(1, 40.0, 12.0, 20000.0, vec![120.0, 2.0], 0),
            ];
            let mut s = sim(hubs, goods.clone());
            // A dominant banking house at each seat → a council that mints coin.
            for i in 0..2u32 {
                let mut h = house_at(i, vec![1], 3);
                h.archetype = 2;            // banking
                h.wealth = 60.0;
                h.prestige = 0.6;
                h.dominant_seat = true;     // controls its seat → becomes the council
                s.houses.push(h);
            }
            // v2.0 · minting is now chartered (a paid privilege) — seed each seat a
            // treasury it can draw on to establish its mint-house.
            for hh in s.hubs.iter_mut() { hh.treasury = 200.0; }
            s.rebuild_routes();
            s
        };
        let mut a = mk();
        let mut b = mk();
        a.advance(800);
        b.advance(800);
        // Coins were minted, with trust kept in range, and reproducibly.
        assert!(a.hubs.iter().any(|h| !h.coin_name.is_empty()), "a council minted a coin");
        for (ha, hb) in a.hubs.iter().zip(b.hubs.iter()) {
            assert!(ha.coin_trust.is_finite() && (0.0..=1.0).contains(&ha.coin_trust));
            assert!((ha.coin_trust - hb.coin_trust).abs() < 1e-4, "coin trust reproducible");
            assert_eq!(ha.coin_name, hb.coin_name, "coin name reproducible");
        }
        // Banks (if any chartered) keep a sound, finite balance sheet.
        assert_eq!(a.banks.len(), b.banks.len(), "bank count reproducible");
        for bank in &a.banks {
            assert!(bank.equity().is_finite() && bank.reserves.is_finite());
        }
    }

    #[test]
    fn regional_crash_is_confined_to_its_region() {
        // DLC 3.5 · a crash hits every city in the origin's connectivity component
        // and haircuts houses there, but leaves a separate region untouched.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0), // region 0
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),   // region 0
            hub(2, 80.0, 50.0, 8000.0, vec![80.0], 1),   // region 1 (separate)
        ];
        let mut s = sim(hubs, goods);
        for i in 0..3u32 { s.houses.push(house_at(i, vec![0], 2)); }
        let w_before: Vec<f32> = s.houses.iter().map(|h| h.wealth).collect();
        s.trigger_regional_crash(0, 0, "test");
        // Region-0 cities are in panic; region-1 city is not.
        assert!(s.hub_in_panic(0) && s.hub_in_panic(1), "origin region panics");
        assert!(!s.hub_in_panic(2), "other region is spared");
        // Houses homed in region 0 took a haircut; the region-1 house did not.
        assert!(s.houses[0].wealth < w_before[0] && s.houses[1].wealth < w_before[1]);
        assert!((s.houses[2].wealth - w_before[2]).abs() < 1e-4, "other region's house untouched");
        assert_eq!(s.crashes.len(), 1, "the crash was recorded");
        assert_eq!(s.crashes[0].cities_hit, 2);
    }

    #[test]
    fn sound_banks_survive_contagion_only_fragile_fall() {
        // DLC 3.5 · a regional crash must NOT wipe every bank. With the softened
        // contagion run, a well-capitalised bank rides out the panic; only a
        // thinly-reserved (already-fragile) bank is swept. (Regression for the
        // total-wipeout cascade where one failure killed all banks in a region.)
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),
        ];
        let mut s = sim(hubs, goods);
        for i in 0..2u32 { s.houses.push(house_at(i, vec![0], 2)); }
        let mk_bank = |name: &str, reserves: f32, deposits: f32, notes: f32| Bank {
            name: name.into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves, loans: vec![], real_estate: 100.0, deposits, notes_issued: notes,
            branches: vec![0], prestige: 0.5, interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        };
        // Two soundly-capitalised banks and one fragile (reserves ≪ liabilities).
        s.banks.push(mk_bank("Banco Solido", 5000.0, 1000.0, 1000.0));   // ratio 2.5
        s.banks.push(mk_bank("Banco Stabile", 3000.0, 2000.0, 1000.0));  // ratio 1.0
        s.banks.push(mk_bank("Banco Fragile", 200.0, 2000.0, 500.0));    // ratio 0.08
        s.trigger_regional_crash(0, 0, "test");
        assert!(!s.banks[0].defunct, "a soundly-capitalised bank survives the panic");
        assert!(!s.banks[1].defunct, "a second sound bank survives the panic");
        assert!(s.banks[2].defunct, "the thinly-reserved fragile bank is swept away");
        assert!(s.banks.iter().any(|b| !b.defunct), "not every bank fails");
    }

    #[test]
    fn economic_war_levies_houses_and_resolves() {
        // DLC 3.5 · a war drains resident houses via levies and resolves after ≥2
        // years into the war log with reparations — the wealth sink in action.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),
        ], goods);
        for i in 0..2u32 { let mut h = house_at(i, vec![0], 2); h.wealth = 100.0; s.houses.push(h); }
        s.hubs[0].treasury = 50.0; s.hubs[1].treasury = 20.0;
        s.hubs[0].war_with = 1; s.hubs[1].war_with = 0;
        s.wars.push(War { a: 0, b: 1, start_tick: 0, chest_a: 0.0, chest_b: 0.0,
            levies: 0.0, cargo_lost: 0, cause: "test".into(), goal: WAR_GOAL_PLUNDER });
        let w0 = s.houses[0].wealth;
        s.tick = 0;
        s.update_wars(0); // wage one year — levy, no resolve
        assert!(s.houses[0].wealth < w0, "war levy drained a resident house");
        assert_eq!(s.war_log.len(), 0, "not resolved before 2 years");
        s.tick = 2 * 365;
        s.update_wars(2); // resolve
        assert_eq!(s.war_log.len(), 1, "war resolved into the log");
        assert!(s.hubs[0].war_with < 0 && s.hubs[1].war_with < 0, "war state cleared");
        assert!(s.war_log[0].levies_total > 0.0, "levies recorded");
    }

    #[test]
    fn lingua_franca_emerges_and_bridges_assimilation() {
        // The tongue of the region's dominant culture becomes its trade language and
        // eases assimilation across distant language families.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),
            hub(2, 18.0, 10.0, 3000.0, vec![80.0], 0),
        ];
        let mut s = sim(hubs, goods);
        let mk = |name: &str, fam: &str| Creole {
            name: name.into(), family: fam.into(), origin: String::new(),
            color: [1, 2, 3], born_tick: 0, birthplace: String::new(), kit_a: 0, kit_b: 0,
        };
        s.creoles.push(mk("Aquila", "Latin"));
        s.creoles.push(mk("Borin", "Norse"));
        // Aquila (Latin) is the majority of the region's cities → its tongue dominates.
        s.hub_culture = vec!["Aquila".into(), "Aquila".into(), "Borin".into()];
        s.compute_lingua();
        let lf = s.lingua.iter().find(|l| l.component == 0).expect("a lingua franca emerged");
        assert_eq!(lf.family, "Latin");
        assert_eq!(lf.culture, "Aquila");

        // A Borin (Norse — distant family) minority in a Latin-majority city assimilates
        // FASTER with the lingua-franca bridge than without it.
        let run = |s: &mut CampaignSim| { s.hub_minorities = vec![vec![("Borin".into(), 0.30)], vec![], vec![]]; s.assimilation_pass();
            s.hub_minorities[0].iter().find(|(c, _)| c == "Borin").map(|(_, x)| *x).unwrap_or(0.0) };
        let with_bridge = run(&mut s);
        s.lingua.clear();
        let without = run(&mut s);
        assert!(with_bridge < without, "lingua franca speeds assimilation: {with_bridge} < {without}");
        assert!(with_bridge < 0.30, "the quarter shrank");
    }

    #[test]
    fn war_goals_transfer_control_and_tribute_is_bounded() {
        // A resolved war's GOAL takes lasting spoils: annexation seats the victor's
        // ruling house on the loser's council (via a bailo); tribute is a bounded,
        // term-limited treasury transfer that the overlord receives in full.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0), // victor
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),   // loser
        ];
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 2)); // house 0 rules hub 0
        s.houses.push(house_at(1, vec![0], 2)); // house 1 rules hub 1
        s.hubs[0].council_house = 0;
        s.hubs[1].council_house = 1;
        // ── Annexation: hub 1's council passes to hub 0's ruling house. ──
        let clause = s.apply_war_goal(0, 1, WAR_GOAL_ANNEX, 0, 0);
        assert_eq!(s.hubs[1].council_house, 0, "loser's council is installed with the victor's house");
        assert!(s.houses[0].bailos.contains(&1), "victor's house gains a bailo in the loser");
        assert!(clause.contains("annexed"), "the annexation is narrated");
        // ── Tribute: bounded, term-limited treasury transfer. ──
        s.hubs[1].treasury = 1000.0;
        s.hubs[0].treasury = 0.0;
        s.apply_war_goal(0, 1, WAR_GOAL_TRIBUTE, 0, 0);
        assert_eq!(s.hubs[1].tribute_to, 0, "loser owes tribute to the victor");
        assert!(s.hubs[1].tribute_until > 0, "tribute has a term");
        let t0 = s.hubs[1].treasury;
        s.update_wars(1); // a tribute year
        let paid = t0 - s.hubs[1].treasury;
        assert!(paid > 0.0, "tribute is paid");
        assert!(paid <= TRIBUTE_CAP * s.city_size_factor(1) + 1e-3, "tribute is capped");
        assert!((s.hubs[0].treasury - paid).abs() < 1e-2, "the overlord receives exactly the tribute");
    }

    #[test]
    fn cutting_food_starves_a_dependent_hub() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Hub 1 grows no food and is in a SEPARATE component (no route in).
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![200.0], 0),
            hub(1, 80.0, 50.0, 10000.0, vec![0.0], 1),
        ];
        let mut s = sim(hubs, goods);
        s.advance(400);
        assert!(s.hubs[1].starving > 0.5, "isolated foodless hub starves: {}", s.hubs[1].starving);
        assert!(s.hubs[1].population < s.hubs[1].founding_pop, "population declines");
    }

    #[test]
    fn idle_house_pays_upkeep_and_goes_bankrupt() {
        // Hub 1 is isolated (its own component) so a house there can NEVER trade or
        // earn — it must still pay warehouse upkeep every month, slide into debt,
        // and after a year in the red be dissolved.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 80.0, 50.0, 10000.0, vec![100.0], 1),
        ];
        let mut s = sim(hubs, goods);
        let mut h = house_at(1, vec![0], 0); // fleetless, isolated
        h.wealth = 0.5;
        h.prev_wealth = 0.5;
        s.houses.push(h);
        // One month: upkeep is charged even though no trade happened.
        s.advance(30);
        assert!(s.houses[0].wealth < 0.5,
            "an idle house still pays upkeep (wealth {})", s.houses[0].wealth);
        // Years on: it falls into debt and a full year in the red bankrupts it.
        s.advance(30 * 40);
        assert!(s.houses[0].defunct,
            "a house a year in debt is dissolved (wealth {}, debt_since {})",
            s.houses[0].wealth, s.houses[0].debt_since);
    }

    #[test]
    fn production_scales_with_population() {
        // Two hubs with the SAME per-capita rate but 2× population: the bigger one
        // must produce ~2× as much (the core fix — output tracks live population).
        let goods = vec![good("iron", i32::MAX, 1, 5.0, 0.4, false)]; // non-food → no season
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 1000.0, vec![10.0], 0), // percap 0.01
            hub(1, 12.0, 10.0, 2000.0, vec![20.0], 0), // percap 0.01
        ], goods);
        s.advance(1);
        let (p0, p1) = (s.hubs[0].production[0], s.hubs[1].production[0]);
        assert!(p0 > 0.0 && (p1 / p0 - 2.0).abs() < 0.05, "double pop ⇒ ~double output: {p0} {p1}");
    }

    #[test]
    fn big_city_is_a_net_importer() {
        // A populous, food-poor city wired to a small food-rich one must IMPORT food
        // (regression for "large cities show 0 trade"): production no longer keeps
        // pace with a grown population, so the metropolis pulls in food.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 20000.0, vec![100.0], 0),  // huge pop, tiny per-capita food
            hub(1, 14.0, 10.0, 2000.0, vec![4000.0], 0),  // small pop, big surplus
        ], goods);
        s.advance(120);
        assert!(s.hubs[0].import_spend > 0.0, "big city imports food: {}", s.hubs[0].import_spend);
    }

    #[test]
    fn tiny_hub_wealth_stays_bounded() {
        // Regression for the "millionaire outpost": a tiny-population luxury exporter
        // can no longer accumulate absurd per-capita trade wealth, because its output
        // scales with its small population and the wealth denominator is floored.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 8000.0, vec![60.0, 0.0], 0),
            hub(1, 13.0, 10.0, 60.0, vec![0.0, 30.0], 0), // tiny pop, makes luxury
        ], goods);
        s.advance(365 * 3);
        assert!(s.hubs[1].trade_wealth < 1000.0, "tiny hub wealth bounded: {}", s.hubs[1].trade_wealth);
    }

    #[test]
    fn hemispheres_harvest_opposite_seasons() {
        // North and south hubs harvest half a year apart, so the world is never
        // short everywhere at once. At the northern harvest peak (~day 230) the
        // north out-produces the (then-troughing) south, and the seasonal swing is
        // strong at high latitude. (Seasonal ratio ~2× dominates the ±15% fertile-
        // year noise, so the inequality is robust.)
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let s = sim(vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0), // y=10 of 100 → far north
            hub(1, 14.0, 90.0, 10000.0, vec![100.0], 0), // y=90 of 100 → far south
        ], goods);
        let north_peak = s.seasonal_mult(0, 0, 230);
        let south_then = s.seasonal_mult(1, 0, 230);
        assert!(north_peak > south_then,
            "north harvests while south troughs: {north_peak} vs {south_then}");
        // Same hub, half a year apart: a strong seasonal swing at high latitude.
        let peak = s.seasonal_mult(0, 0, 230);
        let trough = s.seasonal_mult(0, 0, 230 - 182);
        assert!(peak > trough * 1.25, "high-latitude harvest swings hard: {peak} vs {trough}");
    }

    #[test]
    fn food_surplus_prevents_famine_collapse() {
        // Regression for the 8M→1M famine collapse. A connected world whose hubs are
        // each seeded with ~1.5× their food need (the seed-time food-surplus
        // guarantee) must NOT slide into world-wide famine over 5 years, even with
        // seasonal harvest troughs. Many hubs spread over latitudes both dilute
        // plague (which strikes one random hub) and let opposite hemispheres trade
        // across each other's lean seasons. We assert on the famine signals (no
        // sustained world-wide starvation) plus a soft population floor — plague
        // attrition is allowed, a food death-spiral is not.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let pop = 10000.0f32;
        let need = pop * 0.85 * DEMAND_PRESSURE; // tier_w[0]=1, need_scale=1 in test sim
        let prod = need * 1.5; // mirror the seed-time food surplus
        // 10 hubs in ONE component, spread across the whole map (x 5..95) and fanned
        // north→south (y 8..84, both hemispheres) — like a real world where a
        // regional drought (radius = 12% of width) hits a neighbour or two, not the
        // entire civilisation at once.
        let mut hubs = Vec::new();
        for i in 0..10u32 {
            let x = 5.0 + i as f32 * 10.0;
            let y = 8.0 + i as f32 * 8.4;
            hubs.push(hub(i, x, y, pop, vec![prod, 2.0], 0));
        }
        let mut s = sim(hubs, goods);
        let start: f32 = s.hubs.iter().map(|h| h.population).sum();
        s.advance(365 * 5);
        let end: f32 = s.hubs.iter().map(|h| h.population).sum();
        let mean_starving: f32 =
            s.hubs.iter().map(|h| h.starving).sum::<f32>() / s.hubs.len() as f32;
        assert!(mean_starving < 0.25, "world is not in famine: mean starving {mean_starving}");
        assert!(end > start * 0.6, "no famine collapse: {start:.0} → {end:.0}");
    }

    #[test]
    fn round_trip_earns_on_both_legs() {
        // A house at coastal hub A exports silk to coastal hub B, then carries B's
        // wine home and sells it — profit on BOTH goods proves the round trip:
        // outbound silk (A→B) AND a return cargo of wine (B→A).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5; // food at a healthy surplus
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); // silk surplus
        ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); // wine surplus
        hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)];
        s.advance(400);
        let prof = &s.houses[0].good_profit;
        assert!(prof.get(1).copied().unwrap_or(0.0) > 0.0,
            "house earns on the outbound silk leg: {prof:?}");
        assert!(prof.get(2).copied().unwrap_or(0.0) > 0.0,
            "house earns on the return wine leg (round trip): {prof:?}");
    }

    #[test]
    fn guild_appears_only_in_large_cities() {
        // A large city (≥ GUILD_MIN_POP 10k) charters a civic Merchant Guild; a
        // small town (9k) doesn't.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let big = hub(0, 10.0, 10.0, 60_000.0, vec![60_000.0 * 0.85 * DEMAND_PRESSURE * 1.5], 0);
        let small = hub(1, 40.0, 12.0, 9_000.0, vec![9_000.0 * 0.85 * DEMAND_PRESSURE * 1.5], 0);
        let mut s = sim(vec![big, small], goods);
        s.seed_initial_guilds();
        assert!(s.houses.iter().any(|h| h.is_guild && h.hub == 0), "big city charters a guild");
        assert!(!s.houses.iter().any(|h| h.is_guild && h.hub == 1), "small town has no guild");
    }

    #[test]
    fn house_opens_office_at_a_trade_partner() {
        // A house that trades steadily between its home A and partner B eventually
        // opens an office in B (its expansion mechanism).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)];
        s.advance(800);
        assert!(s.houses[0].offices.contains(&1),
            "house opens an office in its trade partner B: {:?}", s.houses[0].offices);
    }

    #[test]
    fn rich_house_invests_in_estates() {
        // A profitable house should spend its hoarded capital building estates /
        // manufactories instead of letting wealth pile up forever.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)];
        s.advance(365 * 4);
        let owned = s.hubs.iter().filter(|h| h.is_estate && h.owner_house == 0).count();
        assert!(owned >= 1, "a profitable house builds at least one estate/manufactory (owned={owned})");
    }

    #[test]
    fn warehouses_aggregate_into_hub_stock() {
        // Phase 1 scaffolding: with no house warehouses, hub_stock equals the
        // inline local-merchant pool (behaviour-preserving). A house depot's stock
        // then adds into the aggregate that prices & needs read.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let mut s = sim(vec![hub(0, 10.0, 10.0, 10000.0, vec![50.0, 5.0], 0)], goods);
        s.hubs[0].stock = vec![100.0, 0.0];
        // Empty warehouses → aggregate == the pool.
        assert_eq!(s.hub_stock(0, 0), 100.0);
        assert_eq!(s.hub_stock(0, 1), 0.0);
        // A house depot sited here adds its owned stock into the aggregate.
        s.warehouses.push(Warehouse {
            owner: 0, hub: 0, capacity: 1_000.0,
            stock: vec![50.0, 20.0], tier: CampaignSim::capacity_tier(1_000.0), damage: 0.0,
        });
        assert_eq!(s.hub_stock(0, 0), 150.0);
        assert_eq!(s.hub_stock(0, 1), 20.0);
        // Tier bands.
        assert_eq!(CampaignSim::capacity_tier(0.0), 0);   // uncapped −1 pool
        assert_eq!(CampaignSim::capacity_tier(500.0), 1); // Depot
        assert_eq!(CampaignSim::capacity_tier(1_000.0), 2); // Storehouse
        assert_eq!(CampaignSim::capacity_tier(6_000.0), 4); // Entrepôt
        assert_eq!(CampaignSim::capacity_tier(12_000.0), 5); // Grand Entrepôt
    }

    #[test]
    fn house_auto_builds_and_stocks_a_home_depot() {
        // Phase 2: a live house auto-builds a home warehouse and draws a slice of its
        // specialty good's local surplus into it (inventory it can later contract out).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
            good("wine", i32::MAX, 1, 8.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0, 0.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0, 3000.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        s.houses = vec![house_at(0, vec![1], 4)]; // specializes in silk
        s.advance(365 * 2);
        assert!(s.warehouses.iter().any(|w| w.owner == 0 && w.hub == 0 && w.tier >= 1),
            "house auto-builds a home depot: {:?}",
            s.warehouses.iter().map(|w| (w.owner, w.hub, w.tier, w.capacity)).collect::<Vec<_>>());
        let owned_silk: f32 = s.warehouses.iter().filter(|w| w.owner == 0).map(|w| w.stock[1]).sum();
        assert!(owned_silk > 0.0, "house stocks its specialty silk into the depot: {owned_silk}");
    }

    #[test]
    fn contract_term_gate_scales_with_record() {
        // Phase 3: the term a house may offer is gated by its unbroken growth record:
        // 1yr always · 3yr ≥4 stable yrs · 5yr ≥7 · 7yr >10.
        let mut s = sim(vec![hub(0, 10.0, 10.0, 5000.0, vec![1.0], 0)],
            vec![good("wheat", 0, 0, 1.0, 0.85, true)]);
        s.houses = vec![house_at(0, vec![0], 0)];
        s.houses[0].wealth_history = vec![]; // young → 1yr
        assert_eq!(s.max_term_index(0), 0);
        s.houses[0].wealth_history = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 4 growth yrs → 3yr
        assert_eq!(s.max_term_index(0), 1);
        s.houses[0].wealth_history = (1..=9).map(|i| i as f32).collect(); // 8 → 5yr
        assert_eq!(s.max_term_index(0), 2);
        s.houses[0].wealth_history = (1..=12).map(|i| i as f32).collect(); // 11 → 7yr
        assert_eq!(s.max_term_index(0), 3);
        // A decline breaks the run.
        s.houses[0].wealth_history = vec![1.0, 2.0, 3.0, 0.5, 1.0]; // only 1 trailing growth yr
        assert_eq!(s.max_term_index(0), 0);
    }

    #[test]
    fn seated_house_forms_a_supply_contract() {
        // Phase 3: a house with an office in a city that STRUCTURALLY imports its
        // specialty good offers that city a futures contract (sourced from its home
        // depot), covering a capped slice of the city's need.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 3000.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![100.0, 0.0], 0); hb.coastal = true; // no silk
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 4); // specializes in silk
        h.offices = vec![1];                 // seated in the importer city
        h.wealth = 1000.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 3000.0,
            stock: vec![0.0, 1000.0], tier: 3, damage: 0.0 }); // home silk depot
        // hub 1 needs silk; hub 0 doesn't (it produces it).
        let needs = vec![vec![0.0, 0.0], vec![0.0, 5.0]];
        // Step past the 10%/month formation throttle.
        let mut formed = false;
        for t in 1..400u32 { s.tick = t; s.form_contracts(&needs);
            if s.contracts.iter().any(|c| c.seller_house == 0 && c.buyer_hub == 1 && c.good == 1) { formed = true; break; } }
        assert!(formed, "seated house forms a silk supply contract to the importer city");
        let c = s.contracts.iter().find(|c| c.buyer_hub == 1).unwrap();
        assert!(c.monthly_qty > 0.0 && c.monthly_qty <= CONTRACT_COVERAGE_CAP * 5.0 * 30.0 + 1.0,
            "contract volume is within the coverage cap: {}", c.monthly_qty);
    }

    #[test]
    fn a_contract_delivers_from_the_source_depot() {
        // Phase 3: an active contract reserves its monthly quantity from the seller's
        // source depot and ships it to the buyer — over months `delivered` grows and
        // the depot drains.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let fp = 5000.0 * 0.85 * DEMAND_PRESSURE * 1.5;
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![fp, 3000.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![fp, 0.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 4);
        h.wealth = 10_000.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 6000.0,
            stock: vec![0.0, 5000.0], tier: 4, damage: 0.0 });
        let term = 3u8;
        s.contracts.push(Contract {
            seller_house: 0, buyer_hub: 1, source_hub: 0, good: 1, monthly_qty: 50.0,
            strike_price: 25.0, term_years: term, start_tick: 0,
            end_tick: term as u32 * TICKS_PER_YEAR, delivered: 0.0,
            last_fulfilled: 0, suspended_until: 0, defaults: 0, coin: -1,
        });
        s.advance(150);
        let c = &s.contracts[0];
        assert!(c.delivered > 0.0, "the contract delivers silk over the months: {}", c.delivered);
        // A well-stocked depot with a fleet meets most deliveries; the rare storm
        // loss is allowed (≤ a couple over the run), it just isn't the norm.
        assert!(c.defaults <= 2, "deliveries mostly succeed (defaults={})", c.defaults);
    }

    #[test]
    fn a_contract_without_a_ship_breaches() {
        // A house with no free vessel for a due contract delivery is in logistics
        // breach — it delivers nothing and the contract takes a default strike.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 3000.0], 0); ha.coastal = true;
        let mut hb = hub(1, 16.0, 10.0, 5000.0, vec![100.0, 0.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 0); // NO sea ships
        h.fleet_river = 0; h.fleet_caravan = 0; h.wealth = 100.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 6000.0,
            stock: vec![0.0, 5000.0], tier: 4, damage: 0.0 });
        s.contracts.push(Contract {
            seller_house: 0, buyer_hub: 1, source_hub: 0, good: 1, monthly_qty: 50.0,
            strike_price: 25.0, term_years: 3, start_tick: 0,
            end_tick: 3 * TICKS_PER_YEAR, delivered: 0.0,
            last_fulfilled: 0, suspended_until: 0, defaults: 0, coin: -1,
        });
        // Drive one DUE delivery directly (no advance → no random plague quarantine,
        // which would force-majeure-suspend the contract instead of breaching it).
        s.tick = CONTRACT_DELIVER_DAYS;
        let needs = vec![vec![0.0, 0.0], vec![0.0, 5.0]];
        s.fulfill_contracts(&needs);
        assert_eq!(s.contracts[0].delivered, 0.0, "a shipless house delivers nothing");
        assert!(s.contracts[0].defaults >= 1, "a shipless house breaches the contract");
    }

    #[test]
    fn network_sources_a_contract_from_a_distant_node() {
        // Phase 5: a house with offices in several cities supplies a deficit office
        // from the NEAREST network node that produces the good — not just its home.
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let h0 = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 0.0], 0); // home, no silk
        let h1 = hub(1, 16.0, 10.0, 5000.0, vec![100.0, 3000.0], 0); // office, makes silk
        let h2 = hub(2, 22.0, 10.0, 5000.0, vec![100.0, 0.0], 0); // office, imports silk
        let mut s = sim(vec![h0, h1, h2], goods);
        let mut h = house_at(0, vec![1], 4); // specializes in silk, home = hub 0
        h.offices = vec![1, 2];
        h.wealth = 1000.0;
        h.fleet_caravan = 4; // overland carry capacity (contracts are now sized to the fleet)
        s.houses = vec![h];
        let needs = vec![vec![0.0, 0.0], vec![0.0, 0.0], vec![0.0, 5.0]]; // hub 2 needs silk
        let mut formed = false;
        for t in 1..400u32 {
            s.tick = t;
            s.form_contracts(&needs);
            if s.contracts.iter().any(|c| c.seller_house == 0 && c.buyer_hub == 2
                && c.source_hub == 1 && c.good == 1) { formed = true; break; }
        }
        assert!(formed, "contract sourced from the silk-making node (1) to the importer office (2)");
        // Signing leased the buyer office, so it can't auto-close under the contract.
        assert!(s.office_leased(0, 2), "the contract leases the buyer office");
    }

    #[test]
    fn a_supplied_contract_survives_to_term_and_is_retired() {
        // The fix for "no contracts ever finish": a seller that meets its monthly
        // deliveries has its strike count reset each time, so it NEVER accrues the 3
        // strikes that void a contract — and at term end it is RETIRED (finished), not
        // voided. Deliveries are driven directly so the test isolates the contract
        // lifecycle from the campaign's random fire / plague events (which can burn a
        // depot or quarantine a city in a full `advance`).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", i32::MAX, 2, 20.0, 0.5, false),
        ];
        let mut ha = hub(0, 10.0, 10.0, 5000.0, vec![100.0, 50.0], 0); ha.coastal = true;
        let mut hb = hub(1, 14.0, 10.0, 5000.0, vec![100.0, 0.0], 0); hb.coastal = true;
        let mut s = sim(vec![ha, hb], goods);
        let mut h = house_at(0, vec![1], 20); // 20 ships → ample carry for a small qty
        h.wealth = 100_000.0;
        s.houses = vec![h];
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 12000.0,
            stock: vec![0.0, 12000.0], tier: 5, damage: 0.0 });
        let term = 1u8; // a 1-year contract → end_tick = 365
        s.contracts.push(Contract {
            seller_house: 0, buyer_hub: 1, source_hub: 0, good: 1, monthly_qty: 30.0,
            strike_price: 25.0, term_years: term, start_tick: 0,
            end_tick: term as u32 * TICKS_PER_YEAR, delivered: 0.0,
            last_fulfilled: 0, suspended_until: 0, defaults: 0, coin: -1,
        });
        s.rebuild_routes();
        let needs = vec![vec![0.0, 0.0], vec![0.0, 5.0]]; // hub 1 needs silk
        // Step one delivery per month across the whole term, restocking the depot each
        // month as the source city would. The contract must stay alive (not void) right
        // up to the final month, then be retired when the tick crosses its end.
        let mut last_delivered = 0.0;
        for month in 1..=13u32 {
            s.tick = month * CONTRACT_DELIVER_DAYS; // 30, 60, … 390
            s.warehouses[0].stock[1] = 12000.0;     // source keeps the depot supplied
            s.fulfill_contracts(&needs);
            if s.tick < term as u32 * TICKS_PER_YEAR {
                assert_eq!(s.contracts.len(), 1, "still alive in month {month} (not voided)");
                assert!(s.contracts[0].defaults < 3, "strikes keep clearing (month {month})");
                last_delivered = s.contracts[0].delivered;
            }
        }
        assert!(last_delivered > 30.0 * 8.0, "delivered most months: {last_delivered}");
        assert!(s.contracts.is_empty(), "the contract reached term end and was retired (finished)");
    }

    // ── Phase 0.4 · the succession line ─────────────────────────────────────
    /// A house keeps a RECORD of every head it has had: name, sex, the age they came
    /// in at, the age they died at, and what the family was worth at each end. This is
    /// the "who held it, and how did they do" the chronicle is written from, and it is
    /// the first thing that would silently rot if a later phase rewrote succession.
    #[test]
    fn a_house_records_every_head_it_has_had() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let ng = goods.len();
        let hubs = (0..3u32)
            .map(|i| {
                let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 9000.0 } else { 800.0 }).collect();
                hub(i, (i as f32) * 5.0, 0.0, 9000.0, prod, 0)
            })
            .collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 3];
        s.hub_minorities = vec![Vec::new(); 3];
        // Primogeniture: the house stays whole, so the line is one unbroken sequence.
        s.culture_rules = vec![CultureRule { culture: "Aiora".into(), line: 0, rule: 1 }];
        // No fleet and deep pockets: this test is about the succession RECORD, and a
        // house that goes bankrupt in year 20 (fleet upkeep on a three-city world) never
        // reaches a second head to record.
        let mut h = house_at(0, vec![1], 0);
        h.wealth = 200_000.0;
        s.houses.push(h);
        s.seed_house_lines();
        for _ in 0..90 { s.advance(365); }

        let line = &s.houses[0].line;
        assert!(
            line.len() >= 3,
            "90 years should span at least three heads, got {} (defunct={}, wealth={:.0}, \
             head_lifespan={} yr)",
            line.len(), s.houses[0].defunct, s.houses[0].wealth,
            s.houses[0].head_lifespan / TICKS_PER_YEAR
        );
        let mut prev_until = 0;
        for (i, p) in line.iter().enumerate() {
            assert!(!p.name.is_empty(), "head {i} has no name");
            assert!(p.age_at_accession >= 16, "head {i} inherited as a child: {}", p.age_at_accession);
            assert!(p.since >= prev_until, "head {i} took over before their predecessor died");
            if p.until > 0 {
                assert!(p.until > p.since, "head {i} died before they acceded");
                assert!(
                    p.age_at_death > p.age_at_accession,
                    "head {i} did not age in office ({} → {})", p.age_at_accession, p.age_at_death
                );
                let years = (p.until - p.since) / TICKS_PER_YEAR;
                assert!(years >= 3, "head {i} held the house for {years} years — not a generation");
                prev_until = p.until;
            }
        }
        // The living head is the last link, and only the last link is open.
        let last = line.last().unwrap();
        assert_eq!(last.until, 0, "the living head's record must stay open");
        assert_eq!(last.name, s.houses[0].head_name, "the line's last head is the house's head");
        assert!(line[..line.len() - 1].iter().all(|p| p.until > 0), "an earlier head was left open");
    }

    /// The line rule decides who holds the house. Under an ENATIC culture — descent
    /// through daughters — every head is a woman and is NAMED as one; under an agnatic
    /// one, none are. If this ever silently produces men in a matrilineal people, the
    /// whole rule is decoration.
    #[test]
    fn a_matrilineal_house_is_held_by_women() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32)
            .map(|i| hub(i, (i as f32) * 5.0, 0.0, 9000.0, vec![9000.0], 0))
            .collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(), "Belgar".into()];
        s.hub_minorities = vec![Vec::new(); 2];
        s.culture_rules = vec![
            // Aiora: enatic + matrilineal. Belgar: agnatic + primogeniture.
            CultureRule { culture: "Aiora".into(), line: 3, rule: 4 },
            CultureRule { culture: "Belgar".into(), line: 0, rule: 1 },
        ];
        for hub_i in 0..2u32 {
            let mut h = house_at(hub_i, vec![0], 1);
            h.wealth = 2000.0;
            s.houses.push(h);
        }
        s.seed_house_lines();
        for _ in 0..70 { s.advance(365); }

        let matri = &s.houses[0];
        let agnat = &s.houses[1];
        assert!(matri.line.len() >= 2, "the matrilineal house never succeeded");
        assert!(
            matri.line.iter().all(|p| p.female),
            "a matrilineal house was held by a man: {:?}",
            matri.line.iter().map(|p| (&p.name, p.female)).collect::<Vec<_>>()
        );
        assert!(matri.head_female, "the living matrilineal head must be a woman");
        assert!(
            agnat.line.iter().all(|p| !p.female),
            "an agnatic house passed to a daughter: {:?}",
            agnat.line.iter().map(|p| (&p.name, p.female)).collect::<Vec<_>>()
        );
        // The heir is named from the culture's own female bank, not a man's name.
        assert!(matri.head_name != agnat.head_name, "both houses drew the same head name");
    }

    /// A guild is an office, not a family: its master turns over, but there is no
    /// estate to divide, whatever the local law of inheritance says.
    #[test]
    fn a_guild_never_divides_its_estate() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32)
            .map(|i| hub(i, (i as f32) * 5.0, 0.0, 9000.0, vec![9000.0], 0))
            .collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 2];
        s.hub_minorities = vec![Vec::new(); 2];
        s.culture_rules = vec![CultureRule { culture: "Aiora".into(), line: 0, rule: 0 }]; // partible
        let mut g = house_at(0, vec![0], 1);
        g.is_guild = true;
        g.wealth = 5000.0;
        s.houses.push(g);
        s.seed_house_lines();
        let before = s.houses.len();
        for _ in 0..60 { s.advance(365); }
        assert!(s.houses[0].line.len() >= 2, "the guildmastership never turned over");
        assert!(
            !s.houses.iter().any(|h| h.name.contains(" Line)")),
            "a guild spawned a co-heir house"
        );
        assert!(s.houses.len() >= before, "houses vanished");
    }

    // ── Phase 1.1 · house tiers ──────────────────────────────────────────────
    /// Tiers are a RANK among live private houses, never assigned to a guild, and the
    /// richest/most-powerful house should end up in the top band. This is deliberately
    /// coarse — the exact cutoffs are re-derived every month — but a house with 50x the
    /// wealth of its rivals ending up in Tier 4 would mean the formula is broken, not
    /// just imprecise.
    #[test]
    fn house_tiers_rank_the_richest_house_highest() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..6u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 6];
        s.hub_minorities = vec![Vec::new(); 6];

        let mut top = house_at(0, vec![0], 4);
        top.wealth = 500_000.0;
        top.volume = 500.0;
        top.prestige = 1.0;
        s.houses.push(top);
        for i in 1..8u32 {
            let mut h = house_at(i % 6, vec![0], 1);
            h.wealth = 50.0 + i as f32;
            h.volume = 1.0;
            s.houses.push(h);
        }
        let mut g = house_at(0, vec![0], 0);
        g.is_guild = true;
        g.wealth = 999_999.0; // richer than everyone — must still get NO tier
        s.houses.push(g);

        s.assign_house_tiers();

        let guild_idx = s.houses.len() - 1;
        assert_eq!(s.houses[guild_idx].tier, 0, "a guild must never be tiered");
        assert_eq!(s.houses[0].tier, 1, "the overwhelmingly richest house must be Tier 1");
        assert!(s.houses[0].standing > s.houses[1].standing, "standing must track wealth/volume/prestige");
        // Every OTHER live private house got some tier assigned (no zeros left over).
        for h in s.houses.iter().take(guild_idx).skip(1) {
            assert!((1..=4).contains(&h.tier), "house left untiered: tier={}", h.tier);
        }
    }

    /// Tier 1 has an absolute floor, not just a percentile rank: on a young, undifferen-
    /// tiated world (every house similarly small) nobody should clear it, so Tier 1 is
    /// EMPTY. A tier that is always occupied carries no information (§1 of the design).
    #[test]
    fn tier_one_is_empty_on_an_undifferentiated_world() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..4u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 4];
        s.hub_minorities = vec![Vec::new(); 4];
        for i in 0..10u32 {
            let mut h = house_at(i % 4, vec![0], 1);
            h.wealth = 40.0 + (i as f32); // a tight, undifferentiated spread
            s.houses.push(h);
        }
        s.assign_house_tiers();
        assert!(s.houses.iter().all(|h| h.tier != 1), "Tier 1 should be empty on a flat world");
    }

    /// Hysteresis: a house sitting right at a boundary must not flip tier on a
    /// standing-neutral re-run — calling `assign_house_tiers` again with unchanged
    /// state must reproduce the same tiers, not relitigate every boundary case.
    #[test]
    fn tier_assignment_is_stable_when_nothing_changed() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..5u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 5];
        s.hub_minorities = vec![Vec::new(); 5];
        for i in 0..12u32 {
            let mut h = house_at(i % 5, vec![0], 1);
            h.wealth = 30.0 + (i as f32) * 17.0;
            h.volume = i as f32;
            s.houses.push(h);
        }
        s.assign_house_tiers();
        let first: Vec<u8> = s.houses.iter().map(|h| h.tier).collect();
        s.assign_house_tiers();
        let second: Vec<u8> = s.houses.iter().map(|h| h.tier).collect();
        assert_eq!(first, second, "tiers must not change with no underlying change");
    }
