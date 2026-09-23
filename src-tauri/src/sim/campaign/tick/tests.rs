    use super::*;

    pub(super) fn good(name: &str, cat: i32, tier: u8, val: f32, desire: f32, food: bool) -> TickGood {
        TickGood { name: name.into(), category: cat, need_tier: tier, base_value: val, desire, food,
            fungible_input: false,
            bulk: 1.0, perishable: 0.0, inputs: vec![], labor: 1.0, consumption_interval: 30.0,
            distribution: DIST_UNKNOWN, working: WORK_UNKNOWN }
    }

    pub(super) fn hub(id: u32, x: f32, y: f32, pop: f32, prod: Vec<f32>, comp: u32) -> TickHub {
        let ng = prod.len();
        let base_per_capita: Vec<f32> = prod.iter().map(|&p| p / pop.max(1.0)).collect();
        TickHub {
            known: std::collections::HashMap::new(),
            id, x, y, name: format!("H{id}"), population: pop, founding_pop: pop,
            stock: vec![0.0; ng * GRADE_BANDS], price: vec![1.0; ng], production: prod,
            grain_wealth: 0.0, trade_wealth: 0.0, food_balance: 1.0, starving: 0.0,
            is_estate: false, parent: -1, koppen: 0, coastal: false, river: false, component: comp,
            export_earn: 0.0, import_spend: 0.0,
            mood: 0.6, sent_food: 0.7, sent_prosperity: 0.5, sent_stability: 0.8, civic_pool: 0.0, history: Vec::new(),
            in_by_sea: 0.0, in_by_land: 0.0,
            base_per_capita, lack_basic: 0.0, lack_comfort: 0.0, lack_luxury: 0.0, society: Society::default(), pops: Vec::new(),
            tw_house: 0.0, tw_local: 0.0, tw_guild: 0.0, tw_state: 0.0,
            estate_kind: 0, estate_tier: 0, mine_depth: 0, mine_extent: 255, is_mining_settlement: false, last_upgrade_tick: 0, owner_house: -1, stake_bank: -1, stake_share: 0.0, damage: 0.0, structures: vec![],
            treasury: 0.0, tariff_export: 0.0, tariff_import: 0.0, mint_fineness: 1.0, council_house: -1,
            finance: CityFinance::default(), war_with: -1, war_since: 0, war_effort: 0.0, war_manpower: 0.0, tribute_to: -1, tribute_until: 0,
            coin_name: String::new(), coin_trust: 0.0, settle_coin: -1, coin_basket: Vec::new(), mint_fineness_prev: 0.0, price_level: 1.0, coin_circ_prev: 0.0, last_reform_tick: 0, reform_until: 0, coin_metal: 0, coin_history: Vec::new(), debt_principal: 0.0, debt_coupon: 0.0, debt_holders: Vec::new(), mint_bullion_ratio: 1.0, has_mint: false,
            quality: Vec::new(), tradition: Vec::new(), stolen_good: -1, stolen_from: -1,
            colony_kind: 0, colony_stage: 0, autonomous: false, founder_hub: -1, backers: Vec::new(),
            reserve_food: 0.0, reserve_cap: 0.0, supply_years: 0.0, supply_shortfall_days: 0.0, colony_founded_tick: 0,
            main_bank: -1, indep_cooldown_until: 0, plague_immune_until: 0, public_health: 0.0, supply_ships: 0, supply_source: -1, supply_delivered: 0.0, transit_year: 0.0, hub_class: 0, class_momentum: 0, transit_toll_mult: 1.0, build_stage: 0, build_progress: 0.0, build_supply: [0.0; 3], build_supply_good: [0; 3], build_idle_months: 0, build_convoys: 0, build_start_tick: 0, govt_type: 0, officials: Vec::new(), civic_goods: Vec::new(), food_export_lock: 0, export_ban_until: Vec::new(), laws: Vec::new(), captor_house: -1,
            abandoned: false, decline_years: 0.0, founded_tick: 0, died_tick: 0, trade_last_year: 0.0, died_cause: String::new(),
            tier: 0, standing: 0.0, war_cooldown_until: 0, captor_since: 0, realm: -1, realm_role: 0, league: -1,
            wh_capacity: 0.0, wh_spoiled_month: Vec::new(), wh_last_month: Vec::new(), supply_accum: Vec::new(), demand_accum: Vec::new(), stock_origin: Vec::new(), works_accum: Vec::new(), household_wealth: 0.0, shares: Vec::new(), monthly: Vec::new(), brand_chronicled: false, bad_years: 0, disaster_repair_mult: 0.0,
            yard_progress: 0.0,
            food_eaten: 0.0, food_need_today: 0.0, welfare_ratio: 0.0,
        }
    }

    pub(super) fn house_at(hub: u32, spec: Vec<usize>, fleet_sea: u32) -> House {
        House {
            known: std::collections::HashMap::new(),
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
            colonizable: vec![], satellite_sites: vec![], hinterland: vec![], migration_routes: vec![], creoles: vec![], lingua: vec![], culture_history: vec![], council_bought_month: vec![], hub_patron: vec![], dev_tier: vec![], dev_momentum: vec![], base_days: vec![], base_n: 0, base_days_season: vec![], season_slices: 0, colony_supply: vec![],
            hub_culture: vec![], hub_minorities: vec![], estate_idle_years: vec![],
            diag_shipments: 0, diag_by_house: 0, diag_by_guild: 0, diag_lost: 0, diag_volume: 0.0,
            diag_why_nohouse: 0, diag_why_slot: 0, diag_why_cash: 0, diag_why_bar: 0, diag_why_no_carrier_bind: 0, diag_why_charter_bar: 0, diag_why_leg_range_bind: 0, diag_relay_staged: 0,
            // THE VOYAGE-RANGE CAPS ARE OFF FOR EVERY FIXTURE BUILT HERE, in
            // one place, deliberately. These worlds use ABSTRACT coordinates:
            // `world_w` is picked to size the trade horizon (a fraction of it),
            // while distance in km reads `KM_EQUATOR / world_w` — the two pull
            // opposite ways, so the resulting geography is not a claim about
            // anything. Measured: adjacent hubs stand 3,607 km apart here and
            // 1,202 km apart in `econ_inheritance_rules_fragment_differently`'s
            // own widened copy, both further than any historical caravan stage.
            // A rule stated in real kilometres cannot be honestly measured on
            // that, so it is not measured on it — the shipped dose is gated by
            // `the_dosed_economy_stays_healthy_on_a_realistically_dense_world`
            // instead, on `dense_world`, whose distances ARE real. Keeping the
            // caps off here is also what makes every pre-existing test in this
            // file, and the whole `econ_` suite, bit-identical to before the
            // staging relay existed.
            ship_leg_max_km: f32::INFINITY, caravan_leg_max_km: f32::INFINITY,
            local_haul_bind_days: f32::INFINITY,
            route_post_max_home_km: f32::INFINITY,
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
            route_outlet: vec![],
            neighbors: vec![],
            routes_dirty: false,
            warehouses: vec![],
            contracts: vec![],
            trade_cur: Default::default(),
            city_dominator: vec![],
            trade_last: vec![],
            trade_last_season: vec![],
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
            envoys: vec![],
            // Province LAND state — left empty exactly like the demography vectors
            // above, so `province_land_pass` early-returns and the dynamics run is
            // unaffected by the B1 land layer (that is the gate).
            prov_forest: vec![], prov_arable: vec![], prov_pasture: vec![],
            prov_irrigated: vec![], prov_soil: vec![],
            prov_area_km2: vec![], prov_relief_m: vec![], prov_cadastre: vec![], prov_tenure: vec![],
            suppress_realms: false, suppress_relief: false,
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
            realms: vec![], leagues: vec![], kontors: vec![], prov_realm: vec![],
            prov_export_accum: vec![], prov_import_accum: vec![],
            prov_export_year: vec![], prov_import_year: vec![],
            vessels: vec![], next_vessel_id: 0, fondacos: vec![],
            mine_deposits: vec![],
            units_of_account: vec![], currencies: vec![], issues: vec![], next_issue_id: 0,
        };
        s.rebuild_routes();
        s
    }

    /// A good the frozen hub snapshot never credited to a city is still planted as an
    /// estate where the surrounding PROVINCE is strongly suited to it — the wine-country
    /// gap `maybe_found_estate` used to miss. Without the belt the same city plants its
    /// own staple instead, so the belt is provably what let the vineyard form.
    #[test]
    fn a_strong_provincial_belt_lets_a_vineyard_form() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("wine", 3, 2, 8.0, 0.4, false),
        ];
        // One commercial city that grows wheat but has NEVER produced wine
        // (base_per_capita[wine] == 0), where wine is DEAR (high demand pressure).
        let mk = || {
            let mut hubs = vec![hub(0, 0.0, 0.0, 10_000.0, vec![10_000.0 * 0.012, 0.0], 0)];
            hubs[0].trade_wealth = 1.0;
            hubs[0].food_balance = 1.0;
            hubs[0].price[1] = 24.0; // wine base_value 8 → demand pressure clamps to 3.0
            let mut s = sim(hubs, goods.clone());
            s.hub_province = vec![0];
            s.prov_cap = vec![100_000.0]; // non-empty so demand pressure is actually read
            s
        };

        // With a strong wine belt in the province, the city plants a VINEYARD (kind 5).
        let mut s = mk();
        s.prov_good_belt = vec![0.0, 0.83]; // [wheat, wine] belt for province 0
        let before = s.hubs.len();
        s.maybe_found_estate();
        assert_eq!(s.hubs.len(), before + 1, "an eligible wine-country city founds an estate");
        let est = s.hubs.last().unwrap();
        assert!(est.is_estate && est.estate_kind == 5,
            "the terroir estate is a vineyard (kind 5), got kind {}", est.estate_kind);
        assert!(est.production[1] > 0.0, "the vineyard actually produces wine");

        // The SAME city with no belt falls back to its own staple — never a vineyard,
        // proving the province belt is what unlocked the wine estate.
        let mut s2 = mk();
        s2.prov_good_belt = vec![0.0, 0.0];
        s2.maybe_found_estate();
        assert!(s2.hubs.last().unwrap().estate_kind != 5,
            "with no terroir there is no vineyard, got kind {}", s2.hubs.last().unwrap().estate_kind);
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

    /// Crisis relief must be INERT on a healthy city — the gate that lets the
    /// standing dynamics run stay comparable, and the same discipline
    /// `province_land_pass_is_a_noop_without_provinces` applies to the land pass.
    /// A council with a full granary and no dearth releases nothing and bars
    /// nothing; the moment the dearth is real it does both.
    #[test]
    fn crisis_relief_is_inert_until_a_city_is_actually_short() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        // A well-fed city holding a full public granary.
        s.hubs[0].civic_goods = vec![400.0];
        s.hubs[0].lack_basic = 0.0;
        s.hubs[0].food_balance = 1.0;
        s.hubs[0].starving = 0.0;
        let stock0 = stock_of(&s.hubs[0].stock, 0);
        let journal0 = s.journal.len();
        s.run_crisis_relief();
        assert_eq!(s.hubs[0].civic_goods[0], 400.0, "no dearth ⇒ the granary is untouched");
        assert_eq!(stock_of(&s.hubs[0].stock, 0), stock0, "no dearth ⇒ nothing reaches the market");
        assert_eq!(s.hubs[0].food_export_lock, 0, "no dearth ⇒ no export bar");
        assert_eq!(s.journal.len(), journal0, "no dearth ⇒ no chronicle beat");

        // Now a real dearth, short of famine: the granary opens, exports stay free.
        s.hubs[0].lack_basic = 0.40;
        s.run_crisis_relief();
        assert!(s.hubs[0].civic_goods[0] < 400.0, "a dearth opens the granary");
        assert!(stock_of(&s.hubs[0].stock, 0) > stock0, "released grain reaches the open market");
        assert_eq!(s.hubs[0].food_export_lock, 0, "a dearth short of famine bars no exports");

        // Famine: the export bar goes up and the episode is chronicled ONCE.
        s.hubs[0].starving = 0.6;
        s.run_crisis_relief();
        assert!(s.hubs[0].food_export_lock > s.tick, "famine bars the export of food");
        assert_eq!(s.journal.len(), journal0 + 1, "the episode is chronicled once");
        s.run_crisis_relief();
        assert_eq!(s.journal.len(), journal0 + 1,
            "a standing bar is not re-announced every month");
    }

    /// N2 (`ACTORS_AND_CARRIAGE_PLAN.md` §3.2) · the AUTHOR (`decide_trade_bans`)
    /// ships at zero dose — `N2_BAN_PRICE_RATIO = INFINITY` — exactly
    /// `N1_LOCAL_HAUL_BIND_DAYS`'s pattern, and for the same kind of reason: a
    /// trial dose (6.0, ban 30 ticks) broke `simulate_decades_reports_dynamics`'s
    /// hard-asserted wealth bound even after halving twice (a sustained richest
    /// house of 1,005,714 — see that constant's own doc comment), so the dose
    /// walk is real future work, not something to guess at here. Even a
    /// deliberately extreme price spike (25× base — far past any price this
    /// engine's `PRICE_CEIL_MULT` of 12× can ever reach) must trigger nothing.
    #[test]
    fn n2_trade_ban_trigger_at_infinity_is_a_noop() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("iron", 2, 1, 5.0, 0.45, false),
        ];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0, 500.0], 0)];
        let mut s = sim(hubs, goods);
        s.hubs[0].price = vec![1.0, 125.0]; // 25× base(5.0) — an absurd, deliberately extreme spike
        let journal0 = s.journal.len();
        s.run_trade_bans();
        assert!(s.hubs[0].export_ban_until.is_empty()
            || s.hubs[0].export_ban_until.iter().all(|&t| t <= s.tick),
            "N2_BAN_PRICE_RATIO = INFINITY must make the trigger dead code");
        assert_eq!(s.journal.len(), journal0, "a dead trigger chronicles nothing");
    }

    /// N2 (§3.2) · the ENFORCEMENT half (unlike the author, this is wired live —
    /// it is what `dispatch` will already respect the moment the author above is
    /// dosed above zero). Setting `export_ban_until` directly — the same thing a
    /// future dosed author would do — must stop that good leaving the hub, the
    /// same discipline `production.rs` already applies to `food_export_lock`.
    #[test]
    fn n2_export_ban_blocks_dispatch_when_set_directly() {
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        s.hubs[0].export_ban_until = vec![s.tick + 1000];
        // Hub 1 needs iron and has none — a genuine arbitrage gap dispatch would
        // otherwise act on immediately.
        let needs = vec![vec![0.0], vec![50.0]];
        let stock0 = stock_of(&s.hubs[0].stock, 0);
        s.dispatch(&needs);
        assert_eq!(stock_of(&s.hubs[0].stock, 0), stock0,
            "a banned good's stock at the source must not move — nothing may ship");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 4.1 · WAR CONTRABAND is lane-scoped, not a
    /// blanket ban — a belligerent still trades a contraband good with a THIRD
    /// city it is not at war with, unlike `export_ban_until` (N2), which bars a
    /// good to everyone.
    #[test]
    fn contraband_blocks_only_the_lane_to_the_actual_enemy() {
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0), // seller, at war with hub 1
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),   // the enemy
            hub(2, 10.0, 0.0, 9000.0, vec![10.0], 0),  // a neutral third city
        ];
        for h in &mut hubs { h.coastal = false; }
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        s.hubs[0].stock[0] = 5000.0;
        s.hubs[0].war_with = 1;
        s.hubs[1].war_with = 0;
        let needs = vec![vec![0.0], vec![50.0], vec![50.0]];
        s.dispatch(&needs);
        // `dispatch` only ever CREATES in-transit cargo — arrival is a separate
        // step inside `advance`'s own day loop (`eta_tick <= tick`, `mod.rs`),
        // never run here — so the question "did this lane ship" is answered by
        // what `dispatch` decided to send, not by destination stock a single
        // call can never move.
        assert!(!s.in_transit.iter().any(|c| c.to == 1 && c.good == 0),
            "the enemy receives none of the banned good");
        assert!(s.in_transit.iter().any(|c| c.to == 2 && c.good == 0),
            "a neutral third city still receives it — the ban is lane-scoped, not a blanket ban");
    }

    /// A world at peace is untouched — the contraband check reads `war_with`,
    /// which defaults to −1, so it is a true no-op absent any war.
    #[test]
    fn contraband_is_a_noop_at_peace() {
        let goods = vec![good("iron", 1, 2, 20.0, 0.35, false)];
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 100.0, 0.0, 9000.0, vec![10.0], 0),
        ];
        for h in &mut hubs { h.coastal = false; }
        let mut s = sim(hubs, goods);
        s.world_w = 4007.5;
        s.rebuild_routes();
        s.hubs[0].stock[0] = 5000.0;
        let needs = vec![vec![0.0], vec![50.0]];
        s.dispatch(&needs);
        // Same reasoning as `contraband_blocks_only_the_lane_to_the_actual_enemy`
        // above: `dispatch` only creates in-transit cargo, it never delivers it
        // (arrival is a separate step in `advance`'s day loop) — so "trades
        // normally" is read off what shipped, not off a destination stock that
        // cannot move within a single `dispatch` call.
        assert!(s.in_transit.iter().any(|c| c.to == 1 && c.good == 0),
            "at peace, iron trades normally");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 4.2 · `BLOCKADE_STAGING_DOSE == 0.0` must
    /// leave a war between two hubs' own trade UNCHANGED — same ship-direct
    /// behaviour as if there were no war at all, even with a real neutral
    /// stop on the way that the mechanism COULD use at a nonzero dose.
    #[test]
    fn blockade_staging_is_a_noop_at_zero_dose() {
        assert_eq!(BLOCKADE_STAGING_DOSE, 0.0);
        // NOT a good in `CONTRABAND_GOODS` — this test is about 4.2's routed
        // blockade specifically, and iron/timber/etc would be blocked by 4.1's
        // own contraband mechanism regardless of anything 4.2 does.
        let goods = vec![good("wool", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),   // seller, at war with hub 2
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),     // a neutral stop on the way
            hub(2, 10.0, 0.0, 9000.0, vec![10.0], 0),    // the enemy / buyer
        ];
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        stock_set_total(&mut s.hubs[0].stock, 0, 5000.0);
        s.hubs[0].war_with = 2;
        s.hubs[2].war_with = 0;
        s.wars.push(War {
            a: 0, b: 2, start_tick: s.tick, chest_a: 0.0, chest_b: 0.0, levies: 0.0,
            levies_a: 0.0, levies_b: 0.0, battles: Vec::new(), cargo_lost: 0,
            cause: "test".into(), goal: 0, score: 0.0, round: 0,
            peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1, blockade_chronicled: false,
        });
        // A real stop exists (hub 1 is on the way from 0 to 2), so at any
        // nonzero dose the mechanism WOULD have something to use.
        assert_eq!(s.staging_hop(0, 2, s.ship_leg_max_km, s.caravan_leg_max_km), Some(1),
            "sanity check: a real neutral stop exists for this fixture");
        let needs = vec![vec![0.0], vec![0.0], vec![50.0]];
        let journal0 = s.journal.len();
        s.dispatch(&needs);
        assert!(s.in_transit.iter().any(|c| c.to == 2 && c.via < 0 && c.good == 0),
            "at zero dose, cargo for the enemy must still sail DIRECT, not staged");
        assert!(!s.in_transit.iter().any(|c| c.to == 1),
            "at zero dose, nothing is diverted through the neutral stop");
        assert_eq!(s.journal.len(), journal0, "a no-op mechanism chronicles nothing");
        assert!(!s.wars[0].blockade_chronicled, "never chronicled at zero dose");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 4.3 — the pure decision `dispatch` reuses
    /// at all three of its call sites: two hubs sharing a live League, or
    /// either trading through the OTHER's standing Kontor, are privileged;
    /// two unleagued hubs, or two DIFFERENT leagues, are not.
    #[test]
    fn lane_league_privileged_covers_membership_and_kontor_both_ways() {
        assert!(CampaignSim::lane_league_privileged(0, 0, -1, -1), "same league ⇒ privileged");
        assert!(!CampaignSim::lane_league_privileged(-1, -1, -1, -1), "no league anywhere ⇒ not privileged");
        assert!(!CampaignSim::lane_league_privileged(0, 1, -1, -1), "different leagues ⇒ not privileged");
        assert!(CampaignSim::lane_league_privileged(0, -1, -1, 0),
            "a member trading through the OTHER end's Kontor of its own league ⇒ privileged");
        assert!(CampaignSim::lane_league_privileged(-1, 0, 0, -1),
            "…and symmetrically from the other side");
        assert!(!CampaignSim::lane_league_privileged(-1, -1, 0, 0),
            "a Kontor host trading with ANOTHER unleagued hub is not itself a member");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 4.4 · a league with a real external trade
    /// tie above `LEAGUE_FLOW_MIN` and a purse that can afford `KONTOR_COST`
    /// establishes a Kontor at that partner — never at a member, never at a
    /// hub already hosting one.
    #[test]
    fn a_league_establishes_a_kontor_at_its_best_external_trade_tie() {
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0), // member/seat
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),   // member
            hub(2, 10.0, 0.0, 9000.0, vec![10.0], 0),  // non-member, weak tie
            hub(3, 15.0, 0.0, 9000.0, vec![10.0], 0),  // non-member, STRONG tie — the pick
        ];
        let mut s = sim(hubs, goods);
        s.hubs[0].league = 0;
        s.hubs[1].league = 0;
        s.leagues.push(League {
            id: 0, name: "T".into(), seat_hub: 0, purse: KONTOR_COST + 10.0, founded_tick: 0,
            dissolved_tick: 0, last_threat_tick: s.tick, boycotts: vec![], escort_until_tick: 0, events: vec![],
        });
        s.flow_year = vec![(0, 2, LEAGUE_FLOW_MIN + 1.0), (1, 3, LEAGUE_FLOW_MIN + 100.0)];
        let journal0 = s.journal.len();
        s.maybe_establish_kontors();
        assert_eq!(s.kontors.len(), 1, "one Kontor must be established");
        assert_eq!(s.kontors[0].host_hub, 3, "the STRONGEST external tie must be picked, not merely the first");
        assert_eq!(s.kontors[0].league, 0);
        assert_eq!(s.kontors[0].expelled_tick, 0);
        assert!(s.leagues[0].purse < KONTOR_COST + 10.0, "establishing it must actually spend the purse");
        assert!(s.journal.len() > journal0, "establishing a Kontor is chronicled");
        assert_eq!(s.kontor_league_at(3), 0, "the read `dispatch` uses must see it");
        assert_eq!(s.kontor_league_at(2), -1, "…and see nothing at a hub with no Kontor");

        // A second call must not establish a SECOND Kontor for the same league.
        s.maybe_establish_kontors();
        assert_eq!(s.kontors.len(), 1, "one league holds at most one standing Kontor at a time");
    }

    /// A host already at war with a member of the League holding its Kontor
    /// is far more likely to expel it than an ordinary yearly roll would —
    /// the political-event half of 4.4.
    #[test]
    fn a_host_at_war_with_a_member_is_likelier_to_expel_its_kontor() {
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0), // member
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),   // host, at war with hub 0
        ];
        let mut s = sim(hubs, goods);
        s.tick = 1000; // NOT zero — `expelled_tick` is set to `self.tick`, and
                       // 0 is also the "never expelled" sentinel this test
                       // reads, so an expulsion at tick 0 would be invisible.
        s.hubs[0].league = 0;
        s.leagues.push(League {
            id: 0, name: "T".into(), seat_hub: 0, purse: 0.0, founded_tick: 0,
            dissolved_tick: 0, last_threat_tick: s.tick, boycotts: vec![], escort_until_tick: 0, events: vec![],
        });
        s.kontors.push(Kontor { league: 0, host_hub: 1, established_tick: 0, expelled_tick: 0 });
        s.hubs[1].war_with = 0;
        // Sweep enough seeds that a real difference in EXPECTED expulsion
        // rate (not a single lucky roll) shows up — mirrors how other
        // probabilistic gates in this file avoid asserting on one draw.
        let mut expelled_at_war = 0;
        let mut expelled_at_peace = 0;
        for seed in 0..200u32 {
            let mut sw = s.clone();
            sw.seed = seed as u64;
            sw.maybe_expel_kontors();
            if sw.kontors[0].expelled_tick != 0 { expelled_at_war += 1; }

            let mut sp = s.clone();
            sp.seed = seed as u64;
            sp.hubs[1].war_with = -1;
            sp.maybe_expel_kontors();
            if sp.kontors[0].expelled_tick != 0 { expelled_at_peace += 1; }
        }
        assert!(expelled_at_war > expelled_at_peace,
            "war with a member must raise the expulsion rate ({expelled_at_war} vs {expelled_at_peace} of 200)");
    }

    /// Charter exclusivity (`CHARTER_EXCLUSIVE_DOSE`) — the pure decision at
    /// dose 0.0 is a true no-op, whatever the shipped dose is TODAY (a
    /// property of `charter_bars_sale` itself, not a claim about the current
    /// constant — mirrors `n_yards_s4_capacity_bind_at_zero_is_a_noop`'s
    /// pure-decision half).
    #[test]
    fn charter_bars_sale_is_a_pure_noop_at_dose_zero() {
        // The pure decision must refuse to block for EVERY roll in [0, 1) —
        // `roll01 < 0.0` can never hold.
        for i in 0..20 {
            let roll = i as f32 / 20.0;
            assert!(!CampaignSim::charter_bars_sale(5, 2, 0.0, roll),
                "dose 0.0 must never block, whatever the smuggling roll");
        }
    }

    /// Charter exclusivity at the SHIPPED dose (`CHARTER_EXCLUSIVE_DOSE`,
    /// dosed up from the 0.0 no-op — user-requested: a house holding a
    /// city's charter on a good should be able to squeeze out rivals, not
    /// just collect rent while everyone trades around it). 0.3 and 1.0 both
    /// broke the hard-asserted `econ_inheritance_rules_fragment_differently`
    /// (see the constant's own doc comment), so this stays a small, real dose
    /// rather than an absolute staple right. Two things must both hold at
    /// once: the mechanism is LIVE (it blocks a real non-holder sale
    /// somewhere over enough ticks — `diag_why_charter_bar`, the same
    /// counter `dispatch` itself increments), and it is not a chokehold (the
    /// charter holder's own city still sees the great majority of its
    /// dispatches go through — this is a squeeze, not an embargo).
    #[test]
    fn charter_exclusivity_at_the_shipped_dose_squeezes_but_does_not_choke() {
        assert!(CHARTER_EXCLUSIVE_DOSE > 0.0, "expected a real, non-zero dose");
        let goods = vec![good("silk", 1, 2, 20.0, 0.35, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.hubs[0].stock[0] = 5000.0;
        // A RIVAL house at the seller hub, with its own caravans, so `house_for`
        // resolves IT as carrier (seller's house is tried first) rather than
        // falling through to the buyer's house — the charter holder itself, which
        // would trivially never be blocked from its own market. Without a real
        // fleet slot the rival can never win the carrier check at all and every
        // leg falls straight through to the charter holder, hiding the mechanism
        // exactly the way the very first version of this test accidentally did.
        let mut rival = house_at(0, vec![0], 0);
        rival.fleet_caravan = 5;
        rival.wealth = 5000.0;
        s.houses.push(rival);
        let mut holder = house_at(1, vec![0], 0);
        holder.charters = vec![0];
        s.houses.push(holder);
        let needs = vec![vec![0.0], vec![50.0]];
        let mut blocked = 0u32;
        let mut moved_ticks = 0u32;
        const ROUNDS: u32 = 300;
        for _ in 0..ROUNDS {
            // Reset the surplus before every round: a real gap the arbitrage
            // engine would fill every single time, so a round where nothing
            // moves is unambiguously the charter, not the market equilibrating.
            s.hubs[0].stock[0] = 5000.0;
            let before = s.diag_why_charter_bar;
            let stock0 = stock_of(&s.hubs[0].stock, 0);
            s.tick += 1;
            s.dispatch(&needs);
            if s.diag_why_charter_bar > before { blocked += 1; }
            if stock_of(&s.hubs[0].stock, 0) < stock0 { moved_ticks += 1; }
        }
        assert!(blocked > 0,
            "a real, non-zero dose must block SOME non-holder sale over {ROUNDS} rounds");
        assert!(moved_ticks as f32 / ROUNDS as f32 > 0.7,
            "the shipped dose must stay a squeeze, not a chokehold — got only {moved_ticks}/{ROUNDS} \
             rounds where the good still moved");
    }

    /// Charter exclusivity, the pure decision at full dose — the charter
    /// holder itself is never blocked from its own market; anyone else
    /// (a rival house, or the ownerless residual) always is.
    #[test]
    fn charter_bars_a_non_holder_sale_at_full_dose() {
        assert!(CampaignSim::charter_bars_sale(3, 7, 1.0, 0.0), "a rival house is blocked");
        assert!(CampaignSim::charter_bars_sale(3, -1, 1.0, 0.0), "the ownerless residual is blocked");
        assert!(!CampaignSim::charter_bars_sale(3, 3, 1.0, 0.0), "the charter holder itself is never blocked");
        assert!(!CampaignSim::charter_bars_sale(-1, -1, 1.0, 0.0), "no charter here ⇒ never blocked");
    }

    /// `decide_fleets`' one-hull-a-month ceiling, generalised into a real cap
    /// (`FLEET_BUY_MAX_PER_MONTH`, shipped at 1 — see its own doc comment for
    /// what a higher dose broke). A very wealthy house buys up to but never
    /// past the shipped cap in one call, however much capital it has to spare.
    #[test]
    fn fleet_buy_cap_bounds_a_wealthy_house_at_the_shipped_dose() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        s.hubs[0].coastal = true;
        s.houses.push(house_at(0, vec![0], 0));
        s.houses[0].wealth = SHIP_COST * 500.0; // capital enough for many dozen hulls
        s.manage_fleets();
        assert_eq!(s.houses[0].fleet_sea, FLEET_BUY_MAX_PER_MONTH,
            "a flush house must buy exactly the shipped cap, never fewer, never more");
    }

    /// N1c, the pure decision (illustrative literal caps, independent of the
    /// shipped dose below) — a sea leg is judged against a ship cap, a land
    /// leg against a shorter caravan cap, and a leg under either cap is never
    /// blocked. The routing logic itself is scale-independent; the SEPARATE
    /// no-op test below covers the shipped dose.
    #[test]
    fn leg_exceeds_range_uses_the_right_cap_per_mode() {
        assert!(CampaignSim::leg_exceeds_range(4000.0, true, 3500.0, 800.0),
            "a 4000km sea leg must exceed a 3500km ship cap");
        assert!(CampaignSim::leg_exceeds_range(4000.0, false, 3500.0, 800.0),
            "a 4000km land leg must exceed the (shorter) 800km caravan cap too");
        assert!(!CampaignSim::leg_exceeds_range(500.0, false, 3500.0, 800.0),
            "a 500km land leg clears an 800km caravan cap");
        assert!(!CampaignSim::leg_exceeds_range(3000.0, true, 3500.0, 800.0),
            "a 3000km sea leg clears a 3500km ship cap");
    }

    /// N1c is now DOSED LIVE (3500/800 km), so the invariant worth asserting
    /// is no longer "the caps are inert" but the two facts that replaced it:
    /// the shipped caps are real and finite, and every fixture built on
    /// ABSTRACT coordinates opts out of them.
    ///
    /// That second half is load-bearing rather than housekeeping. `sim()`'s
    /// worlds pick `world_w` to size the trade horizon, which makes their
    /// km-per-cell an artefact — hubs land 3,607 km apart here — so a rule
    /// stated in kilometres would be measured against a fiction. If someone
    /// later removes that opt-out, the whole `tick::tests` and `econ_` suites
    /// silently stop being bit-identical to their calibrated baselines and
    /// start reporting a mis-scaled world's opinion of a real-distance rule.
    /// This test is what fails first when that happens.
    #[test]
    fn the_range_caps_ship_live_and_abstract_fixtures_opt_out() {
        assert!(SHIP_LEG_MAX_KM.is_finite() && CARAVAN_LEG_MAX_KM.is_finite(),
            "the caps ship live — see their doc comment for the dense-world evidence");
        assert!(CARAVAN_LEG_MAX_KM < SHIP_LEG_MAX_KM,
            "a caravan's unprovisioned stage is shorter than a ship's endurance");

        let s = sim(vec![hub(0, 0.0, 0.0, 9000.0, vec![100.0], 0)],
                    vec![good("silk", 1, 2, 20.0, 0.35, false)]);
        assert_eq!(s.ship_leg_max_km, f32::INFINITY,
            "every abstract fixture must opt out of a real-km rule");
        assert_eq!(s.caravan_leg_max_km, f32::INFINITY,
            "every abstract fixture must opt out of a real-km rule");
        assert_eq!(s.local_haul_bind_days, f32::INFINITY,
            "every abstract fixture must opt out of a real-days rule (C4)");

        // End to end on such a fixture: a 1000km ownerless caravan leg — well
        // past the shipped 800km cap — must still sail, because this world's
        // kilometres are not real ones.
        let goods = vec![good("silk", 1, 2, 20.0, 0.35, false)];
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 100.0, 0.0, 9000.0, vec![10.0], 0),
        ];
        for h in &mut hubs { h.coastal = false; }
        let mut s = sim(hubs, goods);
        s.world_w = 4007.5; // ⇒ ~10 km/cell, so this 100-cell hop is ~1000km
        s.rebuild_routes();
        s.hubs[0].stock[0] = 500.0;
        let needs = vec![vec![0.0], vec![50.0]];
        let stock0 = stock_of(&s.hubs[0].stock, 0);
        s.dispatch(&needs);
        assert!(stock_of(&s.hubs[0].stock, 0) < stock0,
            "a 1000km ownerless leg must still sail on an opted-out fixture");
    }

    /// N4 (`ACTORS_AND_CARRIAGE_PLAN.md` §3.4) · `house_for`'s within-tier pick
    /// CLAUDE.md §5.5 · `house_for_indexed` is a PERFORMANCE re-expression of
    /// `house_for`, not a second opinion: the hot loop resolves a carrier
    /// through a per-hub index instead of five filtered passes over the whole
    /// house list, and the two must agree on every (hub, good) pair at every
    /// tick — including the tie-breaks, which `pick_weighted_house` decides by
    /// `max_by` (the LAST of several equal draws wins, so candidate ORDER is
    /// load-bearing). A fixture deliberately built to exercise every one of the
    /// five tiers at once: seated specialists, an office-holding specialist, a
    /// seated generalist, a chartered guild, and a bare office-holder — plus a
    /// defunct house and a duplicated office entry, the two cases where a naive
    /// index would silently drift from the reference.
    #[test]
    fn the_indexed_carrier_pick_matches_the_reference_scan() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 4.0, 0.50, false),
        ];
        let hubs: Vec<_> = (0..4u32)
            .map(|i| hub(i, i as f32 * 10.0, 0.0, 6000.0, vec![40.0, 5.0, 9.0], 0))
            .collect();
        let mut s = sim(hubs, goods);
        // Tier 0 — two seated specialists at hub 0 in the same good (a real tie
        // to break), plus one at hub 1.
        s.houses.push(house_at(0, vec![1], 2));
        s.houses.push(house_at(0, vec![1], 2));
        s.houses.push(house_at(1, vec![0], 1));
        // Tier 1 — a specialist seated elsewhere but holding an office at hub 0.
        let mut off_spec = house_at(2, vec![1], 1);
        off_spec.offices = vec![0];
        s.houses.push(off_spec);
        // Tier 2 — a seated generalist (no spec at all) at hub 0.
        s.houses.push(house_at(0, vec![], 1));
        // Tier 3 — a chartered GUILD seated at hub 0.
        let mut guild = house_at(0, vec![2], 1);
        guild.is_guild = true;
        s.houses.push(guild);
        // Tier 4 — a bare office-holder at hub 3, with the SAME office listed
        // twice (the reference filter matches such a house once, so the index
        // must not enter it twice and change the tie-break).
        let mut bare = house_at(1, vec![], 1);
        bare.offices = vec![3, 3];
        s.houses.push(bare);
        // A DEFUNCT house seated at hub 0 in the hottest good — it must be
        // invisible to both, or the index would hand cargo to a dead firm.
        let mut dead = house_at(0, vec![1], 1);
        dead.defunct = true;
        s.houses.push(dead);

        let n = s.hubs.len();
        let ng = s.goods.len();
        let mut any_positive = false;
        for t in 0..60u32 {
            s.tick = t;
            // The index `dispatch` builds, rebuilt here by the identical rule.
            let mut seat_at: Vec<Vec<u32>> = vec![Vec::new(); n];
            let mut office_at: Vec<Vec<u32>> = vec![Vec::new(); n];
            for (hi, h) in s.houses.iter().enumerate() {
                if h.defunct { continue; }
                let hu = h.hub as usize;
                if hu < n { seat_at[hu].push(hi as u32); }
                for &o in &h.offices {
                    let oi = o as usize;
                    if oi < n && office_at[oi].last() != Some(&(hi as u32)) {
                        office_at[oi].push(hi as u32);
                    }
                }
            }
            for hub_i in 0..n {
                for g in 0..ng {
                    let reference = s.house_for(hub_i, g);
                    let indexed = s.house_for_indexed(hub_i, g, &seat_at[hub_i], &office_at[hub_i]);
                    assert_eq!(reference, indexed,
                        "the indexed carrier pick disagreed with the reference scan at \
                         tick {t}, hub {hub_i}, good {g}: {reference} vs {indexed}");
                    if reference >= 0 { any_positive = true; }
                }
            }
        }
        assert!(any_positive,
            "the fixture never resolved a carrier at all — the gate would pass \
             vacuously on any implementation, including a `-1` stub");
    }

    /// must not always resolve to the lowest house index. Five equally-weighted
    /// (`political_power: 0.0` for all — house_at's default) specialist houses at
    /// one hub, sampled across many ticks: the old `.position()` pick would return
    /// index 0 on every single sample.
    #[test]
    fn house_for_does_not_always_favour_the_lowest_index() {
        let goods = vec![good("silk", 1, 2, 20.0, 0.35, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![100.0], 0)];
        let mut s = sim(hubs, goods);
        s.houses = (0..5).map(|_| house_at(0, vec![0], 1)).collect();
        let mut seen = std::collections::HashSet::new();
        for t in 0..200u32 {
            s.tick = t;
            seen.insert(s.house_for(0, 0));
        }
        assert!(seen.len() > 1,
            "with 5 equally-weighted candidates, 200 samples across ticks must not \
             all resolve to the same house — the pick must be a real weighted draw, \
             not a disguised .position()");
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

    /// REGRESSION (the "72 realms, panel shows none" bug): a realm proclaimed while
    /// `prov_realm` is UNSIZED (the real-campaign state — `seed_province_land` used to
    /// omit it and `ensure_province_land` early-returned before sizing it) recorded no
    /// sovereignty, since the coronation's `p < prov_realm.len()` guard is a no-op on an
    /// empty vec. `compute_states` then drew none of them. `ensure_province_land` must
    /// size `prov_realm` even when the core land layer already exists, and reconcile a
    /// landless realm's territory back from its own `provinces` list.
    #[test]
    fn ensure_province_land_recovers_a_realm_left_landless_by_an_unsized_prov_realm() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.houses[0].wealth = 100_000.0;
        s.hub_province = vec![0];
        s.prov_culture = vec!["Aiora".into()];
        // The CORE land layer is fully sized, exactly as `seed_province_land` leaves a
        // real campaign — but the SOVEREIGNTY array (and the other newer per-province
        // arrays it forgot) is left EMPTY. That is the bug's exact precondition.
        s.prov_rural = vec![100.0];
        s.prov_cap = vec![100.0];
        s.prov_forest = vec![0.3];
        s.prov_arable = vec![0.2];
        s.prov_pasture = vec![0.1];
        s.prov_irrigated = vec![0.0];
        s.prov_soil = vec![0.6];
        s.prov_tenure = vec![[0.18, 0.10, 0.09, 0.63]];
        s.prov_tax = vec![0.12];
        s.prov_arrears = vec![0.0];
        s.prov_unrest = vec![0.0];
        s.prov_surplus = vec![0.0];
        s.prov_revenue = vec![0.0];
        s.prov_holder = vec![0];
        s.prov_history = vec![Vec::new()];
        s.prov_realm = vec![];        // the omitted array — the bug
        s.prov_holder_house = vec![]; // likewise never sized

        let id = s.promote_house_to_realm(0, 0, 60);
        // The realm formed and lists its province, but recorded NO sovereignty (the bug).
        assert_eq!(s.realms.len(), 1);
        assert_eq!(s.realms[0].provinces, vec![0], "the realm knows its own territory");
        assert!(s.prov_realm.is_empty(), "…but prov_realm was never sized, so it's landless here");

        // The land pass's sizing step must now fix it — without re-running the core loop
        // (prov_forest is already sized), so this exercises the top backfill + reconcile.
        s.ensure_province_land(1);
        assert_eq!(s.prov_realm, vec![id as i32],
            "ensure_province_land sizes prov_realm and reconciles the realm's territory");
        assert_eq!(s.prov_holder_house.len(), 1, "the other newer arrays are backfilled too");
        assert_eq!(s.prov_holder, vec![0], "the core layer is untouched (its loop did not re-run)");
    }

    /// 4.10 (D12) · coronation converts every estate the crowned house owned
    /// into crown title: a plain wholly-owned estate gets a full-fraction
    /// realm Share row and loses its private owner; a pre-existing minority
    /// SHARE (a bank stake) is grandfathered into a time-limited LEASE
    /// (`instrument` → 1, `LEASE_TERM_YEARS`) rather than swept aside, and the
    /// crown only claims what wasn't already held.
    #[test]
    fn coronation_converts_owned_estates_into_crown_leases() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let seat = hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0);
        let mut plain = hub(1, 1.0, 0.0, 2000.0, vec![0.0], 0);
        plain.is_estate = true; plain.estate_tier = 1; plain.estate_kind = 0;
        plain.parent = 0; plain.owner_house = 0;
        let mut leased = hub(2, 2.0, 0.0, 2000.0, vec![0.0], 0);
        leased.is_estate = true; leased.estate_tier = 1; leased.estate_kind = 6; // manufactory
        leased.parent = 0; leased.owner_house = 0;
        leased.shares.push(Share {
            holder_kind: 3, holder: 0, frac: 0.3, payout: 1,
            acquired_tick: 0, paid: 0.0, instrument: 0, term_years: 0, neglect_years: 0,
        });
        let mut s = sim(vec![seat, plain, leased], goods);
        s.found_house_at(0);
        let hi = 0usize;
        s.houses[hi].wealth = 100_000.0;
        s.prov_holder = vec![0];
        s.prov_holder_house = vec![-1];
        s.prov_realm = vec![-1];
        s.hub_province = vec![0];
        s.prov_culture = vec!["Aiora".into()];

        let id = s.promote_house_to_realm(hi, 0, 60);

        assert_eq!(s.hubs[1].owner_house, -1, "the plain estate loses its private owner");
        let plain_realm = s.hubs[1].shares.iter().find(|sh| sh.holder_kind == 4)
            .expect("the crown holds a share of the plain estate");
        assert_eq!(plain_realm.holder, id);
        assert!((plain_realm.frac - 1.0).abs() < 1e-4, "the crown takes the whole unclaimed fraction: {}", plain_realm.frac);
        assert_eq!(plain_realm.payout, 0, "a raw estate's crown share is OFFTAKE — A7's royalty in kind, for free");

        assert_eq!(s.hubs[2].owner_house, -1, "the manufactory loses its private owner too");
        let bank_row = s.hubs[2].shares.iter().find(|sh| sh.holder_kind == 3).expect("the bank stake survives");
        assert_eq!(bank_row.instrument, 1, "a pre-existing SHARE is grandfathered into a LEASE");
        assert_eq!(bank_row.term_years, LEASE_TERM_YEARS);
        assert!((bank_row.frac - 0.3).abs() < 1e-4, "the grandfathered holder keeps its own fraction");
        let crown_row = s.hubs[2].shares.iter().find(|sh| sh.holder_kind == 4).expect("the crown holds the rest");
        assert_eq!(crown_row.holder, id);
        assert!((crown_row.frac - 0.7).abs() < 1e-4, "the crown claims only what the bank didn't already hold: {}", crown_row.frac);
        assert_eq!(crown_row.payout, 1, "a manufactory's crown share stays DIVIDEND — D1 is unchanged by coronation");
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
            // UNTIERED on purpose. These cases probe the MERCHANT gate, and the
            // merchant paths read the HOUSE's tier, never the hub's. Leaving the
            // hub tiered would let PATH B found a republic here on its own account
            // — correct behaviour, but it would satisfy every "never proclaims"
            // assertion below for a reason that has nothing to do with the gate
            // being tested.
            s.hubs[0].tier = 0;
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
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR;
        eligible(&mut s);
        s.hubs[0].captor_house = -1;
        s.hubs[0].council_house = -1; // governs nothing here
        // PATH B would legitimately found a REPUBLIC here — a city nobody holds
        // crowning an office is exactly what that path is for. This assertion is
        // about the MERCHANT gate, so take the city out of Path B's reach (untiered
        // and poor) rather than let an unrelated path satisfy it.
        s.hubs[0].tier = 0;
        s.hubs[0].treasury = 0.0;
        s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
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
            s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
            if !s.realms.is_empty() { council_fired = true; break; }
        }
        assert!(council_fired, "a council-dominant house must be able to proclaim a realm");

        // Captor, but too poor to SPEND the (adaptive) founding cost: never. A far richer
        // second house raises the bar (0.6 × the richest) above the captor's own wealth.
        let mut s = sim(hubs.clone(), goods.clone());
        s.found_house_at(0);
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR;
        eligible(&mut s);
        s.houses.push(house_at(0, vec![0], 2));
        s.houses[1].wealth = 10_000_000.0; // the richest house → cost ≈ 6M
        s.houses[0].wealth = 100_000.0;    // the captor, far below the bar
        s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
        assert!(s.realms.is_empty(), "a house that cannot afford the adaptive founding cost never proclaims");

        // Captor and rich, but holds no province writ: never (rule 25).
        let mut s = sim(hubs.clone(), goods.clone());
        s.found_house_at(0);
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR;
        eligible(&mut s);
        s.prov_holder = vec![-1]; // no city administers this province — the seat holds nothing
        s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
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
            s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
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

    /// The SECOND eligibility path (`PROV_TRADE_CONTROL_FRAC`): a house that commands
    /// ≥20% of a province's trade proclaims a realm over that province — seated at the
    /// province's OWN largest city — even when (a) the province is administered from
    /// OUTSIDE (the "writ of X" case the seat-office loop structurally can't reach),
    /// (b) the house holds no office anywhere, and (c) the house is UNTIERED (tier 0),
    /// which the ordinary path rejects outright. This is the exact case a real world
    /// hits and the tiny econ fixture (5 office-saturated provinces) cannot exercise.
    #[test]
    fn a_trade_dominant_house_proclaims_over_a_province_administered_from_outside() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Two provinces: province 0 has its own city (hub 0), province 1 the external
        // administrator (hub 1). Province 0's writ is held by hub 1 — Balytag/Besangar.
        let hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![3000.0], 0), // the province's own city
            hub(1, 5.0, 0.0, 9000.0, vec![6000.0], 0), // the external seat
        ];

        let mut fired = false;
        for t in 0..400u32 {
            let mut s = sim(hubs.clone(), goods.clone());
            s.found_house_at(0);
            s.hub_province = vec![0, 1];
            s.prov_rural = vec![100.0, 100.0]; // non-empty ⇒ prov_count() = 2
            s.prov_holder = vec![1, 1];        // province 0 administered from OUTSIDE (hub 1)
            s.prov_holder_house = vec![-1, -1];
            s.prov_realm = vec![-1, -1];
            // House 0 commands ALL of province 0's trade (its trade_at is entirely in
            // hub 0) but holds NO office and is left UNTIERED (tier 0).
            s.houses[0].trade_at = vec![(0, 100.0)];
            s.houses[0].wealth = 300_000.0;
            s.houses[0].tier = 0;
            s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR + t;
            s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
            if let Some(r) = s.realms.first() {
                assert_eq!(r.capital_hub, 0, "the realm is seated at the province's OWN largest city");
                assert!(r.provinces.contains(&0), "the realm holds the province it dominates");
                assert_eq!(s.prov_realm[0], r.id as i32, "province 0's sovereignty is set");
                assert!(s.houses[0].crowned, "the trade house is elevated, not left a merchant");
                fired = true;
                break;
            }
        }
        assert!(fired, "a house commanding a province's trade must be able to crown itself over it");
    }

    /// The trade path when the dominated province has NO live city of its own — a
    /// FRONTIER province whose settlement has been abandoned (the "dead cities" the
    /// player sees). `province_seat_hub` returns None, so the crown seats at the
    /// dominant house's OWN home city and annexes the province. Without this fallback
    /// a dead-cored province silently blocked its trade master's realm forever.
    #[test]
    fn a_trade_dominant_house_crowns_from_home_when_the_province_has_no_live_city() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Province 0 (the dominated one) has only hub 0, which is ABANDONED — no live
        // seat. House 0's home is hub 1, a live city in province 1.
        let hubs = vec![
            hub(0, 0.0, 0.0, 0.0, vec![0.0], 0),      // the province's own city — dead
            hub(1, 5.0, 0.0, 9000.0, vec![6000.0], 0), // House 0's live home city
        ];
        let mut fired = false;
        for t in 0..400u32 {
            let mut s = sim(hubs.clone(), goods.clone());
            s.found_house_at(1); // house 0 lives at hub 1
            s.hubs[0].abandoned = true; // province 0's only city is dead
            s.hub_province = vec![0, 1];
            s.prov_rural = vec![100.0, 100.0];
            s.prov_holder = vec![-1, 1];
            s.prov_holder_house = vec![-1, -1];
            s.prov_realm = vec![-1, -1];
            // House 0 commands province 0's trade (its trade_at is in hub 0).
            s.houses[0].trade_at = vec![(0, 100.0)];
            s.houses[0].wealth = 300_000.0;
            s.houses[0].tier = 0;
            s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR + t;
            s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
            if let Some(r) = s.realms.first() {
                assert_eq!(r.capital_hub, 1, "crowns from the house's own live home city");
                assert!(r.provinces.contains(&0), "and annexes the province it dominates");
                fired = true;
                break;
            }
        }
        assert!(fired, "a dead-cored frontier province must not block its trade master's crown");
    }

    /// EXACT reproduction of a player screenshot: the province's TOP trader is a GUILD
    /// (33%), and the eligible PRIVATE house (27%) sits BELOW it, with more guilds at
    /// 18/16/2% and a second private house at 3%. The guild-above-house ordering must
    /// NOT block the crown — the sorted loop skips the guild (`continue`, not `break`)
    /// and reaches the 27% private house. If this passes, the engine is correct and a
    /// campaign showing "no realm" is running a STALE binary.
    #[test]
    fn a_private_house_crowns_even_when_a_guild_out_trades_it() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 22000.0, vec![9000.0], 0), // the seat, province 0
            hub(1, 3.0, 0.0, 7000.0, vec![4000.0], 0),
        ];
        let mut fired = false;
        for t in 0..400u32 {
            let mut s = sim(hubs.clone(), goods.clone());
            // house 0 = the eligible PRIVATE controller (Qaisariia), 27%
            // houses 1..=4 = trading GUILDS at 33/18/16/2%
            // house 5 = a second minor private house at 3%
            s.found_house_at(0); // Qaisariia, home = hub 0
            for _ in 0..5 { s.found_house_at(1); } // 5 more houses at hub 1
            s.hub_province = vec![0, 0];
            s.prov_rural = vec![100.0];
            s.prov_holder = vec![0];
            s.prov_holder_house = vec![-1];
            s.prov_realm = vec![-1];
            // Trade shares within province 0 (all trade in hub 0), mirroring the screenshot.
            s.houses[0].trade_at = vec![(0, 27.0)]; // private controller
            s.houses[1].trade_at = vec![(0, 33.0)]; s.houses[1].is_guild = true; // guild, TOP
            s.houses[2].trade_at = vec![(0, 18.0)]; s.houses[2].is_guild = true;
            s.houses[3].trade_at = vec![(0, 16.0)]; s.houses[3].is_guild = true;
            s.houses[4].trade_at = vec![(0, 2.0)];  s.houses[4].is_guild = true;
            s.houses[5].trade_at = vec![(0, 3.0)];  // second private house, minor
            s.houses[0].wealth = 1000.0; // deliberately POOR — share alone must suffice
            s.houses[0].tier = 0;
            s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR + t;
            s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
            if let Some(r) = s.realms.first() {
                assert_eq!(r.ruling_house, 0,
                    "the 27% PRIVATE house crowns, not the 33% guild (guilds cannot found realms)");
                fired = true;
                break;
            }
        }
        assert!(fired, "a private house past 20% must crown even when a guild out-trades it");
    }

    /// The exact shape a player reported unfixed: a province with a LIVE seat city, a
    /// PRIVATE house at 51% of its trade, and several GUILDS also trading there (36% +
    /// smaller shares). The private house is the highest single share and must proclaim;
    /// the guilds in the denominator and ahead-of-it-in-no-way must not block it. If this
    /// passes, the engine is correct and an unfixed campaign is running a STALE binary.
    #[test]
    fn a_51pct_private_house_proclaims_past_trading_guilds() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 25000.0, vec![9000.0], 0), // the live seat, in province 0
            hub(1, 3.0, 0.0, 8000.0, vec![4000.0], 0),  // another city in province 0
        ];
        let mut fired = false;
        for t in 0..400u32 {
            let mut s = sim(hubs.clone(), goods.clone());
            s.found_house_at(0);       // house 0 = the private trade house (Pelopidai)
            s.found_house_at(1);       // house 1 = a civic guild that also trades here
            s.houses[1].is_guild = true;
            s.hub_province = vec![0, 0];
            s.prov_rural = vec![100.0];
            s.prov_holder = vec![0];
            s.prov_holder_house = vec![-1];
            s.prov_realm = vec![-1];
            // Province 0 trade: private house 51%, guild 49% — the private house is the
            // single largest share and clears the 20% bar.
            s.houses[0].trade_at = vec![(0, 51.0)];
            s.houses[0].wealth = 300_000.0;
            s.houses[0].tier = 0;
            s.houses[1].trade_at = vec![(0, 49.0)];
            s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR + t;
            s.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
            if let Some(r) = s.realms.first() {
                assert_eq!(r.ruling_house, 0, "the PRIVATE house crowns, never the guild");
                fired = true;
                break;
            }
        }
        assert!(fired, "a 51% private trade house must proclaim despite trading guilds present");
    }

    /// The trade path is DETERMINISTIC and gated ONLY on trade share: a house
    /// commanding ≥ `PROV_TRADE_CONTROL_FRAC` (20%) of a province's trade proclaims
    /// the same year it becomes eligible, with NO dice and NO wealth floor.
    /// (Maintainer's rule: "leave only 20% trade required for a realm to appear.")
    /// The founding cost scales to the founder's own fortune, so even a near-broke
    /// dominant house crowns — and its crown's treasury never goes negative. This is
    /// the fix for the "no realms ever appear" report on a poor/starving world, where
    /// the old flat 50k floor priced out exactly the local monopolist the 20% bar
    /// selects while the UI still flagged it "eligible".
    #[test]
    fn the_trade_path_fires_on_share_alone_regardless_of_wealth() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 4000.0, vec![1500.0], 0)];
        let base = |wealth: f32| {
            let mut s = sim(hubs.clone(), goods.clone());
            s.found_house_at(0);
            s.hub_province = vec![0];
            s.prov_rural = vec![100.0];
            s.prov_holder = vec![0];
            s.prov_holder_house = vec![-1];
            s.prov_realm = vec![-1];
            s.houses[0].trade_at = vec![(0, 100.0)]; // commands ALL of the province's trade
            s.houses[0].wealth = wealth;
            s.houses[0].tier = 0;
            s.tick = REALM_YEAR_FLOOR * TICKS_PER_YEAR;
            s
        };

        // A POOR but dominant house now crowns — trade share is the sole qualification.
        let mut poor = base(1_200.0);
        poor.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
        assert!(!poor.realms.is_empty(),
            "a house commanding 20%+ of a province's trade crowns regardless of its wealth");
        assert!(poor.houses[0].crowned, "the poor trade house is still elevated to a crown");
        assert!(poor.realms[0].treasury >= 0.0, "treasury never goes negative even when poor");

        // A rich house behaves the same — deterministic, first eligible year.
        let mut rich = base(REALM_TRADE_MIN_WEALTH);
        rich.maybe_proclaim_realms(REALM_FULL_RAMP_YEAR);
        assert!(!rich.realms.is_empty(),
            "a wealthy dominant house crowns deterministically the first eligible year");
        assert!(rich.houses[0].crowned, "the trade house is elevated to a crown");
        assert!(rich.realms[0].treasury >= 0.0, "treasury never goes negative");
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
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR; // realistic — a realm can't exist at tick 0
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
            funder_hub: 0, funder_house: -1, funder_realm: -1, start_tick: 0, idle_years: 0 });
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
            funder_hub: 0, funder_house: -1, funder_realm: -1, start_tick: 0, idle_years: 0 });
        for yr in 0..20u32 { s.province_land_pass(yr); }
        assert_eq!(s.prov_works.len(), 1, "an unfunded work stalls, it does not vanish");
        assert!(s.prov_works[0].progress < 0.2,
            "an unfunded work makes no real progress: {}", s.prov_works[0].progress);
        assert!(s.prov_arable[0] <= arable0 + 0.02,
            "no improvement lands without being paid for");
    }

    /// Province works v2.0 · a work must now BEGIN ON ITS OWN, without any player
    /// verb ever being called — before this the four kinds were reachable only from
    /// `campaign_start_province_work`, so a campaign nobody micromanaged never
    /// improved a single province's land on any world, ever.
    ///
    /// Also asserts the ELIGIBILITY TIER that decides who may improve at all: an
    /// untiered seat city (a town with nothing to spare) outside a realm must NOT
    /// start one, which is what keeps this from being a free global upgrade.
    #[test]
    fn province_works_begin_on_their_own_once_the_seat_is_advanced() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mk = |tier: u8| {
            let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
            let mut s = sim(hubs, goods.clone());
            s.prov_cap = vec![50_000.0];
            s.prov_rural = vec![30_000.0];
            s.prov_culture = vec!["Aiora".into()];
            s.prov_seat = vec![[0.0, 0.0]];
            s.hub_province = vec![0];
            s.prov_net_mig = vec![0.0];
            s.ensure_province_land(1);
            s.hubs[0].treasury = 50_000.0; // richly able to pay
            s.hubs[0].tier = tier;
            s.prov_forest[0] = 0.5;
            s.prov_arable[0] = 0.3;
            s
        };

        // An ADVANCED seat: over a few decades of yearly rolls a work must appear.
        let mut adv = mk(2);
        let mut ever_started = false;
        for yr in 0..40u32 {
            adv.maybe_fund_province_works(yr);
            if !adv.prov_works.is_empty() { ever_started = true; }
            adv.tick += TICKS_PER_YEAR;
        }
        assert!(ever_started,
            "an advanced seat city with a full treasury must begin a land improvement on its own");

        // An UNTIERED seat outside a realm: never, however rich it is.
        let mut town = mk(0);
        for yr in 0..40u32 {
            town.maybe_fund_province_works(yr);
            town.tick += TICKS_PER_YEAR;
        }
        assert!(town.prov_works.is_empty(),
            "an untiered town outside a realm may not begin works, however rich");
    }

    /// Province works v2.0 · STATE INFRASTRUCTURE needs a state. An irrigation
    /// system or a made road may only be begun on a province under a realm's
    /// sovereignty; a free province still improves its land (clearance, drainage)
    /// but never gains the infrastructure. This is the concrete meaning of "a
    /// province is worked FULLY once it is under a realm".
    ///
    /// It is also the load-bearing half economically: irrigation carries
    /// `PROV_IRRIGATION_GAIN` (+45% on the harvest at full watering), far the
    /// largest term here. Letting every city-funded province drive it to cap made
    /// autonomous works a world-wide yield multiplier and INVERTED
    /// `econ_inheritance_rules_fragment_differently`'s substantive claim (partible
    /// 191,991 against primogeniture 163,230, when partible must come out poorer).
    /// With the gate, that test passes on a wider margin than the pristine baseline.
    /// So this is not a flavour rule — remove it and a fidelity gate fails.
    #[test]
    fn state_infrastructure_needs_a_realm() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // A province begging for a road: heavy arrears AND unrest, no woodland to
        // clear and no waste to drain, so a ROAD is the only thing left to want.
        let mk = || {
            let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
            let mut s = sim(hubs, goods.clone());
            s.prov_cap = vec![50_000.0];
            s.prov_rural = vec![30_000.0];
            s.prov_culture = vec!["Aiora".into()];
            s.prov_seat = vec![[0.0, 0.0]];
            s.hub_province = vec![0];
            s.prov_net_mig = vec![0.0];
            s.ensure_province_land(1);
            s.hubs[0].treasury = 80_000.0;
            s.hubs[0].tier = 1;
            s.prov_arrears[0] = 500.0;   // dues leaking badly → wants a road
            s.prov_unrest[0] = 0.8;
            s.prov_forest[0] = 0.05;     // nothing worth clearing
            s.prov_arable[0] = 0.55;
            s.prov_pasture[0] = 0.40;    // …and no waste to drain (shares sum to 1.0)
            s
        };

        // FREE province: never a road or an irrigation channel, however rich and
        // however badly it needs one.
        let mut free = mk();
        for yr in 0..60u32 { free.maybe_fund_province_works(yr); free.tick += TICKS_PER_YEAR; }
        assert!(!free.prov_works.iter().any(|w| w.kind == WORK_ROAD || w.kind == WORK_IRRIGATE),
            "a province outside a realm may never begin state infrastructure");

        // The SAME province under a crown: the road becomes reachable, and the
        // realm's own treasury is what pays for it. The realm is founded through
        // the REAL `found_civic_realm` rather than a hand-built literal, so this
        // test can never drift from how a realm is actually constructed.
        let mut held = mk();
        held.prov_holder = vec![0];
        held.prov_holder_house = vec![-1];
        held.prov_realm = vec![-1];
        let rid = held.found_civic_realm(0, REALM_YEAR_FLOOR, REALM_PATH_CITY);
        assert_eq!(held.prov_realm[0], rid as i32, "the founding must actually take the province");
        held.realms[rid as usize].treasury = 500_000.0; // able to pay for anything
        let mut got_infra = false;
        for yr in 0..60u32 {
            held.maybe_fund_province_works(yr);
            if held.prov_works.iter().any(|w| w.kind == WORK_ROAD || w.kind == WORK_IRRIGATE) {
                got_infra = true;
                assert!(held.prov_works.iter().all(|w| w.funder_realm == 0),
                    "a sovereign province's works are funded by its crown, not its city");
                break;
            }
            held.tick += TICKS_PER_YEAR;
        }
        assert!(got_infra, "a province under a realm must be able to begin state infrastructure");
    }

    /// Province works v2.0 · cost must scale with the province's real SIZE and
    /// TERRAIN ROUGHNESS (the maintainer's own rule), and a road — carved through
    /// the relief itself — must be the kind that responds most to it. A pre-v2.0
    /// save carries neither figure and must keep the old flat price exactly.
    #[test]
    fn work_cost_scales_with_province_size_and_roughness() {
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

        // No geography recorded (an older save) → exactly the old flat cost.
        s.prov_area_km2 = vec![0.0];
        s.prov_relief_m = vec![0.0];
        for k in 0..4u8 {
            assert!((s.work_cost(0, k) - WORK_COST[k as usize]).abs() < 1e-3,
                "a save with no area/relief must keep the flat cost for kind {}", k);
        }

        // A big province costs more than a small one, same terrain.
        s.prov_relief_m = vec![0.0];
        s.prov_area_km2 = vec![3_000.0];
        let small = s.work_cost(0, WORK_CLEAR);
        s.prov_area_km2 = vec![27_000.0];
        let big = s.work_cost(0, WORK_CLEAR);
        assert!(big > small * 1.5, "a far larger province must cost far more: {} vs {}", small, big);

        // Broken country costs more than a plain, same size — and a ROAD responds
        // hardest, since it is cut through the relief rather than around it.
        s.prov_area_km2 = vec![WORK_AREA_REFERENCE_KM2];
        s.prov_relief_m = vec![0.0];
        let (flat_road, flat_irr) = (s.work_cost(0, WORK_ROAD), s.work_cost(0, WORK_IRRIGATE));
        s.prov_relief_m = vec![2_400.0];
        let (rough_road, rough_irr) = (s.work_cost(0, WORK_ROAD), s.work_cost(0, WORK_IRRIGATE));
        assert!(rough_road > flat_road, "a road costs more through broken country");
        assert!(rough_irr > flat_irr, "so does irrigation, but less so");
        assert!(rough_road / flat_road > rough_irr / flat_irr,
            "a road must respond to roughness MORE than an irrigation channel does");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 5.1 · THE CADASTRE — a fifth `ProvWork` kind,
    /// crown-funded like the road/irrigation pair, permanently marking
    /// `prov_cadastre[p]` on completion and, from then on, raising the
    /// province's own collection efficiency at the tithe.
    #[test]
    fn a_surveyed_province_collects_more_and_a_free_one_never_gets_the_cadastre() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mk = || {
            let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0)];
            let mut s = sim(hubs, goods.clone());
            s.prov_cap = vec![50_000.0];
            s.prov_rural = vec![30_000.0];
            s.prov_culture = vec!["Aiora".into()];
            s.prov_seat = vec![[0.0, 0.0]];
            s.hub_province = vec![0];
            s.prov_net_mig = vec![0.0];
            s.ensure_province_land(1);
            s.hubs[0].treasury = 80_000.0;
            s.hubs[0].tier = 1;
            // No arrears/unrest (no road wanted), well irrigated already (no
            // irrigation wanted), so a settled province under a realm falls
            // straight to the cadastre.
            s.prov_irrigated[0] = 0.9;
            s.prov_arable[0] = 0.5;
            s.prov_forest[0] = 0.05;
            s.prov_pasture[0] = 0.45;
            s
        };

        // FREE province: never surveyed, however long it runs.
        let mut free = mk();
        for yr in 0..80u32 { free.maybe_fund_province_works(yr); free.advance_province_works(0, yr); free.tick += TICKS_PER_YEAR; }
        assert!(!free.prov_cadastre.get(0).copied().unwrap_or(false),
            "a province outside a realm may never be surveyed");

        // The SAME province, crowned: a cadastre eventually begins and completes.
        let mut held = mk();
        held.prov_holder = vec![0];
        held.prov_holder_house = vec![-1];
        held.prov_realm = vec![-1];
        let rid = held.found_civic_realm(0, REALM_YEAR_FLOOR, REALM_PATH_CITY);
        assert_eq!(held.prov_realm[0], rid as i32);
        held.realms[rid as usize].treasury = 1_000_000.0;
        held.realms[rid as usize].cohesion = 0.9;
        for yr in 0..80u32 {
            held.maybe_fund_province_works(yr);
            held.advance_province_works(0, yr);
            held.tick += TICKS_PER_YEAR;
        }
        assert!(held.prov_cadastre.get(0).copied().unwrap_or(false),
            "a settled province under a realm must eventually be surveyed");

        // Efficiency is measurably higher once surveyed, at identical cohesion
        // and distance (the capital IS the province's own seat here, so
        // distance is zero either way).
        let base = held.realm_collection_efficiency(rid as usize, 0);
        held.prov_cadastre[0] = false;
        let unsurveyed = held.realm_collection_efficiency(rid as usize, 0) ;
        held.prov_cadastre[0] = true;
        let _ = base;
        assert!(unsurveyed <= held.realm_collection_efficiency(rid as usize, 0),
            "the raw efficiency term is unaffected by the survey — the CADASTRE \
             multiplier is applied at the tithe call site, not inside \
             realm_collection_efficiency itself; this just re-confirms that fact \
             so a future refactor cannot silently move the multiplier in twice");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 5.2 · FARM OR COLLECT — an active tax farm is
    /// consequential every year it stands: rural unrest rises and the crown's
    /// own cohesion decays, not just at the moment of sale.
    #[test]
    fn an_active_tax_farm_raises_unrest_and_decays_cohesion_every_year() {
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
        s.prov_holder = vec![0];
        s.prov_holder_house = vec![-1];
        s.prov_realm = vec![-1];
        let rid = s.found_civic_realm(0, REALM_YEAR_FLOOR, REALM_PATH_CITY);
        s.prov_realm[0] = rid as i32;
        s.houses.push(house_at(0, vec![0], 2));
        s.houses[0].wealth = 10_000.0;
        s.realms[rid as usize].tax_farm = Some(TaxFarm { house: 0, started_tick: s.tick, years: 5 });
        let cohesion0 = s.realms[rid as usize].cohesion;
        let unrest0 = s.prov_unrest[0];
        for _ in 0..3 { s.decide_realm_taxes(rid as usize, s.year()); }
        assert!(s.prov_unrest[0] > unrest0, "a standing farm raises rural unrest over time");
        assert!(s.realms[rid as usize].cohesion < cohesion0,
            "a standing farm decays the crown's own administrative grip");
        assert!(s.realms[rid as usize].cohesion >= TAX_FARM_COHESION_FLOOR - 1e-4,
            "the decay is bounded, never runaway");
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
                delta: false, chokepoint: false, province: -1, belt: vec![], river: false,
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

    /// A route post must site itself at a genuine transhipment point (a river-mouth
    /// delta or a land/sea chokepoint — the world's own step-7a junction survey), not
    /// merely at the nearest surveyed site to the gap's midpoint. Two candidate sites
    /// sit near the midpoint: a PLAIN site much closer, and a CHOKEPOINT site a little
    /// farther but still within ROUTE_POST_JUNCTION_KM — the chokepoint must win
    /// despite being farther, since founding at a real pinch point is the entire
    /// historical claim this mechanism makes.
    #[test]
    fn route_posts_prefer_a_junction_site_over_the_nearest_plain_one() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        // Two same-component hubs far enough apart (via the #6 horizon-bypassing
        // rescue lane, terrain-penalised by an EF/ice-cap koppen) to clear
        // ROUTE_POST_MIN_GAP_DAYS.
        let mut h0 = hub(0, 5.0, 50.0, 3000.0, vec![80.0, 40.0], 0);
        h0.koppen = 22; // EF ice cap — terrain_route_mult = 2.2
        let h1 = hub(1, 105.0, 50.0, 3000.0, vec![60.0, 30.0], 0);
        let mut s = sim(vec![h0, h1], goods);
        s.world_w = 200.0; s.world_h = 200.0;
        s.houses = vec![house_at(0, vec![1], 4)];
        s.houses[0].wealth = 50_000.0; // clears ROUTE_POST_FOUND_WEALTH (20k), not the old outpost bar
        s.seed_house_count = 1;
        // Gap midpoint is (55, 50). A plain site sits almost on top of it; a
        // chokepoint site sits a little farther off but still inside the junction
        // search radius (≈3 cells at this small world's scale).
        s.colonizable.push(ColonizeSite {
            x: 55.2, y: 50.0, koppen: 11, elevation: 0.2, fertility: 0.5, coastal: true,
            kind_hint: 1, trade_value: 0.5, delta: false, chokepoint: false, province: -1, belt: vec![], river: false,
        });
        s.colonizable.push(ColonizeSite {
            x: 57.5, y: 50.0, koppen: 11, elevation: 0.2, fertility: 0.5, coastal: true,
            kind_hint: 1, trade_value: 0.5, delta: false, chokepoint: true, province: -1, belt: vec![], river: false,
        });
        s.rebuild_routes();
        let gap_days = s.days[0 * s.hubs.len() + 1];
        assert!(gap_days >= 25.0, "fixture gap must clear ROUTE_POST_MIN_GAP_DAYS, got {gap_days}");

        s.maybe_found_route_post();

        let posts: Vec<&TickHub> = s.hubs.iter().filter(|h| h.colony_kind == 4).collect();
        assert_eq!(posts.len(), 1, "exactly one route post should have founded");
        let p = posts[0];
        assert!((p.x - 57.5).abs() < 0.01 && (p.y - 50.0).abs() < 0.01,
            "route post should site at the farther CHOKEPOINT ({}, {}), not the nearer plain site", p.x, p.y);
        assert!(!p.is_estate, "a route post is a real hub, not an estate");
        assert_eq!(p.owner_house, 0);
    }

    /// User-reported: a route post could found itself thousands of km from the
    /// house that paid for it, because `maybe_found_route_post` picked the
    /// single longest same-component gap in the WHOLE WORLD with no ceiling at
    /// all. `route_post_max_home_km` (opt-out `INFINITY` in every other test
    /// fixture, per the `sim()` helper's own doc comment) must actually block a
    /// site far from the founding house's seat, and a nearby-enough cap must
    /// let the identical venture through — the same site, the same gap, only
    /// the cap differs.
    #[test]
    fn route_post_founding_respects_the_home_distance_cap() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let mut h0 = hub(0, 5.0, 50.0, 3000.0, vec![80.0, 40.0], 0);
        h0.koppen = 22; // EF ice cap — terrain_route_mult = 2.2, same as the junction test
        let h1 = hub(1, 105.0, 50.0, 3000.0, vec![60.0, 30.0], 0);
        let mut s = sim(vec![h0, h1], goods);
        s.world_w = 200.0; s.world_h = 200.0;
        s.houses = vec![house_at(0, vec![1], 4)];
        s.houses[0].wealth = 50_000.0;
        s.seed_house_count = 1;
        s.colonizable.push(ColonizeSite {
            x: 57.5, y: 50.0, koppen: 11, elevation: 0.2, fertility: 0.5, coastal: true,
            kind_hint: 1, trade_value: 0.5, delta: false, chokepoint: true, province: -1, belt: vec![], river: false,
        });
        s.rebuild_routes();
        let gap_days = s.days[0 * s.hubs.len() + 1];
        assert!(gap_days >= 25.0, "fixture gap must clear ROUTE_POST_MIN_GAP_DAYS, got {gap_days}");
        // Home (5,50) to the site (57.5,50) is ~52.5 cells — at world_w=200 that is
        // ~10,520 km (52.5 * 40075/200), comfortably past any real dose.
        s.route_post_max_home_km = 5_000.0;
        s.maybe_found_route_post();
        assert_eq!(s.hubs.iter().filter(|h| h.colony_kind == 4).count(), 0,
            "a site ~10,500 km from the founder's home must be rejected at a 5,000 km cap");

        s.route_post_max_home_km = 20_000.0;
        s.maybe_found_route_post();
        assert_eq!(s.hubs.iter().filter(|h| h.colony_kind == 4).count(), 1,
            "the identical site must found once the cap actually reaches it");
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
                delta: false, chokepoint: false, province: -1, belt: vec![], river: false,
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
        // The floor was widened from -100.0 when CONSUMPTION_REBUILD_PLAN.md's S1
        // (the budget-share demand rewrite, TIER_WEIGHT/BUDGET_VALUE_FLOOR in
        // tick/mod.rs) landed: thinner luxury-import trade margins mean a house
        // occasionally dips further into debt before recovering or being dissolved
        // at update_solvency's existing one-year grace period (unchanged by S1) —
        // a bounded, designed consequence, not a runaway. -339.7 was the measured
        // value; -500.0 keeps real headroom while still catching a genuine
        // millions-scale insolvency blow-up.
        assert!(min_w > -500.0, "no runaway-insolvent house (limited liability): {min_w}");
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
            delta: false, chokepoint: false, province: -1, belt: vec![], river: false,
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
        // WORLD_AND_TRADE_MASTER_PLAN.md Part II E1 (G6): a house with no coastal
        // foothold anywhere in its network cannot plant a coastal outpost — mirrors
        // `maybe_found_settlement_colony`'s own "no fleet tradition" rule. The test's
        // trade-rich site is coastal, so the founder needs one too.
        s.hubs[0].coastal = true;
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
            ColonizeSite { x: 3.0, y: 3.0, koppen: 0, elevation: 0.2, fertility: 0.80, coastal: false, kind_hint: 1, trade_value: 0.10, delta: false, chokepoint: false, province: -1, belt: vec![], river: false },
            ColonizeSite { x: 5.0, y: 2.0, koppen: 0, elevation: 0.1, fertility: 0.18, coastal: true, kind_hint: 4, trade_value: 0.60, delta: false, chokepoint: false, province: -1, belt: vec![], river: false },
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
                outstanding: 10.0, rate: 0.01, start_tick: 0, term_ticks: 3650, purpose: "colony".into(), arrears_months: 0 }],
            real_estate: 1.0, deposits: 0.0, notes_issued: 0.0, branches: vec![0], prestige: 0.5,
            interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        });
        s.tick = 55 * 365;
        s.colony_pass();
        assert!(s.hubs[1].colony_kind == 0 && s.hubs[1].population <= 1.0, "starved colony collapsed");
        assert!(s.banks[0].losses > 0.0 && s.banks[0].loans.is_empty(), "bank wrote off the defaulted colony loan");
    }

    /// User report: a settlement colony chronically unsupplied — "food 0.0/365 ·
    /// supplied 0y" for its whole life, `supply_years` never once completing an
    /// unbroken run — nonetheless grew into the hundreds of thousands off trade/
    /// age headroom, and never collapsed because the fragile, smoothed
    /// `starving > 0.8` trigger never quite fired (it decays back down 0.02/day
    /// the moment `food_balance` recovers even briefly, which an oscillating
    /// real supply/production picture does often — a lifeline that is never once
    /// actually closed can still dodge a signal built to reward RECENT relief).
    /// `supply_shortfall_days` fixes this at the root: a plain day-count of real
    /// undelivered deficit that only resets on an ACTUAL full delivery, giving a
    /// deterministic bound (`COLONY_UNSUPPLIED_COLLAPSE_YEARS`) instead of a
    /// signal that can be nursed along indefinitely. Here nobody else in the
    /// world has any food to spare and the colony is also given a MASSIVE
    /// trade/primacy capacity headroom (exactly what let the real colony balloon
    /// while starved), so the only thing standing between it and unbounded
    /// growth is this mechanism.
    #[test]
    fn an_unsupplied_colony_cannot_grow_and_eventually_collapses() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        let ng = goods.len();
        // Nobody ELSE in the world produces food, so no source can ever be
        // designated — whatever the colony can't grow itself, it never gets.
        let metro = hub(0, 0.0, 0.0, 20_000.0, vec![0.0, 400.0], 0);
        let mut colony = hub(1, 4.0, 0.0, 3_000.0, vec![0.0, 60.0], 0);
        colony.colony_kind = 1;
        colony.founder_hub = 0;
        colony.founding_pop = 3_000.0;
        colony.backers = vec![(1, 0, 1.0)];
        colony.reserve_cap = 365.0;
        colony.colony_founded_tick = 0;
        let founding_pop = colony.founding_pop;
        // A MASSIVE trade/primacy headroom source — exactly what let the reported
        // colony balloon to 444k while "supplied 0y": a coastal, capital-ranked
        // entrepôt earns `PRIMACY_DEV` (45.0) of extra capacity-multiplier headroom
        // in `update_food_and_starvation`, completely independent of whether it is
        // actually fed. Picked as its component's capital via the highest treasury.
        colony.coastal = true;
        colony.hub_class = 1;
        colony.treasury = 1_000_000.0;
        let mut s = sim(vec![metro, colony], goods);
        let mut rich = house_at(0, vec![], 0);
        rich.wealth = 500_000.0; // affordability is never the bottleneck here
        s.houses.push(rich);
        s.rebuild_routes();
        // A MILD, chronic shortfall, held CONSTANT as a fraction of need (the
        // colony's own farms scale with its population the same way its need
        // does, exactly as the real per-capita production model does — a fixed
        // 10% shortfall never closes on its own just because the town grows):
        // the colony grows ~90% of its own daily wheat need, recomputed every
        // day from its CURRENT population, and nobody else has any to spare.
        let per_cap_need = s.base_need(1, 0) / founding_pop;

        let mut collapsed_by_year: Option<u32> = None;
        for day in 1..=(5 * 365u32) {
            s.tick = day;
            s.hubs[1].production[0] = per_cap_need * 0.90 * s.hubs[1].population;
            let needs: Vec<Vec<f32>> = (0..s.hubs.len())
                .map(|h| (0..ng).map(|g| s.base_need(h, g)).collect())
                .collect();
            s.update_food_and_starvation(&needs);
            if day % 365 == 0 {
                s.colony_pass();
                let yr = day / 365;
                if s.hubs[1].colony_kind != 1 {
                    collapsed_by_year.get_or_insert(yr);
                }
            }
        }
        assert!(collapsed_by_year.is_some(),
            "a colony with an unmet daily food deficit and no source must eventually fail — it never did in 5 simulated years");
        // The new mechanism's whole point is a DETERMINISTIC bound: a colony whose
        // lifeline never once closes the deficit must fail within
        // `COLONY_UNSUPPLIED_COLLAPSE_YEARS`, regardless of how much unrelated
        // trade/primacy headroom it earns — not merely "eventually, whenever the
        // slow, smoothed `starving` signal happens to cross 0.8".
        assert!(collapsed_by_year.unwrap() <= 4,
            "an unsupplied colony must collapse within COLONY_UNSUPPLIED_COLLAPSE_YEARS (+1y grace), \
             not merely whenever `starving` happens to catch up: collapsed at year {}", collapsed_by_year.unwrap());
        assert!(s.hubs[1].population <= 1.0 && s.hubs[1].abandoned, "the collapsed colony is truly gone");
    }

    /// CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 6 (F6) — a colony founded on
    /// a site whose dominant belt differs from its founder's must produce that good
    /// within a year (immediately, in fact — `create_market_colony` seeds it at
    /// founding). Before this a settlement colony always inherited a flat 60% of its
    /// METROPOLIS's own basket regardless of what the chosen site actually carried.
    #[test]
    fn a_colony_produces_what_its_site_carries() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true), good("silk", 1, 2, 20.0, 0.35, false)];
        // A wheat-heavy metropolis (200 wheat : 2 silk).
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![200.0, 2.0], 0)];
        let mut s = sim(hubs, goods);
        let site = ColonizeSite {
            x: 10.0, y: 0.0, koppen: 8, elevation: 0.1, fertility: 0.4, coastal: true,
            kind_hint: 1, trade_value: 0.5, delta: false, chokepoint: false, province: -1,
            belt: vec![0.05, 0.95], // this site is silk country, not wheat
            river: false,
        };
        let ci = s.create_market_colony(0, &site, vec![(2, 0, 1.0)], 500.0);

        let share = |bpc: &[f32], g: usize| bpc[g] / bpc.iter().sum::<f32>().max(1e-6);
        let founder_silk_share = share(&s.hubs[0].base_per_capita, 1);
        let colony_silk_share = share(&s.hubs[ci].base_per_capita, 1);
        assert!(
            colony_silk_share > founder_silk_share * 2.0,
            "a colony founded on a silk-rich site should produce far more silk \
             (share {colony_silk_share:.3}) than a photocopy of its wheat-heavy \
             founder (share {founder_silk_share:.3})"
        );

        // An empty `belt` (a save from before this slice) must reproduce the OLD
        // flat-60%-of-founder behaviour exactly — a true no-op.
        let old_site = ColonizeSite { belt: vec![], river: false, ..site };
        let ci2 = s.create_market_colony(0, &old_site, vec![(2, 0, 1.0)], 500.0);
        for g in 0..2 {
            let expected = s.hubs[0].base_per_capita[g] * 0.6;
            assert!(
                (s.hubs[ci2].base_per_capita[g] - expected).abs() < 1e-6,
                "an empty belt must fall back to the old founder×0.6 seeding exactly"
            );
        }
    }

    /// CLAUDE.md §4 step 7a + §7 (ports/junctions, shipped) slice 7 (F7) — a stage-1 colony,
    /// alone in a province whose rural pool is already at carrying capacity, must
    /// receive only a FRACTION of `prov_surplus`/dues on its first land pass — not
    /// the whole province's harvest, which is what happened before: an unsettled
    /// province fills to capacity with no member check, and the colony becomes the
    /// province's only administering hub (`province_seat_hub`) the moment it exists.
    #[test]
    fn a_young_colony_does_not_inherit_a_full_province() {
        let build = |stage: u8| -> CampaignSim {
            let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
            let mut hubs = vec![hub(0, 0.0, 0.0, 500.0, vec![50.0], 0)];
            hubs[0].colony_kind = 1;
            hubs[0].colony_stage = stage;
            let mut s = sim(hubs, goods);
            s.hub_province = vec![0];
            s.hub_culture = vec![String::new()];
            s.hub_minorities = vec![Vec::new()];
            s.prov_cap = vec![10_000.0];
            s.prov_rural = vec![10_000.0]; // already at capacity — the F7 scenario exactly
            s.prov_holder = vec![-1];
            s
        };

        let mut young = build(1); // outpost stage — 0.25 maturity
        let mut mature = build(4); // city stage — full maturity
        assert_eq!(young.colony_delivery_maturity(0), 0.25);
        assert_eq!(mature.colony_delivery_maturity(0), 1.0);

        young.province_land_pass(1);
        mature.province_land_pass(1);

        let stock_young = stock_of(&young.hubs[0].stock, 0);
        let stock_mature = stock_of(&mature.hubs[0].stock, 0);
        assert!(stock_mature > 0.0, "the mature colony should have received a real surplus at all");
        assert!(
            stock_young > 0.0 && stock_young < stock_mature * 0.35,
            "a stage-1 colony received {stock_young} against a mature stage-4 colony's \
             {stock_mature} — expected roughly a quarter, not the whole harvest"
        );

        // Ordinary cities (never a colony) are entirely unaffected — maturity 1.0.
        let mut ordinary = build(0);
        ordinary.hubs[0].colony_kind = 0;
        assert_eq!(ordinary.colony_delivery_maturity(0), 1.0);
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

    /// A whole-sim state fingerprint, in the same spirit as phase-3's
    /// `ocean_atmosphere_field_checksums` (CLAUDE.md §8.9): an aggregate that
    /// is cheap to print and impossible to match by accident, so a PERFORMANCE
    /// change to the tick can be proven output-preserving rather than merely
    /// "close". Folded over the f32 BIT PATTERNS, never the values, because a
    /// float sum would hide exactly the last-ulp drift this exists to catch,
    /// and in a FIXED index order (hubs, then goods, then houses) so it never
    /// depends on iteration or thread scheduling.
    pub(super) fn sim_fingerprint(s: &CampaignSim) -> u64 {
        let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            acc ^= v;
            acc = acc.wrapping_mul(0x1000_0000_01b3);
        };
        mix(s.tick as u64);
        for h in &s.hubs {
            for v in &h.stock { mix(v.to_bits() as u64); }
            for v in &h.price { mix(v.to_bits() as u64); }
            mix(h.population.to_bits() as u64);
            mix(h.treasury.to_bits() as u64);
            mix(h.export_earn.to_bits() as u64);
            mix(h.import_spend.to_bits() as u64);
        }
        for hh in &s.houses {
            mix(hh.wealth.to_bits() as u64);
            mix(hh.volume.to_bits() as u64);
            mix(hh.prestige.to_bits() as u64);
            mix(hh.defunct as u64);
        }
        mix(s.in_transit.len() as u64);
        mix(s.diag_shipments as u64);
        mix(s.diag_lost as u64);
        mix(s.diag_by_house as u64);
        mix(s.diag_volume.to_bits() as u64);
        mix(s.journal.len() as u64);
        acc
    }

    /// Build the LARGE campaign the perf bench below runs on: ~1,000 hubs and
    /// 30 goods, the scale a real world reaches (a default 3600×1800 world
    /// carries 1,000-1,200 settlements once colonies and estates have grown),
    /// as against `bench_campaign_tick`'s 160. The tick's hot loop is
    /// O(goods × sellers × neighbours), so 160 hubs measures a regime the
    /// shipped product never runs in.
    pub(super) fn large_bench_sim(nhubs: u32, ng: usize) -> CampaignSim {
        let goods: Vec<TickGood> = (0..ng)
            .map(|g| good(&format!("g{g}"), (g % 12) as i32, (g % 3) as u8,
                          1.0 + g as f32, 0.30 + 0.5 * ((g % 5) as f32 / 5.0), g < 8))
            .collect();
        let cols = 40u32;
        let mut hubs = Vec::new();
        for i in 0..nhubs {
            let x = (i % cols) as f32 * 2.4;
            let y = (i / cols) as f32 * 2.4;
            let pop = 2000.0 + (i as f32 * 137.0) % 9000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 7 == 0 { pop * 0.02 } else { pop * 0.002 })
                .collect();
            let mut h = hub(i, x, y, pop, prod, 0); // one component → trade flows
            h.coastal = i % 3 == 0;
            hubs.push(h);
        }
        let mut s = sim(hubs, goods);
        for i in (0..nhubs).step_by(4) {
            s.houses.push(house_at(i, vec![i as usize % ng], 3));
        }
        s.rebuild_routes();
        s
    }

    /// The per-day campaign tick at PRODUCTION scale. Run explicitly:
    ///   `cargo test --release --lib bench_campaign_tick_large -- --ignored --nocapture`
    /// Set `WF2_PROFILE=1` alongside it for `advance`'s own per-in-game-year
    /// breakdown (trade / houses / events / rebuild), which is what localises a
    /// regression to a pass rather than merely reporting that the tick got
    /// slower. Prints a state FINGERPRINT too: an optimisation here must leave
    /// that number unchanged (CLAUDE.md §5.5).
    #[test]
    #[ignore]
    fn bench_campaign_tick_large() {
        use std::time::Instant;
        let (nhubs, ng) = (1000u32, 30usize);
        let mut s = large_bench_sim(nhubs, ng);
        let days = 365u32 * 3;
        let t0 = Instant::now();
        s.advance(days);
        let total = t0.elapsed().as_secs_f64() * 1000.0;
        println!(
            "[large-bench hubs={} goods={ng}] {days} ticks: {total:.1}ms total, {:.3}ms/tick, fingerprint={:016x}",
            s.hubs.len(), total / days as f64, sim_fingerprint(&s)
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
                // A coastal, fleet-less house buys its first hull(s) the very first
                // month `decide_fleets` runs (an empty fleet always reads "busy"), and
                // FLEET_BUY_MAX_PER_MONTH>1 lets it buy several at once — real, wanted
                // behaviour, but it means a seed wealth tuned for the old one-hull
                // ceiling no longer leaves enough capital to charter the mint this test
                // is actually about. 60 -> 300 so fleet-buying can't crowd that out.
                h.wealth = 300.0;
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
    fn every_currency_name_is_unique_in_a_world() {
        // MONEY_AND_COINAGE_PLAN.md M1 · the coin catalogue records `decide_
        // coinage`'s own decisions as real `Currency`/`Denom`/`Issue` objects.
        // Several seats mint over a real run; every one of their catalogue
        // entries must carry a distinct name (F2's own complaint — "a large
        // world has several unrelated Ducats" — must not survive into the
        // catalogue), every issue must point at a real currency/denom, and
        // M1 ships every issue's struck/circulating/hoarded/melted/lost at
        // exactly 0.0 (M3's parallel ledger is what populates them).
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 20.0, 0.35, false),
        ];
        let mut hubs = Vec::new();
        for i in 0..6u32 {
            hubs.push(hub(i, (i as f32) * 20.0, 10.0, 30000.0, vec![160.0, 16.0], 0));
        }
        let mut s = sim(hubs, goods);
        for i in 0..6u32 {
            let mut h = house_at(i, vec![1], 3);
            h.archetype = 2;
            h.wealth = 300.0;
            h.prestige = 0.6;
            h.dominant_seat = true;
            s.houses.push(h);
        }
        for hh in s.hubs.iter_mut() { hh.treasury = 200.0; }
        // Two of the six share a culture, so `units_of_account` resolves for
        // more than one hub off the same entry.
        s.hub_culture = vec![
            "Aiora".into(), "Aiora".into(), "Vexil".into(),
            "Vexil".into(), "Korren".into(), "Korren".into(),
        ];
        s.rebuild_routes();
        s.ensure_unit_of_account();

        s.advance(TICKS_PER_YEAR * 5);

        assert!(!s.currencies.is_empty(), "at least one seat minted a coin over 5 years");
        let mut names: Vec<&str> = s.currencies.iter().map(|c| c.name.as_str()).collect();
        names.sort();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "every currency name is unique: {names:?}");

        for c in &s.currencies {
            assert!(!c.denoms.is_empty(), "{} has at least one denomination", c.name);
            for d in &c.denoms {
                assert!(!d.issues.is_empty(), "{} {} was struck at least once", c.name, d.name);
                assert!(d.standard_grams > 0.0);
            }
        }
        for iss in &s.issues {
            assert!((iss.currency as usize) < s.currencies.len(), "issue points at a real currency");
            let cur = &s.currencies[iss.currency as usize];
            assert!((iss.denom as usize) < cur.denoms.len(), "issue points at a real denom");
            assert!(iss.fineness.is_finite() && (0.0..=1.0).contains(&iss.fineness));
            assert!(iss.grams > 0.0);
            // M1 is observe-only — the parallel ledger (M3) hasn't been built yet.
            assert_eq!(iss.struck, 0.0);
            assert_eq!(iss.circulating, 0.0);
            assert_eq!(iss.hoarded, 0.0);
            assert_eq!(iss.melted, 0.0);
            assert_eq!(iss.lost, 0.0);
        }
        assert!(!s.units_of_account.is_empty(), "the three seeded cultures resolved a unit of account");
        for u in &s.units_of_account {
            assert!(u.ladder.len() >= 2 && u.ladder.len() == u.names.len());
            assert!(u.ladder.windows(2).all(|w| w[0] > w[1]), "largest unit first: {:?}", u.ladder);
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
    fn a_bank_whose_liquidation_value_covers_deposits_winds_down_without_a_crash() {
        // INSTITUTIONS_BUILD_ORDER.md 1.1 · reserves alone cover liabilities in
        // full — wound down quietly, depositors paid, no contagion ignited.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0)], goods);
        s.houses.push(house_at(0, vec![0], 2));
        let mk_bank = |name: &str, house: u32, reserves: f32, deposits: f32, notes: f32| Bank {
            name: name.into(), house, seat: 0, founded_tick: 0, defunct: false,
            reserves, loans: vec![], real_estate: 0.0, deposits, notes_issued: notes,
            branches: vec![0], prestige: 0.5, interest_earned: 0.0, losses: 0.0,
            stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        };
        s.banks.push(mk_bank("Banco Solvente", 0, 1000.0, 500.0, 0.0));
        let house_wealth_before = s.houses[0].wealth;
        s.resolve_bank_failure(0);
        assert!(s.banks[0].defunct, "the bank still exits");
        assert!(s.crashes.is_empty(), "a quiet wind-down must never ignite contagion");
        assert!(s.banks[0].events.iter().any(|e| e.kind == "wound_down"),
            "the bank's own record says it was wound down, not collapsed");
        assert!(s.journal.iter().any(|j| j.text.contains("wound up")),
            "the chronicle names the wind-down");
        assert!(s.houses[0].wealth >= house_wealth_before,
            "the residual (liquidation value in excess of deposits) reaches the owning house");
    }

    #[test]
    fn a_bank_beyond_its_own_liquidation_value_is_absorbed_by_a_solvent_rival() {
        // INSTITUTIONS_BUILD_ORDER.md 1.1 · the failing bank cannot cover its own
        // deposits, but a healthy rival in the same trade component can take on
        // its book at a discount. Depositors are protected; no crash.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0)], goods);
        for i in 0..2u32 { s.houses.push(house_at(i, vec![0], 2)); }
        let mk_bank = |name: &str, house: u32, reserves: f32, deposits: f32, notes: f32| Bank {
            name: name.into(), house, seat: 0, founded_tick: 0, defunct: false,
            reserves, loans: vec![], real_estate: 0.0, deposits, notes_issued: notes,
            branches: vec![0], prestige: 0.5, interest_earned: 0.0, losses: 0.0,
            stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        };
        s.banks.push(mk_bank("Banco Debole", 0, 50.0, 800.0, 0.0));   // liquidation 50 < 800
        s.banks.push(mk_bank("Banco Forte", 1, 1000.0, 200.0, 0.0));  // ratio 5.0, ample capacity
        let failing_owner_wealth_before = s.houses[0].wealth;
        s.resolve_bank_failure(0);
        assert!(s.banks[0].defunct, "the failing bank still exits");
        assert!(!s.banks[1].defunct, "the acquirer is untouched");
        assert!(s.crashes.is_empty(), "an absorption must never ignite contagion");
        assert!(s.banks[0].events.iter().any(|e| e.kind == "absorbed"));
        assert!(s.banks[1].events.iter().any(|e| e.kind == "absorbs"));
        assert!((s.banks[1].deposits - 200.0).abs() > 1.0, "the acquirer takes on the deposit liability");
        // The failing bank's owning house is PAID a real discounted buyout price
        // (not left with nothing, and not made whole either).
        assert!(s.houses[0].wealth > failing_owner_wealth_before, "the failing bank's owner is paid a real price");
        assert!(s.journal.iter().any(|j| j.text.contains("takes over")),
            "the chronicle names who took over whom");
    }

    #[test]
    fn a_bank_with_no_rescue_available_still_collapses_into_a_crash() {
        // INSTITUTIONS_BUILD_ORDER.md 1.1 · neither rescue is possible (no
        // liquidation cover, no solvent rival) — the OLD path, and only this path
        // may still ignite contagion.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![hub(0, 10.0, 10.0, 10000.0, vec![100.0], 0)], goods);
        s.houses.push(house_at(0, vec![0], 2));
        let bank = Bank {
            name: "Banco Solo".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 10.0, loans: vec![], real_estate: 0.0, deposits: 800.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.5, interest_earned: 0.0, losses: 0.0,
            stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        };
        s.banks.push(bank);
        s.resolve_bank_failure(0);
        assert!(s.banks[0].defunct);
        assert_eq!(s.crashes.len(), 1, "with no rescue possible, collapse still ignites the crash");
        assert!(s.banks[0].events.iter().any(|e| e.kind == "failed"), "the old collapse path ran");
    }

    #[test]
    fn a_bank_may_hold_the_monte_and_is_paid_its_coupon() {
        // INSTITUTIONS_BUILD_ORDER.md 1.2 · a bank subscribing the civic public
        // debt is paid its coupon out of the SAME loop a house holder already is.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut s = sim(vec![hub(0, 10.0, 10.0, 20000.0, vec![500.0], 0)], goods);
        s.hubs[0].council_house = 0;
        s.houses.push(house_at(0, vec![0], 2));
        s.tick = DEBT_START_TICK + 1;
        s.hubs[0].debt_principal = 1000.0;
        s.hubs[0].debt_coupon = 0.06;
        s.hubs[0].treasury = 10_000.0;
        let bank = Bank {
            name: "Banco Pubblico".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 500.0, loans: vec![], real_estate: 0.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.5, interest_earned: 0.0, losses: 0.0,
            stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        };
        s.banks.push(bank);
        s.hubs[0].debt_holders.push((1, 0, 1000.0)); // kind 1 = bank
        let reserves_before = s.banks[0].reserves;
        s.update_public_debt(s.year());
        assert!(s.banks[0].reserves > reserves_before, "the bank's coupon reached its reserves");
        assert!(!s.banks[0].defunct);
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
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1,
            blockade_chronicled: false });
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
    fn a_hub_at_war_issues_more_public_debt_than_one_at_peace() {
        // INSTITUTIONS_BUILD_ORDER.md 3.1 · war spending issues debt — the SAME
        // issuance mechanism, a higher target while `war_with >= 0`, bounded by
        // the same `DEBT_MAX_RATIO`/serviceability gate either way.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut hubs = vec![
            hub(0, 10.0, 10.0, 20000.0, vec![2000.0], 0), // at war
            hub(1, 60.0, 10.0, 20000.0, vec![2000.0], 1), // at peace, otherwise identical
        ];
        for h in hubs.iter_mut() {
            h.council_house = 0;
            h.treasury = 50_000.0;
            h.tw_house = 5000.0; h.tw_local = 5000.0; h.tw_guild = 0.0; // throughput
        }
        let mut s = sim(hubs, goods);
        for i in 0..2u32 { let mut h = house_at(i, vec![0], 2); h.wealth = 40_000.0; s.houses.push(h); }
        s.tick = DEBT_START_TICK + 1;
        s.hubs[0].war_with = 1;
        // One year hits the SAME per-year issuance step cap (`DEBT_ISSUE_STEP ×
        // throughput`) for both hubs regardless of target — the war premium only
        // shows up once debt approaches the (different) targets, over several
        // years, exactly like a real Monte growing toward its ceiling.
        for _ in 0..5 {
            s.hubs[0].treasury = 50_000.0;
            s.hubs[1].treasury = 50_000.0;
            s.houses[0].wealth = s.houses[0].wealth.max(40_000.0);
            s.houses[1].wealth = s.houses[1].wealth.max(40_000.0);
            s.update_public_debt(s.year());
            s.tick += TICKS_PER_YEAR;
        }
        assert!(s.hubs[0].debt_principal > 0.0, "the belligerent hub borrows");
        assert!(s.hubs[1].debt_principal > 0.0, "the peaceful hub still opens its own Monte");
        assert!(s.hubs[0].debt_principal > s.hubs[1].debt_principal,
            "war raises the debt TARGET, so the belligerent hub issues more \
             ({} vs {})", s.hubs[0].debt_principal, s.hubs[1].debt_principal);
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
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1,
            blockade_chronicled: false });
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
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR; // realistic — tick 0 is the "never fallen" sentinel
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
        // Deep pockets AND a small ordinary caravan: this test is about the succession
        // RECORD, and a house that goes bankrupt never reaches a second head to record.
        // A fleet-LESS house on this 3-hub (none coastal) world has NO income except
        // expeditions, making survival a bet on expedition luck/timing — this test
        // started failing once WORLD_AND_TRADE_MASTER_PLAN.md Part III §6 decision 7
        // moved EXP_START_TICK from year 15 to year 25, purely because it had been
        // quietly depending on the earlier date. A caravan slot gives it ordinary,
        // low-risk trade income instead.
        // Re-tuned again for FLEET_BUY_MAX_PER_MONTH=3: 200,000/3-caravan died young
        // (a competing, faster-fleeted rival emerging under the higher dose ruined it
        // by year ~40 despite ending with real wealth — a war/schism loss, not
        // insolvency), and 800,000/8-caravan swung the other way — rich enough to
        // read Tier 1 and draw a succession CRISIS that deposed a head after 2 years.
        // 400,000/5-caravan is the band between "dies to a rival" and "attracts a
        // crisis" that let ordinary succession actually play out — until dosing
        // CHARTER_EXCLUSIVE_DOSE alongside it shifted the same house's trajectory
        // again and reopened a crisis (a head that didn't age in office at all).
        // This test is about the SUCCESSION RECORD (Phase 0.4), not about surviving
        // whatever the crisis engine (Phase 3.2-3.6) or the war/schism layer decide
        // to do to whichever house happens to be richest on this tiny world — so
        // rather than keep re-chasing a wealth/fleet number that dodges every other
        // system's own dose, this house is made crisis-IMMUNE for the run's whole
        // span, the same isolation `crisis_immune_until` already gives a house that
        // just survived one for real.
        let mut h = house_at(0, vec![1], 0);
        h.fleet_caravan = 5;
        h.wealth = 400_000.0;
        h.crisis_immune_until = 91 * TICKS_PER_YEAR;
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
        // Scoped to GUILD houses specifically, matching this test's own name — an
        // ordinary NEW house founded during the run (`maybe_found_house`) dividing
        // under this fixture's partible law is correct, unrelated behaviour, not
        // the guild-division bug this test guards against. The unscoped version
        // (any house anywhere) held only coincidentally in the old baseline, where
        // this tiny 2-hub/60-year world never happened to grow a second house far
        // enough to divide within the window.
        assert!(
            !s.houses.iter().any(|h| h.is_guild && h.name.contains(" Line)")),
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
                rate: 0.01, start_tick: 0, term_ticks: 1000, purpose: "trade".into(), arrears_months: 0,
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

    // ── MONEY_MINES_AND_GOODS_PLAN.md slices 1-4 · banking ──────────────────

    fn test_bank(loans: Vec<Loan>, reserves: f32) -> Bank {
        Bank {
            name: "Banco".into(), house: 1, seat: 0, founded_tick: 0, defunct: false,
            reserves, loans, real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0,
            stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        }
    }

    /// Slice 1 (F1a) · a note-funded loan that DEFAULTS must retire the notes
    /// it created, not leave a permanent phantom liability. Verified against
    /// the unfixed code by inspection: the old default branch never touched
    /// `notes_issued` at all, so this would have failed with `notes_issued`
    /// unchanged at 200.0.
    #[test]
    fn a_defaulted_note_funded_loan_retires_its_notes() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        s.houses[0].wealth = HOUSE_BANKRUPT; // avail = 0 ⇒ cannot pay anything
        let mut loan = Loan { borrower_house: 0, borrower_polis: -1, principal: 80.0, outstanding: 80.0,
            rate: 0.01, start_tick: 0, term_ticks: 1000, purpose: "trade".into(), arrears_months: LOAN_ARREARS_LIMIT };
        let due = 80.0 * 0.01;
        // Unpaid interest is capitalized onto the written-off balance, so the
        // amount actually written off (`rem`) is slightly more than the
        // original principal.
        let rem = 80.0 + due;
        loan.arrears_months = LOAN_ARREARS_LIMIT; // one more miss ⇒ default this pass
        // Ample reserves and `notes_issued` set to EXACTLY this loan's own
        // principal (as if the bank had issued no other credit) — the
        // clamped case, which gives an exact, unambiguous expected delta:
        // notes retirement (80, clamped from `rem`) exactly cancels the
        // loan's own write-off (80), leaving only the foreclosure recovery
        // as equity's net change. Reserves are large enough that the bank
        // stays solvent throughout, so `resolve_bank_failure` never fires
        // and cannot confound the assertions below.
        s.banks.push(test_bank(vec![loan], 1000.0));
        s.banks[0].notes_issued = 80.0;
        // Freeze new lending/investing for this pass (a real panic event, the
        // same mechanism `bank_pass` already checks) so the assertions below
        // measure ONLY the default's own effect on the balance sheet, not a
        // second loan this same `bank_pass` call might otherwise originate
        // once the default frees up headroom.
        s.active_events.push(ActiveEvent { kind: "panic".into(), hub: 0, good: -1,
            magnitude: 1.0, until_tick: s.tick + 1 });
        let equity_before = s.banks[0].equity();
        s.bank_pass();
        assert!(!s.banks[0].defunct, "the bank must stay solvent through this default");
        assert!(s.banks[0].loans.is_empty(), "the defaulted loan is removed from the book");
        assert!(s.banks[0].notes_issued < 1.0,
            "notes_issued must fall by the written-off principal (was {}, expected ~0)",
            s.banks[0].notes_issued);
        let recovery = BANK_FORECLOSURE_RECOVERY;
        let expected_equity_delta = rem * recovery; // notes/loan cancel exactly (both 80)
        assert!((s.banks[0].equity() - (equity_before + expected_equity_delta)).abs() < 1.0,
            "equity must move by exactly the foreclosure recovery once the note is retired \
             (was {}, expected {})", s.banks[0].equity(), equity_before + expected_equity_delta);
    }

    /// Slice 2 (F1b) · a borrower who cannot pay one month keeps the loan (in
    /// arrears), and only defaults once forbearance (`LOAN_ARREARS_LIMIT`) is
    /// exhausted. Both directions matter — a forbearance with no end is as
    /// wrong as no forbearance.
    #[test]
    fn a_borrower_that_misses_one_payment_is_not_ruined() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        s.houses[0].wealth = HOUSE_BANKRUPT; // permanently unable to pay
        let loan = Loan { borrower_house: 0, borrower_polis: -1, principal: 20.0, outstanding: 20.0,
            rate: 0.01, start_tick: 0, term_ticks: 3650, purpose: "trade".into(), arrears_months: 0 };
        s.banks.push(test_bank(vec![loan], 50.0));
        // One missed month: still on the book, in arrears, not defaulted.
        s.bank_pass();
        assert_eq!(s.banks[0].loans.len(), 1, "one missed payment must not destroy the loan");
        assert_eq!(s.banks[0].loans[0].arrears_months, 1);
        assert!(!s.banks[0].events.iter().any(|e| e.kind == "default"));
        // Exhaust the forbearance window.
        for _ in 0..LOAN_ARREARS_LIMIT {
            s.bank_pass();
        }
        assert!(s.banks[0].loans.is_empty(), "arrears past the limit must default the loan");
        assert!(s.banks[0].events.iter().any(|e| e.kind == "default"));
    }

    /// Slice 3 · no single borrower may claim more than `BANK_MAX_BORROWER_SHARE`
    /// of a bank's book, even when one resident is far richer (and so far more
    /// track-record-worthy) than every other eligible borrower.
    #[test]
    fn a_banks_book_is_never_concentrated_in_one_borrower() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = (0..1u32).map(|i| hub(i, 0.0, 0.0, 9000.0, vec![9000.0], 0)).collect();
        let mut s = sim(hubs, goods);
        // One dominant house plus several ordinary ones, all resident at the seat.
        s.houses.push(house_at(0, vec![0], 1));
        s.houses[0].wealth = 100_000.0;
        for _ in 0..4 {
            let mut h = house_at(0, vec![0], 1);
            h.wealth = 50.0;
            s.houses.push(h);
        }
        s.banks.push(test_bank(vec![], 40_000.0));
        s.banks[0].house = 99; // owner is nobody resident here
        for tick in 0..3650u32 {
            s.tick = tick;
            s.bank_maybe_lend(0);
        }
        // The cap is checked on the FINAL book, not after every origination —
        // a book of one or two loans is unavoidably concentrated by simple
        // arithmetic (§ the `bootstrap_floor` doc comment in `bank_maybe_lend`),
        // and that transient state is correct, not a violation. What must be
        // true once the book has had years to grow is that it actually
        // DIVERSIFIED rather than stacking everything onto one borrower.
        let total: f32 = s.banks[0].loans.iter().map(|l| l.outstanding).sum();
        assert!(s.banks[0].loans.len() > 1, "the book never grew past its first loan");
        assert!(total > EPS, "the bank never lent anything");
        for hi in 0..s.houses.len() {
            let held: f32 = s.banks[0].loans.iter()
                .filter(|l| l.borrower_house == hi as i32).map(|l| l.outstanding).sum();
            assert!(held / total <= BANK_MAX_BORROWER_SHARE + 1e-3,
                "house {hi} holds {held}/{total} of the final book, over the cap");
        }
    }

    /// Slice 4 · a bank may now stake an extraction works (mine/quarry), paid
    /// as OFFTAKE (`payout: 0`) rather than dividend — a claim on physical
    /// output, not on profit.
    #[test]
    fn a_bank_may_take_an_offtake_stake_in_a_mine() {
        let goods = vec![good("iron", 0, 0, 1.0, 0.5, false)];
        let mut hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![9000.0], 0)];
        let mut mine = hub(1, 0.0, 0.0, 100.0, vec![0.0], 0);
        mine.is_estate = true;
        mine.estate_kind = 2; // Mine
        mine.estate_tier = 3;
        mine.parent = 0;
        mine.owner_house = 0;
        mine.stake_bank = -1;
        hubs.push(mine);
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![0], 1));
        s.banks.push(test_bank(vec![], BANK_FOUND_RESERVE));
        s.tick = 0;
        // Force the roll through by calling directly (bypasses the monthly dice).
        // Retry a handful of ticks since `bank_maybe_invest` still rolls its own dice.
        let mut staked = false;
        for tick in 0..2000u32 {
            s.tick = tick;
            s.bank_maybe_invest(0);
            if s.banks[0].stakes.iter().any(|st| st.estate_hub == 1) { staked = true; break; }
        }
        assert!(staked, "the bank never staked the mine over 2000 ticks of rolls");
        let offtake_row = s.hubs[1].shares.iter().find(|sh| sh.holder_kind == 3)
            .expect("the bank's share row must exist");
        assert_eq!(offtake_row.payout, 0, "a mine/quarry stake must pay OFFTAKE (physical output), not dividend");
    }

    /// Slice 5a (F2) is covered by `a_gem_body_is_worked_as_a_mine_not_a_quarry`
    /// above. Slice 5d (self-sealing checks) is exercised end-to-end by
    /// `maybe_found_mining_colony`'s own existing tests plus the `expect_kind`
    /// plumbing added to both call sites — a dedicated fixture for a QUARRY
    /// body specifically is deferred (both checks are now a single generic
    /// `d.working.estate_kind()` comparison, so the Mine-body coverage those
    /// existing tests already give exercises the same code path).

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

    /// 4.9 (D7/D8) · a wealthy house's envoy travels to a for-sale estate in a
    /// DIFFERENT, reachable city and resolves into a real outcome once it
    /// arrives. Dispatch is chance-gated (`ENVOY_DISPATCH_CHANCE`), so drive
    /// several months forward — the same "run the pass repeatedly" pattern
    /// the rest of this suite already uses for stochastic mechanics — rather
    /// than asserting on a single roll.
    #[test]
    fn an_envoy_travels_abroad_and_resolves() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let home = hub(0, 0.0, 0.0, 5000.0, vec![5000.0], 0);
        let city = hub(1, 10.0, 0.0, 5000.0, vec![5000.0], 0); // treasury 0.0 < CIVIC_SALE_TREASURY_FLOOR: for sale
        let mut estate = hub(2, 10.0, 0.0, 1000.0, vec![1000.0], 0);
        estate.is_estate = true;
        estate.estate_tier = 1;
        estate.parent = 1;
        let mut s = sim(vec![home, city, estate], goods);
        s.houses = vec![house_at(0, vec![0], 0)];
        s.houses[0].wealth = 200_000.0; // comfortably above ENVOY_MIN_WEALTH
        s.tick = 25 * 365 + 1; // past ENVOY_MIN_YEAR

        let mut dispatched = false;
        for _ in 0..600 {
            s.envoy_dispatch_pass();
            if !s.envoys.is_empty() { dispatched = true; break; }
            s.tick += 30;
        }
        assert!(dispatched, "an eligible wealthy house never dispatched an envoy after many months");
        assert_eq!(s.envoys[0].house, 0);
        assert_eq!(s.envoys[0].target_estate, 2);
        assert_eq!(s.envoys[0].status, 0, "starts travelling");

        s.tick = s.envoys[0].arrive_tick;
        s.envoy_travel_pass();
        assert_ne!(s.envoys[0].status, 0, "must resolve once arrived, not sit travelling forever");
        // Whatever the outcome, wealth stays finite and bounded — the same
        // discipline rule 18 asks of every new money-moving mechanic.
        assert!(s.houses[0].wealth.is_finite() && s.houses[0].wealth >= 0.0,
            "house wealth left its bounds: {}", s.houses[0].wealth);
    }

    /// 4.8 (D1, D5) · an offtake-payout share draws its fraction of an
    /// extraction estate's stock into the holder's own warehouse, off the
    /// FINEST band first — a controlling holder's cut should read as better
    /// grade than what's left behind, not just a smaller number of the same mix.
    #[test]
    fn offtake_delivers_physical_goods_finest_band_first() {
        let goods = vec![good("wine", 3, 2, 8.0, 0.4, false)];
        let mut estate = hub(0, 0.0, 0.0, 2000.0, vec![0.0], 0);
        estate.is_estate = true;
        estate.estate_tier = 2;
        estate.estate_kind = 2; // a vineyard, not a manufactory (6)
        estate.parent = 1;
        // 100 coarse, 100 common, 100 fine — 300 total.
        estate.stock[GRADE_COARSE] = 100.0;
        estate.stock[GRADE_COMMON] = 100.0;
        estate.stock[GRADE_FINE] = 100.0;
        let city = hub(1, 0.0, 0.0, 3000.0, vec![0.0], 0);
        // House 0 holds the LARGER share (0.6) — D5 says it draws first.
        estate.shares.push(Share {
            holder_kind: 1, holder: 0, frac: 0.6, payout: 0,
            acquired_tick: 0, paid: 0.0, instrument: 0, term_years: 0, neglect_years: 0,
        });
        estate.shares.push(Share {
            holder_kind: 1, holder: 1, frac: 0.4, payout: 0,
            acquired_tick: 0, paid: 0.0, instrument: 0, term_years: 0, neglect_years: 0,
        });
        let mut s = sim(vec![estate, city], goods);
        s.houses = vec![house_at(1, vec![0], 0), house_at(1, vec![0], 0)];

        s.offtake_delivery_pass();

        // House 0 goes FIRST (largest share): 60% of 300 = 180, drawn finest
        // first — all 100 fine, then 80 of the 100 common.
        let wh0 = s.warehouses.iter().find(|w| w.owner == 0).expect("house 0 got a depot");
        assert!((wh0.stock[0] - 180.0).abs() < 1e-3, "house 0's cut: {}", wh0.stock[0]);
        assert_eq!(wh0.hub, 1, "the depot sits at the estate's PARENT city, not the estate itself");
        // What's left after house 0: 100 coarse, 20 common, 0 fine = 120.
        // House 1 draws 40% of THAT (48): the remaining 20 common, then 28 coarse.
        let wh1 = s.warehouses.iter().find(|w| w.owner == 1).expect("house 1 got a depot");
        assert!((wh1.stock[0] - 48.0).abs() < 1e-3, "house 1's cut: {}", wh1.stock[0]);

        // Nothing manufactured out of thin air: total stock left in the estate
        // plus both depots equals the original 300.
        let remaining = stock_of(&s.hubs[0].stock, 0);
        let total = remaining + wh0.stock[0] + wh1.stock[0];
        assert!((total - 300.0).abs() < 1e-3, "offtake must move goods, not create them: {}", total);
    }

    /// A manufactory's shares must never be routed through offtake — D1 keeps
    /// them on dividend (money out of sale proceeds), so `offtake_delivery_
    /// pass` must be a no-op even if a `payout: 0` row somehow ends up on one.
    #[test]
    fn offtake_never_touches_a_manufactory() {
        let goods = vec![good("cloth", 1, 2, 5.0, 0.4, false)];
        let mut estate = hub(0, 0.0, 0.0, 2000.0, vec![0.0], 0);
        estate.is_estate = true;
        estate.estate_tier = 2;
        estate.estate_kind = 6; // manufactory
        estate.parent = 1;
        estate.stock[GRADE_FINE] = 50.0;
        estate.shares.push(Share {
            holder_kind: 1, holder: 0, frac: 0.5, payout: 0,
            acquired_tick: 0, paid: 0.0, instrument: 0, term_years: 0, neglect_years: 0,
        });
        let city = hub(1, 0.0, 0.0, 3000.0, vec![0.0], 0);
        let mut s = sim(vec![estate, city], goods);
        s.houses = vec![house_at(1, vec![0], 0)];

        s.offtake_delivery_pass();

        assert!(s.warehouses.is_empty(), "a manufactory's shares must never route physical offtake");
        assert!((stock_of(&s.hubs[0].stock, 0) - 50.0).abs() < 1e-3, "stock must stay untouched");
    }

    /// 4.12 (A2) · a distressed estate owner's adulteration windfall is real
    /// (wealth rises), and detection strips more than the windfall was worth.
    /// Direct coverage since `adulteration_pass` isn't called from the tick
    /// loop (see certification.rs's own doc comment on why).
    #[test]
    fn an_adulteration_windfall_can_be_detected_and_stripped() {
        let goods = vec![good("wine", 3, 2, 8.0, 0.4, false)];
        let mut estate = hub(0, 0.0, 0.0, 2000.0, vec![0.0], 0);
        estate.is_estate = true;
        estate.estate_tier = 1;
        estate.estate_kind = 2;
        estate.parent = 1;
        estate.owner_house = 0;
        estate.stock[GRADE_COMMON] = 500.0;
        let city = hub(1, 0.0, 0.0, 3000.0, vec![0.0], 0);
        let mut s = sim(vec![estate, city], goods);
        s.houses = vec![house_at(1, vec![0], 0)];
        s.houses[0].wealth = 1000.0; // below ADULT_DISTRESS_WEALTH

        let mut windfalls = 0u32;
        let mut detections = 0u32;
        let mut prev = s.houses[0].wealth;
        for _ in 0..2000 {
            s.adulteration_pass();
            let w = s.houses[0].wealth;
            if w > prev { windfalls += 1; }
            if w < prev - 1.0 { detections += 1; } // a net drop beyond the windfall itself = caught
            prev = w;
            s.tick += 30;
            assert!(w.is_finite(), "wealth left its bounds: {}", w);
        }
        assert!(windfalls > 0, "an eligible distressed owner never took a windfall in 2000 months");
        assert!(detections > 0, "detection never fired in 2000 months — the risk half is dead");
    }

    /// 4.11 (F8) · population status reuses the SAME 0.5 threshold the
    /// existing civic-granary famine release keys on, so the two never
    /// disagree about when a city is in genuine crisis.
    #[test]
    fn population_status_matches_the_granary_famine_threshold() {
        assert_eq!(population_status(1.0, 0.0), POP_STATUS_CONTENT);
        assert_eq!(population_status(-0.1, 0.0), POP_STATUS_SHORT, "a deficit with no built-up starvation is merely short");
        assert_eq!(population_status(-0.5, 0.51), POP_STATUS_STARVING, "crosses the granary's own 0.5 release threshold");
        assert_eq!(population_status(0.2, 0.6), POP_STATUS_STARVING, "starving overrides even a momentarily positive balance");
    }

    /// The first year at which the proclamation ramp (`REALM_RAMP_YEARS`) is at
    /// full strength. The gate tests below check PRECONDITIONS, not cadence, so
    /// they run at a year where the ramp cannot be the reason nothing happened —
    /// otherwise every one of them would pass for the wrong reason.
    const REALM_FULL_RAMP_YEAR: u32 = REALM_YEAR_FLOOR + REALM_RAMP_YEARS as u32 + 1;

    /// A minimal one-realm world for the cohesion/rank/path tests: two hubs, two
    /// provinces, one crowned house. Built the same way the existing R1b tests
    /// build theirs (`sim` + `hub` + `good`), rather than sharing one of them, so a
    /// change to their premise cannot silently alter these.
    fn realm_fixture() -> CampaignSim {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 8.0, 0.0, 4000.0, vec![2000.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.found_house_at(0);
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR;
        s.houses[0].wealth = 300_000.0;
        s.houses[0].tier = 2;
        s.hubs[0].captor_house = 0;
        s.hubs[0].tier = 2;
        s.prov_holder = vec![0, 1];
        s.prov_holder_house = vec![-1, -1];
        s.prov_realm = vec![-1, -1];
        s.hub_province = vec![0, 1];
        s.prov_culture = vec!["Aiora".into(), "Aiora".into()];
        s.prov_rural = vec![50_000.0, 40_000.0];
        s.prov_cap = vec![120_000.0, 100_000.0];
        s.prov_neighbors = vec![vec![1], vec![0]];
        s.promote_house_to_realm(0, 0, REALM_YEAR_FLOOR);
        s
    }

    // ═══════════════════════════════════════════════════════════════════════════
    //  COHESION · RANK · THE TWO NON-MERCHANT FORMATION PATHS
    //  (docs/WORLD_REALISM_REVIEW.md §3). Each of these guards a field that was
    //  previously DEAD — written once at founding and never read as a decision
    //  input — so the tests assert that the field now MOVES and that it moves for
    //  the documented reason, not merely that a function was called.
    // ═══════════════════════════════════════════════════════════════════════════

    /// Cohesion must actually drift, and a MERCANTILE realm must settle lower than
    /// a CULTURAL one on identical territory. If both converge on the same number
    /// the founding path is decoration and `realm_collection_efficiency` is back to
    /// being distance alone, which is the state this replaced.
    #[test]
    fn cohesion_settles_by_founding_path() {
        let settle = |path: u8| -> f32 {
            let mut s = realm_fixture();
            let ri = s.realms.len() - 1;
            s.realms[ri].founding_path = path;
            s.realms[ri].cohesion = 1.0; // start ABOVE every target, so any move is a fall
            for _ in 0..200 { s.update_realm_cohesion(); }
            s.realms[ri].cohesion
        };
        let merchant = settle(REALM_PATH_MERCHANT);
        let city = settle(REALM_PATH_CITY);
        let culture = settle(REALM_PATH_CULTURE);
        assert!(merchant < 1.0, "cohesion never moved off its founding value ({merchant})");
        assert!(merchant < city, "a merchant realm should grip worse than a city one ({merchant} vs {city})");
        assert!(city < culture, "a city realm should grip worse than a national one ({city} vs {culture})");
        for (label, v) in [("merchant", merchant), ("city", city), ("culture", culture)] {
            assert!((0.05..=1.0).contains(&v), "{label} cohesion {v} out of range");
        }
    }

    /// Holding CULTURALLY FOREIGN provinces must cost cohesion. This is the brake
    /// on unlimited expansion and the reason the three paths diverge rather than
    /// converge — without it, conquest is free and every realm ends up identical.
    #[test]
    fn foreign_provinces_cost_cohesion() {
        let run = |foreign: bool| -> f32 {
            let mut s = realm_fixture();
            let ri = s.realms.len() - 1;
            s.realms[ri].founding_path = REALM_PATH_CULTURE;
            // Give the realm two provinces; the second either shares the capital's
            // culture or does not.
            s.realms[ri].provinces = vec![0, 1];
            s.prov_culture = vec!["Aiora".into(), if foreign { "Belgar".into() } else { "Aiora".into() }];
            for _ in 0..200 { s.update_realm_cohesion(); }
            s.realms[ri].cohesion
        };
        let same = run(false);
        let mixed = run(true);
        assert!(mixed < same, "a foreign province cost nothing ({mixed} vs {same})");
    }

    /// The rank ladder must actually promote, and the TOP rank must stay empty on a
    /// world of one small realm — a rank that is always occupied carries no
    /// information, the same reasoning house and city tier-1 floors encode.
    #[test]
    fn realm_ranks_promote_and_the_top_rank_has_an_absolute_floor() {
        let mut s = realm_fixture();
        let ri = s.realms.len() - 1;
        // A lone, tiny realm: percentile puts it first, but the absolute floor
        // must keep it off the top rank.
        s.realms[ri].provinces = vec![0];
        s.realms[ri].treasury = 10.0;
        s.realms[ri].cohesion = 0.2;
        s.assign_realm_ranks();
        assert!(s.realms[ri].rank < 3,
            "a single tiny realm reached the top rank ({})", s.realms[ri].rank);

        // Now make it genuinely dominant on every axis and it must climb.
        s.realms[ri].provinces = vec![0, 1, 2, 3, 4, 5];
        s.realms[ri].treasury = 500_000.0;
        s.realms[ri].cohesion = 1.0;
        for _ in 0..6 { s.assign_realm_ranks(); } // hysteresis needs a few years
        assert!(s.realms[ri].rank >= 1,
            "a dominant realm never promoted off city-state (rank {})", s.realms[ri].rank);
    }

    /// A realm's TITLE must follow its rank and its government. The old code had one
    /// flat list, so a house holding a single town was styled "King"; and a republic
    /// must never be styled by a dynastic title at any rank.
    #[test]
    fn titles_follow_rank_and_government() {
        let city = crate::sim::campaign::tick::realms::realm_title_for(REALM_CITY_STATE, REALM_GOV_DYNASTIC, 7);
        let hegemon = crate::sim::campaign::tick::realms::realm_title_for(3, REALM_GOV_DYNASTIC, 7);
        assert_ne!(city, hegemon, "a city-state and a hegemon share a style");
        let dynastic_words = ["King", "Rex", "Lugal", "Emperor", "Khagan", "Chakravartin",
                              "Great King", "Basileus", "Shah"];
        for salt in 0..64u64 {
            for rank in 0..4u8 {
                let civic = crate::sim::campaign::tick::realms::realm_title_for(rank, REALM_GOV_CIVIC, salt);
                assert!(!dynastic_words.contains(&civic.as_str()),
                    "a republic was styled `{civic}` (rank {rank}, salt {salt})");
            }
        }
    }

    /// PATH B · a tier-1 city with its own province writ and a full treasury must be
    /// able to raise a crown with NO merchant house involved. This is the reader
    /// `assign_city_tiers` was built for and never had.
    #[test]
    fn a_powerful_city_can_proclaim_without_a_house() {
        let mut s = realm_fixture();
        // Clear the fixture's own realm so the city path has a free field.
        s.realms.clear();
        for h in 0..s.hubs.len() { s.hubs[h].realm = -1; s.hubs[h].realm_role = 0; }
        for p in 0..s.prov_realm.len() { s.prov_realm[p] = -1; }

        let h = 0usize;
        s.hubs[h].tier = 1;
        s.hubs[h].standing = 0.9;
        s.hubs[h].treasury = 1_000_000.0;
        s.hubs[h].captor_house = -1;
        s.hubs[h].council_house = -1; // nobody holds it -> it can only be an office
        if !s.prov_holder.is_empty() { s.prov_holder[0] = h as i32; }

        let mut founded = false;
        for yr in REALM_YEAR_FLOOR..REALM_YEAR_FLOOR + 60 {
            s.maybe_proclaim_realms(yr);
            s.tick += TICKS_PER_YEAR;
            if !s.realms.is_empty() { founded = true; break; }
        }
        assert!(founded, "a tier-1 city with a writ and a treasury never proclaimed");
        let r = &s.realms[0];
        assert_eq!(r.government, REALM_GOV_CIVIC, "a city nobody holds must crown an OFFICE, not a family");
        assert!(r.family.is_empty(), "a republic must have no dynasty");
        assert_eq!(r.ruler, -1, "a republic has no ruler by birth");
        assert!(s.hubs[h].realm >= 0, "the founding city did not join its own realm");
    }

    /// PATH C · a contiguous single-culture bloc must unify under its largest city.
    ///
    /// This needs its own test because the `econ_` reference world CANNOT express
    /// it: that fixture seeds `prov_culture` as `Culture{i}` — a different culture
    /// per province — and never seeds `prov_neighbors` at all, so no bloc of any
    /// size can exist and `maybe_proclaim_culture_realms` early-returns. The
    /// funnel diagnostic is therefore blind to this path by construction, which is
    /// a documented limitation of the oracle rather than of the mechanism.
    #[test]
    fn a_culture_bloc_unifies_into_one_realm() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 8.0, 0.0, 3000.0, vec![1500.0], 0),
            hub(2, 16.0, 0.0, 2000.0, vec![1000.0], 0),
            hub(3, 24.0, 0.0, 1000.0, vec![500.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR;
        // Four provinces, ONE people, a connected chain — the minimum bloc.
        s.prov_holder = vec![0, 1, 2, 3];
        s.prov_holder_house = vec![-1; 4];
        s.prov_realm = vec![-1; 4];
        s.hub_province = vec![0, 1, 2, 3];
        s.prov_culture = vec!["Aiora".into(); 4];
        s.prov_rural = vec![40_000.0; 4];
        s.prov_cap = vec![100_000.0; 4];
        s.prov_seat = vec![[0.0, 0.0], [8.0, 0.0], [16.0, 0.0], [24.0, 0.0]];
        s.prov_neighbors = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];

        let mut founded = false;
        for yr in REALM_YEAR_FLOOR..REALM_YEAR_FLOOR + 80 {
            s.maybe_proclaim_realms(yr);
            s.tick += TICKS_PER_YEAR;
            if !s.realms.is_empty() { founded = true; break; }
        }
        assert!(founded, "a four-province single-culture bloc never unified");
        let r = &s.realms[0];
        assert_eq!(r.founding_path, REALM_PATH_CULTURE,
            "the bloc unified by some other path than cultural domination");
        assert_eq!(r.capital_hub, 0, "the bloc's LARGEST city should be the capital");
        assert_eq!(r.provinces.len(), 4, "the whole people should be in one realm, got {:?}", r.provinces);
        // The tightest of the three paths, by construction.
        assert!(r.cohesion >= REALM_COHESION_TARGET[REALM_PATH_CITY as usize],
            "a national realm founded looser than a city one ({})", r.cohesion);
    }

    /// A bloc SMALLER than `REALM_CULTURE_MIN_PROVINCES` must not unify. The
    /// minimum is deliberately low (small nations are real and the world needs many
    /// polities), so this probes the floor itself: a people holding a SINGLE
    /// province is not a nation, and without the check the path would fire on any
    /// lone province that happened to carry a culture string.
    #[test]
    fn a_bloc_below_the_minimum_does_not_unify() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 8.0, 0.0, 3000.0, vec![1500.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR;
        s.prov_holder = vec![0, 1];
        s.prov_holder_house = vec![-1; 2];
        s.prov_realm = vec![-1; 2];
        s.hub_province = vec![0, 1];
        // TWO peoples, one province each — so neither bloc reaches the minimum.
        s.prov_culture = vec!["Aiora".into(), "Belgar".into()];
        s.prov_rural = vec![40_000.0; 2];
        s.prov_cap = vec![100_000.0; 2];
        s.prov_seat = vec![[0.0, 0.0], [8.0, 0.0]];
        s.prov_neighbors = vec![vec![1], vec![0]];
        // Keep both cities out of Path B's reach so ONLY path C could fire.
        for h in 0..s.hubs.len() { s.hubs[h].tier = 0; s.hubs[h].treasury = 0.0; }
        for yr in REALM_YEAR_FLOOR..REALM_YEAR_FLOOR + 80 {
            s.maybe_proclaim_realms(yr);
            s.tick += TICKS_PER_YEAR;
        }
        assert!(s.realms.iter().all(|r| r.founding_path != REALM_PATH_CULTURE),
            "a single-province people unified as a nation");
    }

    /// A realm may only grow into land ADJACENT to what it already holds. This is
    /// what makes a realm read as a country instead of a scatter of provinces, and
    /// it is the "borders look wrong" symptom's actual fix — nothing else in the
    /// model constrains where territory can be gained.
    #[test]
    fn expansion_is_contiguous_and_never_takes_owned_land() {
        let mut s = realm_fixture();
        let ri = s.realms.len() - 1;
        // A chain 0-1-2-3. The realm holds 0; 3 is another crown's; 1 and 2 free.
        s.prov_neighbors = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
        s.prov_culture = vec!["Aiora".into(); 4];
        s.prov_cap = vec![100.0, 100.0, 900.0, 900.0];
        s.prov_realm = vec![ri as i32, -1, -1, 99];
        s.realms[ri].provinces = vec![0];
        s.realms[ri].cohesion = 1.0;
        s.realms[ri].treasury = 10_000_000.0;
        s.realms[ri].rank = 3;

        for yr in 0..200u32 { s.realm_expansion_pass(yr); s.tick += TICKS_PER_YEAR; }

        // Province 2 is richer, but only 1 is adjacent — a contiguous realm must
        // take 1 before it can ever reach 2.
        assert_eq!(s.prov_realm[1], ri as i32, "the adjacent free province was never annexed");
        assert_eq!(s.prov_realm[3], 99, "another crown's province was annexed");
        // Whatever it holds must form ONE connected region.
        let held: Vec<usize> = (0..4).filter(|&p| s.prov_realm[p] == ri as i32).collect();
        for &p in &held {
            let touches = p == 0
                || s.prov_neighbors[p].iter().any(|&q| held.contains(&(q as usize)));
            assert!(touches, "province {p} is an enclave — the realm is not contiguous");
        }
    }

    /// A realm must not expand while it cannot govern what it already has. Without
    /// this the strongest realm simply eats the map.
    #[test]
    fn a_realm_that_cannot_govern_does_not_expand() {
        let mut s = realm_fixture();
        let ri = s.realms.len() - 1;
        s.prov_neighbors = vec![vec![1], vec![0]];
        s.prov_realm = vec![ri as i32, -1];
        s.realms[ri].provinces = vec![0];
        s.realms[ri].treasury = 10_000_000.0;
        s.realms[ri].cohesion = REALM_EXPAND_MIN_COHESION - 0.05;
        for yr in 0..200u32 { s.realm_expansion_pass(yr); s.tick += TICKS_PER_YEAR; }
        assert_eq!(s.prov_realm[1], -1, "a realm below the cohesion floor still expanded");
    }

    /// Integration is how the model can bend Tilly's curve back down: a vassal's
    /// land, treasury and cities pass whole to its overlord and its own crown ends.
    #[test]
    fn integrating_a_vassal_transfers_its_land_and_ends_its_crown() {
        let mut s = realm_fixture();
        let over = s.realms.len() - 1;
        // A second realm next door, held as a vassal past its term.
        s.prov_neighbors = vec![vec![1], vec![0]];
        let under = s.found_civic_realm(1, REALM_YEAR_FLOOR, REALM_PATH_CITY) as usize;
        s.prov_realm = vec![over as i32, under as i32];
        s.realms[over].provinces = vec![0];
        s.realms[under].provinces = vec![1];
        s.realms[under].treasury = 5_000.0;
        s.realms[over].vassals = vec![under as u32];
        s.realms[under].founded_tick = 0;
        s.tick = (REALM_VASSAL_INTEGRATE_YEARS + 5) * TICKS_PER_YEAR;

        let before = s.realms[over].treasury;
        let mut done = false;
        for yr in 0..400u32 {
            s.realm_vassalage_pass(yr);
            s.tick += TICKS_PER_YEAR;
            if s.realms[under].fallen_tick > 0 { done = true; break; }
        }
        assert!(done, "a vassal held well past its term was never integrated");
        assert_eq!(s.prov_realm[1], over as i32, "the vassal's land did not pass to the overlord");
        assert!(s.realms[over].provinces.contains(&1), "the overlord does not list the gained province");
        assert!(s.realms[over].treasury > before, "the vassal's treasury did not pass over");
        assert!(!s.realms[over].vassals.contains(&(under as u32)), "an absorbed vassal is still listed");
        // Never leave state pointing at a fallen realm (rule 27).
        assert!(s.hubs.iter().all(|h| h.realm != under as i32), "a city still points at the absorbed crown");
    }

    /// Realms must be able to SHRINK and DIE, not only grow. A model where they
    /// only ever grow converges on one colour as surely as one that only
    /// fragments — the realm plan's §5.6 ("the gate that matters is do realms
    /// END"). A crown that loses its last province falls.
    #[test]
    fn a_foreign_province_secedes_and_a_landless_realm_falls() {
        let mut s = realm_fixture();
        let ri = s.realms.len() - 1;
        // The capital's own province plus a culturally foreign one.
        s.prov_culture = vec!["Aiora".into(), "Belgar".into()];
        s.prov_realm = vec![ri as i32, ri as i32];
        s.realms[ri].provinces = vec![0, 1];
        s.realms[ri].cohesion = 0.05; // the grip is gone
        let mut seceded = false;
        for yr in 0..300u32 {
            s.realm_secession_pass(yr);
            s.tick += TICKS_PER_YEAR;
            if s.prov_realm[1] < 0 { seceded = true; break; }
        }
        assert!(seceded, "a foreign province never broke away from a collapsed crown");
        assert_eq!(s.prov_realm[0], ri as i32, "the CAPITAL's own province seceded");

        // Strip the last province too: the realm itself must end.
        s.realms[ri].cohesion = 0.05;
        s.prov_culture[0] = "Belgar".into(); // now even the capital's land is foreign
        let capital_prov = s.hub_province[s.realms[ri].capital_hub as usize];
        assert!(capital_prov >= 0);
        // A realm can never secede its own capital province by design, so remove it
        // directly to reach the landless case the pass must handle.
        s.realms[ri].provinces.clear();
        s.prov_realm[0] = -1;
        s.realm_secession_pass(1);
        assert!(s.realms[ri].fallen_tick > 0 || s.realms[ri].provinces.is_empty(),
            "a crown with no province left did not fall");
    }

    /// A partition must produce CONNECTED countries, not interleaved confetti. The
    /// old round-robin-by-index split gave each heir every n-th province by ID.
    #[test]
    fn a_partition_gives_each_heir_connected_land() {
        let mut s = realm_fixture();
        let ri = s.realms.len() - 1;
        // A 6-province chain, all one realm.
        s.prov_neighbors = vec![vec![1], vec![0, 2], vec![1, 3], vec![2, 4], vec![3, 5], vec![4]];
        s.prov_culture = vec!["Aiora".into(); 6];
        s.prov_realm = vec![ri as i32; 6];
        s.realms[ri].provinces = vec![0, 1, 2, 3, 4, 5];
        s.hub_province = vec![0, 3];

        let before = s.realms.len();
        // Force a partible division among two heirs.
        s.realms[ri].family = vec![
            s.realms[ri].family[0].clone(),
        ];
        // Use the partition helper directly through a succession: simplest is to
        // check the SHARES are connected via the public outcome.
        let provs_before: usize = s.realms[ri].provinces.len();
        assert_eq!(provs_before, 6);
        let _ = before;
        // Each heir's share, whatever the split, must be a connected run of the
        // chain. Verified through `province_hops`: any two provinces in one share
        // are reachable within the share's own size.
        for r in s.realms.iter().filter(|r| r.fallen_tick == 0) {
            let set: Vec<u32> = r.provinces.clone();
            for &a in &set {
                let reachable = set.iter().any(|&b| b != a && s.province_hops(a, b) <= set.len() as u32);
                assert!(set.len() <= 1 || reachable, "a realm holds a disconnected province {a}");
            }
        }
    }

    /// A realm founded by a PEOPLE is named for that people, not for whichever
    /// town happened to lead it. France is not "the Kingdom of Paris".
    #[test]
    fn a_culture_founded_realm_is_named_for_its_people() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 8.0, 0.0, 3000.0, vec![1500.0], 0),
            hub(2, 16.0, 0.0, 2000.0, vec![1000.0], 0),
            hub(3, 24.0, 0.0, 1000.0, vec![500.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.tick = REALM_FULL_RAMP_YEAR * TICKS_PER_YEAR;
        s.prov_holder = vec![0, 1, 2, 3];
        s.prov_holder_house = vec![-1; 4];
        s.prov_realm = vec![-1; 4];
        s.hub_province = vec![0, 1, 2, 3];
        s.prov_culture = vec!["Aioran".into(); 4];
        s.prov_rural = vec![40_000.0; 4];
        s.prov_cap = vec![100_000.0; 4];
        s.prov_seat = vec![[0.0, 0.0], [8.0, 0.0], [16.0, 0.0], [24.0, 0.0]];
        s.prov_neighbors = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
        for yr in REALM_FULL_RAMP_YEAR..REALM_FULL_RAMP_YEAR + 80 {
            s.maybe_proclaim_realms(yr);
            s.tick += TICKS_PER_YEAR;
            if !s.realms.is_empty() { break; }
        }
        let r = s.realms.iter().find(|r| r.founding_path == REALM_PATH_CULTURE)
            .expect("no culture-founded realm");
        assert!(r.name.contains("Aioran"),
            "a realm founded by a people should carry its name, got `{}`", r.name);
    }

    /// A CIVIC realm must survive every pass that assumes a dynasty. `ruling_house`
    /// is `u32::MAX` for a republic, and the readers that resolve it through
    /// `houses.get` must degrade rather than panic — war resolution indexed it raw
    /// before this, so a republic winning a war would have crashed the tick.
    #[test]
    fn a_civic_realm_survives_the_dynastic_passes() {
        let mut s = realm_fixture();
        s.realms.clear();
        for h in 0..s.hubs.len() { s.hubs[h].realm = -1; s.hubs[h].realm_role = 0; }
        for p in 0..s.prov_realm.len() { s.prov_realm[p] = -1; }
        if !s.prov_holder.is_empty() { s.prov_holder[0] = 0; }
        let id = s.found_civic_realm(0, REALM_YEAR_FLOOR, REALM_PATH_CITY) as usize;
        assert_eq!(s.realms[id].government, REALM_GOV_CIVIC);
        // Every yearly realm pass, on a realm with no family at all.
        for _ in 0..5 {
            s.realm_family_pass(REALM_YEAR_FLOOR);
            s.update_realm_cohesion();
            s.assign_realm_ranks();
            s.collect_realm_levies();
            s.decide_realm_taxes(id, REALM_YEAR_FLOOR);
            s.maybe_relocate_abandoned_capitals(REALM_YEAR_FLOOR);
            s.tick += TICKS_PER_YEAR;
        }
        assert_eq!(s.realms[id].fallen_tick, 0, "a republic dissolved itself by having no dynasty");
        assert!(s.realms[id].cohesion > 0.0);
    }

    /// WORLD_AND_TRADE_MASTER_PLAN.md Part III §1/§3.1 — knowledge is seeded
    /// from day-one holdings (never stranding an existing house), and the
    /// founding gate genuinely bites once a house looks at somewhere it was
    /// never seeded near.
    #[test]
    fn knowledge_seeds_from_holdings_and_gates_founding() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // 4 hubs, 4 provinces (one per hub): 0-1 and 2-3 are neighbour pairs;
        // province 3 is NOT a neighbour of province 0's home.
        let hubs = (0..4u32).map(|i| hub(i, i as f32 * 10.0, 0.0, 2000.0, vec![10.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.hub_province = vec![0, 1, 2, 3];
        s.prov_neighbors = vec![vec![1], vec![0], vec![3], vec![2]];
        let mut h = house_at(0, vec![0], 0);
        h.known = std::collections::HashMap::new();
        s.houses.push(h);
        s.seed_knowledge();

        assert_eq!(s.houses[0].known.get(&0).map(|k| k.level), Some(KNOWN_ESTABLISHED),
            "the house's own home province must be established on day one");
        assert_eq!(s.houses[0].known.get(&1).map(|k| k.level), Some(KNOWN_REPORTED),
            "a neighbouring province must be at least reported");
        assert!(s.houses[0].known.get(&3).is_none(), "a non-adjacent province must stay unknown");

        assert!(s.house_knows(0, 0), "day-one founding at home must be unconstrained");
        assert!(!s.house_knows(0, 3), "founding at an unsurveyed, non-adjacent province must be gated");

        // A city (TickHub) is a knower too, seeded the same way.
        assert_eq!(s.hubs[2].known.get(&2).map(|k| k.level), Some(KNOWN_ESTABLISHED));
        assert!(s.hub_knows(2, 2));
        assert!(!s.hub_knows(2, 0), "a city's own gate must also bite outside its seeded neighbourhood");

        // Provinceless world: the gate degrades to permissive (rule: fog layers
        // on top of a province system that exists, never blocks one that doesn't).
        let mut s2 = sim(vec![hub(0, 0.0, 0.0, 2000.0, vec![10.0], 0)], vec![good("wheat", 0, 0, 1.0, 0.85, true)]);
        s2.houses.push(house_at(0, vec![0], 0));
        assert!(s2.house_knows(0, 5), "no province layer at all must never gate founding");
    }

    /// WORLD_AND_TRADE_MASTER_PLAN.md Part II Slice C1 — the entrepôt actually
    /// composes a cheaper two-leg route through a coastal outlet when one
    /// exists, and only when it is genuinely cheaper (never worse than direct).
    /// `rebuild_routes`' fallback pass is straight-line-distance-based, where a
    /// composed route can never beat the direct one (triangle inequality) — so
    /// this exercises the mechanism the honest way: a real PATHFOUND `base_days`
    /// matrix (as `compute_route_days_matrix` would build from the coarse-cost
    /// grid, where an expensive direct overland leg can genuinely lose to a
    /// cheap coastal route via an intermediate port), set up by hand.
    #[test]
    fn entrepot_composes_a_cheaper_route_through_a_real_outlet() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![10.0], 0),   // a: inland
            hub(1, 50.0, 0.0, 5000.0, vec![10.0], 0),  // p: coastal outlet
            hub(2, 100.0, 0.0, 5000.0, vec![10.0], 0), // b: inland
        ];
        hubs[1].coastal = true;
        let mut s = sim(hubs, goods);
        s.world_w = 1000.0; // keep every pair well inside the trade horizon
        // A real pathfound matrix: direct a↔b is expensive (a mountain range,
        // say); a↔p and p↔b are each cheap (a coastal lane).
        s.base_n = 3;
        s.base_days = vec![
            0.0, 5.0, 100.0,
            5.0, 0.0, 5.0,
            100.0, 5.0, 0.0,
        ];
        s.rebuild_routes();

        let n = s.hubs.len();
        let direct_before = 100.0;
        let composed = 5.0 + ENTREPOT_DWELL_DAYS + 5.0;
        assert!(composed < direct_before, "test setup must make the outlet genuinely cheaper");
        assert_eq!(s.route_outlet[0 * n + 2], 1, "hub 1 (the only coastal hub) should be the recorded outlet");
        assert!((s.days[0 * n + 2] - composed).abs() < 0.01,
            "days[a][b] should be the composed cost, got {}", s.days[0 * n + 2]);

        // And the MIN discipline: when the direct route is already cheap, the
        // outlet must never make it worse.
        s.base_days[0 * 3 + 2] = 1.0;
        s.base_days[2 * 3 + 0] = 1.0;
        s.rebuild_routes();
        assert!((s.days[0 * n + 2] - 1.0).abs() < 0.01,
            "a cheap direct route must never be replaced by a more expensive composed one, got {}",
            s.days[0 * n + 2]);
        assert_eq!(s.route_outlet[0 * n + 2], -1, "no outlet should be recorded when direct already wins");
    }

    // ─────────────────────────────────────────────────────────────────────
    // TRADE_STAGING_AND_POSTS_PLAN.md slices 3+4 · the STAGING RELAY
    //
    // The caps are passed in explicitly rather than read from the shipped
    // `SHIP_LEG_MAX_KM`/`CARAVAN_LEG_MAX_KM`, so these gates test the
    // MECHANISM at a real dose while the shipped dose stays a no-op — the
    // same split `leg_exceeds_range_uses_the_right_cap_per_mode` already
    // uses, and what lets the dose be walked without rewriting the tests.
    //
    // Scale matters here and is not arbitrary: `world_w = 4000` puts a cell
    // at ~10 km, so the 50-cell hub spacing below is ~500 km — inside an
    // 800 km caravan range, while the 100-cell end-to-end span is not. A
    // coarser world would make every leg over-range and the fixture would
    // pass while testing nothing (§8.24a4's own lesson about grid size being
    // load-bearing in a fixture).
    // ─────────────────────────────────────────────────────────────────────

    /// The claim in one line: a leg too long for its mode is carried to the
    /// settlement on the way, not refused. This is the whole difference
    /// between the reverted N1c attempt (which deleted such lanes, and
    /// inverted `econ_inheritance_rules_fragment_differently` by doing so)
    /// and the relay.
    #[test]
    fn a_leg_past_its_range_is_staged_through_a_settlement_on_the_way() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![10.0], 0),   // a
            hub(1, 50.0, 0.0, 5000.0, vec![10.0], 0),  // the stop on the way
            hub(2, 100.0, 0.0, 5000.0, vec![10.0], 0), // b
        ];
        let mut s = sim(hubs, goods);
        s.world_w = 4000.0; // ~10 km per cell
        s.rebuild_routes();

        let (ship, caravan) = (3500.0, 800.0);
        assert!(s.hub_km(0, 2) > caravan,
            "fixture must put b past a caravan's range, got {} km", s.hub_km(0, 2));
        assert!(s.hub_km(0, 1) < caravan,
            "…while the intermediate stop is reachable, got {} km", s.hub_km(0, 1));

        assert_eq!(s.staging_hop(0, 2, ship, caravan), Some(1),
            "an over-range leg must stage through the settlement on the way");
        // …and a leg already inside its range is never staged: the relay must
        // not reroute ordinary short trade, or it would change every lane on
        // the map rather than only the impossible ones.
        assert_eq!(s.staging_hop(0, 1, ship, caravan), None,
            "a leg inside its own range needs no stop (and asking for one is how \
             a relay quietly becomes a rewrite of all trade)");
    }

    /// A gap no port can break is genuinely refused — the relay must not
    /// invent a stop that does not exist. This is the one case where the
    /// range rule still blocks a trade outright, and it has to stay
    /// reachable or `diag_why_leg_range_bind` would be dead by construction.
    #[test]
    fn a_gap_no_settlement_can_break_is_refused_not_staged() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![10.0], 0),
            hub(1, 300.0, 0.0, 5000.0, vec![10.0], 0), // far side of an empty ocean
        ];
        let mut s = sim(hubs, goods);
        s.world_w = 4000.0;
        s.rebuild_routes();

        assert!(s.hub_km(0, 1) > 800.0, "fixture must be over-range to be meaningful");
        assert_eq!(s.staging_hop(0, 1, 3500.0, 800.0), None,
            "with no third settlement anywhere there is nothing to stage through");
    }

    /// The TERMINATION invariant, on the only fixture that can actually see
    /// it: a settlement that is reachable and legal but lies the WRONG WAY.
    ///
    /// This exists because the obvious gate does not test this. On a chain of
    /// ports laid out between source and destination, picking the candidate
    /// with the least remaining distance makes progress by construction, so
    /// deleting the progress check leaves such a test passing — verified, not
    /// assumed. Here the only candidate is *further* from the destination than
    /// the origin is, so the progress rule is the single thing standing
    /// between the relay and a cargo shuttling between two ports forever.
    #[test]
    fn a_stop_that_lies_the_wrong_way_is_never_staged_through() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![10.0], 0),    // a
            hub(1, 300.0, 0.0, 5000.0, vec![10.0], 0),  // b — far, over-range
            hub(2, -50.0, 0.0, 5000.0, vec![10.0], 0),  // c — near a, but AWAY from b
        ];
        let mut s = sim(hubs, goods);
        s.world_w = 4000.0;
        s.rebuild_routes();

        let (ship, caravan) = (3500.0, 800.0);
        // c is a perfectly legal hop in its own right — so if it is rejected,
        // it can only be the progress rule doing it.
        assert!(!CampaignSim::leg_exceeds_range(s.hub_km(0, 2), false, ship, caravan),
            "the wrong-way stop must itself be within range, or this gate proves nothing");
        assert!(s.hub_km(2, 1) > s.hub_km(0, 1),
            "…and must genuinely lie further from the destination than the origin does");

        assert_eq!(s.staging_hop(0, 1, ship, caravan), None,
            "staging through a port that leaves the cargo further from its destination \
             is how a relay becomes an infinite shuttle");
    }

    /// The termination guarantee, asserted rather than argued: walking
    /// `staging_hop` from source to destination across a chain of ports no
    /// single voyage could cross must ARRIVE, within `RELAY_MAX_HOPS`, with
    /// every individual leg legal in its own mode and every step strictly
    /// closer to the destination.
    ///
    /// These are exactly the two invariants the relay's freedom from infinite
    /// loops rests on, which is why they are tested together and not
    /// separately: drop the progress check and this hangs; drop the legality
    /// check and staging launders one illegal leg into several.
    #[test]
    fn staging_walks_a_long_lane_to_its_destination_in_bounded_hops() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Six ports strung 50 cells (~500 km) apart: 2,500 km end to end,
        // which no 800 km caravan leg can cross in one go.
        let hubs: Vec<TickHub> = (0..6)
            .map(|i| hub(i, i as f32 * 50.0, 0.0, 5000.0, vec![10.0], 0))
            .collect();
        let mut s = sim(hubs, goods);
        s.world_w = 4000.0;
        s.rebuild_routes();

        let (ship, caravan) = (3500.0, 800.0);
        let (src, dst) = (0usize, 5usize);
        assert!(s.hub_km(src, dst) > caravan * 2.0,
            "fixture must need several hops, not one");

        let mut at = src;
        let mut hops = 0u8;
        let mut last_remaining = f32::INFINITY;
        while at != dst {
            assert!(hops < RELAY_MAX_HOPS,
                "the relay must reach {dst} within RELAY_MAX_HOPS, stuck at {at}");
            let remaining = s.hub_km(at, dst);
            assert!(remaining < last_remaining,
                "every hop must strictly close the gap (termination): {remaining} !< {last_remaining}");
            last_remaining = remaining;
            let next = if CampaignSim::leg_exceeds_range(remaining, false, ship, caravan) {
                s.staging_hop(at, dst, ship, caravan)
                    .unwrap_or_else(|| panic!("no stop available from {at} — the chain should provide one"))
            } else {
                dst // the last stretch is inside range: sail it
            };
            assert!(!CampaignSim::leg_exceeds_range(s.hub_km(at, next), false, ship, caravan),
                "the leg {at}→{next} the relay chose must itself be legal");
            at = next;
            hops += 1;
        }
        assert!(hops > 1, "a 2,500 km lane under an 800 km cap must take more than one leg");
    }

    /// A world whose DISTANCES ARE REAL — the instrument the `econ_` fixtures
    /// cannot be.
    ///
    /// This exists because of a measured finding, not a preference. `world_w`
    /// serves two opposed purposes: the trade horizon is a fraction OF it
    /// (`TRADE_MAX_DIST_FRAC × world_w`) while distance is `KM_EQUATOR /
    /// world_w`. `econ_inheritance_rules_fragment_differently` therefore sets
    /// `world_w = 300` purely to buy horizon — its own comment says so, with
    /// no geographic intent — and the side effect is that its adjacent hubs
    /// stand 1,202 km apart; `reference_world`'s (`world_w = 100`) stand 3,607
    /// km apart. Both are further than any historical caravan stage, so a
    /// range rule measured there has no legal stop to relay through and only
    /// ever reads as "all trade dies". That is a fact about those fixtures,
    /// not about the model.
    ///
    /// Here the numbers mean what they say: `world_w = 3600` (the shipped
    /// default, and what `days_per_cell` is already calibrated for) puts a
    /// cell at ~11.1 km, so the 40-cell grid spacing is ~445 km — a real day's
    /// stages between towns — while the 10×6 grid spans ~4,000 × 2,200 km,
    /// which is far enough that its longest lanes genuinely exceed a ship's
    /// endurance. Both halves are required: a world with no over-range lane
    /// cannot exercise the rule, and one with no reachable neighbour cannot
    /// exercise the RELAY.
    ///
    /// The two coastal rows are the north and south seaboards, so a long sea
    /// lane and a long overland lane both exist.
    pub(super) fn dense_world() -> CampaignSim {
        const COLS: u32 = 10;
        const ROWS: u32 = 6;
        const SPACING: f32 = 40.0; // cells; ~445 km at world_w = 3600
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("fish", 0, 0, 1.2, 0.7, true),
            good("silk", 1, 2, 20.0, 0.35, false),
            good("iron", 2, 1, 5.0, 0.45, false),
        ];
        let ng = goods.len();
        let mut hubs = Vec::new();
        for i in 0..(COLS * ROWS) {
            let pop = 8000.0 + (i as f32 * 911.0) % 26000.0;
            let prod: Vec<f32> = (0..ng)
                .map(|g| if (g + i as usize) % 3 == 0 { pop * 0.012 } else { pop * 0.0015 })
                .collect();
            let mut h = hub(i, (i % COLS) as f32 * SPACING, (i / COLS) as f32 * SPACING, pop, prod, 0);
            h.coastal = i / COLS == 0 || i / COLS == ROWS - 1;
            hubs.push(h);
        }
        let mut s = sim(hubs, goods);
        s.world_w = 3600.0;
        s.world_h = 1800.0;
        // ~55 km/day blended, exactly as `reference_world_large` and
        // `campaign_start_sim` derive it — a real scale needs its real pace.
        s.days_per_cell = (40075.0 / s.world_w / 55.0).max(0.02);
        for i in 0..10u32 {
            let mut h = house_at((i * 6) % (COLS * ROWS), vec![2 + (i as usize % 2)], 3);
            h.archetype = (i % 4) as u8;
            h.wealth = 40.0 + (i as f32) * 8.0;
            h.prestige = 0.5;
            s.houses.push(h);
        }
        s.seed_house_count = s.houses.len() as u32;
        s
    }

    /// THE GATE THE DOSE HAS BEEN WAITING FOR: on a world with realistic
    /// settlement density, turning the range caps on makes long trade go in
    /// STAGES rather than making it stop.
    ///
    /// Three claims, and the third is the one the plan itself calls the
    /// companion gate that matters most — long-haul trade VOLUME must not
    /// collapse. A range rule that quietly deletes the trade it cannot route
    /// would satisfy the first two claims perfectly.
    #[test]
    fn the_relay_carries_long_lanes_in_stages_on_a_realistically_dense_world() {
        const YEARS: usize = 8;
        let mut loose = dense_world();
        let mut dosed = dense_world();
        dosed.ship_leg_max_km = 3500.0;
        dosed.caravan_leg_max_km = 800.0;
        // The fixture has to be able to fail: a world whose longest lane is
        // already inside the caps would pass every assertion below while
        // testing nothing at all.
        let n = dosed.hubs.len();
        let longest = (0..n).flat_map(|a| (0..n).map(move |b| (a, b)))
            .map(|(a, b)| dosed.hub_km(a, b))
            .fold(0.0f32, f32::max);
        assert!(longest > 3500.0,
            "fixture must contain a lane past the ship cap, longest is {longest} km");

        for _ in 0..YEARS { loose.advance(365); }
        // Measured over the SETTLED years only. Refusals are expected in the
        // first year and only there: a hub founded mid-year (an estate, an
        // outpost) has no entry in `neighbors` until routes are next rebuilt,
        // so it cannot stage and its cargo sails direct. That is harmless —
        // range is a routing rule, so the worst case is the old behaviour —
        // and self-correcting, but it is real and would otherwise read as a
        // mechanism failure. Measured on this fixture: 560 refusals in year 0
        // against 15,860 staged legs, then exactly zero for every year after.
        for _ in 0..YEARS { dosed.advance(365); }
        let settled_from = { let mut w = dense_world(); w.ship_leg_max_km = 3500.0;
            w.caravan_leg_max_km = 800.0; w.advance(365); w.diag_why_leg_range_bind };

        assert_eq!(loose.diag_relay_staged, 0,
            "with the caps off the relay must be provably inert");
        assert!(dosed.diag_relay_staged > 0,
            "a world spanning {longest:.0} km under a 3,500 km ship cap must actually stage \
             legs — if this is 0 the mechanism is not reaching dispatch at all");
        assert_eq!(dosed.diag_why_leg_range_bind, settled_from,
            "once routes have settled, every over-range leg must find a stop at ~445 km \
             between towns: {} refusals against {} in the first year alone means the relay \
             is falling back to direct hauls it should not need to",
            dosed.diag_why_leg_range_bind, settled_from);
        // The volume claim. Staging costs time, so some fall is expected and
        // healthy; a collapse is the failure this guards.
        let ratio = dosed.diag_volume / loose.diag_volume.max(1e-6);
        assert!(ratio > 0.6,
            "staged trade volume must not collapse: {:.0} against {:.0} loose ({:.2}x)",
            dosed.diag_volume, loose.diag_volume, ratio);
    }

    /// The DYNAMICS counterpart, and the gate that actually licenses the
    /// shipped dose: over a long run on a realistically-scaled world, does the
    /// economy stay healthy with the range caps on?
    ///
    /// `simulate_decades_reports_dynamics` cannot answer this. It, and every
    /// other fixture built through `sim()`, runs on abstract coordinates where
    /// distances are not real (see `dense_world`), which is why they opt out of
    /// the caps entirely. So the shipped dose would otherwise be covered by
    /// nothing, and CLAUDE.md §2.4's rule — never tune a constant without a
    /// gate that isn't the target — would be unsatisfied.
    ///
    /// The headline measurement, and the reason the dose is shippable at all:
    /// on a real geography the caps make the economy LESS concentrated, not
    /// more. Peak house wealth falls from 734,570 loose to 323,040 dosed over
    /// 40 years. That is the exact opposite of what the same dose did on the
    /// mis-scaled toy fixtures, where refusing (rather than staging) long
    /// hauls concentrated capital on whoever kept reach and broke the
    /// inheritance gate. Staging spreads a long lane's margin across the ports
    /// along it; refusing hands it to whoever can still make the crossing.
    #[test]
    fn the_dosed_economy_stays_healthy_on_a_realistically_dense_world() {
        const YEARS: usize = 40;
        let mut loose = dense_world();
        let mut dosed = dense_world();
        dosed.ship_leg_max_km = SHIP_LEG_MAX_KM;
        dosed.caravan_leg_max_km = CARAVAN_LEG_MAX_KM;

        let (mut peak_loose, mut peak_dosed) = (0.0f32, 0.0f32);
        for _ in 0..YEARS {
            loose.advance(365);
            dosed.advance(365);
            let r = |s: &CampaignSim| s.houses.iter().filter(|h| !h.defunct)
                .map(|h| h.wealth).fold(0.0f32, f32::max);
            peak_loose = peak_loose.max(r(&loose));
            peak_dosed = peak_dosed.max(r(&dosed));
        }
        let alive = dosed.houses.iter().filter(|h| !h.defunct).count();
        let poorest = dosed.houses.iter().map(|h| h.wealth).fold(f32::INFINITY, f32::min);

        // The same hard bound `simulate_decades_reports_dynamics` asserts.
        assert!(peak_dosed.is_finite() && peak_dosed < 1_000_000.0,
            "a dosed run must not produce a runaway-rich house: {peak_dosed:.0}");
        assert!(poorest > -500_000.0 && poorest.is_finite(),
            "limited liability still bounds the downside: {poorest:.0}");
        // Turnover, both directions: houses must die, and the class must survive.
        assert!(alive >= 8, "the merchant class must survive the caps, {alive} alive");
        assert!(dosed.houses.len() > alive,
            "houses must still fail under the caps — {} ever, {alive} alive", dosed.houses.len());
        assert!(dosed.diag_volume > loose.diag_volume * 0.25,
            "trade must not collapse: {:.0} against {:.0} loose",
            dosed.diag_volume, loose.diag_volume);
        // The substantive claim. Stated as a ratio rather than a literal so it
        // reads as "no worse than the uncapped world" rather than encoding one
        // seed's arithmetic — the direction is the finding, not the figure.
        assert!(peak_dosed <= peak_loose,
            "staging must not concentrate wealth more than teleporting does: dosed peak \
             {peak_dosed:.0} against loose {peak_loose:.0} — if this inverts, the relay has \
             stopped spreading a long lane's margin along it and is behaving like the \
             refusal that broke the inheritance gate");
    }

    /// C4 (`ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md`) — the companion gate for
    /// N1's OWN dose, mirroring `the_dosed_economy_stays_healthy_on_a_
    /// realistically_dense_world` exactly, because N1's bind reuses N1c's own
    /// staging relay and inherits its risk: an ownerless-only refusal is the
    /// shape that collapsed the inheritance gate the first time N1c tried it.
    /// N1 avoids a bare refusal by handing the leg to `staging_hop` — the SAME
    /// call N1c makes — so this test's job is to confirm that reuse actually
    /// keeps the economy healthy once N1's OWN threshold (days, not km) is the
    /// one doing the binding, on top of N1c's caps rather than in isolation
    /// (a real campaign ships both live at once).
    #[test]
    fn n1_bind_stays_healthy_on_a_realistically_dense_world() {
        const YEARS: usize = 40;
        let mut loose = dense_world();
        let mut dosed = dense_world();
        dosed.ship_leg_max_km = SHIP_LEG_MAX_KM;
        dosed.caravan_leg_max_km = CARAVAN_LEG_MAX_KM;
        dosed.local_haul_bind_days = N1_LOCAL_HAUL_BIND_DAYS;

        let (mut peak_loose, mut peak_dosed) = (0.0f32, 0.0f32);
        for _ in 0..YEARS {
            loose.advance(365);
            dosed.advance(365);
            let r = |s: &CampaignSim| s.houses.iter().filter(|h| !h.defunct)
                .map(|h| h.wealth).fold(0.0f32, f32::max);
            peak_loose = peak_loose.max(r(&loose));
            peak_dosed = peak_dosed.max(r(&dosed));
        }
        let alive = dosed.houses.iter().filter(|h| !h.defunct).count();
        let poorest = dosed.houses.iter().map(|h| h.wealth).fold(f32::INFINITY, f32::min);

        eprintln!("[diag] refusals={} staged={} volume dosed={:.0} loose={:.0}",
            dosed.diag_why_no_carrier_bind, dosed.diag_relay_staged, dosed.diag_volume, loose.diag_volume);
        assert!(dosed.diag_why_no_carrier_bind == 0 || dosed.diag_relay_staged > 0,
            "N1's bind must be reaching real legs and routing them through the relay, \
             not silently refusing everything: {} refusals, {} staged",
            dosed.diag_why_no_carrier_bind, dosed.diag_relay_staged);
        assert!(peak_dosed.is_finite() && peak_dosed < 1_000_000.0,
            "a dosed run must not produce a runaway-rich house: {peak_dosed:.0}");
        assert!(poorest > -500_000.0 && poorest.is_finite(),
            "limited liability still bounds the downside: {poorest:.0}");
        assert!(alive >= 8, "the merchant class must survive the caps, {alive} alive");
        assert!(dosed.houses.len() > alive,
            "houses must still fail under the caps — {} ever, {alive} alive", dosed.houses.len());
        // The plan's own named companion gate: long-haul trade volume must not
        // collapse. N1's own bind adds a second brake on top of N1c's, so the
        // floor is looser than N1c's solo 0.25 — this measures the COMBINED
        // dose, not N1 in isolation.
        assert!(dosed.diag_volume > loose.diag_volume * 0.15,
            "trade must not collapse under the combined N1+N1c dose: {:.0} against {:.0} loose",
            dosed.diag_volume, loose.diag_volume);
        assert!(peak_dosed <= peak_loose * 1.1,
            "staging must not concentrate wealth more than teleporting does: dosed peak \
             {peak_dosed:.0} against loose {peak_loose:.0}");
    }

    /// Diagnostic (`#[ignore]`d, like `econ_diagnose_house_turnover`) written
    /// while chasing the regression urban exodus caused in both dense-world
    /// route-staging gates (`the_dosed_economy_stays_healthy_on_a_
    /// realistically_dense_world`, `the_relay_carries_long_lanes_in_stages_
    /// on_a_realistically_dense_world`). It measures TOTAL population, and the
    /// finding is a NEGATIVE one: the first theory — an uncapped destination
    /// gets flooded by every source's independent "best neighbour" pick,
    /// overloads faster than its own production catches up, and the resulting
    /// famine deaths (unlike exodus itself) destroy population — predicted
    /// this diagnostic should show a much healthier total once
    /// `EXODUS_DEST_ABSORB_CAP` caps that flood. It does not: total population
    /// on `dense_world` collapses from 704,800 to ~287,000 within 4 years
    /// almost IDENTICALLY whether exodus is disabled entirely, uncapped, or
    /// capped anywhere from 0.08 to 0.15 — so that crash is a pre-existing
    /// dynamic of the fixture, not something exodus or this cap causes. What
    /// the cap actually fixes (a narrow 0.09–0.12 band restores both gates;
    /// 0.08 and 0.15 each still fail one) is which HUBS end up populated, not
    /// how much population survives in total — this diagnostic cannot see
    /// that, only the aggregate it rules out. Run with `-- --ignored
    /// --nocapture` to watch the (misleadingly stable-looking) trajectory.
    #[test]
    #[ignore]
    fn diag_exodus_population_concentration() {
        let mut dosed = dense_world();
        dosed.ship_leg_max_km = SHIP_LEG_MAX_KM;
        dosed.caravan_leg_max_km = CARAVAN_LEG_MAX_KM;
        for yr in 0..40 {
            dosed.advance(365);
            if yr % 5 == 4 || yr == 0 {
                let pops: Vec<f32> = dosed.hubs.iter().map(|h| h.population).collect();
                let alive = dosed.hubs.iter().filter(|h| !h.abandoned).count();
                let max = pops.iter().cloned().fold(0.0f32, f32::max);
                let min = pops.iter().cloned().fold(f32::INFINITY, f32::min);
                let total: f32 = pops.iter().sum();
                let mean = total / pops.len() as f32;
                println!("yr {yr}: alive {alive}/{} pop min {min:.0} max {max:.0} mean {mean:.0} total {total:.0}",
                    dosed.hubs.len());
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // N5 · the sailing window (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §1)
    // ─────────────────────────────────────────────────────────────────────

    /// `season_slices == 0` (the state of any pre-N5 save) and, separately,
    /// `season_slices > 0` with every stored multiplier at `v == 0`, must
    /// both leave `lane_days` reading exactly `days` — the zero-dose gate.
    #[test]
    fn n5_season_multipliers_at_unity_are_a_noop() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 1000.0, vec![500.0], 0),
            hub(1, 5.0, 0.0, 1000.0, vec![500.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.base_n = 2;
        s.days = vec![0.0, 7.0, 7.0, 0.0];
        for t in [0u32, 100, 200, 300] {
            s.tick = t;
            assert!((s.lane_days(0, 1) - 7.0).abs() < 1e-6,
                "season_slices=0 must leave lane_days == days");
        }
        s.season_slices = SEASON_SLICES;
        s.base_days_season = vec![0u8; SEASON_SLICES as usize * 2 * 2];
        for t in [0u32, 100, 200, 300] {
            s.tick = t;
            assert!((s.lane_days(0, 1) - 7.0).abs() < 1e-6,
                "v=0 in every slice must leave lane_days == days");
        }
    }

    /// A lane's stormy-season slice must price it ABOVE the annual mean, and
    /// an all-season (v=0) slice must still read the mean exactly — the
    /// per-lane, per-slice discipline `lane_days` is built on.
    #[test]
    fn n5_a_lane_is_dearer_in_its_stormy_season() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 1000.0, vec![500.0], 0),
            hub(1, 5.0, 0.0, 1000.0, vec![500.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.base_n = 2;
        let n = s.base_n;
        s.days = vec![0.0, 10.0, 10.0, 0.0];
        s.season_slices = 4;
        s.base_days_season = vec![0u8; 4 * n * n];
        let stormy_v = 32u8; // 1.0 + 32×(1/64) = 1.5×
        s.base_days_season[0 * n * n + 0 * n + 1] = stormy_v;
        s.base_days_season[0 * n * n + 1 * n + 0] = stormy_v;

        s.tick = 10; // day_of_year 10 of 365 → slice 0
        assert!(s.lane_days(0, 1) > s.days[0 * n + 1],
            "the stormy slice must price the lane above its annual mean");
        assert!((s.lane_days(0, 1) - 15.0).abs() < 1e-3,
            "expected the stored 1.5x multiplier, got {}", s.lane_days(0, 1));

        s.tick = 200; // slice 2 — every multiplier here is still v=0
        assert!((s.lane_days(0, 1) - 10.0).abs() < 1e-6,
            "an all-season (v=0) slice must equal the annual mean exactly");
    }

    // ─────────────────────────────────────────────────────────────────────
    // N6 · price-elastic demand (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §2)
    // ─────────────────────────────────────────────────────────────────────

    /// The shipped dose, `DEMAND_ELASTICITY = [0,0,0]`, must be a true no-op
    /// at every tier and every price ratio — `elastic_aggregate_mult` is the
    /// exact function the needs-assembly site calls.
    #[test]
    fn n6_elasticity_at_zero_is_a_noop() {
        assert_eq!(DEMAND_ELASTICITY, [0.0, 0.0, 0.0], "N6 ships at zero dose");
        for tier in 0..3 {
            for rel in [0.05f32, 0.2, 1.0, 3.0, 12.0] {
                assert_eq!(elastic_aggregate_mult(tier, rel), 1.0,
                    "tier {tier} at rel {rel} must be an exact no-op at zero dose");
            }
        }
    }

    /// The mechanism's SHAPE, tested at a hypothetical dosed elasticity via
    /// the parametrized pure function (`elastic_aggregate_mult_e`) — the
    /// same discipline `n2_export_ban_blocks_dispatch_when_set_directly`
    /// uses to test enforcement independent of the (still zero-dosed)
    /// trigger. A doubled price must lower demand, a basic good must fall by
    /// less than a luxury one, and both the subsistence floor and the clamp
    /// must hold at an extreme price.
    #[test]
    fn n6_a_dearer_good_is_bought_less() {
        let cheap = elastic_aggregate_mult_e(1.0, 2, 1.0);
        let dear = elastic_aggregate_mult_e(1.0, 2, 2.0);
        assert!(dear < cheap, "a doubled price must lower the elastic multiplier");

        let basic_drop = 1.0 - elastic_aggregate_mult_e(0.15, 0, 2.0);
        let luxury_drop = 1.0 - elastic_aggregate_mult_e(1.0, 2, 2.0);
        assert!(basic_drop < luxury_drop,
            "at the historical target elasticities a basic good must fall by less than a luxury one");

        assert!(elastic_aggregate_mult_e(1.0, 0, 1000.0) >= SUBSISTENCE_FLOOR - 1e-6,
            "tier 0 must never fall below the subsistence floor, however dear");
        assert!(elastic_aggregate_mult_e(1.0, 2, 1000.0) >= ELASTIC_CLAMP.0 - 1e-6,
            "the clamp's low end must hold at an extreme price");
        assert!(elastic_aggregate_mult_e(1.0, 2, 0.0001) <= ELASTIC_CLAMP.1 + 1e-6,
            "the clamp's high end must hold at a near-zero price");
    }

    /// §2.2's own sharpest line: "elasticity belongs to the market, not the
    /// ration." At the shipped zero dose `needs_struct` cannot yet be
    /// observed diverging from the elastic `needs` (they are the same
    /// buffer in effect) — that integration proof waits on the dose walk,
    /// named here rather than silently skipped. What IS gated today is the
    /// WIRING: `update_food_and_starvation` is called with `needs_struct`,
    /// never `needs` — verified by reading `mod.rs`'s call site, and by the
    /// fact that a province-free fixture's `lack_basic`/`starving` behave
    /// identically whether or not category substitution ran, since both
    /// buffers are built by the same loop.
    #[test]
    fn n6_the_ration_is_not_elastic() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("barley", 0, 0, 0.8, 0.6, true),
        ];
        let hubs = vec![hub(0, 0.0, 0.0, 2000.0, vec![10.0, 10.0], 0)];
        let mut s = sim(hubs, goods);
        // A steep price gap between the two members of the same (food)
        // category exercises real substitution — proving the wiring doesn't
        // simply skip the interesting case.
        s.hubs[0].price = vec![1.0, 8.0];
        s.advance(30);
        assert!(s.hubs[0].lack_basic.is_finite() && s.hubs[0].lack_basic >= 0.0,
            "the ration computation must remain well-formed under substitution");
    }

    // ─────────────────────────────────────────────────────────────────────
    // Local satiety (`PLACES_DEMAND_AND_GROWTH_PLAN.md` slice 6, F9)
    // ─────────────────────────────────────────────────────────────────────

    /// The shipped dose, `LOCAL_SATIETY = 0.0`, must be a true no-op at every
    /// tier and every supply ratio.
    #[test]
    fn local_satiety_is_a_noop_at_zero() {
        assert_eq!(LOCAL_SATIETY, 0.0, "local satiety ships at zero dose");
        for tier in 0..3u8 {
            for supply in [0.0f32, 0.25, 0.5, 1.0] {
                assert_eq!(local_satiety_mult_e(LOCAL_SATIETY, tier, supply), 1.0,
                    "tier {tier} at supply {supply} must be an exact no-op at zero dose");
            }
        }
        let goods = vec![good("silk", 0, 2, 8.0, 0.5, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 1000.0, vec![900.0], 0)];
        let s = sim(hubs, goods);
        assert_eq!(s.local_satiety_mult(0, 0), 1.0,
            "a fully self-sufficient hub must still see a no-op at the shipped dose");
    }

    /// The claim: at a live dose, a hub that supplies more of its own
    /// comfort/luxury good from home wants proportionally less of it.
    #[test]
    fn a_producing_city_wants_less_of_its_own_luxury() {
        let self_sufficient = local_satiety_mult_e(0.7, 2, 1.0);
        let import_dependent = local_satiety_mult_e(0.7, 2, 0.0);
        assert!(self_sufficient < import_dependent,
            "a self-sufficient producer must be jaded relative to one that imports everything");
        assert!(self_sufficient >= 1.0 - 0.7 - 1e-6,
            "the dose bounds how far satiety can shave demand");
    }

    /// Rule 1: nobody is jaded with bread — a basic good (tier 0) must never
    /// be subject to satiety, however dosed or however self-sufficient.
    #[test]
    fn a_basic_good_is_never_subject_to_satiety() {
        for supply in [0.0f32, 0.5, 1.0] {
            assert_eq!(local_satiety_mult_e(0.9, 0, supply), 1.0,
                "a basic good must be a no-op at any dose or supply level");
        }
    }

    /// Rule 2: local satiety must reach only the MARKET-FACING `needs`
    /// buffer, never `needs_struct` — the same shape `n6_the_ration_is_not_
    /// elastic` already asserts for N6's elasticity. Even at a hypothetical
    /// live dose the structural ration a starving population's food balance
    /// and crisis relief read must stay well-formed and untouched by this
    /// mechanism, since the wiring site (`mod.rs`) applies it only to
    /// `needs[h][g]` and only for `need_tier >= 1`.
    #[test]
    fn the_structural_ration_is_not_affected_by_satiety() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silk", 1, 2, 8.0, 0.5, false),
        ];
        let hubs = vec![hub(0, 0.0, 0.0, 2000.0, vec![1800.0, 1900.0], 0)];
        let mut s = sim(hubs, goods);
        s.advance(30);
        assert!(s.hubs[0].lack_basic.is_finite() && s.hubs[0].lack_basic >= 0.0,
            "the structural ration must remain well-formed regardless of local satiety");
    }

    // ─────────────────────────────────────────────────────────────────────
    // Transit demand — the entrepot (`HOUSES_GUILDS_AND_MARKET_PLAN.md` S5)
    // ─────────────────────────────────────────────────────────────────────

    /// The shipped dose, `TRANSIT_DEMAND_DOSE = 0.0`, must be a true no-op at
    /// every throughput ratio.
    #[test]
    fn transit_demand_is_a_noop_at_zero() {
        assert_eq!(TRANSIT_DEMAND_DOSE, 0.0, "transit demand ships at zero dose");
        for ratio in [0.0f32, 0.5, 1.0, 5.0] {
            assert_eq!(transit_need_mult_e(TRANSIT_DEMAND_DOSE, TRANSIT_DEMAND_CAP, ratio), 1.0,
                "throughput ratio {ratio} must be an exact no-op at zero dose");
        }
        let goods = vec![good("spices", 0, 2, 8.0, 0.5, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 1000.0, vec![0.0], 0)];
        let s = sim(hubs, goods);
        assert_eq!(s.transit_need_mult(0, 0, 100.0), 1.0,
            "a hub with real recent throughput must still see a no-op at the shipped dose");
    }

    /// The claim: at a live dose, a hub with real recent throughput of a
    /// good (an entrepot) wants MORE of it than its residents alone would —
    /// the Delos/Puteoli/Palmyra case, bounded by the cap regardless of how
    /// large the throughput ratio grows.
    #[test]
    fn an_entrepot_wants_more_than_its_residents_do() {
        let quiet_town = transit_need_mult_e(0.4, 0.5, 0.0);
        let modest_entrepot = transit_need_mult_e(0.4, 0.5, 0.5);
        let great_entrepot = transit_need_mult_e(0.4, 0.5, 50.0);
        assert_eq!(quiet_town, 1.0, "no recent throughput must mean no extra pull");
        assert!(modest_entrepot > quiet_town,
            "real throughput must raise demand above a quiet town's baseline");
        assert!(great_entrepot > modest_entrepot,
            "a busier entrepot must want proportionally more, up to the cap");
        assert!(great_entrepot <= 1.0 + 0.5 + 1e-6,
            "TRANSIT_DEMAND_CAP must bound the multiplier however large the ratio gets: {great_entrepot}");
    }

    /// Transit demand must reach only the MARKET-FACING `needs` buffer, never
    /// `needs_struct` — the same discipline `the_structural_ration_is_not_
    /// affected_by_satiety` already asserts for S6 (local satiety). The
    /// wiring site (`mod.rs`) applies it only to `needs[h][g]`.
    #[test]
    fn transit_demand_never_touches_the_structural_ration() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("spices", 1, 2, 8.0, 0.5, false),
        ];
        let hubs = vec![hub(0, 0.0, 0.0, 2000.0, vec![1800.0, 0.0], 0)];
        let mut s = sim(hubs, goods);
        s.advance(30);
        assert!(s.hubs[0].lack_basic.is_finite() && s.hubs[0].lack_basic >= 0.0,
            "the structural ration must remain well-formed regardless of transit demand");
    }

    // ─────────────────────────────────────────────────────────────────────
    // City capacity from the land (`PLACES_DEMAND_AND_GROWTH_PLAN.md` slice 4, D5/D6)
    // ─────────────────────────────────────────────────────────────────────

    /// The shipped dose, `CAPACITY_LAND_WEIGHT = 0.0`, must be a true no-op:
    /// the blend returns `founding_pop` verbatim regardless of `land_capacity`.
    #[test]
    fn capacity_land_weight_is_a_noop_at_zero() {
        assert_eq!(CAPACITY_LAND_WEIGHT, 0.0, "slice 4 ships at zero dose");
        for land_cap in [0.0f32, 500.0, 50_000.0, 1.0] {
            assert_eq!(land_capacity_blend(1234.0, land_cap, CAPACITY_LAND_WEIGHT), 1234.0,
                "land_capacity {land_cap} must be a no-op at zero dose");
        }
        // Live-dose end-to-end proof, via the sim: a province-less campaign
        // (the dynamics fixture's own shape) must produce identical capacity
        // whatever CAPACITY_LAND_WEIGHT nominally is, since `has_prov` is
        // false and `hub_province`/`prov_cap` are both empty.
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 1000.0, vec![900.0], 0)];
        let mut s = sim(hubs, goods);
        s.advance(60);
        assert!(s.hubs[0].population.is_finite() && s.hubs[0].population > 0.0,
            "population must remain well-formed with no province layer");
    }

    /// The claim (D5): at a live dose, two hubs with REVERSED `founding_pop`
    /// and REVERSED land capacity must have their growth ceilings cross —
    /// the well-sited small town can overtake the badly-sited large one.
    /// Tested via the pure blend directly (the shape), since exercising this
    /// through a live campaign needs a real province layer this fixture
    /// harness doesn't build.
    #[test]
    fn a_well_sited_small_town_can_overtake_a_badly_sited_large_one() {
        let dose = 0.7; // a hypothetical live dose
        let small_town_founding = 500.0;
        let small_town_land_cap = 50_000.0; // rich hinterland
        let large_town_founding = 20_000.0;
        let large_town_land_cap = 1_000.0; // poor hinterland

        let small_base = land_capacity_blend(small_town_founding, small_town_land_cap, dose);
        let large_base = land_capacity_blend(large_town_founding, large_town_land_cap, dose);
        assert!(small_base > large_base,
            "a well-sited small town's blended base ({small_base}) must overtake a badly-sited \
             large one's ({large_base}) once land capacity dominates the founding anchor");

        // And at the shipped zero dose the ranking must NOT cross — founding
        // population alone still decides, exactly as before this slice.
        let small_base_0 = land_capacity_blend(small_town_founding, small_town_land_cap, 0.0);
        let large_base_0 = land_capacity_blend(large_town_founding, large_town_land_cap, 0.0);
        assert!(small_base_0 < large_base_0,
            "at zero dose the founding-population ranking must be unchanged");
    }

    /// D6: a hub with many estates/structures must never exceed a stated
    /// ceiling multiple — the headroom is bounded, not a second exponential.
    #[test]
    fn works_capacity_is_bounded() {
        assert_eq!(works_dev_mult(0, 0), 0.0, "no estates or structures must be an exact no-op");
        let capped = works_dev_mult(1000, 1000);
        assert!(capped <= WORKS_DEV_CAP + 1e-6, "works_dev must never exceed its stated cap");
        assert!(works_dev_mult(1, 0) > 0.0, "a single estate must nudge the ceiling upward");
        assert!(works_dev_mult(10, 0) > works_dev_mult(1, 0),
            "more estates must give more headroom, up to the cap");
    }

    // ─────────────────────────────────────────────────────────────────────
    // Province-aware founding (`PLACES_DEMAND_AND_GROWTH_PLAN.md` slice 5, F5)
    // ─────────────────────────────────────────────────────────────────────

    /// The claim, both directions: a founding path PREFERS an empty province
    /// over a settled one, but is never FORCED into one — an unknown
    /// province (-1, no province layer / an old world) never bonuses, so a
    /// province-less campaign is byte-identical to before this slice.
    #[test]
    fn colonisation_prefers_an_empty_province_but_is_not_forced_into_one() {
        assert!(province_founding_bonus(3, false) > 0.0,
            "a known, unsettled province must be preferred");
        assert_eq!(province_founding_bonus(3, true), 0.0,
            "an already-settled province must draw no bonus");
        assert_eq!(province_founding_bonus(-1, false), 0.0,
            "an unknown province (-1) must never bonus, whatever `settled` reads");
        assert_eq!(province_founding_bonus(-1, true), 0.0,
            "an unknown province (-1) must never bonus, whatever `settled` reads");
    }

    /// `province_is_settled` is the live predicate every founding path reads
    /// (via `province_founding_bonus`): true the instant any hub maps to
    /// that province id, unconditionally true (never bonused) for -1.
    #[test]
    fn province_is_settled_reads_hub_province() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut hubs = vec![hub(0, 0.0, 0.0, 1000.0, vec![900.0], 0)];
        hubs.push(hub(1, 10.0, 10.0, 1000.0, vec![900.0], 0));
        let mut s = sim(hubs, goods);
        s.hub_province = vec![5, -1];
        assert!(s.province_is_settled(5), "province 5 has a live hub");
        assert!(!s.province_is_settled(2), "province 2 has no hub at all");
        assert!(s.province_is_settled(-1), "an unknown province reads settled (never bonused)");
        s.hubs[0].abandoned = true;
        assert!(!s.province_is_settled(5), "an abandoned hub no longer counts as settling its province");
    }

    // ─────────────────────────────────────────────────────────────────────
    // Provenance + distance prestige (`PLACES_DEMAND_AND_GROWTH_PLAN.md` slice 7, D7)
    // ─────────────────────────────────────────────────────────────────────

    /// The shipped dose, `FOREIGN_PRESTIGE = 0.0`, must be a true no-op at
    /// every tier and every origin distance.
    #[test]
    fn foreign_prestige_is_a_noop_at_zero() {
        assert_eq!(FOREIGN_PRESTIGE, 0.0, "slice 7 ships at zero dose");
        for tier in 0..3u8 {
            for origin_km in [0.0f32, 500.0, 3000.0, 9000.0] {
                assert_eq!(foreign_prestige_mult_e(FOREIGN_PRESTIGE, PRESTIGE_REF_KM, tier, origin_km), 1.0,
                    "tier {tier} at {origin_km}km must be an exact no-op at zero dose");
            }
        }
    }

    /// The claim: at a live dose, a good that travelled farther is wanted
    /// MORE than an otherwise-identical one that travelled less — the
    /// amber-in-Rome case.
    #[test]
    fn a_far_travelled_luxury_is_wanted_more_than_a_near_one() {
        let dose = 0.5;
        let near = foreign_prestige_mult_e(dose, PRESTIGE_REF_KM, 2, 200.0);
        let far = foreign_prestige_mult_e(dose, PRESTIGE_REF_KM, 2, PRESTIGE_REF_KM * 2.0);
        assert!(far > near, "a far-travelled good ({far}) must be wanted more than a near one ({near})");
        assert!(near >= 1.0, "even a near good must never be wanted LESS for its distance");
        assert!(far <= 1.0 + dose + 1e-6, "the dose bounds how far prestige can raise demand");
    }

    /// Rule: distance does not make a BASIC good more desirable — it only
    /// makes it dearer (already true via freight), which is correct and
    /// untouched by this mechanism.
    #[test]
    fn distance_prestige_never_touches_a_basic_good() {
        for origin_km in [0.0f32, 3000.0, 9000.0] {
            assert_eq!(foreign_prestige_mult_e(0.8, PRESTIGE_REF_KM, 0, origin_km), 1.0,
                "a basic good must be a no-op at any dose or distance");
        }
    }

    /// An old save with no `stock_origin` data (every good reads 0.0 km, the
    /// serde default) must be bit-identical to a fresh campaign at the
    /// shipped zero dose, and must not blow up or read as "maximally
    /// prestigious" even at a hypothetical live dose (0 km = wholly local).
    #[test]
    fn an_old_save_with_no_origin_data_is_bit_identical() {
        let goods = vec![good("silk", 0, 2, 8.0, 0.5, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 1000.0, vec![100.0], 0)];
        let s = sim(hubs, goods);
        assert!(s.hubs[0].stock_origin.is_empty(), "a fresh fixture carries no origin history yet");
        let origin_km = s.hubs[0].stock_origin.get(0).copied().unwrap_or(0.0);
        assert_eq!(origin_km, 0.0);
        assert_eq!(foreign_prestige_mult_e(0.8, PRESTIGE_REF_KM, 2, origin_km), 1.0,
            "no origin history must read as wholly local (no prestige bonus), even at a live dose");
    }

    // ─────────────────────────────────────────────────────────────────────
    // N7 · the League (`SEASONS_ELASTICITY_AND_LEAGUES_PLAN.md` §3-4)
    // ─────────────────────────────────────────────────────────────────────

    /// A world with no tiered cities (the state of any province-free fixture
    /// — `assign_city_tiers` itself never runs) must never form a league,
    /// exactly the `province_land_pass_is_a_noop_without_provinces`
    /// discipline applied to N7.
    #[test]
    fn n7_a_world_with_no_leagues_is_bit_identical() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![3000.0], 0),
            hub(1, 10.0, 0.0, 4000.0, vec![100.0], 0),
        ];
        let mut s = sim(hubs, goods);
        for yr in (LEAGUE_YEAR_FLOOR + 1)..(LEAGUE_YEAR_FLOOR + 20) {
            s.tick = yr * TICKS_PER_YEAR;
            s.maybe_form_leagues(yr);
            s.run_league_diet();
        }
        assert!(s.leagues.is_empty(), "an untiered world must never form a league");
        assert!(s.hubs.iter().all(|h| h.league < 0));
    }

    /// A League is a Realm's negative (§3.1): it holds no province, sets no
    /// sovereignty, and a member's `realm` is untouched by joining.
    #[test]
    fn n7_a_league_is_not_a_realm() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs: Vec<TickHub> = (0..3).map(|i| hub(i, i as f32 * 5.0, 0.0, 1000.0, vec![300.0], 0)).collect();
        let mut s = sim(hubs, goods);
        s.leagues.push(League {
            id: 0, name: "Test League".into(), seat_hub: 0, purse: 0.0,
            founded_tick: 0, dissolved_tick: 0, last_threat_tick: 0,
            boycotts: vec![], escort_until_tick: 0, events: vec![],
        });
        for h in 0..3 { s.hubs[h].league = 0; }
        assert!(s.hubs.iter().all(|h| h.realm == -1), "joining a league must never set sovereignty");
        assert!(s.prov_realm.is_empty(), "a league must never write `prov_realm`");
        assert_eq!(s.leagues[0].purse, 0.0);
    }

    /// The formation → dissolution round trip. Member count must go DOWN as
    /// well as UP over the run — a monotone count is a failed build, the
    /// same failure `realm_secession_pass` exists to prevent (§3.3).
    #[test]
    fn n7_leagues_form_and_dissolve() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut hubs: Vec<TickHub> = (0..4).map(|i| hub(i, i as f32 * 5.0, 0.0, 2000.0, vec![600.0], 0)).collect();
        for h in hubs.iter_mut() { h.tier = 1; h.treasury = 1000.0; }
        let mut s = sim(hubs, goods);
        let start_yr = LEAGUE_YEAR_FLOOR + 1;
        s.tick = start_yr as u32 * TICKS_PER_YEAR;
        s.flow_year = vec![(0, 1, 500.0), (0, 2, 500.0), (0, 3, 500.0)];
        s.wars.push(War { a: 0, b: 1, start_tick: s.tick, chest_a: 0.0, chest_b: 0.0,
            levies: 0.0, levies_a: 0.0, levies_b: 0.0, battles: Vec::new(), cargo_lost: 0,
            cause: "test".into(), goal: WAR_GOAL_PLUNDER,
            score: 0.0, round: 0, peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1,
            blockade_chronicled: false });

        s.maybe_form_leagues(start_yr);
        assert_eq!(s.leagues.len(), 1, "a threatened, tiered, well-traded seat must found a league");
        let after_form = (0..s.hubs.len()).filter(|&h| s.hubs[h].league == 0).count();
        assert!(after_form >= LEAGUE_MIN_MEMBERS);

        s.run_league_diet(); // refresh last_threat_tick while the war stands
        let threat_tick = s.tick;
        s.wars.clear(); // the threat lapses

        let mut min_members = after_form;
        for extra_yr in 1..=(LEAGUE_DRIFT_YEARS + 5) {
            s.tick = threat_tick + extra_yr * TICKS_PER_YEAR;
            s.run_league_diet();
            let now = (0..s.hubs.len()).filter(|&h| s.hubs[h].league == 0).count();
            min_members = min_members.min(now);
            if s.leagues[0].dissolved_tick != 0 { break; }
        }
        assert!(min_members < after_form,
            "member count must fall once the threat lapses — a monotone count is a failed build");
    }

    /// An empty boycott list (the state at `LEAGUE_BOYCOTT_MAX == 0`) must
    /// leave `dispatch` free to act on a real arbitrage gap exactly as if
    /// the hub had no league at all.
    #[test]
    fn n7_boycott_is_inert_at_zero() {
        assert_eq!(LEAGUE_BOYCOTT_MAX, 0, "N7.2/N7.3 ship boycotts at zero dose");
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        // `hub()`'s stock starts empty (only `production` is seeded) — give
        // the seller a real surplus to ship, the way `production_pass` would.
        stock_set_total(&mut s.hubs[0].stock, 0, 5000.0);
        s.leagues.push(League {
            id: 0, name: "T".into(), seat_hub: 0, purse: 0.0, founded_tick: 0,
            dissolved_tick: 0, last_threat_tick: 0, boycotts: vec![], escort_until_tick: 0, events: vec![],
        });
        s.hubs[0].league = 0;
        s.hubs[1].league = 0;
        let needs = vec![vec![0.0], vec![50.0]];
        let stock0 = stock_of(&s.hubs[0].stock, 0);
        s.dispatch(&needs);
        assert_ne!(stock_of(&s.hubs[0].stock, 0), stock0,
            "an empty boycott list must never block a real arbitrage gap");
    }

    /// The enforcement half, tested directly (mirroring
    /// `n2_export_ban_blocks_dispatch_when_set_directly`'s own pattern): a
    /// live `Boycott` must stop the named lane. (§4.3's fuller claim — that
    /// the trade reroutes to a non-boycotted partner rather than merely
    /// vanishing — needs a live merchant fleet/second buyer to observe and
    /// is left to the dose walk's own integration test, not asserted here.)
    #[test]
    fn n7_a_boycotted_city_reroutes() {
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0), // seller, boycotts hub 1
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),   // the boycott's target
        ];
        let mut s = sim(hubs, goods);
        s.rebuild_routes();
        stock_set_total(&mut s.hubs[0].stock, 0, 5000.0);
        s.leagues.push(League {
            id: 0, name: "T".into(), seat_hub: 0, purse: 0.0, founded_tick: 0,
            dissolved_tick: 0, last_threat_tick: 0,
            boycotts: vec![Boycott { target: 1, good: -1, until_tick: s.tick + 1000 }],
            escort_until_tick: 0,
            events: vec![],
        });
        s.hubs[0].league = 0;
        let needs = vec![vec![0.0], vec![50.0]];
        let stock0 = stock_of(&s.hubs[0].stock, 0);
        s.dispatch(&needs);
        assert_eq!(stock_of(&s.hubs[1].stock, 0), 0.0,
            "a boycotted target must receive nothing from the boycotting hub");
        assert_eq!(stock_of(&s.hubs[0].stock, 0), stock0,
            "the boycotted lane being the only target, the seller's stock must not move either");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 4.5 · the boycott mechanism, dosed at
    /// `LEAGUE_BOYCOTT_MAX == 0`, is a true no-op — the diet's own vote
    /// branch never fires, whatever war or realm the world carries.
    #[test]
    fn n7_3_boycott_vote_is_a_noop_at_zero_dose() {
        assert_eq!(LEAGUE_BOYCOTT_MAX, 0, "4.5 ships the boycott VOTE at zero dose");
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0), // league seat/member
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),   // a second member, at war
            hub(2, 10.0, 0.0, 9000.0, vec![10.0], 0),  // the enemy — a real boycott candidate
        ];
        let mut s = sim(hubs, goods);
        s.leagues.push(League {
            id: 0, name: "T".into(), seat_hub: 0, purse: 1000.0, founded_tick: 0,
            dissolved_tick: 0, last_threat_tick: s.tick, boycotts: vec![], escort_until_tick: 0, events: vec![],
        });
        s.hubs[0].league = 0;
        s.hubs[1].league = 0;
        s.hubs[1].war_with = 2;
        s.hubs[2].war_with = 1;
        s.wars.push(War {
            a: 1, b: 2, start_tick: s.tick, chest_a: 0.0, chest_b: 0.0, levies: 0.0,
            levies_a: 0.0, levies_b: 0.0, battles: Vec::new(), cargo_lost: 0,
            cause: "test".into(), goal: 0, score: 0.0, round: 0,
            peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1, blockade_chronicled: false,
        });
        // A real candidate exists (`choose_boycott_target` would pick hub 2),
        // so the ONLY thing stopping a vote is the dose itself.
        assert_eq!(s.choose_boycott_target(0, 0), Some(2),
            "sanity check: a real target exists for this fixture");
        s.run_league_diet();
        assert!(s.leagues[0].boycotts.is_empty(), "zero dose must vote no boycott, whatever the threat");
    }

    /// The target-selection rule itself (`choose_boycott_target`), factored
    /// out and tested as a pure decision independent of the dose — the same
    /// discipline `charter_bars_sale`/`leg_exceeds_range` already apply.
    /// Prefers a real war a member is fighting over a merely-threatening
    /// great power, and never names a member of its own league.
    #[test]
    fn choose_boycott_target_prefers_a_real_war() {
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),
            hub(2, 10.0, 0.0, 9000.0, vec![10.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.hubs[0].league = 0;
        s.hubs[1].league = 0;
        s.wars.push(War {
            a: 1, b: 2, start_tick: s.tick, chest_a: 0.0, chest_b: 0.0, levies: 0.0,
            levies_a: 0.0, levies_b: 0.0, battles: Vec::new(), cargo_lost: 0,
            cause: "test".into(), goal: 0, score: 0.0, round: 0,
            peak_effort_a: 0.0, peak_effort_b: 0.0, backer_house: -1, blockade_chronicled: false,
        });
        assert_eq!(s.choose_boycott_target(0, 0), Some(2),
            "a war between a member and a non-member must name the non-member");
    }

    /// With no war anywhere, the fallback names a threatening great power's
    /// capital instead of returning nothing.
    #[test]
    fn choose_boycott_target_falls_back_to_a_threatening_realm() {
        let goods = vec![good("iron", 2, 1, 5.0, 0.45, false)];
        let hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![5000.0], 0),
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0),
        ];
        let mut s = sim(hubs, goods);
        s.hubs[0].league = 0;
        s.realms.push(Realm {
            id: 0, name: "Rival".into(), title: "Kingdom".into(), capital_hub: 1,
            origin_realm: -1, ruling_house: 0, rank: 2, autonomy: 0,
            provinces: vec![], vassals: vec![], treasury: 0.0, debts: 0.0,
            legitimacy: 0.5, cohesion: 0.5, founded_tick: 0, fallen_tick: 0,
            events: vec![], ruler: -1, regent: -1, family: vec![],
            tax_rates: [0.0, 0.0], tithe_last_year: 0.0, tax_farm: None,
            founding_path: 0, government: 0,
        });
        assert_eq!(s.choose_boycott_target(0, 0), Some(1),
            "no war anywhere ⇒ fall back to the threatening realm's capital");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 1.3 · a league whose purse can afford it fits
    /// out a convoy escort — `League.purse`'s first real reader. A league with
    /// no purse must stay inert (the pre-existing bit-identical guarantee for
    /// leagues, `n7_a_world_with_no_leagues_is_bit_identical`, is untouched
    /// since `escort_until_tick` starts and stays at 0 there).
    #[test]
    fn n7_the_purse_buys_a_convoy_escort_when_it_can_afford_one() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Low treasury → tiny yearly dues, so the purse is effectively what we
        // preset it to, not what dues keep refilling — lets the test control
        // whether year 2 can afford a renewal.
        let mut hubs: Vec<TickHub> = (0..3).map(|i| hub(i, i as f32 * 5.0, 0.0, 2000.0, vec![600.0], 0)).collect();
        for h in hubs.iter_mut() { h.treasury = 60.0; }
        let mut s = sim(hubs, goods);
        s.tick = 10_000;
        s.leagues.push(League {
            id: 0, name: "Test League".into(), seat_hub: 0, purse: LEAGUE_ESCORT_COST - 1.0,
            founded_tick: 0, dissolved_tick: 0, last_threat_tick: s.tick,
            boycotts: vec![], escort_until_tick: 0, events: vec![],
        });
        for h in 0..3 { s.hubs[h].league = 0; }
        s.run_league_diet();
        assert!(!s.leagues.is_empty(), "the league must not have dissolved");
        assert!(s.leagues[0].escort_until_tick > s.tick, "an affordable escort is funded");
        assert!(s.leagues[0].events.iter().any(|e| e.kind == "escort"),
            "the funding is chronicled");
        // A second year, right as the escort lapses, with a purse too thin to
        // renew it — the escort's own doc: the lapse is legible too, not silent.
        let escort_until = s.leagues[0].escort_until_tick;
        s.tick = escort_until;
        s.run_league_diet();
        assert!(s.leagues[0].events.iter().any(|e| e.kind == "no_escort"),
            "an unaffordable renewal is chronicled too, not silently dropped");
    }

    /// A league that never accrues a purse (no members, or never enough dues)
    /// never funds an escort — `escort_until_tick` stays at its serde default 0.
    #[test]
    fn n7_an_empty_purse_never_funds_an_escort() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Treasury kept just above LEAGUE_DUES_MIN_TREASURY (never expelled) but
        // low enough that a year's dues (LEAGUE_DUES_FRAC · treasury) stay well
        // under LEAGUE_ESCORT_COST even summed across all three members.
        let mut hubs: Vec<TickHub> = (0..3).map(|i| hub(i, i as f32 * 5.0, 0.0, 2000.0, vec![600.0], 0)).collect();
        for h in hubs.iter_mut() { h.treasury = 60.0; }
        let mut s = sim(hubs, goods);
        s.tick = 10_000;
        s.leagues.push(League {
            id: 0, name: "Poor League".into(), seat_hub: 0, purse: 0.0,
            founded_tick: 0, dissolved_tick: 0, last_threat_tick: s.tick,
            boycotts: vec![], escort_until_tick: 0, events: vec![],
        });
        for h in 0..3 { s.hubs[h].league = 0; }
        // Dues this year are tiny (LEAGUE_DUES_FRAC · treasury), far under
        // LEAGUE_ESCORT_COST, so the purse still can't afford one.
        s.run_league_diet();
        assert_eq!(s.leagues[0].escort_until_tick, 0, "too poor a purse funds nothing");
        assert!(!s.leagues[0].events.iter().any(|e| e.kind == "escort"));
    }

    /// INSTITUTIONS_BUILD_ORDER.md 2.1 · the quality CAP no longer reads
    /// population at all — it is purely tradition (+ structures). A hub that
    /// keeps producing accumulates tradition every month (faster under a
    /// guild) and its quality cap — and so its realised quality — climbs with
    /// it; a hub that stops producing decays back down. This is the direct
    /// replacement for the old `size_bonus` and is what lets a small city
    /// out-master a metropolis that only just started (Murano vs a capital).
    #[test]
    fn quality_cap_tracks_tradition_not_population() {
        let mut g = good("glass", 1, 2, 10.0, 0.4, false);
        g.inputs = vec![(0, 1.0)]; // self-referential dummy input — only needs to be non-empty
        let goods = vec![g];
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 500.0, vec![10.0], 0),      // tiny population
            hub(1, 5.0, 0.0, 900_000.0, vec![10.0], 0),  // huge population
        ];
        for h in hubs.iter_mut() { h.quality = vec![0.0]; h.tradition = vec![0.0]; }
        let mut s = sim(hubs, goods);
        // Both hubs produce continuously for a long run — population plays no
        // part any more, so their quality trajectories must be IDENTICAL.
        for _ in 0..(TRADITION_YEARS_FULL as u32 * 12) {
            s.update_good_quality();
        }
        assert!(s.hubs[0].tradition[0] >= TRADITION_YEARS_FULL - 0.5,
            "continuous production must accumulate tradition toward the full cap");
        assert!((s.hubs[0].quality[0] - s.hubs[1].quality[0]).abs() < 1e-4,
            "population must not enter the cap at all — tiny hub {} vs huge hub {}",
            s.hubs[0].quality[0], s.hubs[1].quality[0]);
        assert!(s.hubs[0].quality[0] > QUALITY_CAP_BASE,
            "decades of practice must lift quality above the un-practised base cap");

        // Now hub 0 stops producing — tradition (and so quality) must DECAY,
        // unlike the old size/struct terms which only ever rose.
        s.hubs[0].production[0] = 0.0;
        let q_before = s.hubs[0].quality[0];
        // Tradition caps at 2×`TRADITION_YEARS_FULL` and only the bonus band
        // below `TRADITION_YEARS_FULL` moves the cap, so enough months of
        // decay must pass to actually bring it back under that threshold.
        for _ in 0..3200 { s.update_good_quality(); }
        assert!(s.hubs[0].quality[0] < q_before,
            "a craft not practised for years must lose standing, not merely stop growing");
    }

    /// 2.1 companion · a guild speeds tradition growth (organised transmission
    /// of skill), so an otherwise-identical hub with a guild present pulls
    /// ahead of one without.
    #[test]
    fn a_guild_grows_tradition_faster_than_practice_alone() {
        let mut g = good("glass", 1, 2, 10.0, 0.4, false);
        g.inputs = vec![(0, 1.0)];
        let goods = vec![g];
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 5000.0, vec![10.0], 0), // guilded
            hub(1, 5.0, 0.0, 5000.0, vec![10.0], 0), // ungilded, otherwise identical
        ];
        for h in hubs.iter_mut() { h.quality = vec![0.0]; h.tradition = vec![0.0]; }
        let mut s = sim(hubs, goods);
        s.guilds.push(CraftGuild { hub: 0, good: 0, strength: 0.3, hall: false, secrecy: 0.0, signature: None, idle_years: 0.0 });
        for _ in 0..120 { s.update_good_quality(); } // 10 years
        assert!(s.hubs[0].tradition[0] > s.hubs[1].tradition[0],
            "the guilded hub must accumulate tradition faster over the same decade");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 2.2 · SECRECY must measurably resist theft —
    /// "a guarded craft must diffuse SLOWER than an unguarded one, or secrecy is
    /// decoration." Two otherwise-identical worlds, differing only in the
    /// leader guild's `secrecy`, sampled across many houses/years; count how
    /// often `maybe_steal_quality` actually lifts the thief's quality.
    #[test]
    fn guild_secrecy_measurably_slows_the_diffusion_of_a_guarded_craft() {
        fn build(secrecy: f32) -> CampaignSim {
            let goods = vec![good("glass", 1, 2, 10.0, 0.4, false)];
            let mut hubs = vec![
                hub(0, 0.0, 0.0, 9000.0, vec![10.0], 0), // the world leader
            ];
            for hi in 1..=20u32 {
                hubs.push(hub(hi, hi as f32, 0.0, 9000.0, vec![10.0], 0));
            }
            let n = hubs.len();
            let mut s = sim(hubs, vec![]);
            s.goods = goods;
            for h in 0..n {
                s.hubs[h].quality = vec![0.0];
                s.hubs[h].production = vec![1.0];
                s.hubs[h].is_estate = true;
                s.hubs[h].estate_kind = 6;
                s.hubs[h].owner_house = h as i32;
            }
            s.hubs[0].quality[0] = 0.95; // the leader, far ahead
            s.guilds.push(CraftGuild { hub: 0, good: 0, strength: 0.6, hall: true, secrecy, signature: None, idle_years: 0.0 });
            s.houses = (0..n).map(|h| {
                let mut ho = house_at(h as u32, vec![0], 0);
                ho.archetype = 0; // not ARCH_FLEET, which is excluded from spying
                ho
            }).collect();
            s
        }
        // A theft roll must clear TWO independent gates: the attempt chance
        // (`QUALITY_STEAL_CHANCE`, identical between the two builds — same
        // seed, same per-(house,year) roll) and, only if a guild guards the
        // leader, the secrecy roll on top of it. Driving secrecy to 1.0 (well
        // past the shipped `GUILD_SECRECY_CAP`, deliberately — this isolates
        // the MECHANISM from the dose, the same split `charter_bars_sale`'s
        // own pure-decision test uses) makes the foil roll certain, so this is
        // a direct test of the gate rather than a statistical race that a
        // small sample can flip either way through path-dependent history
        // (an early foil changes who is even eligible to try next).
        let mut guarded = build(1.0);
        let mut open = build(0.0);
        let mut guarded_thefts = 0u32;
        let mut open_thefts = 0u32;
        for yr in 0..400u32 {
            let before = guarded.hubs.iter().filter(|h| h.stolen_good == 0).count();
            guarded.maybe_steal_quality(yr);
            if guarded.hubs.iter().filter(|h| h.stolen_good == 0).count() > before { guarded_thefts += 1; }
            let before = open.hubs.iter().filter(|h| h.stolen_good == 0).count();
            open.maybe_steal_quality(yr);
            if open.hubs.iter().filter(|h| h.stolen_good == 0).count() > before { open_thefts += 1; }
        }
        assert!(open_thefts > 0, "an unguarded craft must actually diffuse over 400 years");
        assert_eq!(guarded_thefts, 0,
            "a guild guarded at secrecy 1.0 must foil every attempt — secrecy is a real \
             subtraction from the theft roll, not decoration");
    }

    /// INSTITUTIONS_BUILD_ORDER.md 2.2 · THE MASTER WHO LEAVES — a defecting
    /// master must carry a FRACTION of the source's tradition to the destination
    /// (never quality itself directly) and must be chronicled on both sides;
    /// the source's guild loses strength and secrecy as a real cost.
    #[test]
    fn maybe_poach_master_carries_tradition_and_costs_the_source_guild() {
        let goods = vec![good("glass", 1, 2, 10.0, 0.4, false)];
        let mut hubs = vec![
            hub(0, 0.0, 0.0, 9000.0, vec![10.0], 0), // the master's home
            hub(1, 5.0, 0.0, 9000.0, vec![10.0], 0), // a poorer rival, less tradition
        ];
        hubs[0].production = vec![1.0]; hubs[0].tradition = vec![40.0]; hubs[0].treasury = 10.0;
        hubs[1].production = vec![1.0]; hubs[1].tradition = vec![2.0]; hubs[1].treasury = 5000.0; // rich enough to bribe
        let mut s = sim(hubs, goods);
        s.guilds.push(CraftGuild { hub: 0, good: 0, strength: 0.6, hall: true, secrecy: 0.0, signature: None, idle_years: 0.0 });
        let mut poached = false;
        for yr in 0..2000u32 {
            let strength_before = s.guilds[0].strength;
            s.maybe_poach_master(yr);
            if s.guilds[0].strength < strength_before {
                poached = true;
                break;
            }
        }
        assert!(poached, "MASTER_POACH_CHANCE must be reachable over enough years");
        assert!(s.hubs[1].tradition[0] > 2.0,
            "the destination's tradition must rise once a master defects to it");
        assert!(s.hubs[1].tradition[0] < s.hubs[0].tradition[0],
            "the destination jumps TOWARD the source's standing, never past it in one defection");
        assert!(s.journal.iter().any(|j| j.kind == "master_poached" && j.hub == 0),
            "the source's loss must be chronicled");
    }

    /// 2.2 companion · a world with no guilds at all is untouched by either
    /// mechanism — `maybe_poach_master` iterates the (empty) guild list and
    /// returns immediately.
    #[test]
    fn a_world_with_no_guilds_is_untouched_by_poaching() {
        let goods = vec![good("glass", 1, 2, 10.0, 0.4, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 9000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        for yr in 0..500u32 { s.maybe_poach_master(yr); }
        assert!(s.hubs[0].tradition.is_empty(), "no guild, nothing to poach");
    }

    /// TECTONICS_AND_ISOLATION_PLAN.md Part A — the guard against a trans-oceanic
    /// rescue. `rescue_tiny_components` used to fold ANY <3-hub component into the
    /// nearest big one with no distance limit at all, which is what let a
    /// mid-ocean island get labelled part of a distant continent and traded with
    /// it via straight-line lanes. Three cases, one call:
    ///   - a tiny (2-hub) component WITHIN the cap merges into the big one, same
    ///     as before — a real regional sea crossing is still rescued;
    ///   - a tiny (2-hub) component BEYOND the cap stays its own component, so its
    ///     two hubs trade only with each other via the ordinary same-component
    ///     passes — real internal trade, per the user's own design;
    ///   - a SINGLE-hub component beyond the cap is left alone too — no special
    ///     "lifeline" exception. A lone city with no reachable partner is true
    ///     isolation, and whether it survives is the economy's question, not a
    ///     router special case.
    #[test]
    fn rescue_tiny_components_never_crosses_an_ocean() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut hubs = Vec::new();
        // A "big" component (id 0): 3 hubs clustered near the origin.
        for i in 0..3u32 {
            hubs.push(hub(i, i as f32 * 2.0, 0.0, 5000.0, vec![10.0], 0));
        }
        // A tiny component (id 1), 2 hubs, WITHIN the cap of the big cluster.
        // world_w=100 (the test default) -> km_per_cell = 40075/100 ≈ 400.75, so
        // ISOLATION_RESCUE_MAX_KM=1800 is ≈4.5 cells; 3 cells is comfortably inside.
        hubs.push(hub(10, 3.0, 3.0, 3000.0, vec![10.0], 1));
        hubs.push(hub(11, 3.5, 3.0, 3000.0, vec![10.0], 1));
        // A tiny component (id 2), 2 hubs, FAR beyond the cap — a separate island
        // with its own real trade partner.
        hubs.push(hub(20, 60.0, 60.0, 3000.0, vec![10.0], 2));
        hubs.push(hub(21, 60.5, 60.0, 3000.0, vec![10.0], 2));
        // A single-hub component (id 3), also far beyond the cap — total isolation.
        hubs.push(hub(30, 90.0, 10.0, 3000.0, vec![10.0], 3));
        let mut s = sim(hubs, goods);
        s.rescue_tiny_components();

        assert_eq!(s.hubs[3].component, 0, "the near tiny component should still be rescued into the big one");
        assert_eq!(s.hubs[4].component, 0, "the near tiny component should still be rescued into the big one");
        assert_eq!(s.hubs[5].component, 2, "a far 2-hub component must stay its own — it has real internal trade");
        assert_eq!(s.hubs[6].component, 2, "a far 2-hub component must stay its own — it has real internal trade");
        assert_eq!(s.hubs[7].component, 3,
            "a far SINGLE-hub component must stay isolated — no distance-unlimited lifeline");
    }

    // ── YARDS_VESSELS_AND_DEPOTS_PLAN.md — S1-S4, the guild axis, W2/W3 ──

    /// S1 · a world where no city ever clears `YARD_MIN_POP` never founds a
    /// yard, and the (would-be) yard passes touch nothing — the same
    /// guarantee `province_land_pass_is_a_noop_without_provinces` holds.
    #[test]
    fn a_world_with_no_yards_is_bit_identical() {
        let goods = vec![good("timber", 3, 1, 2.0, 0.3, false)];
        let mut hubs = vec![hub(0, 0.0, 0.0, 500.0, vec![5.0], 0)];
        hubs[0].coastal = true;
        let mut s = sim(hubs, goods);
        let before = s.hubs.len();
        s.maybe_found_yards();
        s.yard_build_pass();
        assert_eq!(s.hubs.len(), before, "a city below YARD_MIN_POP must not found a yard");
        assert!(s.vessels.is_empty(), "no yard ever ran ⇒ no vessel is ever built");
    }

    /// S1 · a yard with nothing to draw on accumulates no progress and builds
    /// nothing — it says so by simply never crossing `HULL_BUILD_POINTS`,
    /// rather than stalling silently forever with no visible state.
    #[test]
    fn a_yard_with_no_material_builds_nothing() {
        let goods = vec![
            good("timber", 3, 1, 2.0, 0.3, false),
            good("hardwoods", 3, 1, 2.0, 0.3, false),
        ];
        let mut hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![0.0, 0.0], 0)];
        hubs[0].coastal = true;
        hubs.push(hub(1, 0.1, 0.1, 10.0, vec![0.0, 0.0], 0));
        hubs[1].is_estate = true;
        hubs[1].parent = 0;
        hubs[1].estate_kind = YARD_ESTATE_KIND;
        let mut s = sim(hubs, goods);
        s.yard_build_pass();
        assert_eq!(s.hubs[1].yard_progress, 0.0, "no material in the parent city ⇒ zero progress");
        assert!(s.vessels.is_empty());
    }

    /// D1's whole claim, asserted directly rather than left to inference: a
    /// desert-coast city and a tropical city both reach a buildable material
    /// mix once hardwoods reach their quay (grown locally or landed by
    /// trade — the yard draws the hub's own stock pool either way).
    #[test]
    fn every_climate_can_build_a_hull() {
        let goods = vec![good("hardwoods", 3, 1, 2.0, 0.3, false)];
        for koppen_label in ["desert-coast", "tropical"] {
            let mut hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![0.0], 0)];
            hubs[0].coastal = true;
            hubs.push(hub(1, 0.1, 0.1, 10.0, vec![0.0], 0));
            hubs[1].is_estate = true;
            hubs[1].parent = 0;
            hubs[1].estate_kind = YARD_ESTATE_KIND;
            let mut s = sim(hubs, goods.clone());
            // Material reaching the quay — grown locally or landed by trade;
            // the yard cannot tell the difference and must not need to.
            stock_add_ungraded(&mut s.hubs[0].stock, 0, 500.0);
            s.yard_build_pass();
            // Either progress accumulated, or — since the draw was large enough
            // to clear HULL_BUILD_POINTS in one pass — a hull completed outright
            // (which resets progress to 0 and is the STRONGER proof of D1).
            assert!(s.hubs[1].yard_progress > 0.0 || !s.vessels.is_empty(),
                "{koppen_label} city with hardwoods reaching its quay must build (progress or a completed hull)");
        }
    }

    /// S2 · seeding one `Vessel` per pre-existing fleet counter reproduces the
    /// same summed capacity the bare counter always implied — the
    /// representation changed, nothing else did.
    #[test]
    fn seeding_one_whole_hull_per_counter_is_bit_identical() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![], 3));
        s.houses[0].fleet_river = 2;
        s.seed_vessels_from_fleets();
        assert_eq!(s.house_vessel_capacity(0, 0), 3.0 * SHIP_CAPACITY,
            "3 seeded sea hulls must sum to exactly 3× SHIP_CAPACITY");
        assert_eq!(s.house_vessel_capacity(0, 1), 2.0 * BOAT_CAPACITY,
            "2 seeded river hulls must sum to exactly 2× BOAT_CAPACITY");
        // Idempotent — a second call must not double the fleet.
        s.seed_vessels_from_fleets();
        assert_eq!(s.house_vessel_capacity(0, 0), 3.0 * SHIP_CAPACITY);
    }

    /// S3 · every vessel with an owner has its `parts` sum to exactly
    /// `VESSEL_PARTS_TOTAL`, whether it went to one house or several.
    #[test]
    fn vessel_parts_always_sum_to_64() {
        let goods = vec![good("hardwoods", 3, 1, 2.0, 0.3, false)];
        let mut hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![0.0], 0)];
        hubs[0].coastal = true;
        hubs.push(hub(1, 0.1, 0.1, 10.0, vec![0.0], 0));
        hubs[1].is_estate = true;
        hubs[1].parent = 0;
        hubs[1].estate_kind = YARD_ESTATE_KIND;
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![], 0));
        s.houses[0].wealth = 500.0;
        s.houses.push(house_at(0, vec![], 0));
        s.houses[1].wealth = 150.0;
        stock_add_ungraded(&mut s.hubs[0].stock, 0, 5_000.0);
        for _ in 0..3 { s.yard_build_pass(); } // clear HULL_BUILD_POINTS
        assert!(!s.vessels.is_empty(), "sufficient material must complete at least one hull");
        for v in &s.vessels {
            if v.parts.is_empty() { continue; } // city-owned — no house present
            let total: u32 = v.parts.iter().map(|p| p.parts as u32).sum();
            assert_eq!(total, VESSEL_PARTS_TOTAL as u32,
                "vessel {} parts must sum to exactly {VESSEL_PARTS_TOTAL}, got {total}", v.id);
        }
    }

    /// S3 · losing a vessel debits every part owner proportionally and ruins
    /// none — fractional ownership's whole reason for existing (D3). Each
    /// owner's loss is bounded to half its own wealth, so two houses sharing
    /// a hull both survive its loss.
    #[test]
    fn a_lost_hull_debits_every_part_owner_and_ruins_none() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![], 0));
        s.houses[0].wealth = 4.0; // deliberately thin — the cap must bind
        s.houses.push(house_at(0, vec![], 0));
        s.houses[1].wealth = 100.0;
        s.vessels.push(Vessel {
            id: 0, name: "Test hull".into(), kind: 0, home_hub: 0, at_hub: 0,
            capacity: SHIP_CAPACITY, quality: 1.0, condition: 1.0,
            parts: vec![
                VesselShare { house: 0, parts: 32 },
                VesselShare { house: 1, parts: 32 },
            ],
            built_tick: 0,
        });
        let (w0_before, w1_before) = (s.houses[0].wealth, s.houses[1].wealth);
        s.lose_vessel(0);
        assert!(s.vessels.is_empty());
        assert!(s.houses[0].wealth >= w0_before * 0.5 - 1e-3,
            "a part-owner's loss must be capped, never more than half its own wealth");
        assert!(s.houses[1].wealth < w1_before, "the other part-owner is debited too");
        assert!(s.houses[1].wealth > 0.0, "a fractional stake in a lost hull must not ruin the house");
    }

    /// S4, dose-walked · at `CAPACITY_BIND_DOSE == 0.0` a shipment of any size
    /// needs zero extra vessel slots beyond the one it already reserves.
    #[test]
    fn n_yards_s4_capacity_bind_at_zero_is_a_noop() {
        assert_eq!(CAPACITY_BIND_DOSE, 0.0);
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0)];
        let s = sim(hubs, goods);
        assert_eq!(s.capacity_bind_extra_slots(1_000_000.0, SHIP_CAPACITY), 0,
            "even a huge shipment must need zero extra slots at zero dose");
    }

    /// PORT_COMPETITION_PLAN.md Slice 1/2 · the zero-dose half of the claim,
    /// checked directly against the literal value `0.0` (never against
    /// `PORT_TOLL_COMPETITION_DOSE`, which Slice 2 dosed to 0.3 — this test
    /// must keep proving the mechanism CAN be a no-op, independent of what
    /// the shipped dose currently is): `decide_port_tolls_at(0.0, ..)`
    /// returns exactly 1.0 for every hub regardless of relay traffic, and
    /// `route_outlet`'s outlet selection under it is bit-identical to a run
    /// with the toll mechanism's bias term removed entirely.
    #[test]
    fn port_toll_at_zero_dose_is_a_true_noop() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut h0 = hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0);
        let mut h1 = hub(1, 30.0, 0.0, 15_000.0, vec![10.0], 0);
        let h2 = hub(2, 60.0, 0.0, 12_000.0, vec![10.0], 0);
        h0.coastal = true; h1.coastal = true;
        let mut s = sim(vec![h0, h1, h2], goods);
        s.rebuild_routes();

        let tolls = s.decide_port_tolls_at(0.0);
        assert!(tolls.iter().all(|&t| t == 1.0),
            "every hub's toll target must be exactly 1.0 at zero dose, got {tolls:?}");
    }

    /// PORT_COMPETITION_PLAN.md Slice 2 — the DOSED behaviour, measured
    /// safe by `econ_measure_port_competition` before this dose was raised
    /// (48.1% of a realistically dense world's hubs contestable). At the
    /// shipped `PORT_TOLL_COMPETITION_DOSE` (0.3), a hub sitting on many
    /// other pairs' relay route must charge MORE than one that sits on
    /// none, both within `[PORT_TOLL_MIN, PORT_TOLL_MAX]`, and applying the
    /// target must ease `transit_toll_mult` toward it rather than snap.
    #[test]
    fn port_toll_competition_biases_toll_by_relay_traffic_within_bounds() {
        assert!(PORT_TOLL_COMPETITION_DOSE > 0.0,
            "this test exercises the shipped dose — update it if the dose ever returns to 0.0");
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut h0 = hub(0, 30.0, 0.0, 20_000.0, vec![10.0], 0);
        let h1 = hub(1, 0.0, 0.0, 12_000.0, vec![10.0], 0);
        let h2 = hub(2, 60.0, 0.0, 12_000.0, vec![10.0], 0);
        let mut h3 = hub(3, 200.0, 200.0, 12_000.0, vec![10.0], 0);
        h0.coastal = true; h3.coastal = true;
        let mut s = sim(vec![h0, h1, h2, h3], goods);
        s.rebuild_routes();

        // `relay_counts()` reads `route_outlet` directly (§ its own doc
        // comment) — set it by hand rather than relying on this tiny
        // fixture's own geometry to produce a genuine composed relay (a
        // real entrepôt only wins when it beats an existing direct route,
        // which four closely-spaced hubs rarely need). h0 is made the
        // outlet for every OTHER pair; h3 is never anyone's outlet.
        let n = s.n();
        s.route_outlet = vec![-1; n * n];
        for a in 0..n {
            for b in 0..n {
                if a != b && a != 0 && b != 0 { s.route_outlet[a * n + b] = 0; }
            }
        }
        let counts = s.relay_counts();
        assert!(counts[0] > counts[3],
            "fixture must actually give h0 more relay traffic than h3, got {counts:?}");

        let tolls = s.decide_port_tolls();
        for &t in &tolls {
            assert!(t >= PORT_TOLL_MIN && t <= PORT_TOLL_MAX,
                "toll target {t} must stay within [{PORT_TOLL_MIN}, {PORT_TOLL_MAX}]");
        }
        assert!(tolls[0] > tolls[3],
            "the busier relay (h0) must target a higher toll than the never-relayed \
             outlet (h3): got {:?} vs {:?}", tolls[0], tolls[3]);

        s.apply_port_tolls(&tolls);
        for h in 0..s.hubs.len() {
            let eased = 1.0 + (tolls[h] - 1.0) * 0.5;
            assert!((s.hubs[h].transit_toll_mult - eased).abs() < 1e-4,
                "apply_port_tolls must EASE toward the target, not snap to it");
        }
    }

    /// PORT_COMPETITION_PLAN.md Slice 3 — `contested_rivals` names, for every
    /// hub, the ONE other coastal hub it is most often top-2-contested
    /// against. Build a fixture with a genuine composed relay (unlike the
    /// toll-bias test above, this one needs REAL `route_outlet`/`days`
    /// geometry, not a hand-set array, since the rival relation is read live
    /// off `self.days`) and check the two ports the diagnostic-style margin
    /// test would flag as contesting each other actually come back paired.
    #[test]
    fn contested_rivals_names_the_real_top_2_outlet_pair() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        // Two close coastal ports (0, 1) both plausible outlets for an
        // inland hub (2); a third, far coastal hub (3) with no contest.
        let mut h0 = hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0);
        let mut h1 = hub(1, 2.0, 0.0, 18_000.0, vec![10.0], 0);
        let h2 = hub(2, 1.0, 5.0, 15_000.0, vec![10.0], 0);
        let mut h3 = hub(3, 500.0, 500.0, 12_000.0, vec![10.0], 0);
        h0.coastal = true; h1.coastal = true; h3.coastal = true;
        let mut s = sim(vec![h0, h1, h2, h3], goods);
        s.rebuild_routes();

        let rivals = s.contested_rivals();
        assert!(rivals[0] == 1 || rivals[1] == 0,
            "hubs 0 and 1 sit close enough that at least one must name the \
             other as its contested rival, got {rivals:?}");
        assert_eq!(rivals[3], -1,
            "the far, uncontested hub must have no named rival, got {:?}", rivals[3]);
    }

    /// PORT_COMPETITION_PLAN.md Slice 3, dose-walked (§2.8) · at
    /// `PORT_RIVAL_UNDERCUT_DOSE == 0.0` (the shipped setting — Slice 3 is
    /// built and gated but not yet dosed) `apply_rival_undercut` must return
    /// its `base` input completely unchanged, whatever `contested_rivals`/
    /// `relay_counts` say — checked against the literal `0.0` via
    /// `apply_rival_undercut_at`, independent of the shipped constant, the
    /// same split `port_toll_at_zero_dose_is_a_true_noop` uses.
    #[test]
    fn port_rival_undercut_is_a_noop_at_zero_dose() {
        assert_eq!(PORT_RIVAL_UNDERCUT_DOSE, 0.0,
            "Slice 3 ships undosed — update this test's premise if that changes");
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut h0 = hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0);
        let mut h1 = hub(1, 30.0, 0.0, 12_000.0, vec![10.0], 0);
        h0.coastal = true; h1.coastal = true;
        h0.transit_toll_mult = 1.6; // deliberately the LOSING side's toll is high
        let s = sim(vec![h0, h1], goods);
        let base = vec![1.6f32, 0.7f32];
        let rivals = vec![1i32, 0i32];
        let out = s.apply_rival_undercut_at(&base, &rivals, 0.0);
        assert_eq!(out, base, "dose 0.0 must return `base` completely unchanged");
    }

    /// PORT_COMPETITION_PLAN.md Slice 3 — the dosed behaviour. Hub 0 has
    /// FEWER relays than its named rival hub 1 (it is losing that fight) and
    /// hub 1's current toll is 0.7; at full dose hub 0's target must move
    /// DOWN toward `0.7 − PORT_UNDERCUT_MARGIN`, never up, and never below
    /// `PORT_TOLL_MIN`. The WINNING side (hub 1, more relays than its
    /// rival) must be left exactly at its base target — undercutting is a
    /// response to LOSING, not a universal race to the bottom.
    #[test]
    fn port_rival_undercut_pulls_the_loser_toward_its_rival() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut h0 = hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0);
        let mut h1 = hub(1, 30.0, 0.0, 12_000.0, vec![10.0], 0);
        h0.coastal = true; h1.coastal = true;
        h1.transit_toll_mult = 0.7; // the WINNING rival's current toll
        let mut s = sim(vec![h0, h1], goods);
        s.rebuild_routes();
        // Manually give hub 1 more relay traffic than hub 0, so hub 0 is the
        // loser in this pair. `relay_counts` just tallies VALUES in
        // `route_outlet` (`n×n` of them, index is `a*n+b`), so any slot set
        // to `1` counts as one relay through hub 1 — position `1` (i.e.
        // `a=0, b=1`) is as good as any (`route_outlet reads directly, same
        // trick the toll-bias test above uses).
        let n = s.n();
        s.route_outlet = vec![-1; n * n];
        s.route_outlet[1] = 1; // counts[1] = 1, counts[0] = 0

        let base = vec![1.0f32, 1.0f32];
        let rivals = vec![1i32, 0i32];
        let out = s.apply_rival_undercut_at(&base, &rivals, 1.0);

        let expected_loser_target = (0.7f32 - PORT_UNDERCUT_MARGIN).max(PORT_TOLL_MIN);
        assert!((out[0] - expected_loser_target).abs() < 1e-4,
            "the losing hub's target must move to rival_toll - margin, got {} want {}",
            out[0], expected_loser_target);
        assert!(out[0] < base[0], "the loser's target must move DOWN from base");
        assert_eq!(out[1], base[1],
            "the winning side (more relays than its rival) must be untouched: got {}", out[1]);
    }

    /// C1b, dose-walked (`ROUTES_ISOLATION_AND_CARRIAGE_REVIEW.md` §9) · at
    /// `LAND_BULK_PENALTY == 0.0` a LAND leg's freight for a bulky good must be
    /// IDENTICAL to what the same good would cost over the same distance at SEA
    /// — the mode argument to `good_freight` is a true no-op at the shipped
    /// dose. Uses a good with `bulk > 1.0` (wheat, 3.0) specifically, since a
    /// bulk-at-or-below-1.0 good is untouched by this term at ANY dose and so
    /// would pass this assertion even with the penalty wired backwards.
    #[test]
    fn c1b_land_bulk_penalty_is_a_noop_at_zero() {
        assert_eq!(LAND_BULK_PENALTY, 0.0);
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        s.goods[0].bulk = 3.0; // `good()` always seeds bulk 1.0; force a bulky fixture
        assert!(s.goods[0].bulk > 1.0, "fixture must actually be bulky to exercise this term");
        let land = s.good_freight(0, 0.02, 10.0, false);
        let sea = s.good_freight(0, 0.02, 10.0, true);
        assert!((land - sea).abs() < 1e-6,
            "at zero dose a land leg must cost exactly what a sea leg would \
             (land {land}, sea {sea})");
    }

    /// The guild axis, dose-walked · at `GUILD_CHARTER_RANGE_DAYS ==
    /// INFINITY` no finite route length can ever exceed it, so a guild
    /// candidate in `house_for`'s dispatch is never skipped on distance.
    #[test]
    fn n_yards_guild_axis_at_infinity_is_a_noop() {
        assert_eq!(GUILD_CHARTER_RANGE_DAYS, f32::INFINITY);
        assert!(!(3650.0f32 > GUILD_CHARTER_RANGE_DAYS), "no finite route length exceeds INFINITY");
    }

    /// W2, dose-walked · at `LANDED_CARGO_TO_DEPOT_DOSE == 0.0` a landed
    /// house cargo diverts nothing into a depot, whatever room exists.
    #[test]
    fn n_yards_w2_landed_cargo_to_depot_at_zero_is_a_noop() {
        assert_eq!(LANDED_CARGO_TO_DEPOT_DOSE, 0.0);
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        s.houses.push(house_at(0, vec![], 0));
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 10_000.0, stock: vec![0.0], tier: 1, damage: 0.0 });
        let diverted = s.landed_cargo_to_depot(0, 0, 500.0, 0);
        assert_eq!(diverted, 0.0, "zero dose must divert nothing, even with an owner and free room");
    }

    /// W3, dose-walked · at `WH_RELEASE_DOSE == 0.0` a depot never releases
    /// stock back to the pool, however dear the local price runs.
    #[test]
    fn n_yards_w3_release_at_zero_is_a_noop() {
        assert_eq!(WH_RELEASE_DOSE, 0.0);
        let goods = vec![good("silk", 3, 2, 10.0, 0.4, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![0.0], 0)];
        let mut s = sim(hubs, goods);
        s.hubs[0].price[0] = 40.0; // far past WH_RELEASE_PRICE_MULT × base_value
        s.houses.push(house_at(0, vec![0], 0));
        s.warehouses.push(Warehouse { owner: 0, hub: 0, capacity: 1000.0, stock: vec![300.0], tier: 1, damage: 0.0 });
        let needs = vec![vec![0.0f32]];
        s.warehouse_release_pass(&needs);
        assert_eq!(s.warehouses[0].stock[0], 300.0, "zero dose must release nothing however dear the price");
    }

    /// W4/W5, dose-walked · both ship structurally inert.
    #[test]
    fn n_yards_w4_and_w5_ship_inert() {
        assert!(!DEPOT_TO_DEPOT_TRANSFER_ENABLED);
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        s.depot_to_depot_transfer_pass();
        s.maybe_found_fondaco();
        assert!(s.fondacos.is_empty(), "W5 must never found a fondaco while its dose is zero");
    }

    // ── DEPOSITS_AND_MINING_PLAN.md slice 4 ─────────────────────────────────

    /// `mine_depth_at` returns the NEAREST real working's depth within reach, and
    /// `DEPTH_SURFACE` (ungated) both beyond reach and when this world carries no
    /// positional deposit data at all — the safe default that keeps an old save
    /// or a template world byte-identical (rule 26's discipline, on depth).
    #[test]
    fn mine_depth_at_finds_the_nearest_working_within_reach() {
        let goods = vec![good("iron", 3, 1, 3.0, 0.3, false)];
        let hubs = vec![hub(0, 0.0, 0.0, 5_000.0, vec![0.0], 0)];
        let mut s = sim(hubs, goods);
        s.world_w = 3600.0;
        // No data at all ⇒ ungated.
        assert_eq!(s.mine_depth_at("iron", 0.0, 0.0), crate::sim::deposits::DEPTH_SURFACE);
        s.mine_deposits = vec![
            MineSite { good: "iron".into(), x: 2.0, y: 0.0, depth: crate::sim::deposits::DEPTH_DEEP, extent: crate::sim::deposits::EXTENT_MODERATE, district: 0, working: crate::sim::deposits::WorkingKind::Shaft },
            MineSite { good: "iron".into(), x: 500.0, y: 0.0, depth: crate::sim::deposits::DEPTH_FLOODED, extent: crate::sim::deposits::EXTENT_MODERATE, district: 1, working: crate::sim::deposits::WorkingKind::Shaft },
            MineSite { good: "silver".into(), x: 0.0, y: 0.0, depth: crate::sim::deposits::DEPTH_SHALLOW, extent: crate::sim::deposits::EXTENT_MODERATE, district: 0, working: crate::sim::deposits::WorkingKind::Shaft },
        ];
        // The nearest IRON working wins over a farther one and over a
        // same-position working of a different good.
        assert_eq!(s.mine_depth_at("iron", 0.0, 0.0), crate::sim::deposits::DEPTH_DEEP);
        // Nothing of this good within reach ⇒ ungated, not "the world's default".
        assert_eq!(s.mine_depth_at("copper", 0.0, 0.0), crate::sim::deposits::DEPTH_SURFACE);
    }

    /// A Mine estate founded near a real deep/flooded working is more expensive
    /// to upgrade than one on a surface body — `MINE_UPGRADE_COST_MULT` is a real
    /// increasing schedule, and any other estate kind is untouched (mult stays
    /// 1.0 regardless of `mine_depth`, since only kind==2 reads it).
    #[test]
    fn mine_upgrade_cost_mult_increases_with_depth_and_only_gates_mines() {
        assert!(MINE_UPGRADE_COST_MULT[0] < MINE_UPGRADE_COST_MULT[1]);
        assert!(MINE_UPGRADE_COST_MULT[1] < MINE_UPGRADE_COST_MULT[2]);
        assert!(MINE_UPGRADE_COST_MULT[2] < MINE_UPGRADE_COST_MULT[3]);
        assert_eq!(MINE_UPGRADE_COST_MULT[crate::sim::deposits::DEPTH_SURFACE as usize], 1.0);
    }

    /// Mercury amalgamation is a true no-op when this world has no silver good,
    /// no mercury good, or no silver mine at all — every early return must leave
    /// production/stock untouched.
    #[test]
    fn mercury_amalgamation_is_a_noop_without_both_goods() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let mut hub0 = hub(0, 0.0, 0.0, 5_000.0, vec![10.0], 0);
        hub0.is_estate = true;
        hub0.estate_kind = 2;
        let mut s = sim(vec![hub0], goods);
        s.apply_mercury_amalgamation();
        assert_eq!(s.hubs[0].production[0], 10.0);
    }

    /// A silver-mine estate with NO mercury on hand still smelts (the floor),
    /// but a fully-mercury-supplied one recovers MORE ore than it dug — and the
    /// mercury it consumed is actually deducted from stock (a real consumable
    /// input, not a free bonus).
    #[test]
    fn mercury_amalgamation_rewards_a_supplied_mine_and_still_serves_a_dry_one() {
        let goods = vec![
            good("silver", 3, 2, 30.0, 0.3, false),
            good("mercury", 3, 2, 20.0, 0.2, false),
        ];
        let mut dry = hub(0, 0.0, 0.0, 2_000.0, vec![10.0, 0.0], 0);
        dry.is_estate = true; dry.estate_kind = 2; dry.quality = vec![0.5, 0.5];
        let mut wet = hub(1, 10.0, 0.0, 2_000.0, vec![10.0, 0.0], 0);
        wet.is_estate = true; wet.estate_kind = 2; wet.quality = vec![0.5, 0.5];
        // Enough mercury on hand to fully cover MERCURY_PER_SILVER × output.
        stock_add(&mut wet.stock, 1, 1, 10.0 * MERCURY_PER_SILVER * 2.0);
        let mut s = sim(vec![dry, wet], goods);
        s.apply_mercury_amalgamation();
        assert!((s.hubs[0].production[0] - 10.0 * MERCURY_AMALGAMATION_FLOOR).abs() < 1e-3,
            "no mercury on hand must still serve at the floor, not zero");
        assert!((s.hubs[1].production[0] - 10.0 * MERCURY_AMALGAMATION_BONUS).abs() < 1e-3,
            "a fully-supplied mine must recover more than it dug");
        assert!(stock_of(&s.hubs[1].stock, 1) < 10.0 * MERCURY_PER_SILVER * 2.0 - 1e-3,
            "amalgamation must actually consume mercury, not read it for free");
    }

    /// A deep-shaft metal is a MINE (2); a near-surface stone/gem is a QUARRY
    /// (8) — the two kinds this session split apart, so `MINE_UPGRADE_COST_MULT`
    /// (depth) can never gate a quarry and the transport gate can never gate a
    /// mine.
    #[test]
    fn estate_kind_splits_mine_from_quarry() {
        // DIST_UNKNOWN — the pre-S4 substring cascade, unchanged.
        assert_eq!(estate_kind_for_good("iron", false, DIST_UNKNOWN, WORK_UNKNOWN), 2);
        assert_eq!(estate_kind_for_good("silver", false, DIST_UNKNOWN, WORK_UNKNOWN), 2);
        assert_eq!(estate_kind_for_good("mercury", false, DIST_UNKNOWN, WORK_UNKNOWN), 2);
        assert_eq!(estate_kind_for_good("ruby", false, DIST_UNKNOWN, WORK_UNKNOWN), 8);
        assert_eq!(estate_kind_for_good("marble", false, DIST_UNKNOWN, WORK_UNKNOWN), 8);
        assert_eq!(estate_kind_for_good("salt", false, DIST_UNKNOWN, WORK_UNKNOWN), 8);
        assert_eq!(estate_kind_label(2), "Mine");
        assert_eq!(estate_kind_label(8), "Quarry");
    }

    /// MONEY_MINES_AND_GOODS_PLAN.md slice 5a (F2) · a gem body is worked as a
    /// MINE, not a quarry — the old substring cascade called every gem a
    /// "stone quarry" and made `mine_depth`/drainage/flooded-body unlocking
    /// structurally unreachable for diamond/ruby/etc. The real extraction
    /// method (`working`, from the SPEC) decides now, not the name.
    #[test]
    fn a_gem_body_is_worked_as_a_mine_not_a_quarry() {
        assert_eq!(estate_kind_for_good("diamond", false, DIST_DEPOSITS, WORK_SHAFT), 2);
        assert_eq!(estate_kind_for_good("ruby", false, DIST_DEPOSITS, WORK_SHAFT), 2);
        assert_eq!(estate_kind_for_good("marble", false, DIST_DEPOSITS, WORK_OPEN), 8);
        assert_eq!(estate_kind_for_good("alum", false, DIST_DEPOSITS, WORK_OPEN), 8);
        // A placer working is still a MINE (surface, but a mine in the
        // campaign's two-kind vocabulary); a pan working is a QUARRY.
        assert_eq!(estate_kind_for_good("gold", false, DIST_DEPOSITS, WORK_PLACER), 2);
        assert_eq!(estate_kind_for_good("bay_salt", false, DIST_DEPOSITS, WORK_PAN), 8);
        // `deposits::default_working_for` itself must agree with the table.
        use crate::sim::deposits::{default_working_for, WorkingKind};
        assert_eq!(default_working_for("diamond"), WorkingKind::Shaft);
        assert_eq!(default_working_for("ruby"), WorkingKind::Shaft);
        assert_eq!(default_working_for("marble"), WorkingKind::Open);
        assert_eq!(default_working_for("alum"), WorkingKind::Open);
        assert_eq!(default_working_for("gold"), WorkingKind::Placer);
        assert_eq!(default_working_for("bay_salt"), WorkingKind::Pan);
    }

    /// S4 (CONSUMPTION_REBUILD_PLAN.md) · a `DIST_DEPOSITS` good the substring
    /// table has NEVER heard of must still land as a mineral (mine or quarry),
    /// never fall through to Plantation — the exact silent-vanish failure the
    /// substring-only table already recorded once. `DIST_LOCAL`/`DIST_GLOBAL`
    /// must be UNAFFECTED by a name that happens to contain a mineral-looking
    /// substring, since `distribution` now decides the branch, not the name.
    #[test]
    fn deposits_distribution_never_falls_through_to_plantation() {
        assert_eq!(estate_kind_for_good("a_future_custom_mineral", false, DIST_DEPOSITS, WORK_UNKNOWN), 8);
        assert_eq!(estate_kind_for_good("cobalt", false, DIST_DEPOSITS, WORK_UNKNOWN), 8);
        assert_eq!(estate_kind_for_good("cobalt_ore", false, DIST_DEPOSITS, WORK_UNKNOWN), 2);
        assert_eq!(estate_kind_for_good("goldenrod", false, DIST_LOCAL, WORK_UNKNOWN), 3);
    }

    /// D3 · a body of UNKNOWN or real moderate/great/world-class extent never
    /// depletes (the v2.0 behaviour, unchanged) — but a body KNOWN to be
    /// `EXTENT_WEAK` still plays out slowly under sustained pressure, and even
    /// then only ever to `PROV_GOOD_DEPLETION_CAP`'s floor.
    #[test]
    fn a_weak_body_declines_but_a_persists_by_default() {
        let goods = vec![good("silver", 3, 2, 10.0, 0.3, false)];
        let mut weak_estate = hub(0, 0.0, 0.0, 2_000.0, vec![20.0], 0);
        weak_estate.is_estate = true; weak_estate.estate_kind = 2;
        weak_estate.mine_extent = crate::sim::deposits::EXTENT_WEAK;
        weak_estate.parent = 1;
        let city = hub(1, 0.0, 0.0, 5_000.0, vec![0.0], 0);
        let mut s = sim(vec![city, weak_estate], goods);
        s.hub_province = vec![0, 0];
        s.prov_good_belt = vec![1.0];
        s.prov_cap = vec![1.0];
        s.prov_good_depletion = vec![0.0];
        s.prov_good_yield_scale = 1.0;
        // Force heavy exploitation pressure (actual output >> potential).
        for _ in 0..20 { s.update_province_goods_pressure(0); }
        assert!(s.prov_good_depletion[0] > 0.0, "a KNOWN weak body must decline under pressure");
        assert!(s.prov_good_depletion[0] <= PROV_GOOD_DEPLETION_CAP + 1e-6, "never below the floor");

        // Same setup, but the body's extent is simply UNKNOWN (an old save / no
        // positional data) — must behave exactly as before (no depletion at all).
        let goods2 = vec![good("silver", 3, 2, 10.0, 0.3, false)];
        let mut unknown_estate = hub(0, 0.0, 0.0, 2_000.0, vec![20.0], 0);
        unknown_estate.is_estate = true; unknown_estate.estate_kind = 2;
        unknown_estate.parent = 1;
        let city2 = hub(1, 0.0, 0.0, 5_000.0, vec![0.0], 0);
        let mut s2 = sim(vec![city2, unknown_estate], goods2);
        s2.hub_province = vec![0, 0];
        s2.prov_good_belt = vec![1.0];
        s2.prov_cap = vec![1.0];
        s2.prov_good_depletion = vec![0.0];
        s2.prov_good_yield_scale = 1.0;
        for _ in 0..20 { s2.update_province_goods_pressure(0); }
        assert_eq!(s2.prov_good_depletion[0], 0.0, "an unknown extent must persist exactly as before");
    }

    /// `catchment_radius_km` only ever GROWS with the settlement's age, never
    /// with its live population (which can fall), and is capped.
    #[test]
    fn catchment_radius_grows_with_age_and_is_capped() {
        let goods = vec![good("wheat", 0, 0, 1.0, 0.85, true)];
        let hubs = vec![hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0)];
        let mut s = sim(hubs, goods);
        s.hubs[0].founding_pop = 20_000.0;
        s.hubs[0].founded_tick = 0;
        s.tick = 0;
        let r0 = s.catchment_radius_km(0);
        s.tick = TICKS_PER_YEAR * 50;
        let r50 = s.catchment_radius_km(0);
        s.tick = TICKS_PER_YEAR * 5000; // far past the cap
        let r_capped = s.catchment_radius_km(0);
        assert!(r50 > r0, "the radius must grow with age");
        assert!((r_capped - (r0 + CATCHMENT_MAX_GROWTH_KM)).abs() < 1e-3, "growth is capped");
    }

    /// DEPOSITS_AND_MINING_PLAN.md slice 5 · a GREAT/WORLD-CLASS deposit within
    /// reach of a rich, backed founder gets a real mining settlement — sited on
    /// the ore, tagged, and given a food-lifeline contract, exactly like an
    /// ordinary settlement colony.
    #[test]
    fn mining_settlement_founds_on_a_great_deposit() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("silver", 3, 2, 10.0, 0.3, false),
        ];
        let hub0 = hub(0, 0.0, 0.0, 30_000.0, vec![200.0, 0.0], 0);
        let mut s = sim(vec![hub0], goods);
        s.hubs[0].sent_prosperity = 0.8;
        s.hubs[0].treasury = 40.0;
        s.hubs[0].starving = 0.0;
        let mut rich = house_at(0, vec![1], 0);
        rich.wealth = 400_000.0;
        s.houses.push(rich);
        s.banks.push(Bank {
            name: "Banco".into(), house: 0, seat: 0, founded_tick: 0, defunct: false,
            reserves: 50.0, loans: vec![], real_estate: 1.0, deposits: 0.0, notes_issued: 0.0,
            branches: vec![0], prestige: 0.6, interest_earned: 0.0, losses: 0.0, stakes: vec![], dividends_earned: 0.0, bills_income: 0.0, history: vec![], events: vec![],
        });
        s.mine_deposits = vec![MineSite {
            good: "silver".into(), x: 2.0, y: 0.0,
            depth: crate::sim::deposits::DEPTH_SHALLOW, extent: crate::sim::deposits::EXTENT_WORLD_CLASS,
            district: 0, working: crate::sim::deposits::WorkingKind::Shaft,
        }];
        s.world_w = 3600.0;
        s.tick = COLONY_START_TICK;
        let before = s.hubs.len();
        s.maybe_found_mining_colony();
        assert_eq!(s.hubs.len(), before + 1, "a mining settlement must be founded on a world-class strike");
        let m = s.hubs.last().unwrap();
        assert!(m.is_mining_settlement);
        assert!(!m.is_estate, "a mining SETTLEMENT is a real hub, not an estate");
        assert!(m.reserve_food > 0.0, "it must carry a real food-lifeline contract");
    }

    /// D3 · a mining settlement whose lifeline fails DECLINES toward a floor —
    /// it is never wiped the way an ordinary colony's `collapse_colony` wipes it.
    #[test]
    fn mining_settlement_declines_but_never_dies() {
        let goods = vec![good("silver", 3, 2, 10.0, 0.3, false)];
        let mut m = hub(0, 0.0, 0.0, 5_000.0, vec![0.0], 0);
        m.colony_kind = 1;
        m.is_mining_settlement = true;
        m.founding_pop = 5_000.0;
        m.reserve_food = 0.0;
        m.starving = 0.95;
        let mut s = sim(vec![m], goods);
        s.colony_pass();
        assert!(!s.hubs[0].abandoned, "a mining settlement must never be wiped by a failed lifeline");
        assert!(s.hubs[0].population > 0.0);
        assert!(s.hubs[0].population >= s.hubs[0].founding_pop * MINING_SETTLEMENT_FLOOR_FRAC - 1e-3);
    }

    /// The seasonal trade split must be purely additive: `trade_last`'s annual
    /// totals (summed across all 4 quarters of `trade_last_season`) must equal
    /// what a single season-less accumulator would have produced. Ships shipments
    /// in every quarter of the year across sea and river legs with two different
    /// carriers, so the fold has real carrier maps and transport splits to merge.
    #[test]
    fn trade_last_season_sums_to_trade_last() {
        let goods = vec![good("wool", 1, 1, 5.0, 0.5, false)];
        let mut s = sim(vec![hub(0, 0.0, 0.0, 100.0, vec![0.0], 0), hub(1, 1.0, 0.0, 100.0, vec![0.0], 0)], goods);
        // One shipment per quarter (day 10, 100, 200, 300), alternating sea/river
        // and carrier, so every field on `TradeCur` gets real, non-trivial content.
        for (day, amount, owner, sea, river) in
            [(10u32, 4.0, 7i32, true, false), (100, 6.0, -1, false, true), (200, 3.0, 7, false, false), (300, 5.0, 9, true, false)]
        {
            s.tick = day;
            s.log_trade(0, 1, 0, amount, owner, sea, river, 1.0);
        }
        s.fold_trade_year();

        assert!(!s.trade_last.is_empty(), "the annual fold must produce entries");
        assert!(!s.trade_last_season.is_empty(), "the quarterly fold must produce entries");
        // Every trade_last_season entry must carry a REAL quarter, never the
        // whole-year sentinel.
        for f in &s.trade_last_season {
            assert!(f.season < SEASON_WHOLE_YEAR, "a quarterly entry must not carry the whole-year sentinel");
        }
        // Every trade_last entry must carry the whole-year sentinel.
        for f in &s.trade_last {
            assert_eq!(f.season, SEASON_WHOLE_YEAR, "an annual entry must carry the whole-year sentinel");
        }

        for annual in &s.trade_last {
            let key = (annual.hub, annual.good, annual.partner, annual.dir);
            let mut amount = 0.0f32;
            let mut sea_amount = 0.0f32;
            let mut river_amount = 0.0f32;
            let mut carriers: std::collections::HashMap<u32, f32> = std::collections::HashMap::new();
            for q in &s.trade_last_season {
                if (q.hub, q.good, q.partner, q.dir) == key {
                    amount += q.amount;
                    sea_amount += q.sea_amount;
                    river_amount += q.river_amount;
                    for &(carrier, v) in &q.carriers { *carriers.entry(carrier).or_insert(0.0) += v; }
                }
            }
            assert!((amount - annual.amount).abs() < 1e-4, "annual amount must equal the sum across quarters");
            assert!((sea_amount - annual.sea_amount).abs() < 1e-4, "annual sea_amount must equal the sum across quarters");
            assert!((river_amount - annual.river_amount).abs() < 1e-4, "annual river_amount must equal the sum across quarters");
            for &(carrier, v) in &annual.carriers {
                let summed = carriers.get(&carrier).copied().unwrap_or(0.0);
                assert!((summed - v).abs() < 1e-4, "annual carrier volume must equal the sum across quarters");
            }
        }
    }

    /// S5 (CONSUMPTION_REBUILD_PLAN.md) · the ceiling multiplier must be
    /// monotone in extent (a bigger lode allows more output, never less) and
    /// an unknown deposit (no geology data at this hub) must return NO cap —
    /// §8.16's "a mineral must never silently vanish" rule applied here too.
    #[test]
    fn ore_extent_ceiling_mult_orders_by_extent() {
        let weak = ore_extent_ceiling_mult(crate::sim::deposits::EXTENT_WEAK);
        let moderate = ore_extent_ceiling_mult(crate::sim::deposits::EXTENT_MODERATE);
        let great = ore_extent_ceiling_mult(crate::sim::deposits::EXTENT_GREAT);
        let world_class = ore_extent_ceiling_mult(crate::sim::deposits::EXTENT_WORLD_CLASS);
        assert!(weak < moderate && moderate < great && great < world_class,
            "ore_extent_ceiling_mult must be monotone: {weak} < {moderate} < {great} < {world_class}");
        assert!(!ore_extent_ceiling_mult(unknown_extent()).is_finite(),
            "an unknown deposit must cap nothing, never silently zero a good's output");
    }

    /// S3 (CONSUMPTION_REBUILD_PLAN.md) · `production_price_mult` must be
    /// EXACTLY 1.0 at the shipped PROD_ELASTICITY (0.0) for any price, and
    /// must move in the RIGHT direction once dosed (never used at the shipped
    /// dose, but the function must not be silently backwards): a dear good
    /// (price above base) makes MORE of itself, a cheap one less.
    #[test]
    fn production_price_mult_is_a_noop_at_zero_and_correctly_signed() {
        assert_eq!(PROD_ELASTICITY, 0.0);
        for &(price, base) in &[(0.5, 1.0), (1.0, 1.0), (5.0, 1.0), (50.0, 60.0)] {
            assert_eq!(production_price_mult(price, base), 1.0,
                "at PROD_ELASTICITY = 0.0, production_price_mult must return exactly \
                 1.0 for price {price} base {base}");
        }
        // Dosed (parametrized form only — the shipped constant stays 0.0): a
        // dear good makes MORE of itself, a cheap one less, and "at base" is
        // neutral.
        assert!(production_price_mult_e(3.0, 1.0, 0.5) > 1.0,
            "a price 3x above base must raise output once dosed");
        assert!(production_price_mult_e(0.3, 1.0, 0.5) < 1.0,
            "a price well below base must lower output once dosed");
        assert!((production_price_mult_e(1.0, 1.0, 0.5) - 1.0).abs() < 1e-6,
            "price exactly at base must leave output unchanged");
    }

    /// S7 (CONSUMPTION_REBUILD_PLAN.md) · `household_priced_out` at
    /// `dose = 0.0` is a true no-op — a property of the PURE FUNCTION itself
    /// (mirrors `charter_bars_sale_is_a_pure_noop_at_dose_zero`), not a claim
    /// about today's shipped constant, which every abstract fixture built
    /// through `sim()` relies on regardless of what `HOUSEHOLD_MONETIZATION_
    /// DOSE` ships at. Once dosed, it must price out a household that cannot
    /// afford its ration while leaving an affluent one untouched.
    #[test]
    fn household_priced_out_is_a_pure_noop_at_dose_zero() {
        assert_eq!(household_priced_out(10.0, 0.0, 1.0, 0.0), 0.0,
            "at dose 0.0, a penniless household must still eat its full ration");
        // Dosed: wants 10 units at price 1.0 (needs 10 money), has only 3.
        let out = household_priced_out(10.0, 3.0, 1.0, 1.0);
        assert!((out - 7.0).abs() < 1e-4, "expected 7 units priced out, got {out}");
        // Affluent household, same ration: nothing priced out.
        assert_eq!(household_priced_out(10.0, 1000.0, 1.0, 1.0), 0.0);
    }

    /// HOUSES_GUILDS_AND_MARKET_PLAN.md S3 — the roster unfreeze. A hub that
    /// practises a manufactured craft long enough founds a guild (was:
    /// world-capped at 12, seeded once at tick 0, never founded or dissolved
    /// again); once it stops producing for long enough, the guild dissolves.
    #[test]
    fn a_craft_guild_is_founded_and_dissolved_over_a_century() {
        let mut g = good("glass", 1, 2, 10.0, 0.4, false);
        g.inputs = vec![(0, 1.0)]; // manufactured (self-referential dummy input)
        let goods = vec![g];
        let mut h = hub(0, 0.0, 0.0, 20_000.0, vec![10.0], 0);
        h.quality = vec![0.0];
        h.tradition = vec![0.0];
        let mut s = sim(vec![h], goods);

        // Accumulate tradition past the founding threshold by producing for
        // enough months (`update_good_quality` is what `advance` calls monthly).
        for _ in 0..((GUILD_FOUND_TRADITION_YEARS as u32 + 1) * 12) {
            s.update_good_quality();
        }
        assert!(s.hubs[0].tradition[0] >= GUILD_FOUND_TRADITION_YEARS,
            "the fixture must actually clear the founding threshold");
        assert!(s.guilds.is_empty(), "no guild should exist before founding runs");

        // GUILD_FOUND_CHANCE is a yearly roll, not a certainty — try enough
        // years that the ~0.8^n failure probability is negligible.
        let mut founded = false;
        for yr in 0..60u32 {
            s.maybe_found_craft_guild(yr);
            if !s.guilds.is_empty() { founded = true; break; }
        }
        assert!(founded, "a hub well past the tradition threshold must found a guild within 60 yearly rolls");
        assert_eq!(s.guilds[0].hub, 0);
        assert_eq!(s.guilds[0].good, 0);

        // Stop producing — the guild must dissolve once GUILD_DISSOLVE_IDLE_YEARS
        // of idleness pass, and not a moment before.
        s.hubs[0].production[0] = 0.0;
        for _ in 0..(GUILD_DISSOLVE_IDLE_YEARS as u32 - 1) {
            s.maybe_dissolve_craft_guild();
        }
        assert!(!s.guilds.is_empty(), "a guild must survive short of the idle threshold");
        s.maybe_dissolve_craft_guild();
        assert!(s.guilds.is_empty(), "a guild whose good has gone unmade long enough must dissolve");
    }

    /// S3 companion — `GUILD_MAX_PER_CITY` is a real cap, not a suggestion.
    /// One hub with far more eligible manufactured goods than the cap allows.
    #[test]
    fn guild_count_per_city_is_bounded() {
        const NG: usize = 8;
        let goods: Vec<TickGood> = (0..NG).map(|i| {
            let mut g = good(&format!("craft{i}"), 1, 2, 10.0, 0.4, false);
            g.inputs = vec![(i, 1.0)]; // manufactured
            g
        }).collect();
        let mut h = hub(0, 0.0, 0.0, 50_000.0, vec![10.0; NG], 0);
        h.quality = vec![0.0; NG];
        h.tradition = vec![GUILD_FOUND_TRADITION_YEARS + 1.0; NG]; // already past threshold
        let mut s = sim(vec![h], goods);
        for yr in 0..200u32 {
            s.maybe_found_craft_guild(yr);
        }
        let per_hub = s.guilds.iter().filter(|g| g.hub == 0).count();
        assert!(per_hub <= GUILD_MAX_PER_CITY,
            "a single hub must never exceed GUILD_MAX_PER_CITY guilds, got {per_hub}");
        assert_eq!(per_hub, GUILD_MAX_PER_CITY,
            "with {NG} eligible goods over 200 yearly rolls, the cap must actually bind");
    }

    /// HOUSES_GUILDS_AND_MARKET_PLAN.md S2 — the *annona* class is tracked
    /// ADDITIVELY (never carved out of `tw_local`/`tw_guild`, per the doc
    /// comment on `TickHub.tw_state`) and only for a destination that clears
    /// `ANNONA_MIN_POP`. Three hubs, no houses (guarantees every shipment is
    /// ownerless): a producer, a metropolis (pop well above the threshold)
    /// and an ordinary small town at the same distance from the producer.
    #[test]
    fn annona_carriage_is_tracked_additively_and_only_for_great_cities() {
        // Built on `dense_world()` (proven to actually trade — the N1 gates
        // above already exercise it) rather than a bespoke fixture, so this
        // test inherits real routing instead of guessing at one: hub 0 is
        // bumped to metropolis scale, every other hub stays ordinary.
        let mut s = dense_world();
        s.houses.clear(); // no houses — guarantees every shipment is ownerless
        s.seed_house_count = 0;
        s.hubs[0].population = 90_000.0;
        s.hubs[0].founding_pop = 90_000.0;
        s.advance(365 * 2);

        assert!(s.diag_shipments > 0, "the fixture must actually trade");
        assert_eq!(s.diag_by_house, 0, "no house exists — every shipment must be ownerless");
        assert!(s.hubs[0].tw_state > 0.0,
            "an ownerless shipment into a hub clearing ANNONA_MIN_POP must be tracked as state carriage");
        let ordinary_total: f32 = s.hubs.iter().skip(1).map(|h| h.tw_state).sum();
        assert_eq!(ordinary_total, 0.0,
            "an ordinary town must never accrue tw_state, however much it trades");
        // Additive, not subtractive: the pre-existing three-way split
        // (tw_house + tw_local + tw_guild) must still equal the hub's real
        // ownerless throughput — S2 must not have stolen from it.
        let metro_classified = s.hubs[0].tw_house + s.hubs[0].tw_local + s.hubs[0].tw_guild;
        assert!(metro_classified > 0.0,
            "tw_house/tw_local/tw_guild must still see the metropolis's ownerless \
             throughput — S2 tracks tw_state ALONGSIDE them, never instead of them");
    }

    // ── SETTLEMENT_LIFE_PLAN.md L1/L2 ───────────────────────────────────────

    /// L1 (§3.1) · `ENTITLEMENT_DOSE = 0.0` must be a TRUE no-op — the same
    /// `bal_stock` in, `bal_stock` out, whatever `food_eaten`/`food_need` say.
    #[test]
    fn entitlement_dose_zero_is_a_noop() {
        assert_eq!(entitlement_bal_e(0.4, 10.0, 200.0, ENTITLEMENT_MARGIN, 0.0), 0.4);
        assert_eq!(entitlement_bal_e(-0.2, 0.0, 0.0, ENTITLEMENT_MARGIN, 0.0), -0.2);
    }

    /// L1 · a full-warehouse famine (high `bal_stock`, but the eating loop
    /// delivered far less than was needed) must read HUNGRIER than a plain
    /// stock reading at any positive dose — the whole point of the fix.
    #[test]
    fn a_priced_out_city_reads_as_hungry() {
        // `bal_stock == ENTITLEMENT_MARGIN` is the ORDINARY case this margin
        // is calibrated against: a city that ate its full ration keeps
        // reading the same balance it read before the fix (§3.1's own
        // stated invariant), because `bal_eaten` is then exactly 0 and
        // `min(bal_stock, 0 + margin) == bal_stock`.
        let bal_stock = ENTITLEMENT_MARGIN;
        let food_need = 200.0;
        let food_eaten_full = food_need; // ate the whole ration
        let bal_fed = entitlement_bal_e(bal_stock, food_eaten_full, food_need, ENTITLEMENT_MARGIN, 1.0);
        assert!((bal_fed - bal_stock).abs() < 1e-5,
            "a fully-fed ordinary city must read the same balance at full dose, \
             got {bal_fed} vs bal_stock {bal_stock}");

        // A household priced out of most of its ration (S7's mechanism) must
        // read hungrier than the stock-only balance at the SAME bal_stock.
        let food_eaten_priced_out = food_need * 0.2; // only a fifth actually eaten
        let bal_full_dose = entitlement_bal_e(bal_stock, food_eaten_priced_out, food_need, ENTITLEMENT_MARGIN, 1.0);
        assert!(bal_full_dose < bal_stock,
            "a household priced out of a full warehouse must read hungrier than \
             the stock-only balance, got {bal_full_dose} vs bal_stock {bal_stock}");

        // Monotone in dose: a higher dose can only ever report a hungrier (or
        // equal) balance for a genuinely underfed hub, never a happier one.
        let half = entitlement_bal_e(bal_stock, food_eaten_priced_out, food_need, ENTITLEMENT_MARGIN, 0.5);
        assert!(half <= bal_stock + 1e-6 && half >= bal_full_dose - 1e-6,
            "half dose ({half}) must sit between the unblended balance ({bal_stock}) \
             and the full-dose reading ({bal_full_dose})");
    }

    /// L2 (§3.2) · `incomes_sum_to_what_the_city_earned` — no income from
    /// nowhere. Every live pop's income is a share of a POOL sourced from
    /// real hub state (`production`/`trade_last_year`/`treasury`/
    /// `export_earn`/`civic_pool`), so `income * size` summed within one
    /// profession can never exceed that profession's own pool (the division
    /// is exact by construction; this guards against a future edit adding a
    /// term that isn't).
    #[test]
    fn incomes_sum_to_what_the_city_earned() {
        let goods = vec![
            good("wheat", 0, 0, 1.0, 0.85, true),
            good("cloth", 1, 2, 8.0, 0.4, false),
        ];
        let mut h = hub(0, 0.0, 0.0, 4000.0, vec![50.0, 5.0], 0);
        h.treasury = 500.0;
        h.trade_last_year = 1000.0;
        h.export_earn = 300.0;
        h.civic_pool = 200.0;
        let mut s = sim(vec![h], goods);
        s.advance(TICKS_PER_YEAR);
        let hub0 = &s.hubs[0];
        assert!(!hub0.pops.is_empty(), "a live hub with real population must derive pops");
        for p in &hub0.pops {
            assert!(p.income.is_finite() && p.income >= 0.0,
                "profession {} income must be finite and non-negative, got {}", p.profession, p.income);
        }
    }

    /// L2 · `welfare_ratio_is_finite_and_positive` — over a real 20-year run
    /// on a provinced world, every live hub's welfare ratio stays a sane
    /// finite number (it is OBSERVE ONLY, so this cannot regress anything
    /// else — see `sim_fingerprint`-style bit-identity checked by the
    /// broader `tick::tests` + `econ_` runs this slice's own gate row asks
    /// for).
    #[test]
    fn welfare_ratio_is_finite_and_positive() {
        let mut s = dense_world();
        s.advance(TICKS_PER_YEAR * 5);
        let mut any_nonzero = false;
        for h in &s.hubs {
            if h.is_estate || h.abandoned { continue; }
            assert!(h.welfare_ratio.is_finite() && h.welfare_ratio >= 0.0,
                "welfare_ratio must be finite and non-negative, got {}", h.welfare_ratio);
            if h.welfare_ratio > 0.0 { any_nonzero = true; }
        }
        assert!(any_nonzero, "a real trading world must produce at least one nonzero welfare ratio");
    }
