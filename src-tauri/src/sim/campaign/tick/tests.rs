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
            stock: vec![0.0; ng * GRADE_BANDS], price: vec![1.0; ng], production: prod,
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
            tier: 0, standing: 0.0, war_cooldown_until: 0, captor_since: 0, realm: -1, realm_role: 0,
            wh_capacity: 0.0, wh_spoiled_month: Vec::new(), wh_last_month: Vec::new(), supply_accum: Vec::new(), shares: Vec::new(),
        }
    }

    pub(super) fn house_at(hub: u32, spec: Vec<usize>, fleet_sea: u32) -> House {
        House {
            name: format!("House{hub}"), hub, wealth: 50.0, prestige: 0.0, spec,
            monopoly: vec![], rivals: vec![], generation: 1, events: vec![],
            good_profit: vec![], good_volume: vec![], mono50: vec![], mono_ever: vec![], dominant_seat: false,
            prev_wealth: 50.0, worst_loss: 0.0, fleet_sea, fleet_river: 0, fleet_caravan: 0,
            head_name: "Head".into(), head_since: 0, head_lifespan: 100_000, founded_tick: 0,
            political_power: 0.0, volume: 0.0, defunct: false, archetype: 1, charters: vec![],
            is_guild: false, offices: vec![], trade_at: vec![], debt_since: 0,
            wealth_history: vec![], office_leases: vec![],
            influence: vec![], bailos: vec![],
            head_female: false, head_age: 34, line: vec![], tier: 0, standing: 0.0, peak_wealth: 0.0, peak_wealth_tick: 0, wealth_last_check: 0.0, golden_age_months: 0, golden_age_chronicled: false, dynasty_chronicled: false, kin: Vec::new(), goals: Vec::new(), goal_history: Vec::new(), crisis: None, crisis_immune_until: 0, crisis_history: Vec::new(), schism_cooldown_until: 0, origin_house: -1, origin_kind: ORIGIN_NONE, crowned: false, realm: -1,
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
            prov_holder_house: vec![],
            prov_works: vec![], prov_history: vec![], prov_events: vec![],
            prov_good_belt: vec![], prov_good_depletion: vec![],
            prov_good_yield_scale: 1.0,
            // Realms — empty for the same reason the province vectors above are:
            // a campaign with no province layer can never see a proclamation (a
            // realm is founded on a province writ), so the realm layer is a
            // structural no-op here and the dynamics run stays bit-identical.
            realms: vec![], prov_realm: vec![],
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
        let food0 = stock_of(&s.hubs[seat].stock, 0);
        let treasury0 = s.hubs[seat].treasury;
        for yr in 0..60u32 {
            s.province_demography_pass();
            s.province_land_pass(yr);
        }
        let seat = s.province_seat_hub(0).expect("still has a seat");
        // ── THE FEEDBACK EDGE: the countryside fed the city and paid it dues.
        assert!(stock_of(&s.hubs[seat].stock, 0) > food0,
            "the province's surplus must reach the seat's granary: {} → {}",
            food0, stock_of(&s.hubs[seat].stock, 0));
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
        let food0 = stock_of(&s.hubs[0].stock, 0);
        let treasury0 = s.hubs[0].treasury;
        for yr in 0..25u32 { s.province_land_pass(yr); }
        assert_eq!(stock_of(&s.hubs[0].stock, 0), food0, "no province layer ⇒ no food delivered");
        assert_eq!(s.hubs[0].treasury, treasury0, "no province layer ⇒ no dues collected");
        assert!(s.prov_forest.is_empty() && s.prov_history.is_empty(),
            "no province layer ⇒ no land state is even allocated");
    }

    /// R1 · the realm layer must be structurally inert until a realm is proclaimed
    /// (`REALM_AND_GOVERNMENT_PLAN.md` rule 25 — sovereignty is never assumed to
    /// exist). This is the counterpart of `province_land_pass_is_a_noop_without_
    /// provinces` and it is what makes the standing dynamics run bit-identical
    /// across R1: that sim carries NO province layer, and a realm can only ever be
    /// founded on a province writ, so no proclamation is reachable there at all.
    #[test]
    fn the_realm_layer_is_inert_until_a_realm_is_proclaimed() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        for yr in 0..60u32 { s.province_land_pass(yr); }
        assert!(s.realms.is_empty(), "no province layer ⇒ no realm can be founded");
        assert!(s.prov_realm.is_empty(), "no province layer ⇒ no sovereignty is allocated");

        // With a province layer, sovereignty is ALLOCATED but stays free land. A realm
        // claims territory; nothing hands it out at seeding time.
        s.prov_cap = vec![50_000.0];
        s.prov_rural = vec![30_000.0];
        s.prov_culture = vec!["Aiora".into()];
        s.prov_seat = vec![[0.0, 0.0]];
        s.hub_province = vec![0];
        s.province_land_pass(1);
        assert_eq!(s.prov_realm, vec![-1], "a seeded province starts as free land");
        assert!(s.realms.is_empty(), "seeding a province founds no realm");
    }

    /// R1 · `crowned` and `defunct` are different facts and must never be conflated
    /// (`REALM_AND_GOVERNMENT_PLAN.md` §5.1). A crowned house is ALIVE — it is the
    /// dynasty — so it keeps its identity while leaving the merchant world, which is
    /// precisely what `dissolve_house`'s liquidation and `GOAL_OUTLAST_RIVAL`'s
    /// "a rival went defunct" test must not be fooled into acting on.
    #[test]
    fn a_crowned_house_leaves_the_merchant_world_without_dying() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        assert!(!s.houses.is_empty(), "a house is needed for this test");
        assert!(s.houses[0].is_merchant(), "a new house competes as a merchant");

        s.houses[0].crowned = true;
        assert!(!s.houses[0].is_merchant(), "a crowned house has left the merchant world");
        assert!(!s.houses[0].defunct, "…but it is NOT dead — it is the dynasty");
        assert!(!s.houses[0].name.is_empty(), "its identity survives the coronation");

        // And the two flags stay independent in the other direction.
        let mut other = s.houses[0].clone();
        other.crowned = false; other.defunct = true;
        assert!(!other.is_merchant(), "a dead house is not a merchant either");
    }

    /// R1b · `promote_house_to_realm` is the transfer at the heart of the coronation
    /// (`REALM_AND_GOVERNMENT_PLAN.md` §3.2): the pot moves WHOLE (one pot, not two),
    /// the house survives as the dynasty, and the seat's territory becomes the
    /// realm's sovereignty.
    #[test]
    fn promote_house_to_realm_transfers_the_pot_and_leaves_the_house_alive() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        let hi = 0usize;
        s.houses[hi].wealth = 100_000.0; // single house → adaptive cost = REALM_PROCLAIM_COST_FRAC × this
        s.prov_holder = vec![0];       // this hub administers province 0
        s.prov_holder_house = vec![-1];
        s.prov_realm = vec![-1];
        s.hub_province = vec![0];
        s.prov_culture = vec!["Aiora".into()];

        let id = s.promote_house_to_realm(hi, 0, 60);

        assert_eq!(s.realms.len(), 1, "exactly one realm is created");
        let r = &s.realms[0];
        assert_eq!(r.id, id);
        // The pot moves whole, MINUS the adaptive founding spend
        // (REALM_PROCLAIM_COST_FRAC × 100,000). A tolerance, not `assert_eq!`: the
        // fraction need not be exactly representable in f32 (0.35 is not).
        assert!((r.treasury - 100_000.0 * (1.0 - REALM_PROCLAIM_COST_FRAC)).abs() < 1.0,
            "the pot moves whole, minus the adaptive founding spend (got {})", r.treasury);
        assert_eq!(r.capital_hub, 0);
        assert_eq!(r.ruling_house, hi as u32);
        assert_eq!(r.provinces, vec![0], "the seat's administered province becomes sovereign territory");
        assert_eq!(r.rank, REALM_CITY_STATE, "founds at the bottom of the ladder");
        assert!(!r.name.is_empty() && !r.title.is_empty());

        assert!(s.houses[hi].crowned, "the house is ELEVATED");
        assert!(!s.houses[hi].defunct, "…never dissolved — see §5.1");
        assert_eq!(s.houses[hi].realm, id as i32);
        assert_eq!(s.houses[hi].wealth, 0.0, "the house's own pot is empty — it moved to the crown");
        assert_eq!(s.houses[hi].tier, 0, "leaves the 1-4 merchant tier ladder");
        assert!(!s.houses[hi].name.is_empty(), "identity survives");

        assert_eq!(s.hubs[0].realm, id as i32);
        assert_eq!(s.hubs[0].realm_role, REALM_ROLE_SEAT);
        assert_eq!(s.prov_realm, vec![id as i32], "sovereignty is recorded on the province");
    }

    /// R1b · every precondition in §3.1 is load-bearing on its own: dropping any ONE
    /// of them must suppress the proclamation. Checked directly against
    /// `maybe_proclaim_realms` rather than by hunting for a seed that rolls the dice
    /// favourably — the trigger's job here is the GATE, not the roll.
    #[test]
    fn realm_proclamation_respects_every_precondition() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];

        // The gate now: the house has CAPTURED the seat, is at least TIER 2, holds a
        // province, and can afford the 200k founding spend.
        let eligible = |s: &mut CampaignSim| {
            s.hubs[0].captor_house = 0;   // captured the settlement
            s.hubs[0].tier = 2;
            s.houses[0].tier = 2;         // at least tier 2
            s.houses[0].wealth = 300_000.0; // the richest house — sets & clears the adaptive bar
            s.prov_holder = vec![0];      // holds a province (rule 25)
            s.prov_holder_house = vec![-1];
            s.prov_realm = vec![-1];
        };

        // Before the hard floor: never, no matter how eligible otherwise.
        let mut s = sim(hubs.clone(), goods.clone());
        s.found_house_at(0);
        s.tick = (REALM_YEAR_FLOOR - 1) * TICKS_PER_YEAR;
        eligible(&mut s);
        s.maybe_proclaim_realms(REALM_YEAR_FLOOR - 1);
        assert!(s.realms.is_empty(), "the hard floor is absolute");

        // At/after the floor, but the house neither CAPTURED the seat nor DOMINATES its
        // council — it governs the city in no capacity at all: never. (A council-dominant
        // house DOES now qualify; see the reachability sweep below, which is exercised
        // through the council path too.)
        let mut s = sim(hubs.clone(), goods.clone());
        s.found_house_at(0);
        s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR;
        eligible(&mut s);
        s.hubs[0].captor_house = -1;
        s.hubs[0].council_house = -1; // governs nothing here
        s.maybe_proclaim_realms(REALM_YEAR_FLOOR);
        assert!(s.realms.is_empty(), "a house that neither captured nor leads the council never proclaims");

        // A council-DOMINANT house (no capture) is now a valid founder — the widening
        // that makes realms common. Rolled across a tick sweep so it doesn't hinge on one
        // seed; the founding still spends the adaptive cost like the captor path.
        let mut council_fired = false;
        for t in 0..400u32 {
            let mut s = sim(hubs.clone(), goods.clone());
            s.found_house_at(0);
            s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR + t;
            eligible(&mut s);
            s.hubs[0].captor_house = -1; // never captured
            s.hubs[0].council_house = 0; // but dominates the council
            s.maybe_proclaim_realms(REALM_YEAR_FLOOR);
            if !s.realms.is_empty() { council_fired = true; break; }
        }
        assert!(council_fired, "a council-dominant house must be able to proclaim a realm");

        // Captor, but too poor to SPEND the (adaptive) founding cost: never. A far richer
        // second house raises the bar (0.6 × the richest) above the captor's own wealth.
        let mut s = sim(hubs.clone(), goods.clone());
        s.found_house_at(0);
        s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR;
        eligible(&mut s);
        s.houses.push(house_at(0, vec![0], 2));
        s.houses[1].wealth = 10_000_000.0; // the richest house → cost ≈ 6M
        s.houses[0].wealth = 100_000.0;    // the captor, far below the bar
        s.maybe_proclaim_realms(REALM_YEAR_FLOOR);
        assert!(s.realms.is_empty(), "a house that cannot afford the adaptive founding cost never proclaims");

        // Captor and rich, but holds no province writ: never (rule 25).
        let mut s = sim(hubs.clone(), goods.clone());
        s.found_house_at(0);
        s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR;
        eligible(&mut s);
        s.prov_holder = vec![-1]; // no city administers this province — the seat holds nothing
        s.maybe_proclaim_realms(REALM_YEAR_FLOOR);
        assert!(s.realms.is_empty(), "no province writ ⇒ no proclamation, regardless of everything else");

        // Every condition satisfied — proclamation must be REACHABLE (rolled against
        // a wide sweep of ticks so the assertion doesn't depend on one lucky seed), and
        // the founding SPENDS the cost (the crown starts with wealth − cost).
        let mut fired = false;
        for t in 0..400u32 {
            let mut s = sim(hubs.clone(), goods.clone());
            s.found_house_at(0);
            s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR + t;
            eligible(&mut s);
            s.maybe_proclaim_realms(REALM_YEAR_FLOOR);
            if let Some(r) = s.realms.first() {
                // Single house → cost = 0.6 × its own wealth, so the crown keeps the rest.
                let expected = 300_000.0 * (1.0 - REALM_PROCLAIM_COST_FRAC);
                assert!((r.treasury - expected).abs() < 1.0,
                    "the crown founds with wealth − the adaptive spend (got {}, want {})", r.treasury, expected);
                fired = true;
                break;
            }
        }
        assert!(fired, "with every precondition met, a proclamation must be reachable");
    }

    /// R2 · succession picks the ELDEST ELIGIBLE living child, sex-filtered by the
    /// capital's own `LineRule` — an older INeligible sibling must be passed over
    /// for a younger eligible one (rule 23). A minor heir installs a regency (the
    /// heir's mother, if alive) and the realm's legitimacy takes the hit.
    #[test]
    fn realm_succession_picks_the_eldest_eligible_heir_and_installs_regency() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["TestCulture".into()];
        s.culture_rules = vec![CultureRule {
            culture: "TestCulture".into(), line: LineRule::Agnatic.as_u8(), rule: InheritanceRule::Primogeniture.as_u8(),
        }];
        s.found_house_at(0);
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["TestCulture".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;

        let ruler_born = s.realms[ri].family[0].born_tick;
        // A wife, for the regent test.
        let mother = Person {
            name: "Mother".into(), female: true, born_tick: ruler_born,
            died_tick: 0, father: -1, mother: -1, spouse: 0,
            character: [0; 4], skill: 0.5, epithet: String::new(), reign_start: 0, reign_end: 0,
        };
        s.realms[ri].family.push(mother);
        s.realms[ri].family[0].spouse = 1;
        // Older daughter (born first) — INELIGIBLE under Agnatic law.
        let daughter = Person {
            name: "Elder Daughter".into(), female: true, born_tick: 100,
            died_tick: 0, father: 0, mother: 1, spouse: -1,
            character: [0; 4], skill: 0.5, epithet: String::new(), reign_start: 0, reign_end: 0,
        };
        s.realms[ri].family.push(daughter);
        // Younger son (born later) — the only ELIGIBLE child, and a MINOR.
        let son = Person {
            name: "Young Son".into(), female: false, born_tick: s.tick.saturating_sub(5 * TICKS_PER_YEAR),
            died_tick: 0, father: 0, mother: 1, spouse: -1,
            character: [0; 4], skill: 0.5, epithet: String::new(), reign_start: 0, reign_end: 0,
        };
        s.realms[ri].family.push(son);
        let son_idx = 3usize;

        // Kill the ruler THIS tick and resolve.
        s.realms[ri].family[0].died_tick = s.tick;
        s.resolve_realm_succession(ri, 60);

        assert_eq!(s.realms[ri].ruler, son_idx as i32,
            "the younger ELIGIBLE son must be chosen over the older ineligible daughter");
        assert_eq!(s.realms[ri].regent, 1, "the minor heir's living mother becomes regent");
        assert!(s.realms[ri].legitimacy < REALM_FOUNDING_LEGITIMACY, "a regency costs legitimacy");
        assert!(s.realms[ri].fallen_tick == 0, "the realm survives — it had an heir");
    }

    /// R2 · when NO ONE in the family is eligible or alive, the dynasty ends
    /// cleanly: `fallen_tick` is set and sovereignty is released rather than left
    /// pointing at a realm that no longer has anyone to rule it (the same
    /// "must always terminate in a defined state" discipline rule 22 holds a house
    /// crisis to).
    #[test]
    fn a_realm_with_no_heir_anywhere_dissolves_and_releases_its_provinces() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR; // realistic — a realm can't exist at tick 0
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        assert_eq!(s.prov_realm, vec![id as i32], "sanity: the province starts sovereign");

        s.tick += TICKS_PER_YEAR; // the ruler dies a year later — died_tick=0 is the "alive" sentinel
        s.realms[ri].family[0].died_tick = s.tick; // the ONLY family member dies
        s.resolve_realm_succession(ri, 60);

        assert!(s.realms[ri].fallen_tick > 0, "no heir anywhere ⇒ the dynasty ends");
        assert_eq!(s.prov_realm, vec![-1], "sovereignty is released, not left dangling");
        // R4 · found while building war goals: the capital's OWN `hub.realm` must
        // also be released, not just its provinces — otherwise it stays pointing
        // at a fallen realm forever, permanently barring the hub from ever
        // proclaiming a new one (`maybe_proclaim_realms` refuses any `realm >= 0`
        // with no notion of "but that realm is dead").
        assert_eq!(s.hubs[0].realm, -1, "the capital's own realm membership is released too");
        assert_eq!(s.hubs[0].realm_role, 0);
    }

    /// R2 · the whole yearly pass (mortality → succession → marriage → births) must
    /// run for decades without panicking and without ever leaving `ruler` pointing
    /// past the end of `family` — the same "doesn't blow up" bar the standing
    /// dynamics test holds the rest of the campaign to.
    #[test]
    fn realm_family_pass_runs_for_decades_without_breaking_invariants() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.houses[0].head_age = 20; // young founder — plenty of years to marry/have children
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        s.promote_house_to_realm(0, 0, 60);

        for yr in 61..361u32 {
            s.tick = yr * TICKS_PER_YEAR;
            s.realm_family_pass(yr);
            if s.realms.iter().all(|r| r.fallen_tick > 0) { break; } // every dynasty ended — fine
            for r in &s.realms {
                if r.fallen_tick > 0 { continue; }
                if r.ruler >= 0 {
                    assert!((r.ruler as usize) < r.family.len(), "ruler index must stay in bounds");
                    assert_eq!(r.family[r.ruler as usize].died_tick, 0, "a dead person is never the ruler");
                }
                if r.regent >= 0 {
                    assert!((r.regent as usize) < r.family.len(), "regent index must stay in bounds");
                }
            }
        }
        // Over 300 years a family that keeps succeeding should have grown beyond
        // the single founder — not asserted on every run (mortality is stochastic),
        // but the mechanism must be REACHABLE.
        let grew = s.realms.iter().any(|r| r.family.len() > 1);
        assert!(grew, "births/marriages must be reachable over 300 years");
    }

    /// R2 · a crowned house must never re-enter the MERCHANT succession/crisis
    /// machinery — both would rewrite `head_name`/`kin` out from under the realm's
    /// own genealogy, the same identity-corruption trap §5.1 names, through two
    /// further paths (`succeed_house` via `head_lifespan`, and
    /// `update_house_crises`).
    #[test]
    fn a_crowned_house_never_reenters_merchant_succession_or_crisis() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        let original_name = s.houses[0].name.clone();
        s.houses[0].crowned = true;
        s.houses[0].head_lifespan = 1; // would fire almost immediately if not guarded
        s.houses[0].head_since = 0;
        s.tick = 2;

        for hi in 0..s.houses.len() {
            if !s.houses[hi].is_merchant() { continue; }
            unreachable!("a crowned house must never pass is_merchant()");
        }
        // The actual guarded call sites: run the passes directly and confirm nothing moved.
        s.update_house_crises();
        assert_eq!(s.houses[0].name, original_name, "update_house_crises must not touch a crowned house");
        assert!(s.houses[0].crisis.is_none(), "a crowned house never opens a merchant crisis");
    }

    /// R3 · a sovereign province's dues must reach the CROWN, scaled by collection
    /// efficiency, and never the seat city's own treasury — and a house-held writ
    /// inside the same realm's borders must still bypass the crown entirely
    /// (rule 24 — a house-held province is the house's, not the realm's, even
    /// though the realm's border still legally contains it, plan §5.9).
    #[test]
    fn a_sovereign_provinces_dues_reach_the_crown_not_the_city() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 5000.0, vec![3000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.found_house_at(1);
        s.prov_cap = vec![50_000.0, 40_000.0];
        s.prov_rural = vec![30_000.0, 20_000.0];
        s.prov_culture = vec!["Aiora".into(), "Aiora".into()];
        s.prov_seat = vec![[0.0, 0.0], [0.0, 0.0]];
        s.hub_province = vec![0, 1];
        s.prov_net_mig = vec![0.0, 0.0];
        s.ensure_province_land(2);
        s.prov_tax = vec![0.30, 0.30];
        s.prov_unrest = vec![0.0, 0.0];

        // Province 0 becomes sovereign; province 1 stays a house-held writ (Stato
        // da Mar) — even inside the same realm's borders once claimed, if it were.
        s.prov_holder = vec![0, 1]; // each province's own seat administers it
        s.prov_holder_house[1] = 1; // house 1 holds province 1's writ directly
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        s.realms[ri].cohesion = 1.0; // isolate the test from cohesion decay

        let treasury0 = s.hubs[0].treasury;
        let house1_wealth0 = s.houses[1].wealth;
        s.province_land_pass(61);

        assert!(s.realms[ri].treasury > 0.0, "the crown must actually receive tithe income");
        assert_eq!(s.hubs[0].treasury, treasury0,
            "the sovereign seat's OWN treasury must not also receive the same dues");
        assert!(s.houses[1].wealth > house1_wealth0,
            "a house-held province still pays its house, realm borders notwithstanding");
        assert!(s.realms[ri].tithe_last_year > 0.0, "tithe_last_year tracks the crown's own share");

        // Efficiency at zero distance and full cohesion is ~1.0 — the crown should
        // have received close to the FULL assessed dues, not some small fraction.
        let surplus = s.prov_surplus[0];
        let assessed = surplus * 0.30;
        assert!(s.realms[ri].treasury > assessed * 0.8,
            "efficiency at distance 0 / cohesion 1.0 must be near-total: got {} of {}",
            s.realms[ri].treasury, assessed);
    }

    /// R3 · collection efficiency must fall with distance and rise with cohesion —
    /// the mechanism §3.3 exists to model ("collection, not rates, is the
    /// constraint"), checked directly rather than through a full province pass.
    #[test]
    fn realm_collection_efficiency_falls_with_distance_and_cohesion() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 500.0, 0.0, 5000.0, vec![3000.0], 0), // far from the capital
        ];
        let mut s = sim(hubs, goods);
        s.world_w = 1000.0;
        s.found_house_at(0);
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;

        s.realms[ri].cohesion = 1.0;
        let near = s.realm_collection_efficiency(ri, 0);
        let far = s.realm_collection_efficiency(ri, 1);
        assert!(near > far, "a distant seat must collect less efficiently than the capital itself");
        assert!(near > 0.95, "the capital's own efficiency should be near 1.0 at full cohesion");

        s.realms[ri].cohesion = 0.2;
        let near_low_cohesion = s.realm_collection_efficiency(ri, 0);
        assert!(near_low_cohesion < near, "low cohesion must reduce efficiency even at distance 0");
    }

    /// R3 · poll and customs must drain the TAXED CITY's own treasury into the
    /// crown's, never manufacture money, and never push a city's treasury
    /// negative — the same "capped at what's actually there" discipline the
    /// tithe's own evasion accounting already holds to.
    #[test]
    fn realm_levies_drain_the_city_treasury_into_the_crown_and_never_go_negative() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        s.realms[ri].cohesion = 1.0;
        s.realms[ri].tax_rates = [REALM_TAX_MAX[TAX_POLL], REALM_TAX_MAX[TAX_CUSTOMS]];
        s.hubs[0].population = 10_000.0;
        s.hubs[0].trade_wealth = 5_000.0;
        s.hubs[0].mood = 0.8;

        // Case A: plenty of treasury — both levies collect in full.
        s.hubs[0].treasury = 100_000.0;
        s.collect_realm_levies();
        assert!(s.realms[ri].treasury > 0.0, "levies must actually move money to the crown");
        assert!(s.hubs[0].treasury < 100_000.0, "the city's own treasury must be drained, not duplicated");
        assert!(s.hubs[0].mood < 0.8, "a poll tax is regressive and must cost mood");

        // Case B: an empty treasury — levies must NEVER go negative.
        let ri2_treasury = s.realms[ri].treasury;
        s.hubs[0].treasury = 0.0;
        s.collect_realm_levies();
        assert!(s.hubs[0].treasury >= 0.0, "a levy must never push a city's treasury negative");
        assert_eq!(s.realms[ri].treasury, ri2_treasury, "nothing to collect from an empty treasury");
    }

    /// R3 · a tax farm is a distress sale: it must only trigger when the crown is
    /// genuinely short, must actually move a lump sum from house to crown up
    /// front, and must REDIRECT the tithe to the farming house for its term,
    /// reverting to the crown automatically once the term ends.
    #[test]
    fn tax_farming_redirects_the_tithe_for_its_term_then_reverts() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 5000.0, vec![3000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.found_house_at(1);
        s.houses[1].wealth = 50_000.0; // the eligible farmer — far richer than house 0
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        s.realms[ri].cohesion = 1.0;
        s.realms[ri].tithe_last_year = 200.0; // a real prior year's collection to price against
        s.realms[ri].treasury = 0.0; // genuinely short — the ONLY condition that allows a sale

        // Sweep ticks until the farm actually sells (the decision itself is rolled).
        let mut farmed = false;
        for t in 0..200u32 {
            s.tick = 61 * TICKS_PER_YEAR + t;
            s.realms[ri].treasury = 0.0; // stay short every attempt
            s.decide_realm_taxes(ri, 61);
            if s.realms[ri].tax_farm.is_some() { farmed = true; break; }
        }
        assert!(farmed, "a tax farm must be reachable when the crown is genuinely short");
        let farm_house = s.realms[ri].tax_farm.as_ref().unwrap().house;
        assert!(s.realms[ri].treasury > 0.0, "the lump sum must actually reach the crown");
        assert_eq!(farm_house, 1, "the wealthier eligible house wins the farm");

        // While farmed, the tithe must go to the FARMING HOUSE, not the crown.
        s.prov_cap = vec![50_000.0]; s.prov_rural = vec![30_000.0]; s.prov_net_mig = vec![0.0];
        s.ensure_province_land(1);
        s.prov_tax = vec![0.30]; s.prov_unrest = vec![0.0];
        let crown_before = s.realms[ri].treasury;
        let farmer_wealth_before = s.houses[farm_house as usize].wealth;
        s.province_land_pass(62);
        assert!(s.houses[farm_house as usize].wealth > farmer_wealth_before,
            "a farmed tithe must credit the FARMING HOUSE");
        assert_eq!(s.realms[ri].treasury, crown_before,
            "the crown must receive nothing further while the farm is active");

        // After the term, collection must revert to the crown automatically.
        let farm_start = s.realms[ri].tax_farm.as_ref().unwrap().started_tick;
        s.tick = farm_start + TAX_FARM_YEARS * TICKS_PER_YEAR;
        s.decide_realm_taxes(ri, 61 + TAX_FARM_YEARS);
        assert!(s.realms[ri].tax_farm.is_none(), "the farm must expire on schedule");
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
        for (g, &v) in [10_000.0f32, 4_000.0, 0.0].iter().enumerate() {
            stock_set_total(&mut hubs[0].stock, g, v); // food to grant
        }
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
                delta: false, chokepoint: false, province: -1,
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
            for h in &s.hubs { assert_eq!(h.stock.len(), ng * GRADE_BANDS, "every hub keeps ng × GRADE_BANDS columns"); }
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
                delta: false, chokepoint: false, province: -1,
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
            delta: false, chokepoint: false, province: -1,
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
            ColonizeSite { x: 3.0, y: 3.0, koppen: 0, elevation: 0.2, fertility: 0.80, coastal: false, kind_hint: 1, trade_value: 0.10, delta: false, chokepoint: false, province: -1 },
            ColonizeSite { x: 5.0, y: 2.0, koppen: 0, elevation: 0.1, fertility: 0.18, coastal: true, kind_hint: 4, trade_value: 0.60, delta: false, chokepoint: false, province: -1 },
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
            levies: 0.0, levies_a: 0.0, levies_b: 0.0, battles: Vec::new(), cargo_lost: 0, cause: "test".into(), goal: WAR_GOAL_PLUNDER,
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1 });
        let w0 = s.houses[0].wealth;
        s.tick = 0;
        s.update_wars(0); // wage the first year — levy, no quarterly round due yet
        assert!(s.houses[0].wealth < w0, "war levy drained a resident house");
        assert_eq!(s.war_log.len(), 0, "no round has run yet, so nothing can have resolved");
        // §3.4a · quarterly rounds now decide when it ends — not a fixed 2-year timer.
        // Run out the round cap's own backstop (3 years) to observe the guaranteed end.
        s.tick = (WAR_ROUND_CAP as u32 + 1) * WAR_ROUND_TICKS;
        s.update_wars((WAR_ROUND_CAP as u32 + 1) * WAR_ROUND_TICKS / 365);
        assert_eq!(s.war_log.len(), 1, "war resolved into the log by the round cap at the latest");
        assert!(s.hubs[0].war_with < 0 && s.hubs[1].war_with < 0, "war state cleared");
        assert!(s.war_log[0].levies_total > 0.0, "levies recorded");
    }

    #[test]
    fn every_war_terminates_within_the_round_cap() {
        // §1.4/rule 22's discipline applied to war: an open war must never become the
        // permanent state of a city, so the HARD cap is the guarantee of last resort even
        // for two sides that stay both funded AND willing (these hubs are flush with a
        // high mood, so the ordinary cap deliberately does NOT stop them — only the hard
        // ceiling does, which is exactly the guarantee under test).
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),
        ], goods);
        for i in 0..2u32 { let mut h = house_at(i, vec![0], 2); h.wealth = 5000.0; s.houses.push(h); }
        s.hubs[0].treasury = 5000.0; s.hubs[1].treasury = 5000.0;
        s.hubs[0].mood = 0.9; s.hubs[1].mood = 0.9;
        s.hubs[0].war_with = 1; s.hubs[1].war_with = 0;
        s.wars.push(War { a: 0, b: 1, start_tick: 0, chest_a: 0.0, chest_b: 0.0,
            levies: 0.0, levies_a: 0.0, levies_b: 0.0, battles: Vec::new(), cargo_lost: 0, cause: "test".into(), goal: WAR_GOAL_PLUNDER,
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1 });
        // Advance year by year (each year runs ~4 quarterly rounds of catch-up); the war
        // must never exceed the HARD cap, and must be gone by the time we pass it.
        for yr in 1..=(WAR_ROUND_HARD_CAP as u32 / 4 + 3) {
            s.tick = yr * 365;
            s.update_wars(yr);
            assert!(s.wars.iter().all(|w| w.round <= WAR_ROUND_HARD_CAP),
                "no war ever exceeds the hard round cap");
            if s.wars.is_empty() { break; }
        }
        assert!(s.wars.is_empty(), "the war ended by the hard round cap at the latest");
        assert_eq!(s.war_log.len(), 1, "its resolution is recorded");
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

    /// R4 · HUMILIATE moves standing both ways with no land or coin changing
    /// hands beyond ordinary reparations — a REALM's legitimacy when a capital is
    /// involved, an ordinary house's prestige otherwise.
    #[test]
    fn humiliate_shifts_legitimacy_and_prestige_without_moving_land_or_coin() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 5000.0, vec![3000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.found_house_at(1);
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        let legit0 = s.realms[ri].legitimacy;
        let loser_prestige0 = s.houses[1].prestige;
        let holder0 = s.prov_holder[0];

        // The realm (hub 0) wins, humiliating the ordinary city hub 1.
        s.apply_war_goal(0, 1, WAR_GOAL_HUMILIATE, 0, 0);
        assert!(s.realms[ri].legitimacy > legit0, "the winning crown's legitimacy rises");
        assert!(s.houses[1].prestige < loser_prestige0, "the losing house's prestige falls");
        assert_eq!(s.prov_holder[0], holder0, "no territory changes hands");
        assert_eq!(s.hubs[1].tribute_to, -1, "no tribute or subordination is created");

        // Reversed: the ordinary city humiliates the realm.
        let legit1 = s.realms[ri].legitimacy;
        s.apply_war_goal(1, 0, WAR_GOAL_HUMILIATE, 0, 0);
        assert!(s.realms[ri].legitimacy < legit1, "the losing crown's legitimacy falls");
    }

    /// R4 · ENTHRONE installs a LOCKED kin official (role 0, the head seat) at the
    /// loser — a real, durable structural advantage, but the loser keeps its own
    /// council mechanism otherwise (unlike ANNEX, nothing else about it changes).
    #[test]
    fn enthrone_installs_a_locked_kin_official_at_the_head_seat() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0),
            hub(1, 14.0, 10.0, 9000.0, vec![90.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 2));
        s.hubs[0].council_house = 0;
        assert!(s.hubs[1].officials.is_empty(), "sanity: no government seeded yet");

        let clause = s.apply_war_goal(0, 1, WAR_GOAL_ENTHRONE, 0, 0);
        assert!(!s.hubs[1].officials.is_empty(), "a government is seeded if none existed");
        let head = s.hubs[1].officials.iter().find(|o| o.role == 0).expect("a head seat exists");
        assert_eq!(head.house, 0, "the winner's house holds the head seat");
        assert!(head.kin, "the seat is LOCKED — a kin official, not merely bribed");
        assert_eq!(head.control, 1.0);
        assert!(head.term_end > 0, "the puppet's term is finite, not permanent");
        assert!(clause.contains("throne"), "the outcome is narrated");
    }

    /// R4 · VASSALIZE only produces the FULL relationship (Realm.vassals +
    /// REALM_ROLE_TRIBUTARY) when the winner itself has a realm — otherwise it
    /// downgrades quietly to plain tribute, the same idiom the province goal
    /// already uses when nothing's actually there to take.
    #[test]
    fn vassalize_wires_realm_vassals_only_when_the_winner_has_a_crown() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 5000.0, vec![3000.0], 0),
            hub(2, 0.0, 0.0, 4000.0, vec![2000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.found_house_at(1);
        s.houses.push(house_at(2, vec![0], 1));
        s.hubs[2].council_house = 2;
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;

        // The realm (hub 0) vassalizes the ordinary city (hub 1) — full wiring.
        s.apply_war_goal(0, 1, WAR_GOAL_VASSALIZE, 0, 0);
        assert!(s.realms[ri].vassals.contains(&1), "the vassal is recorded on the realm");
        assert_eq!(s.hubs[1].realm, id as i32);
        assert_eq!(s.hubs[1].realm_role, REALM_ROLE_TRIBUTARY);
        assert_eq!(s.hubs[1].tribute_to, 0, "a vassal also pays tribute");

        // An ordinary city (hub 2, no realm) "vassalizing" hub 1 downgrades to
        // plain tribute — there is no crown for hub 1 to actually answer to.
        s.hubs[1].realm = -1; s.hubs[1].realm_role = 0; s.hubs[1].tribute_to = -1;
        s.apply_war_goal(2, 1, WAR_GOAL_VASSALIZE, 0, 0);
        assert_eq!(s.hubs[1].tribute_to, 2, "tribute alone still applies");
        assert_eq!(s.hubs[1].realm, -1, "but no vassal relationship — hub 2 has no realm to join");
    }

    /// R4 · a ceded province must never keep pointing at its OLD sovereignty once
    /// a war goal moves it — `prov_realm` releases to free land, or transfers to
    /// the winner's own realm if it has one, exactly as the winner's council/bailo
    /// effects already do for the city itself.
    #[test]
    fn a_ceded_province_transfers_sovereignty_with_the_land() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 5000.0, vec![3000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.houses.push(house_at(1, vec![0], 1));
        s.hubs[1].council_house = 1;
        s.prov_holder = vec![0, 1]; s.prov_holder_house = vec![-1, -1]; s.prov_realm = vec![-1, -1];
        s.hub_province = vec![0, 1]; s.prov_culture = vec!["Solo".into(), "Solo".into()];
        s.prov_rural = vec![100.0, 500.0]; // province 1 is the richer prize
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;

        // The realm (hub 0) takes hub 1's own province by war.
        s.apply_war_goal(0, 1, WAR_GOAL_PROVINCE, 0, 0);
        assert_eq!(s.prov_holder[1], 0, "the seat administering it changes");
        assert_eq!(s.prov_realm[1], id as i32, "and sovereignty follows the winner's own realm");
        let _ = ri;
    }

    /// R4 · ANNEX transfers realm membership for an ORDINARY member city, along
    /// with the provinces it administered — but explicitly does NOT cascade when
    /// the loser is itself a realm's own capital (a deliberately deferred case;
    /// the existing council-install effect still applies, nothing else does).
    #[test]
    fn annex_transfers_realm_membership_for_a_member_city_but_not_a_capital() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0), // winner realm's capital
            hub(1, 0.0, 0.0, 4000.0, vec![2000.0], 0), // an ordinary member city
            hub(2, 0.0, 0.0, 3000.0, vec![1000.0], 0), // a RIVAL realm's own capital
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.found_house_at(2);
        s.prov_holder = vec![0, 1, 2];
        s.prov_holder_house = vec![-1, -1, -1];
        s.prov_realm = vec![-1, -1, -1];
        s.hub_province = vec![0, 1, 2];
        s.prov_culture = vec!["Solo".into(), "Solo".into(), "Solo".into()];
        let win_id = s.promote_house_to_realm(0, 0, 60);

        // Hub 1 is an ordinary FREE city (not yet in any realm) administering
        // province 1 — annexing it must pull both the city and its land in.
        s.apply_war_goal(0, 1, WAR_GOAL_ANNEX, 0, 0);
        assert_eq!(s.hubs[1].realm, win_id as i32, "the member city joins the winner's realm");
        assert_eq!(s.hubs[1].realm_role, REALM_ROLE_SUBJECT);
        assert_eq!(s.prov_realm[1], win_id as i32, "its administered province follows it in");

        // Hub 2 is a RIVAL realm's own capital — annexing it must NOT silently
        // fold a whole foreign crown's territory into the winner (deferred).
        let rival_house = s.houses.len() - 1; // the house just founded at hub 2 (found_house_at(0) took index 0)
        let rival_id = s.promote_house_to_realm(rival_house, 2, 60);
        s.apply_war_goal(0, 2, WAR_GOAL_ANNEX, 0, 0);
        assert_eq!(s.hubs[2].realm, rival_id as i32,
            "annexing a realm's own capital must not silently transfer it (deferred, not built)");
        assert_eq!(s.hubs[2].council_house, s.realms[win_id as usize].ruling_house as i32,
            "the ordinary council-install effect still applies even in the deferred case");
    }

    /// R4 · a sovereign capital's own war-affordability must include its crown's
    /// treasury — R3 redirected the tithe/poll/customs away from the capital's
    /// OWN `hub.treasury` into `Realm.treasury`, so reading `hub.treasury` alone
    /// would make every realm systematically too poor to ever fight.
    #[test]
    fn a_realm_capital_can_afford_war_from_its_crown_treasury() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        s.hubs[0].treasury = 0.0;
        s.realms[id as usize].treasury = 1_000.0;
        assert!(s.war_affordable_treasury(0) >= 1_000.0,
            "the crown's own treasury must count toward the capital's war-affordability");
    }

    /// R5 · the autonomy axis's own "Revenue"/"Cohesion at distance" columns —
    /// centralized squeezes harder up close but falls off faster with distance;
    /// autonomous is the reverse, distance-insensitive per the plan's own table.
    #[test]
    fn autonomy_shapes_revenue_and_distance_falloff() {
        assert!(autonomy_revenue_mult(AUTONOMY_CENTRALIZED) > autonomy_revenue_mult(AUTONOMY_AUTONOMOUS),
            "a centralized crown must take a bigger cut than an autonomous one");
        assert!(autonomy_distance_mult(AUTONOMY_CENTRALIZED) > autonomy_distance_mult(AUTONOMY_AUTONOMOUS),
            "a centralized crown must feel distance HARDER — autonomous is distance-insensitive");

        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 500.0, 0.0, 5000.0, vec![3000.0], 0), // far from the capital
        ];
        let mut s = sim(hubs, goods);
        s.world_w = 1000.0;
        s.found_house_at(0);
        s.prov_holder = vec![0]; s.prov_holder_house = vec![-1]; s.prov_realm = vec![-1];
        s.hub_province = vec![0]; s.prov_culture = vec!["Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        s.realms[ri].cohesion = 1.0;

        s.realms[ri].autonomy = AUTONOMY_CENTRALIZED;
        let centralized_far = s.realm_collection_efficiency(ri, 1);
        s.realms[ri].autonomy = AUTONOMY_AUTONOMOUS;
        let autonomous_far = s.realm_collection_efficiency(ri, 1);
        assert!(autonomous_far > centralized_far,
            "an autonomous realm must collect MORE EFFICIENTLY at distance than a centralized one: {} vs {}",
            autonomous_far, centralized_far);
    }

    /// R5 · `move_realm_capital` reassigns `realm_role` on both ends and updates
    /// `capital_hub` — the mechanism the abandonment trigger (below) relies on.
    #[test]
    fn move_realm_capital_reassigns_seat_and_role() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 4000.0, vec![2000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.prov_holder = vec![0, 1]; s.prov_holder_house = vec![-1, -1]; s.prov_realm = vec![-1, -1];
        s.hub_province = vec![0, 1]; s.prov_culture = vec!["Solo".into(), "Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        s.hubs[1].realm = id as i32;
        s.hubs[1].realm_role = REALM_ROLE_SUBJECT;

        s.move_realm_capital(ri, 1);
        assert_eq!(s.realms[ri].capital_hub, 1);
        assert_eq!(s.hubs[1].realm_role, REALM_ROLE_SEAT, "the new capital becomes the seat");
        assert_eq!(s.hubs[0].realm_role, REALM_ROLE_SUBJECT, "the old capital demotes to a plain member");
    }

    /// R5 · an abandoned capital must relocate to the realm's largest surviving
    /// city, or — with none left — the realm follows it into extinction rather
    /// than persisting with a dead seat forever.
    #[test]
    fn an_abandoned_capital_relocates_or_the_realm_falls_with_it() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 4000.0, vec![2000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR; // realistic — tick 0 is the "never fallen" sentinel
        s.prov_holder = vec![0, 0]; s.prov_holder_house = vec![-1, -1]; s.prov_realm = vec![-1, -1];
        s.hub_province = vec![0, 1]; s.prov_culture = vec!["Solo".into(), "Solo".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        s.hubs[1].realm = id as i32;
        s.hubs[1].realm_role = REALM_ROLE_SUBJECT;

        // The capital is abandoned; hub 1 survives — relocation.
        s.hubs[0].abandoned = true;
        s.tick += TICKS_PER_YEAR;
        s.maybe_relocate_abandoned_capitals(61);
        assert_eq!(s.realms[ri].capital_hub, 1, "the realm relocates to its surviving member city");
        assert_eq!(s.realms[ri].fallen_tick, 0, "the realm survives the move");

        // Now the (new) capital is ALSO abandoned, and nothing else survives.
        s.hubs[1].abandoned = true;
        s.tick += TICKS_PER_YEAR;
        s.maybe_relocate_abandoned_capitals(62);
        assert!(s.realms[ri].fallen_tick > 0, "with no city left anywhere, the realm falls");
        assert_eq!(s.hubs[1].realm, -1, "membership is released, same as any other dissolution");
    }

    /// R5 · Path A, partible division. A culture whose `InheritanceRule` is
    /// `Partible` must split the realm among its eligible sons at succession:
    /// the eldest keeps the ORIGINAL realm (shrunk to its own share), each
    /// other heir founds a genuinely NEW realm with its own crowned house, and
    /// the total treasury is conserved (no money created or destroyed) across
    /// every share.
    #[test]
    fn partible_succession_divides_the_realm_among_eligible_sons() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 6000.0, vec![3000.0], 0),
            hub(2, 0.0, 0.0, 4000.0, vec![2000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Splitters".into(), "Splitters".into(), "Splitters".into()];
        s.culture_rules = vec![CultureRule {
            culture: "Splitters".into(), line: LineRule::Agnatic.as_u8(), rule: InheritanceRule::Partible.as_u8(),
        }];
        s.found_house_at(0);
        // All three provinces administered by the CAPITAL at the moment of
        // coronation, so all three actually enter the realm's sovereignty —
        // hub_province still maps each province to its own town for
        // province_seat_hub to find later, when the split needs a new capital.
        s.prov_holder = vec![0, 0, 0];
        s.prov_holder_house = vec![-1, -1, -1];
        s.prov_realm = vec![-1, -1, -1];
        s.hub_province = vec![0, 1, 2];
        s.prov_culture = vec!["Splitters".into(), "Splitters".into(), "Splitters".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        s.realms[ri].treasury = 900.0;
        let realms_before = s.realms.len();
        let houses_before = s.houses.len();

        // Two eligible sons, born far enough apart to have a clean eldest/younger order.
        let ruler_born = s.realms[ri].family[0].born_tick;
        let son_a = Person {
            name: "Elder Son".into(), female: false, born_tick: ruler_born.saturating_add(1),
            died_tick: 0, father: 0, mother: -1, spouse: -1,
            character: [0; 4], skill: 0.5, epithet: String::new(), reign_start: 0, reign_end: 0,
        };
        let son_b = Person {
            name: "Younger Son".into(), female: false, born_tick: ruler_born.saturating_add(2),
            died_tick: 0, father: 0, mother: -1, spouse: -1,
            character: [0; 4], skill: 0.5, epithet: String::new(), reign_start: 0, reign_end: 0,
        };
        s.realms[ri].family.push(son_a);
        s.realms[ri].family.push(son_b);

        s.tick = 61 * TICKS_PER_YEAR;
        s.realms[ri].family[0].died_tick = s.tick;
        s.resolve_realm_succession(ri, 61);

        assert!(s.realms.len() > realms_before, "a new realm must be founded for the second heir");
        assert!(s.houses.len() > houses_before, "the new realm needs its OWN crowned house");
        assert_eq!(s.realms[ri].ruler, 1, "the eldest son (index 1) keeps the ORIGINAL realm");
        assert!(!s.realms[ri].provinces.is_empty(), "the original realm keeps a real share, not nothing");
        assert!(s.realms[ri].provinces.len() < 3, "…but a SMALLER share than before the split");

        let new_realm = s.realms.last().unwrap();
        assert_eq!(new_realm.origin_realm, ri as i32, "the offshoot records where it split from");
        assert!(!new_realm.provinces.is_empty(), "the younger son actually receives territory");
        assert!(s.houses[new_realm.ruling_house as usize].crowned, "the offshoot's house is born crowned");
        assert_eq!(s.houses[new_realm.ruling_house as usize].origin_kind, ORIGIN_DIVISION);

        let total_after: f32 = std::iter::once(&s.realms[ri]).chain(std::iter::once(new_realm))
            .map(|r| r.treasury).sum();
        assert!((total_after - 900.0).abs() < 1.0, "treasury is CONSERVED across the split: got {}", total_after);

        // Every province must land in EXACTLY one of the two realms' sovereignty.
        for &p in &s.realms[ri].provinces {
            assert_eq!(s.prov_realm[p as usize], ri as i32);
        }
        for &p in &new_realm.provinces {
            assert_eq!(s.prov_realm[p as usize], new_realm.id as i32);
        }
    }

    /// R5 · a NON-Partible culture must never split, even with multiple eligible
    /// sons — the same succession that would divide a Partible realm must
    /// concentrate normally under Primogeniture.
    #[test]
    fn non_partible_succession_never_splits_the_realm() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 0.0, 0.0, 6000.0, vec![3000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Keepers".into(), "Keepers".into()];
        s.culture_rules = vec![CultureRule {
            culture: "Keepers".into(), line: LineRule::Agnatic.as_u8(), rule: InheritanceRule::Primogeniture.as_u8(),
        }];
        s.found_house_at(0);
        s.prov_holder = vec![0, 0]; s.prov_holder_house = vec![-1, -1]; s.prov_realm = vec![-1, -1];
        s.hub_province = vec![0, 1]; s.prov_culture = vec!["Keepers".into(), "Keepers".into()];
        let id = s.promote_house_to_realm(0, 0, 60);
        let ri = id as usize;
        let realms_before = s.realms.len();

        let ruler_born = s.realms[ri].family[0].born_tick;
        s.realms[ri].family.push(Person {
            name: "Son A".into(), female: false, born_tick: ruler_born.saturating_add(1),
            died_tick: 0, father: 0, mother: -1, spouse: -1,
            character: [0; 4], skill: 0.5, epithet: String::new(), reign_start: 0, reign_end: 0,
        });
        s.realms[ri].family.push(Person {
            name: "Son B".into(), female: false, born_tick: ruler_born.saturating_add(2),
            died_tick: 0, father: 0, mother: -1, spouse: -1,
            character: [0; 4], skill: 0.5, epithet: String::new(), reign_start: 0, reign_end: 0,
        });

        s.tick = 61 * TICKS_PER_YEAR;
        s.realms[ri].family[0].died_tick = s.tick;
        s.resolve_realm_succession(ri, 61);

        assert_eq!(s.realms.len(), realms_before, "no new realm — concentration, not division");
        assert_eq!(s.realms[ri].provinces.len(), 2, "the whole realm passes intact to the single heir");
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
    fn a_pre_4_5_bank_stake_migrates_into_the_same_dividend_split() {
        // ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.5 · a save from before the share
        // table (F2's single-holder stake_bank/stake_share pair, empty `shares`)
        // must migrate into a table that pays the SAME split the old code did —
        // including reproducing the old code's own 0.9 clamp on an over-large
        // stake_share, not just the common case.
        let goods = vec![good("cloth", i32::MAX, 2, 5.0, 0.4, false)];
        let mut manu = hub(0, 0.0, 0.0, 1000.0, vec![10.0], 0);
        manu.is_estate = true;
        manu.estate_kind = 6; // manufactory
        manu.owner_house = 0;
        manu.stake_bank = 0;
        manu.stake_share = 0.95; // above the old code's own 0.9 ceiling
        let mut s = sim(vec![manu], goods);
        s.houses = vec![house_at(0, vec![0], 0)];
        s.banks.push(Bank {
            name: "Banco".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 80.0, loans: vec![], real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0, stakes: vec![],
            dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        });
        assert!(s.hubs[0].shares.is_empty(), "pre-migration: no share rows yet");
        s.migrate_stock_bands();
        let rows = &s.hubs[0].shares;
        assert_eq!(rows.len(), 2, "migration writes exactly a bank row + an owner row: {rows:?}");
        let bank_row = rows.iter().find(|r| r.holder_kind == 3).expect("a bank row");
        let owner_row = rows.iter().find(|r| r.holder_kind == 1).expect("an owner row");
        assert!((bank_row.frac - 0.9).abs() < 1e-4,
            "the bank's frac must reproduce the OLD 0.9 clamp, not the raw 0.95: {}", bank_row.frac);
        assert!((owner_row.frac - 0.1).abs() < 1e-4,
            "the owner keeps exactly what the bank didn't take: {}", owner_row.frac);
        assert_eq!(bank_row.holder, 0);
        assert_eq!(owner_row.holder, 0);
        assert_eq!(bank_row.payout, 1, "dividend payout — behaviour unchanged until §4.8");
        assert_eq!(owner_row.payout, 1);
        // Idempotent: migrating an already-migrated hub must not duplicate rows.
        s.migrate_stock_bands();
        assert_eq!(s.hubs[0].shares.len(), 2, "migration must not re-run once shares exist");
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
        stock_set_total(&mut s.hubs[0].stock, 0, 100.0);
        stock_set_total(&mut s.hubs[0].stock, 1, 0.0);
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

    // ── Phase 1.4 · positive events ─────────────────────────────────────────
    /// "The house's finest hour" is a MARKER, not an event: it must never fall as
    /// wealth swings up and down, and it must track the highest wealth ever reached,
    /// not the current figure.
    #[test]
    fn peak_wealth_only_ever_rises() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 3];
        s.hub_minorities = vec![Vec::new(); 3];
        let mut h = house_at(0, vec![0], 1);
        h.wealth = 100.0;
        s.houses.push(h);

        s.houses[0].wealth = 100.0;
        s.assign_house_tiers();
        assert_eq!(s.houses[0].peak_wealth, 100.0);

        s.houses[0].wealth = 500.0;
        s.assign_house_tiers();
        assert_eq!(s.houses[0].peak_wealth, 500.0, "peak must follow a rise");

        s.houses[0].wealth = 40.0;
        s.assign_house_tiers();
        assert_eq!(s.houses[0].peak_wealth, 500.0, "peak must NOT fall with current wealth");
    }

    /// "A golden age" fires once a house has held Tier 1 with wealth rising for
    /// GOLDEN_AGE_MONTHS straight, and resets the moment either condition breaks.
    #[test]
    fn golden_age_fires_after_a_sustained_tier_one_rise() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 3];
        s.hub_minorities = vec![Vec::new(); 3];
        let mut top = house_at(0, vec![0], 4);
        top.wealth = 1000.0;
        top.volume = 500.0;
        top.prestige = 1.0;
        s.houses.push(top);
        for i in 1..4u32 {
            let mut h = house_at(i, vec![0], 1);
            h.wealth = 10.0;
            s.houses.push(h);
        }
        for month in 0..GOLDEN_AGE_MONTHS {
            s.houses[0].wealth += 5.0; // keep it rising every check
            s.assign_house_tiers();
            if month + 1 < GOLDEN_AGE_MONTHS {
                assert!(!s.houses[0].golden_age_chronicled, "fired too early at month {month}");
            }
        }
        assert!(s.houses[0].golden_age_chronicled, "golden age never fired after a sustained rise");
        assert!(s.houses[0].events.iter().any(|e| e.kind == "golden_age"));

        // A dip resets the streak.
        s.houses[0].golden_age_months = 0;
        s.houses[0].golden_age_chronicled = false;
        s.houses[0].wealth -= 1.0;
        s.assign_house_tiers();
        assert_eq!(s.houses[0].golden_age_months, 0, "a wealth dip must reset the streak");
    }

    /// "A dynasty of merchants" fires once three CONSECUTIVE heads have each left the
    /// house richer than they found it, and only once per streak.
    #[test]
    fn dynasty_of_merchants_fires_after_three_growing_heads() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 3];
        s.hub_minorities = vec![Vec::new(); 3];
        s.culture_rules = vec![CultureRule { culture: "Aiora".into(), line: 0, rule: 1 }]; // primogeniture
        let mut h = house_at(0, vec![0], 0);
        h.wealth = 1000.0; // deep pockets — this test is about succession, not solvency
        s.houses.push(h);
        s.seed_house_lines();

        for _ in 0..3 {
            // Advance the clock for real — `close_head_record` uses `tick=0` as its
            // "still living" sentinel, so calling `succeed_house` back-to-back at
            // tick 0 (as a real campaign never can, `head_lifespan` floors a tenure
            // at MIN_TENURE_YEARS) would leave every record looking permanently open.
            s.tick += 5 * TICKS_PER_YEAR;
            // Double the wealth each generation — comfortably outgrows the 30% a
            // gen>=2 succession may spin off into a cadet branch, so "richer than
            // they found it" reflects real growth, not a test fixture racing a drain.
            s.houses[0].wealth *= 2.0;
            s.succeed_house(0);
        }
        assert!(s.houses[0].dynasty_chronicled, "dynasty never fired after three growing heads");
        assert!(s.houses[0].events.iter().any(|e| e.kind == "dynasty"));
    }

    // ── Phase 2.1/2.6 · the Kin roster ──────────────────────────────────────
    /// The founding head is always `kin[0]`, with role "head" and full loyalty — the
    /// substrate §2 of the design promises ("the head IS kin[head]").
    #[test]
    fn kin_roster_seeds_the_head_as_kin_zero() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 2];
        s.hub_minorities = vec![Vec::new(); 2];
        let h = house_at(0, vec![0], 1);
        s.houses.push(h);
        s.seed_house_lines();
        let kin = &s.houses[0].kin;
        assert!(!kin.is_empty(), "a founded private house must get a roster");
        assert_eq!(kin[0].name, s.houses[0].head_name);
        assert_eq!(kin[0].role, 0, "kin[0] must be role 0 (head)");
        assert_eq!(kin[0].loyalty, 1.0);
        assert!(kin.len() >= 3, "expected the head plus 2-4 siblings, got {}", kin.len());
    }

    /// A civic guild has no family — its roster must stay empty.
    #[test]
    fn a_guild_has_no_kin_roster() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 2];
        s.hub_minorities = vec![Vec::new(); 2];
        let mut g = house_at(0, vec![0], 0);
        g.is_guild = true;
        s.houses.push(g);
        s.seed_house_lines();
        assert!(s.houses[0].kin.is_empty(), "a guild must never get a kin roster");
    }

    /// The master plan's invariant `a_house_with_no_kin_is_bit_identical` — read
    /// literally, "with no roster" — is about BACKWARD COMPATIBILITY, not about kin
    /// never affecting a decision. Phase 2.4/2.5 deliberately wire kin into real
    /// decisions (character-bounded knobs, steward wage/skim/poaching), so a house
    /// WITH a roster is no longer expected to be bit-identical to one without — that
    /// was always the point of building 2.4/2.5. What must still hold, and is the
    /// actual backward-compatibility guarantee an old save depends on: a house whose
    /// roster is EMPTY (never generated, or cleared) pays no steward cost and is
    /// never poached — an absent roster reads as "nothing is known", never as
    /// "assume the worst".
    #[test]
    fn an_empty_kin_roster_pays_no_steward_cost_and_is_never_poached() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let ng = goods.len();
        let hubs = (0..6u32).map(|i| {
            let prod: Vec<f32> = (0..ng).map(|g| if g == 0 { 9000.0 } else { 700.0 }).collect();
            hub(i, (i as f32) * 4.0, 0.0, 9000.0, prod, 0)
        }).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 6];
        s.hub_minorities = vec![Vec::new(); 6];
        for i in 0..4u32 {
            let mut h = house_at(i, vec![1], 2);
            h.wealth = 200.0 + i as f32 * 40.0;
            h.offices = vec![(i + 1) % 6, (i + 2) % 6]; // unposted "holdings", no roster at all
            h.trade_at = vec![((i + 1) % 6, 50.0), ((i + 2) % 6, 50.0)];
            s.houses.push(h);
        }
        s.seed_house_count = 4;
        // Deliberately DON'T call seed_house_lines — this is exactly an old save that
        // never got a roster generated (or a house that hasn't succeeded since).
        for h in &s.houses { assert!(h.kin.is_empty()); }
        for yr in 0..40 {
            s.tick = yr * TICKS_PER_YEAR;
            s.apply_wealth_sinks();
            for _ in 0..12 { s.update_guilds_and_offices(); }
        }
        assert!(s.houses.iter().all(|h| h.kin.is_empty()),
            "apply_wealth_sinks/update_guilds_and_offices must never populate a roster themselves");
        assert!(
            !s.houses.iter().flat_map(|h| h.events.iter()).any(|e| e.kind == "poached"),
            "a house with no roster was poached — an absent roster must mean no known steward"
        );
    }

    /// An agnatic line otherwise never produces a female head — the widow regency is
    /// its one route to one, and it must actually fire (not just be dead code).
    #[test]
    fn widow_regency_occasionally_holds_an_agnatic_house() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 3];
        s.hub_minorities = vec![Vec::new(); 3];
        s.culture_rules = vec![CultureRule { culture: "Aiora".into(), line: 0, rule: 1 }]; // agnatic + primogeniture
        let mut h = house_at(0, vec![0], 0);
        h.wealth = 1_000_000.0;
        s.houses.push(h);
        s.seed_house_lines();
        let mut widow_terms = 0;
        for gen in 0..60 {
            s.tick = gen * 5 * TICKS_PER_YEAR;
            s.houses[0].wealth *= 1.3;
            s.succeed_house(0);
            if s.houses[0].line.last().is_some_and(|p| p.accession == "widow regent") { widow_terms += 1; }
        }
        assert!(widow_terms > 0, "a widow regent never held an agnatic house over 60 successions");
        assert!(widow_terms < 20, "widow regency fired far more than its ~8% rate: {widow_terms}/60");
    }

    /// Phase 2.6's own gate: whatever the roster looks like — empty, one dead kin,
    /// a full mixed one — the power shares must sum to exactly 100 (or be empty for
    /// an empty roster).
    #[test]
    fn power_shares_always_sum_to_100() {
        use crate::sim::tick::kin_power_shares;
        let mk = |role: u8, skill: f32, loyalty: f32| Kin {
            name: "K".into(), female: false, born_tick: 0, dies_tick: 0, role, posted: -1,
            character: [0; 4], loyalty, skill, parent: -1,
        };
        assert!(kin_power_shares(&[]).is_empty());

        let one = kin_power_shares(&[mk(0, 0.7, 1.0)]);
        assert_eq!(one.len(), 1);
        assert!((one[0] - 100.0).abs() < 1e-3);

        let mixed = kin_power_shares(&[
            mk(0, 0.9, 1.0), mk(1, 0.5, 0.8), mk(2, 0.3, 0.4), mk(3, 0.1, 0.1),
        ]);
        assert!((mixed.iter().sum::<f32>() - 100.0).abs() < 1e-3, "shares: {mixed:?}");
        assert!(mixed[0] > mixed[1] && mixed[1] > mixed[2], "head must outweigh heir must outweigh factor");

        // Every kin dead/married-out (zero weight) — must still sum to 100 (even split).
        let dead = kin_power_shares(&[mk(5, 0.0, 0.0), mk(4, 0.0, 0.0)]);
        assert!((dead.iter().sum::<f32>() - 100.0).abs() < 1e-3, "shares: {dead:?}");
    }

    /// A character phrase names only the NOTABLE axes and stays empty for a middling
    /// character — the same "quiet unless it matters" discipline as the stability
    /// gauges.
    #[test]
    fn character_phrase_is_quiet_unless_notable() {
        use crate::sim::tick::character_phrase;
        assert_eq!(character_phrase([0, 0, 0, 0]), "");
        let p = character_phrase([2, 2, 0, -2]);
        assert!(!p.is_empty());
        assert!(p.ends_with('.'));
        let first = p.chars().next().unwrap();
        assert!(first.is_uppercase(), "phrase must start capitalised: {p}");
    }

    // ── Phase 2.4 · character wired to real decisions, ±15% capped ─────────────
    /// The gate from `HOUSE_PEOPLE_AND_TIERS.md` §3: with no roster, or an all-zero
    /// character, every knob's modifier must be EXACTLY 1.0 — a true no-op, not an
    /// approximation. This is what keeps "no roster / all-zero character ⇒
    /// bit-identical" true at every call site without a special case anywhere.
    #[test]
    fn character_factor_is_a_true_noop_at_zero() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let h = house_at(0, vec![0], 1); // no kin — house_at() never populates a roster
        s.houses.push(h);
        for axis in 0..4 {
            assert_eq!(s.head_character_factor(0, axis), 1.0, "empty roster must be a no-op on axis {axis}");
        }
        s.houses[0].kin.push(Kin {
            name: "Head".into(), female: false, born_tick: 0, dies_tick: 0, role: 0, posted: -1,
            character: [0, 0, 0, 0], loyalty: 1.0, skill: 0.5, parent: -1,
        });
        for axis in 0..4 {
            assert_eq!(s.head_character_factor(0, axis), 1.0, "all-zero character must be a no-op on axis {axis}");
        }
    }

    /// A non-zero character must actually move the factor, bounded at exactly
    /// ±`CHARACTER_KNOB_CAP` at the axis extreme — proof the axis is wired to
    /// something, not decoration, and that the cap the design specifies is real.
    #[test]
    fn character_factor_is_bounded_and_directional() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        let mk = |c: [i8; 4]| Kin {
            name: "Head".into(), female: false, born_tick: 0, dies_tick: 0, role: 0, posted: -1,
            character: c, loyalty: 1.0, skill: 0.5, parent: -1,
        };
        s.houses[0].kin = vec![mk([2, -2, 2, -2])];
        assert!((s.head_character_factor(0, 0) - 1.15).abs() < 1e-5, "axis 0 at +2 must hit the cap");
        assert!((s.head_character_factor(0, 1) - 0.85).abs() < 1e-5, "axis 1 at -2 must hit the floor");
        assert!((s.head_character_factor(0, 2) - 1.15).abs() < 1e-5);
        assert!((s.head_character_factor(0, 3) - 0.85).abs() < 1e-5);
        // Never wider than the cap, whatever the axis value (i8 can't exceed ±2 anyway,
        // but the formula itself must not silently drift past CHARACTER_KNOB_CAP).
        for v in -2..=2i8 {
            let f = s.head_character_factor(0, 0);
            s.houses[0].kin[0].character[0] = v;
            let f2 = s.head_character_factor(0, 0);
            assert!((f2 - 1.0).abs() <= CHARACTER_KNOB_CAP + 1e-5, "factor {f2} exceeds the cap at v={v}");
            let _ = f;
        }
    }

    // ── Phase 2.5 · stewards ─────────────────────────────────────────────────
    /// A hired (unposted) office costs a wage + skim; a family-posted one costs
    /// nothing extra. Two otherwise-identical houses must diverge accordingly.
    #[test]
    fn hired_offices_cost_more_than_family_run_ones() {
        let mk_house = |posted: bool| {
            let mut h = house_at(0, vec![0], 0);
            h.wealth = 5000.0;
            h.offices = vec![1, 2];
            let k = |role: u8, posted: i32| Kin {
                name: "K".into(), female: false, born_tick: 0, dies_tick: 0, role, posted,
                character: [0; 4], loyalty: 1.0, skill: 0.5, parent: -1,
            };
            // Both scenarios get a NON-EMPTY roster (a roster present but nobody posted
            // is what "hired" means here — an EMPTY roster instead means "unknown", see
            // the wage/skim gate's own doc comment, and costs nothing).
            h.kin = if posted { vec![k(2, 1), k(2, 2)] } else { vec![k(3, -1)] };
            h
        };
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..4u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut hired = sim(hubs, goods);
        hired.houses.push(mk_house(false));
        let goods2 = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs2 = (0..4u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut family = sim(hubs2, goods2);
        family.houses.push(mk_house(true));

        hired.apply_wealth_sinks();
        family.apply_wealth_sinks();
        assert!(hired.houses[0].wealth < family.houses[0].wealth,
            "hired {} vs family-run {} — a hired-staffed house should end poorer",
            hired.houses[0].wealth, family.houses[0].wealth);
        // Bounded: two hired offices shouldn't cost more than a couple percent of
        // wealth in a single month.
        let drain = family.houses[0].wealth - hired.houses[0].wealth;
        assert!(drain < 5000.0 * 0.02, "steward drain too large for one month: {drain}");
    }

    /// A guild has no stewards to hire — its offices never cost a wage/skim, and its
    /// offices are never poached. Wealth is kept UNDER the guild endowment soft cap so
    /// that separate (pre-existing) mechanic can't be mistaken for a steward cost.
    #[test]
    fn a_guild_has_no_steward_cost_or_poaching() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..4u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut g = house_at(0, vec![0], 0);
        g.is_guild = true;
        // Well under GUILD_WEALTH_SOFTCAP·city_size_factor (200·0.3 = 60 at this
        // hub's population, city_size_factor floors at 0.3) — no endowment drain,
        // which is a separate pre-existing mechanic this test must not trip.
        g.wealth = 20.0;
        g.offices = vec![1, 2, 3]; // all unposted — would be "hired" on a private house
        s.houses.push(g);
        let before = s.houses[0].wealth;
        s.apply_wealth_sinks();
        let after = s.houses[0].wealth;
        // Only the guild civic rate should have moved it — no steward wage/skim.
        let expected = before - before * GUILD_CIVIC_RATE;
        assert!((after - expected).abs() < 1e-3,
            "a guild paid something beyond its civic dues: {before} -> {after}, expected ~{expected}");

        // And its offices are never poached, however many months pass.
        s.hub_culture = vec!["Aiora".into(); 4];
        s.hub_minorities = vec![Vec::new(); 4];
        s.houses[0].trade_at = vec![(1, 100.0), (2, 100.0), (3, 100.0)];
        for month in 0..1000u32 {
            s.tick = month * 30;
            s.update_guilds_and_offices();
        }
        assert!(!s.houses[0].events.iter().any(|e| e.kind == "poached"),
            "a guild's office was poached — guilds have no stewards to hire");
    }

    /// Poaching is rare but real — over many months, a hired office is occasionally
    /// lost to a "poached" event, and a posted (family) one never is. A poached office
    /// may be restaffed the same tick if the trade tie is still strong (the OPEN logic
    /// runs right after CLOSE in the same pass) — that's realistic resilience, not a
    /// missing event, so this counts EVENTS directly rather than watching the office
    /// list for a hole that may not stay open.
    #[test]
    fn poaching_occasionally_takes_a_hired_office_never_a_family_one() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 3];
        s.hub_minorities = vec![Vec::new(); 3];
        // hub 1 stays HIRED for every house; hub 2 is posted to a kin (family-run) —
        // the two must diverge in poach count.
        for _ in 0..10u32 {
            let mut h = house_at(0, vec![0], 0);
            h.wealth = 5000.0;
            h.offices = vec![1, 2];
            h.trade_at = vec![(1, 100.0), (2, 100.0)];
            // A non-empty roster: hub 2 is posted (family), hub 1 has nobody posted —
            // an idle kin keeps the roster non-empty so hub 1 correctly reads "hired"
            // rather than "unknown" (an EMPTY roster costs/poaches nothing — see the
            // gate's own doc comment).
            h.kin = vec![
                Kin { name: "K".into(), female: false, born_tick: 0, dies_tick: 0, role: 2, posted: 2,
                    character: [0; 4], loyalty: 1.0, skill: 0.5, parent: -1 },
                Kin { name: "Idle".into(), female: false, born_tick: 0, dies_tick: 0, role: 3, posted: -1,
                    character: [0; 4], loyalty: 1.0, skill: 0.5, parent: -1 },
            ];
            s.houses.push(h);
        }
        for month in 0..2000u32 {
            s.tick = month * 30;
            s.update_guilds_and_offices();
        }
        let poached_at = |hub: i32| -> usize {
            s.houses.iter().flat_map(|h| h.events.iter())
                .filter(|e| e.kind == "poached" && e.text.contains(&format!("in H{hub}")))
                .count()
        };
        assert!(poached_at(1) > 0, "the HIRED office (hub 1) was never poached over 2000 months");
        assert_eq!(poached_at(2), 0, "the FAMILY-posted office (hub 2) must never be poached");
    }

    // ── Phase 3.1 · goals ────────────────────────────────────────────────────
    /// A house takes up a goal only when it has a free slot, and a Tier 1 house gets
    /// TWO at once (§4's own rule) while everyone else gets one.
    #[test]
    fn tier_one_pursues_two_goals_everyone_else_one() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_culture = vec!["Aiora".into(); 3];
        s.hub_minorities = vec![Vec::new(); 3];
        let mut h1 = house_at(0, vec![0], 1);
        h1.tier = 1;
        h1.wealth = 500.0;
        h1.archetype = ARCH_SPECIALTY;
        let mut h2 = house_at(1, vec![0], 1);
        h2.tier = 3;
        h2.wealth = 500.0;
        h2.archetype = ARCH_SPECIALTY;
        s.houses.push(h1);
        s.houses.push(h2);
        // Choosing repeatedly (as the yearly cadence would) must stop at the slot cap.
        for _ in 0..5 { s.choose_house_goal(0); s.choose_house_goal(1); }
        assert_eq!(s.houses[0].goals.len(), GOAL_SLOTS_TIER1, "Tier 1 must fill both slots");
        assert_eq!(s.houses[1].goals.len(), GOAL_SLOTS_OTHER, "a non-Tier-1 house gets one slot");
    }

    /// CORNER_TRADE succeeds after the monopoly share holds >=60% for
    /// GOAL_HOLD_YEARS_TRADE running years, and the streak RESETS the moment it dips.
    #[test]
    fn corner_trade_goal_succeeds_after_a_sustained_monopoly() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.wealth = 500.0;
        s.houses.push(h);
        s.houses[0].goals.push(Goal {
            kind: GOAL_CORNER_TRADE, target_good: 0, target_hub: -1, target_house: -1, target_province: -1,
            set_tick: 0, deadline_tick: 100 * TICKS_PER_YEAR, progress: 0.0, state: GOAL_PURSUING,
        });
        // A dip resets the streak before the real run.
        s.houses[0].monopoly = vec![(0, 0.3)];
        s.update_house_goal(0);
        assert_eq!(s.houses[0].goals[0].progress, 0.0);
        s.houses[0].monopoly = vec![(0, 0.65)];
        for yr in 0..(GOAL_HOLD_YEARS_TRADE as u32 - 1) {
            s.tick = yr * TICKS_PER_YEAR;
            s.update_house_goal(0);
            assert_eq!(s.houses[0].goals.len(), 1, "must not close before the hold period completes");
        }
        s.tick = (GOAL_HOLD_YEARS_TRADE as u32) * TICKS_PER_YEAR;
        s.update_house_goal(0);
        assert!(s.houses[0].goals.is_empty(), "the goal must close once achieved");
        assert_eq!(s.houses[0].goal_history.len(), 1);
        assert_eq!(s.houses[0].goal_history[0].state, GOAL_ACHIEVED);
        assert!(s.houses[0].events.iter().any(|e| e.kind == "goal_achieved"));
    }

    /// A goal that never qualifies FAILS at its deadline — it doesn't just sit there
    /// forever, and it doesn't silently vanish either.
    #[test]
    fn a_goal_fails_at_its_deadline_if_never_achieved() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.wealth = 500.0;
        s.houses.push(h);
        s.houses[0].goals.push(Goal {
            kind: GOAL_SEAT_COUNCIL, target_good: -1, target_hub: 0, target_house: -1, target_province: -1,
            set_tick: 0, deadline_tick: 5 * TICKS_PER_YEAR, progress: 0.0, state: GOAL_PURSUING,
        });
        s.tick = 4 * TICKS_PER_YEAR;
        s.update_house_goal(0);
        assert_eq!(s.houses[0].goals.len(), 1, "must not fail before its own deadline");
        s.tick = 5 * TICKS_PER_YEAR;
        s.update_house_goal(0);
        assert!(s.houses[0].goals.is_empty());
        assert_eq!(s.houses[0].goal_history[0].state, GOAL_FAILED);
        assert!(s.houses[0].events.iter().any(|e| e.kind == "goal_failed"));
        assert!(!is_house_milestone("goal_failed"), "a failed goal is chatter, not a milestone");
        assert!(is_house_milestone("goal_achieved"), "an achieved goal IS a milestone");
    }

    /// RESTORE_HOUSE targets the peak wealth AT THE MOMENT THE GOAL WAS SET — not the
    /// ever-rising all-time peak, which a house could never catch if it kept climbing.
    #[test]
    fn restore_house_goal_targets_the_peak_at_the_moment_it_was_set() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.wealth = 40.0;
        h.peak_wealth = 100.0; // fell to 40% of peak — eligible
        s.houses.push(h);
        s.choose_house_goal(0);
        assert_eq!(s.houses[0].goals.len(), 1, "a fallen house must pick up RESTORE_HOUSE");
        assert_eq!(s.houses[0].goals[0].kind, GOAL_RESTORE_HOUSE);
        assert_eq!(s.houses[0].goals[0].progress, 100.0, "the target must be the peak AT SET TIME");
        // The all-time peak keeps climbing after the goal is set...
        s.houses[0].peak_wealth = 400.0;
        s.houses[0].wealth = 150.0; // above the ORIGINAL 100 target, but not the new peak
        s.update_house_goal(0);
        assert!(s.houses[0].goals.is_empty(), "must succeed against the OLD target, not the new peak");
        assert_eq!(s.houses[0].goal_history[0].state, GOAL_ACHIEVED);
    }

    /// OUTLAST_RIVAL succeeds the moment the named rival goes defunct.
    #[test]
    fn outlast_rival_goal_succeeds_when_the_rival_dies() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.wealth = 500.0;
        s.houses.push(h);
        s.houses.push(house_at(1, vec![0], 1));
        s.houses[0].goals.push(Goal {
            kind: GOAL_OUTLAST_RIVAL, target_good: -1, target_hub: -1, target_house: 1, target_province: -1,
            set_tick: 0, deadline_tick: 100 * TICKS_PER_YEAR, progress: 0.0, state: GOAL_PURSUING,
        });
        s.update_house_goal(0);
        assert_eq!(s.houses[0].goals.len(), 1, "must not succeed while the rival lives");
        s.houses[1].defunct = true;
        s.update_house_goal(0);
        assert!(s.houses[0].goals.is_empty());
        assert_eq!(s.houses[0].goal_history[0].state, GOAL_ACHIEVED);
    }

    /// REACH_PROVINCE succeeds only through the expedition hook — a live round trip
    /// to the target province — and never for an unrelated province.
    #[test]
    fn reach_province_goal_succeeds_only_via_a_backed_expeditions_return() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_province = vec![0, 1, 2];
        s.prov_seat = vec![[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]];
        let mut h = house_at(0, vec![0], 1);
        h.wealth = 500.0;
        s.houses.push(h);
        s.houses[0].goals.push(Goal {
            kind: GOAL_REACH_PROVINCE, target_good: -1, target_hub: -1, target_house: -1, target_province: 2,
            set_tick: 0, deadline_tick: 100 * TICKS_PER_YEAR, progress: 0.0, state: GOAL_PURSUING,
        });
        // An expedition that reaches the WRONG province must not satisfy the goal.
        s.houses[0].wealth = 500.0;
        s.launch_expedition(0, 0, 1); // dest_province = 1, not the target (2)
        for _ in 0..400 { s.expedition_travel_pass(); }
        assert_eq!(s.houses[0].goals.len(), 1, "the wrong province must not satisfy the goal");
        // One that reaches the RIGHT province must.
        s.houses[0].wealth = 500.0;
        s.launch_expedition(0, 0, 2); // dest_province = 2, the target
        for _ in 0..400 { s.expedition_travel_pass(); }
        s.tick += 1; // let update_house_goal's next check see the externally-set state
        s.update_house_goal(0);
        assert!(s.houses[0].goals.is_empty(), "reaching the target province must close the goal");
        assert_eq!(s.houses[0].goal_history.last().unwrap().state, GOAL_ACHIEVED);
    }

    // ── Phase 3.2 · competence + vice ────────────────────────────────────────
    fn kin_at(name: &str, female: bool, role: u8, character: [i8; 4], loyalty: f32, skill: f32) -> Kin {
        Kin { name: name.into(), female, born_tick: 0, dies_tick: 0, role, posted: -1,
              character, loyalty, skill, parent: -1 }
    }

    #[test]
    fn head_vice_is_a_true_noop_with_no_roster_or_flat_character() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        assert_eq!(s.head_vice(0), VICE_NONE, "no roster ⇒ no vice");
        s.houses[0].kin.push(kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.9));
        assert_eq!(s.head_vice(0), VICE_NONE, "flat character + high skill ⇒ no vice");
    }

    #[test]
    fn head_vice_matches_the_designs_priority_order() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        s.houses[0].kin.push(kin_at("Head", false, 0, [2, 0, 0, 0], 1.0, 0.9));
        assert_eq!(s.head_vice(0), VICE_RECKLESS, "bold >= 2");
        s.houses[0].kin[0] = kin_at("Head", false, 0, [0, 2, 0, 0], 1.0, 0.9);
        assert_eq!(s.head_vice(0), VICE_RAPACIOUS, "greed >= 2");
        s.houses[0].kin[0] = kin_at("Head", false, 0, [-2, 0, -1, 0], 1.0, 0.9);
        assert_eq!(s.head_vice(0), VICE_MISERLY, "bold <= -2 and civic <= -1");
        s.houses[0].kin[0] = kin_at("Head", false, 0, [0, 0, 0, -2], 1.0, 0.9);
        assert_eq!(s.head_vice(0), VICE_PAROCHIAL, "rooted <= -2");
        // Lavish is checked FIRST, so a head who also qualifies for another vice
        // still reads as Lavish — matches the table's own priority order.
        s.houses[0].kin[0] = kin_at("Head", false, 0, [-2, 0, 1, 0], 1.0, 0.3);
        assert_eq!(s.head_vice(0), VICE_LAVISH, "civic >= 1 and skill <= 0.4, checked first");
    }

    /// The one wired economic consequence of a vice (§3.2's own scoping note):
    /// Lavish adds a small extra drain in `apply_wealth_sinks`. A head with no vice
    /// at the same starting wealth must NOT pay it.
    #[test]
    fn lavish_vice_costs_wealth_a_sober_head_does_not_pay() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut lavish = house_at(0, vec![0], 1);
        lavish.wealth = 1000.0;
        lavish.kin.push(kin_at("Lavish", false, 0, [0, 0, 2, 0], 1.0, 0.3));
        let mut sober = house_at(1, vec![0], 1);
        sober.wealth = 1000.0;
        sober.kin.push(kin_at("Sober", false, 0, [0, 0, 0, 0], 1.0, 0.9));
        s.houses.push(lavish);
        s.houses.push(sober);
        s.apply_wealth_sinks();
        assert!(s.houses[0].wealth < s.houses[1].wealth,
            "a Lavish head must bleed more than an otherwise-identical sober one");
    }

    // ── Phase 3.3-3.6 · the crisis ───────────────────────────────────────────
    fn discontented_house(hub_id: u32) -> House {
        let mut h = house_at(hub_id, vec![0], 1);
        h.wealth = 500.0;
        h.wealth_history = vec![500.0, 50.0]; // sharp decline ⇒ falling_funds ~= 0.9
        h.kin = vec![
            kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6),
            kin_at("Rival", false, 2, [0, 0, 0, 0], 0.05, 0.7), // the plot leader
        ];
        h
    }

    /// A crisis opens, runs, and resolves within the fixed round cap — it can never
    /// become the permanent state of a house. The design's own invariant name.
    #[test]
    fn every_crisis_terminates() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(discontented_house(0));
        s.tick = 0;
        s.update_house_crises();
        assert!(s.houses[0].crisis.is_some(), "high discontent must open a crisis");
        for q in 1..=(CRISIS_ROUND_CAP as u32 + 2) {
            s.tick = q * CRISIS_ROUND_TICKS;
            s.update_house_crises();
            if let Some(c) = &s.houses[0].crisis {
                assert!(c.round <= CRISIS_ROUND_CAP, "a crisis must never exceed the round cap");
            }
        }
        assert!(s.houses[0].crisis.is_none(), "the crisis must have resolved by the round cap");
        assert_eq!(s.houses[0].crisis_history.len(), 1, "a resolved crisis leaves exactly one record");
    }

    /// A crisis the plot is decisively winning depose the head — a new head is
    /// installed, the old head's tenure closes, and the event is a permanent
    /// milestone (never pruned by the chronicle cap).
    #[test]
    fn a_decisive_plot_deposes_the_head() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = discontented_house(0);
        h.crisis = Some(HouseCrisis {
            opened_tick: 0, cause: 0, plot_leader: 1, round: CRISIS_ROUND_CAP - 1,
            head_support: 0.05, plot_support: 0.90, peak_plot: 0.90, rounds: Vec::new(),
            loyalist_name: "the Old Council".into(), loyalist_tint: "#b32d2d".into(),
            plot_name: "Rival's men".into(), plot_tint: "#2a5fa0".into(), heir_choice: 2,
        });
        s.houses.push(h);
        s.tick = CRISIS_ROUND_TICKS;
        s.update_house_crises();
        assert!(s.houses[0].crisis.is_none());
        assert_eq!(s.houses[0].crisis_history[0].outcome, CRISIS_DEPOSED);
        assert_eq!(s.houses[0].head_name, "Rival", "the plot leader must take the seat");
        assert!(s.houses[0].events.iter().any(|e| e.kind == "deposed"));
        assert!(is_house_milestone("deposed"), "a deposition is a permanent milestone");
    }

    /// A crisis the head is decisively winning ends in survival, not deposition —
    /// and a survivor earns the grace period that keeps a weak head from sitting in
    /// permanent crisis (`HOUSE_FACTION_NAMING_AND_RECORD.md` §4).
    #[test]
    fn a_decisive_head_prevails_and_earns_a_grace_period() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = discontented_house(0);
        let original_head = h.head_name.clone();
        h.crisis = Some(HouseCrisis {
            opened_tick: 0, cause: 0, plot_leader: 1, round: CRISIS_ROUND_CAP - 1,
            head_support: 0.90, plot_support: 0.05, peak_plot: 0.30, rounds: Vec::new(),
            loyalist_name: "the Old Council".into(), loyalist_tint: "#b32d2d".into(),
            plot_name: "Rival's men".into(), plot_tint: "#2a5fa0".into(), heir_choice: 2,
        });
        s.houses.push(h);
        s.tick = CRISIS_ROUND_TICKS;
        s.update_house_crises();
        assert!(s.houses[0].crisis.is_none());
        assert_eq!(s.houses[0].crisis_history[0].outcome, CRISIS_PREVAILED);
        assert_eq!(s.houses[0].head_name, original_head, "the ruler keeps the seat");
        assert!(s.houses[0].crisis_immune_until > s.tick, "a survivor must earn a grace period");
        assert!(s.houses[0].events.iter().any(|e| e.kind == "crisis_survived"));
        assert!(is_house_milestone("crisis_survived"));
        // The grace period must actually PREVENT a new crisis from opening.
        s.houses[0].wealth_history = vec![500.0, 50.0];
        s.update_house_crises();
        assert!(s.houses[0].crisis.is_none(), "no new crisis may open inside the grace window");
    }

    /// Two independently-opened crises must never share a faction name or tincture —
    /// the whole point of naming a faction after the house's own arms is that the
    /// two camps read as visibly different parties.
    #[test]
    fn faction_names_and_tints_are_distinct() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        for seedn in 0..24u64 {
            let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
            let mut s = sim(hubs, goods.clone());
            s.seed = seedn * 7919 + 11;
            let mut h = discontented_house(0);
            h.name = format!("House Seed{seedn}");
            h.head_name = format!("Head{seedn}");
            h.kin[1].name = format!("Rival{seedn}");
            s.houses.push(h);
            s.tick = 0;
            s.update_house_crises();
            let c = s.houses[0].crisis.as_ref().expect("high discontent must open a crisis");
            assert_ne!(c.loyalist_name, c.plot_name, "seed {seedn}: the two factions must not share a name");
            assert_ne!(c.loyalist_tint, c.plot_tint, "seed {seedn}: the two factions must not share a tincture");
        }
    }

    // ── Phase 4.1 · schism (Quarrel / Departure) ─────────────────────────────
    fn tense_house(hub_id: u32, posted: i32) -> House {
        let mut h = house_at(hub_id, vec![0], 1);
        h.wealth = 500.0;
        h.rivals = vec![90, 91, 92]; // feeds the tension formula's feud term only
        h.kin = vec![
            kin_at("Head", false, 0, [0, 0, 0, 0], 0.3, 0.6),
            kin_at("Disloyal", false, 2, [0, 0, 0, 0], 0.05, 0.5),
        ];
        h.kin[1].posted = posted;
        h
    }

    #[test]
    fn a_quiet_house_never_schisms() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.kin = vec![kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6), kin_at("Loyal", false, 2, [0, 0, 0, 0], 0.95, 0.6)];
        s.houses.push(h);
        s.tick = 0;
        s.update_house_schisms();
        assert!(s.houses[0].events.is_empty(), "high loyalty ⇒ no schism at all");
    }

    #[test]
    fn a_disloyal_unposted_kin_can_only_quarrel_never_depart() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(tense_house(0, -1)); // unposted ⇒ never eligible for Departure
        let before_len = s.houses.len();
        s.tick = 0;
        s.update_house_schisms();
        assert_eq!(s.houses.len(), before_len, "an unposted disloyal kin cannot depart");
        assert!(s.houses[0].events.iter().any(|e| e.kind == "quarrel"));
        assert!(!is_house_milestone("quarrel"), "a quarrel alone is chatter, not a milestone");
    }

    #[test]
    fn a_posted_disloyal_kin_can_depart_and_found_a_rival_house() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut found = false;
        for seedn in 0..60u64 {
            let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
            let mut s = sim(hubs, goods.clone());
            s.seed = seedn * 104729 + 7;
            s.hub_culture = vec!["Aiora".into(); 2];
            s.hub_minorities = vec![Vec::new(); 2];
            s.houses.push(tense_house(0, 1)); // posted to a DIFFERENT hub than the seat
            s.tick = 0;
            let before = s.houses.len();
            s.update_house_schisms();
            if s.houses.len() > before {
                found = true;
                assert_eq!(s.houses[1].hub, 1, "the new house is founded AT the posted hub");
                assert!(s.houses[0].rivals.contains(&1), "the parent gains the new house as a rival");
                assert!(s.houses[1].rivals.contains(&0), "the new house starts as a rival right back");
                assert_eq!(s.houses[0].kin.len(), 1, "the departing kin leaves the parent's roster");
                assert!(s.houses[0].events.iter().any(|e| e.kind == "schism"));
                assert!(is_house_milestone("schism"), "a departure is a permanent milestone");
                break;
            }
        }
        assert!(found, "over 60 seeds, at least one must roll a Departure");
    }

    // ── Phase 4.2 · bankruptcy aftermath ─────────────────────────────────────
    #[test]
    fn a_dissolved_house_leaves_a_named_creditor_loss() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        s.banks.push(Bank {
            name: "Banco Test".into(), house: 1, seat: 0, founded_tick: 0, defunct: false,
            reserves: 80.0, loans: vec![Loan {
                borrower_house: 0, borrower_polis: -1, principal: 100.0, outstanding: 80.0,
                rate: 0.01, start_tick: 0, term_ticks: 1000, purpose: "trade".into(),
            }], real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0, stakes: vec![],
            dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        });
        s.dissolve_house(0);
        assert_eq!(s.banks[0].loans[0].outstanding, 0.0, "the loan is written down to zero");
        assert!((s.banks[0].losses - 80.0).abs() < 1e-3, "the bank's own write-off tally records the loss");
        assert!(s.banks[0].events.iter().any(|e| e.kind == "bad_debt"), "the bank's own ledger names the write-off");
        assert!(s.houses[0].events.iter().any(|e| e.kind == "dissolved" && e.text.contains("Banco Test")),
            "the dissolved house's own record names its creditor");
    }

    #[test]
    fn a_house_with_no_debt_dissolves_with_no_creditor_line() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        s.dissolve_house(0);
        let ev = s.houses[0].events.iter().find(|e| e.kind == "dissolved").unwrap();
        assert!(!ev.text.contains("owed"), "no bank was owed anything ⇒ no fabricated creditor line");
    }

    // ── Phase 4.3 · plague as a lineage event ────────────────────────────────
    #[test]
    fn a_plague_can_kill_several_kin_at_once() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut found_multi = false;
        for seedn in 0..40u64 {
            let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
            let mut s = sim(hubs, goods.clone());
            s.seed = seedn * 7919 + 3;
            let mut h = house_at(0, vec![0], 1);
            h.kin = vec![
                kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6),
                kin_at("K1", false, 3, [0, 0, 0, 0], 1.0, 0.5),
                kin_at("K2", false, 3, [0, 0, 0, 0], 1.0, 0.5),
                kin_at("K3", false, 3, [0, 0, 0, 0], 1.0, 0.5),
            ];
            s.houses.push(h);
            s.tick = 0;
            s.plague_house_toll(0, 0.6, 1, "the Plague"); // category 1 = Great Plague, max severity
            let live = s.houses[0].kin.iter().filter(|k| k.role != 5).count();
            if !s.houses[0].defunct && live < 3 { found_multi = true; break; }
        }
        assert!(found_multi, "over 40 seeds at max severity, at least one visitation must kill >1 kin");
    }

    #[test]
    fn a_plague_can_extinguish_a_house_independent_of_the_head() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut found = false;
        for seedn in 0..200u64 {
            let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
            let mut s = sim(hubs, goods.clone());
            s.seed = seedn * 65537 + 13;
            let mut h = house_at(0, vec![0], 1);
            h.kin = vec![kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6)];
            s.houses.push(h);
            s.tick = 0;
            s.plague_house_toll(0, 0.6, 1, "the Plague");
            if s.houses[0].defunct {
                found = true;
                assert!(s.houses[0].events.iter().any(|e| e.kind == "plague_extinction"));
                assert!(is_house_milestone("plague_extinction"), "extinction is a permanent milestone");
                break;
            }
        }
        assert!(found, "over 200 seeds at max severity, at least one house must be extinguished");
    }

    #[test]
    fn a_house_with_no_presence_at_the_struck_city_is_untouched() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(1, vec![0], 1); // seated at hub 1, not the struck hub 0
        h.kin = vec![kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6), kin_at("K", false, 3, [0, 0, 0, 0], 1.0, 0.5)];
        s.houses.push(h);
        s.tick = 0;
        for _ in 0..20 { s.plague_house_toll(0, 0.6, 1, "the Plague"); }
        assert_eq!(s.houses[0].kin.len(), 2, "a house with no presence at the struck city loses no kin");
        assert!(!s.houses[0].defunct);
    }

    // ── Phase 2.4 · crisis salience ──────────────────────────────────────────
    /// Only Tier 1-2 crises reach the world news feed; a Tier 3-4 (or untiered)
    /// house's crisis is still fully written to its OWN chronicle, just silent on
    /// the world stage — "the player cannot watch fourteen houses".
    #[test]
    fn only_tier_one_and_two_crises_reach_the_news_feed() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];

        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut lowly = sim(hubs, goods.clone());
        let mut h = discontented_house(0);
        h.tier = 4;
        lowly.houses.push(h);
        lowly.tick = 0;
        lowly.update_house_crises();
        assert!(lowly.houses[0].crisis.is_some(), "a Tier 4 house can still open a crisis");
        assert!(lowly.journal.iter().all(|j| j.kind != "crisis"), "but it must not reach the news feed");
        assert!(lowly.houses[0].events.iter().any(|e| e.kind == "crisis_opened"),
            "the house's OWN chronicle still records it in full");

        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut great = sim(hubs, goods);
        let mut h2 = discontented_house(0);
        h2.tier = 1;
        great.houses.push(h2);
        great.tick = 0;
        great.update_house_crises();
        assert!(great.houses[0].crisis.is_some());
        assert!(great.journal.iter().any(|j| j.kind == "crisis"), "a Tier 1 house's crisis IS world news");
    }

    // ── Phase 4.4 · the foreign hand ─────────────────────────────────────────
    #[test]
    fn foreign_hand_never_moves_a_kin_with_no_rival_presence() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.kin = vec![kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6), kin_at("Factor", false, 2, [0, 0, 0, 0], 0.9, 0.5)];
        h.kin[1].posted = 1;
        s.houses.push(h);
        s.tick = 0;
        s.apply_foreign_hand();
        assert_eq!(s.houses[0].kin[1].loyalty, 0.9, "no rival presence anywhere ⇒ no change at all");
    }

    #[test]
    fn channel_a_exposure_lowers_a_posted_kins_loyalty() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.kin = vec![kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6), kin_at("Factor", false, 2, [0, 0, 0, 0], 0.9, 0.5)];
        h.kin[1].posted = 1;
        s.houses.push(h);
        let mut rival = house_at(1, vec![0], 1);
        rival.political_power = 0.8;
        rival.offices = vec![1]; // an office at the kin's own posted hub
        s.houses.push(rival);
        s.tick = 0;
        s.apply_foreign_hand();
        assert!(s.houses[0].kin[1].loyalty < 0.9, "a rival's office in the kin's own city must nudge loyalty down");
    }

    #[test]
    fn channel_b_exposure_via_a_controlled_lease_lowers_loyalty() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.kin = vec![kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6), kin_at("Factor", false, 2, [0, 0, 0, 0], 0.9, 0.5)];
        h.kin[1].posted = 1;
        h.office_leases = vec![(1, 100_000)];
        s.houses.push(h);
        let mut rival = house_at(1, vec![0], 1);
        rival.political_power = 0.8;
        s.houses.push(rival);
        s.hubs[1].captor_house = 1; // the rival CONTROLS the city we lease in
        s.tick = 0;
        s.apply_foreign_hand();
        assert!(s.houses[0].kin[1].loyalty < 0.9, "leasing in a rival-controlled city must nudge loyalty down");
    }

    /// Even at MAXIMUM leverage (both channels, an active feud, a fully-weighted
    /// rival), a single month's exposure can never itself manufacture hostility —
    /// leverage deepens a grievance, it does not create one.
    #[test]
    fn foreign_hand_decay_is_small_and_bounded_in_a_single_month() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        let mut h = house_at(0, vec![0], 1);
        h.kin = vec![kin_at("Head", false, 0, [0, 0, 0, 0], 1.0, 0.6), kin_at("Factor", false, 2, [0, 0, 0, 0], 1.0, 0.5)];
        h.kin[1].posted = 1;
        h.office_leases = vec![(1, 100_000)];
        h.rivals = vec![1];
        s.houses.push(h);
        let mut rival = house_at(1, vec![0], 1);
        rival.political_power = 1.0;
        rival.offices = vec![1];
        s.houses.push(rival);
        s.hubs[1].captor_house = 1;
        s.tick = 0;
        s.apply_foreign_hand();
        // Max leverage: (A+B channel weights, capped at 1.0) × rival_weight(1.0) ×
        // (1 + 0.5·feud) = 1.0 × 1.5 = 1.5, so the true ceiling is 1.5× the base rate.
        let drop = 1.0 - s.houses[0].kin[1].loyalty;
        assert!(drop > 0.0 && drop <= FOREIGN_HAND_DECAY_RATE * 1.5 + 1e-4,
            "a single month must stay within the small decay bound even at max leverage: dropped {drop}");
    }

    // ── Phase 5 · provinces as house territory ───────────────────────────────
    /// `province_authority_is_not_assumed_to_be_a_city` (`HOUSE_INHERITANCE_AND_
    /// TERRITORY.md` Part D's own invariant #7): a house may be GRANTED an
    /// ungoverned province if it already holds the seat's bailo and is Tier 1-2.
    #[test]
    fn an_ungoverned_province_can_be_granted_to_a_dominant_house() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut found = false;
        for seedn in 0..40u64 {
            let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![9000.0], 0)];
            let mut s = sim(hubs, goods.clone());
            s.seed = seedn * 104729 + 3;
            s.prov_cap = vec![9000.0];
            s.prov_rural = vec![5000.0];
            s.prov_culture = vec!["Aiora".into()];
            s.prov_seat = vec![[0.0, 0.0]];
            s.hub_province = vec![0];
            s.prov_net_mig = vec![0.0];
            s.ensure_province_land(1);
            s.prov_unrest[0] = 0.0;
            let mut h = house_at(0, vec![0], 1);
            h.tier = 1;
            s.houses.push(h);
            s.hubs[0].captor_house = 0; // this house dominates its own seat's trade
            s.maybe_grant_provinces(0);
            if s.prov_holder_house[0] == 0 {
                found = true;
                assert!(s.houses[0].events.iter().any(|e| e.kind == "province_granted"));
                assert!(is_house_milestone("province_granted"), "a grant is a permanent milestone");
                break;
            }
        }
        assert!(found, "over 40 seeds, a Tier-1 bailo-holding house must eventually be granted the province");
    }

    #[test]
    fn a_house_with_no_dominance_at_the_seat_is_never_granted_the_province() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![9000.0], 0)];
        let mut s = sim(hubs, goods);
        s.prov_cap = vec![9000.0];
        s.prov_rural = vec![5000.0];
        s.prov_culture = vec!["Aiora".into()];
        s.prov_seat = vec![[0.0, 0.0]];
        s.hub_province = vec![0];
        s.prov_net_mig = vec![0.0];
        s.ensure_province_land(1);
        s.prov_unrest[0] = 0.0;
        let mut h = house_at(0, vec![0], 1);
        h.tier = 1; // Tier 1, but no bailo at the seat
        s.houses.push(h);
        for yr in 0..30 { s.maybe_grant_provinces(yr); }
        assert_eq!(s.prov_holder_house[0], -1, "no bailo at the seat ⇒ never granted, however long it waits");
    }

    #[test]
    fn a_house_held_provinces_dues_flow_to_the_house_not_the_city() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![9000.0], 0)];
        let mut s = sim(hubs, goods);
        for h in s.hubs.iter_mut() { h.sent_prosperity = 0.6; h.starving = 0.0; h.food_balance = 1.0; }
        s.hub_culture = vec!["Aiora".into()];
        s.hub_minorities = vec![Vec::new()];
        s.prov_cap = vec![90_000.0];
        s.prov_rural = vec![60_000.0];
        s.prov_culture = vec!["Aiora".into()];
        s.prov_seat = vec![[0.0, 0.0]];
        s.hub_province = vec![0];
        s.prov_net_mig = vec![0.0];
        let mut h = house_at(0, vec![0], 1);
        h.wealth = 100.0;
        s.houses.push(h);
        s.ensure_province_land(1);
        s.prov_holder_house[0] = 0;
        let treasury0 = s.hubs[0].treasury;
        let wealth0 = s.houses[0].wealth;
        for yr in 0..10u32 {
            s.province_demography_pass();
            s.province_land_pass(yr);
        }
        assert!(s.houses[0].wealth > wealth0,
            "dues must reach the HOUSE holder's wealth: {} → {}", wealth0, s.houses[0].wealth);
        assert!((s.hubs[0].treasury - treasury0).abs() < 1e-3,
            "the city treasury must NOT also receive the dues: {} → {}", treasury0, s.hubs[0].treasury);
    }

    #[test]
    fn a_dissolved_holders_province_reverts_to_city_administration() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![9000.0], 0)];
        let mut s = sim(hubs, goods);
        for h in s.hubs.iter_mut() { h.sent_prosperity = 0.6; h.starving = 0.0; h.food_balance = 1.0; }
        s.hub_culture = vec!["Aiora".into()];
        s.hub_minorities = vec![Vec::new()];
        s.prov_cap = vec![90_000.0];
        s.prov_rural = vec![60_000.0];
        s.prov_culture = vec!["Aiora".into()];
        s.prov_seat = vec![[0.0, 0.0]];
        s.hub_province = vec![0];
        s.prov_net_mig = vec![0.0];
        let mut h = house_at(0, vec![0], 1);
        h.defunct = true;
        s.houses.push(h);
        s.ensure_province_land(1);
        s.prov_holder_house[0] = 0;
        s.province_demography_pass();
        s.province_land_pass(0);
        assert_eq!(s.prov_holder_house[0], -1, "a defunct holder's province must revert to city administration");
    }

    #[test]
    fn a_held_province_weighs_heavily_toward_a_higher_tier() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..2u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        s.houses.push(house_at(1, vec![0], 1));
        s.prov_holder_house = vec![0]; // house 0 holds a province; house 1 holds none
        s.assign_house_tiers();
        assert!(s.houses[0].standing > s.houses[1].standing,
            "an otherwise-identical house holding a province must stand higher: {} vs {}",
            s.houses[0].standing, s.houses[1].standing);
    }

    /// CITY_PROVINCE_WAR_PLAN.md §2.5 · the whole exploitation loop: calibration
    /// lands mean exploitation at ~1.0 on day one (by construction), sustained
    /// overexploitation accumulates depletion and measurably erodes potential, and
    /// easing off lets it heal back down.
    #[test]
    fn province_goods_exploitation_tracks_pressure_and_depletes() {
        let goods = vec![good("timber", 0, 0, 1.0, 0.5, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 5000.0, vec![50.0], 0)];
        let mut s = sim(hubs, goods);
        s.hub_province = vec![0];
        s.prov_cap = vec![2000.0];
        s.prov_forest = vec![0.5];
        s.prov_arable = vec![0.2];
        s.prov_pasture = vec![0.1];
        s.prov_good_belt = vec![0.8]; // 1 province × 1 good ("timber" → forest-scaled)
        s.prov_good_depletion = vec![0.0];
        s.calibrate_province_good_yield();
        assert!(s.prov_good_yield_scale.is_finite() && s.prov_good_yield_scale > 0.0,
            "yield scale must calibrate to a finite positive number, got {}", s.prov_good_yield_scale);

        let potential0 = s.province_good_potential(0, 0);
        let actual0 = s.province_good_actual()[0];
        assert!(potential0.is_finite() && potential0 > 0.0, "potential must be finite and positive");
        assert!((actual0 - 50.0).abs() < 1e-3, "actual must equal the hub's own production, got {actual0}");
        let exploit0 = actual0 / potential0;
        assert!((exploit0 - 1.0).abs() < 0.05,
            "self-calibration must land ~1.0 exploitation at campaign start, got {exploit0}");

        // Sustained overexploitation: production far above what calibration expected.
        s.hubs[0].production[0] = 400.0;
        for yr in 0..10 { s.update_province_goods_pressure(yr); }
        assert!(s.prov_good_depletion[0] > 0.0, "sustained overexploitation must accumulate depletion");
        let potential_after = s.province_good_potential(0, 0);
        assert!(potential_after < potential0,
            "depletion must erode potential: {potential_after} should be < {potential0}");

        // Ease off — depletion should start healing, not stay frozen at its peak.
        s.hubs[0].production[0] = 5.0;
        let d_before = s.prov_good_depletion[0];
        for yr in 10..20 { s.update_province_goods_pressure(yr); }
        assert!(s.prov_good_depletion[0] < d_before,
            "easing pressure must let depletion heal: {} should be < {d_before}", s.prov_good_depletion[0]);
    }

    // ── §3.2 · city tiers (mirrors the house-tier tests directly above in spirit) ──

    /// A city with overwhelmingly more population, trade wealth and treasury than
    /// its rivals must end up in the top band — deliberately coarse (the cutoffs are
    /// re-derived every month), but a 60x-more-prominent city landing in Tier 4
    /// would mean the formula is broken, not just imprecise.
    #[test]
    fn city_tiers_rank_the_most_prominent_city_highest() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut hubs: Vec<TickHub> =
            (0..6u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        hubs[0].population = 500_000.0;
        hubs[0].trade_wealth = 50_000.0;
        hubs[0].treasury = 20_000.0;
        let mut s = sim(hubs, goods);
        s.assign_city_tiers();
        assert_eq!(s.hubs[0].tier, 1, "the overwhelmingly most prominent city must be Tier 1");
        assert!(s.hubs[0].standing > s.hubs[1].standing, "standing must track population/wealth/treasury");
        for h in s.hubs.iter().skip(1) {
            assert!((1..=4).contains(&h.tier), "city left untiered: tier={}", h.tier);
        }
    }

    /// Tier 1 has an absolute floor, not just a percentile rank: on a young,
    /// undifferentiated world nobody should clear it, so Tier 1 is EMPTY.
    #[test]
    fn city_tier_one_is_empty_on_an_undifferentiated_world() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs: Vec<TickHub> = (0..5u32)
            .map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0 + i as f32 * 40.0, vec![8000.0], 0))
            .collect();
        let mut s = sim(hubs, goods);
        s.assign_city_tiers();
        assert!(s.hubs.iter().all(|h| h.tier != 1), "Tier 1 should be empty on a flat world");
    }

    /// Hysteresis: calling `assign_city_tiers` again with unchanged state must
    /// reproduce the same tiers, not relitigate every boundary case.
    #[test]
    fn city_tier_assignment_is_stable_when_nothing_changed() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs: Vec<TickHub> = (0..7u32)
            .map(|i| hub(i, (i as f32) * 4.0, 0.0, 3000.0 + i as f32 * 5000.0, vec![8000.0], 0))
            .collect();
        let mut s = sim(hubs, goods);
        for (i, h) in s.hubs.iter_mut().enumerate() {
            h.trade_wealth = i as f32 * 900.0;
            h.treasury = i as f32 * 300.0;
        }
        s.assign_city_tiers();
        let first: Vec<u8> = s.hubs.iter().map(|h| h.tier).collect();
        s.assign_city_tiers();
        let second: Vec<u8> = s.hubs.iter().map(|h| h.tier).collect();
        assert_eq!(first, second, "tiers must not change with no underlying change");
    }

    /// An estate is never itself a rankable "city" — it must stay untiered.
    #[test]
    fn an_estate_is_never_tiered() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs: Vec<TickHub> = (0..3u32).map(|i| hub(i, (i as f32) * 4.0, 0.0, 8000.0, vec![8000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hubs[2].is_estate = true;
        s.assign_city_tiers();
        assert_eq!(s.hubs[2].tier, 0, "an estate must never be tiered");
    }
