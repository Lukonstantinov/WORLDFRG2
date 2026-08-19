// Split from the former monolithic src/bridge/tauri.ts (invoke wrappers, one per Rust command).
import { invoke } from "@tauri-apps/api/core";
import type { BankBrief, CampaignDiagnostics, CampaignSnapshot, CityPriceIndex, CityRank, CitySchematic, CityWarehouseInfo, CoinSnapshot, CoinUseCity, ColonyDetail, ColonyGateStatus, ColonySummary, CrashRecord, CrisisBrief, CultureBrief, CulturePresenceGrid, CurrencyBrief, DynastiesPayload, EpidemicBrief, EraFrame, ExpeditionsPayload, FeudRow, FigureBrief, FuturesLane, GoalsBrief, GoodAtlas, GoodMarketRow, GuildBrief, HouseBrief, HouseHistory, HouseLedger, HouseLineage, HouseStability, HubDetail, InequalitySnapshot, JournalEntry, KinBrief, LandmarkBrief, MerchantRoute, MigrationRouteBrief, MintBrief, MonetaryEvent, NotablePerson, PolisBrief, PopBrief, ProvinceLand, ProvisioningBrief, ReservesPayload, SatelliteBrief, SpecCenter, TradeBasin, TradeCorridor, TradeFlows, TradeTrunk, WarehouseInfo, WarsPayload, WorksCardInfo, WorldEconomy, CampaignFileInfo, WorldHumanLayerStatus, ProvinceRepairReport } from "@types";

/** DLC 3.5 · the live campaign's dynamic trade-flow trunks (last year's actual
 *  shipped volume, routed over the cost grid + bundled; width ∝ volume). */
export async function campaignGetTradeFlow(
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
): Promise<TradeTrunk[]> {
  return invoke("campaign_get_trade_flow", {
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
  });
}

export async function campaignGetCorridors(
  rivers: { points: [number, number][] }[],
  reach: number,
  maxCrossing: number,
): Promise<TradeCorridor[]> {
  return invoke("campaign_get_corridors", {
    riversJson: JSON.stringify(rivers),
    reach,
    maxCrossing,
  });
}

/** Live financed expeditions crawling toward distant lands + recent failed ✕'s. */
export async function campaignGetExpeditions(): Promise<ExpeditionsPayload> {
  return invoke("campaign_get_expeditions");
}

// ── DLC 1 "Living Trade" tick simulation ──

/** Seed a fresh living-trade sim from the static economy snapshot (step 10). A RUNNING
 *  campaign is never restarted by this — it returns the current sim unchanged. */
export async function campaignStartSim(seed: number): Promise<CampaignSnapshot> {
  return invoke("campaign_start_sim", { seed });
}

/** Start a FRESH dynamic campaign on the same world/economy (a "new game"). Clears the
 *  current sim and reseeds — the caller must first SAVE the running campaign to its own
 *  .campaign file so it is preserved. */
export async function campaignNewGame(seed: number): Promise<CampaignSnapshot> {
  return invoke("campaign_new_game", { seed });
}

/** COLD START: zero the just-started campaign's entire economy (houses, guilds, banks,
 *  coinage, warehouses, wealth, institutions) and reset every city to a small seed, so on
 *  unpause the world builds its trade network and cities up from nothing. Only valid on a
 *  fresh, unadvanced campaign (tick 0). */
export async function campaignColdStart(): Promise<CampaignSnapshot> {
  return invoke("campaign_cold_start", {});
}

/** Advance the sim by N days. The backend keeps the sim resident in memory and
 *  autosaves on a year/wall-clock cadence — call `campaignPersist` to force a flush. */
export async function campaignAdvance(ticks: number): Promise<CampaignSnapshot> {
  return invoke("campaign_advance", { ticks });
}

/** Force-flush the resident sim to disk (call on pause / before close). */
export async function campaignPersist(): Promise<void> {
  return invoke("campaign_persist");
}

/** Current sim snapshot (inactive when no sim has been started). */
export async function campaignGetState(): Promise<CampaignSnapshot> {
  return invoke("campaign_get_state");
}

/** Journal rows, filtered by hub and/or good (-1 = any). */
export async function campaignGetJournal(hub: number, good: number): Promise<JournalEntry[]> {
  return invoke("campaign_get_journal", { hub, good });
}

/** Atlas 2.0 · named trade basins clustered from the yearly flow ledger. */
export async function campaignGetTradeBasins(): Promise<TradeBasin[]> {
  return invoke("campaign_get_trade_basins");
}

/** Batch 1 · per-good Trade Heat points [x, y, volume] (good by name). */
export async function campaignGetGoodHeat(good: string): Promise<[number, number, number][]> {
  return invoke("campaign_get_good_heat", { good });
}

/** Batch 1 · era scrubber: the world at the end of `year` (null = not in ring). */
export async function campaignGetEraFrame(year: number): Promise<EraFrame | null> {
  return invoke("campaign_get_era_frame", { year });
}

/** #1/#23 · per-culture world census for the Peoples panel. */
export async function campaignGetCultures(): Promise<CultureBrief[]> {
  return invoke("campaign_get_cultures");
}

/** Coarse "where this people lives" raster for the Peoples-panel mini-map. */
export async function campaignGetCulturePresence(name: string): Promise<import("@types").CulturePresenceGrid> {
  return invoke("campaign_get_culture_presence", { name });
}

/** Notable people (merchant magnates / dynastic heads) of one people. */
export async function campaignGetCultureNotables(name: string): Promise<import("@types").NotablePerson[]> {
  return invoke("campaign_get_culture_notables", { name });
}

/** 6-monthly population series [year, population] for one people (line chart). */
export async function campaignGetCultureHistory(name: string): Promise<[number, number][]> {
  return invoke("campaign_get_culture_history", { name });
}

/** #1/#23 · per-hub share `[x,y,share]` of ONE culture for the map overlay. */
export async function campaignCultureHubs(name: string): Promise<[number, number, number][]> {
  return invoke("campaign_culture_hubs", { name });
}

/** Full live detail for one settlement (sentiment + market + history), or null
 *  when no campaign sim is running / the hub isn't in it. */
export async function campaignGetHub(id: number): Promise<HubDetail | null> {
  return invoke("campaign_get_hub", { id });
}

/** Colony detail (Supply subtab) — null for non-colony hubs. */
export async function campaignGetColony(id: number): Promise<ColonyDetail | null> {
  return invoke("campaign_get_colony", { id });
}

/** Live construction state for a satellite still being built — null once finished. */
export async function campaignGetSatellite(id: number): Promise<SatelliteBrief | null> {
  return invoke("campaign_get_satellite", { id });
}

/** Route-bound migration flows (polylines along the trade network) for the overlay. */
export async function campaignGetMigrationRoutes(): Promise<MigrationRouteBrief[]> {
  return invoke("campaign_get_migration_routes");
}

/** Council right-of-first-buy / provisioning state for a city (Provisioning tab). */
export async function campaignGetProvisioning(id: number): Promise<ProvisioningBrief | null> {
  return invoke("campaign_get_provisioning", { id });
}

/** Empire-wide colony roster (settlement colonies + house outposts). */
export async function campaignGetColonies(): Promise<ColonySummary[]> {
  return invoke("campaign_get_colonies");
}

/** Read-only colony-founding gate status (the "why none yet?" diagnostics). */
export async function campaignColonyGates(): Promise<ColonyGateStatus | null> {
  return invoke("campaign_colony_gates");
}

/** World-economy panel data (per-good world prices + price-index series). */
export async function campaignGetWorldEconomy(): Promise<WorldEconomy> {
  return invoke("campaign_get_world_economy");
}

/** #30 · live per-city cost-of-living basket index from the running campaign. */
export async function campaignCityPriceIndex(): Promise<import("@types").CityPriceIndex[]> {
  return invoke("campaign_city_price_index");
}

/** All merchant families (active first, richest first). */
export async function campaignGetHouses(): Promise<HouseBrief[]> {
  return invoke("campaign_get_houses");
}

/** #29 · wealth-inequality (Gini) + social-mobility snapshot from the live sim. */
export async function campaignGetInequality(): Promise<import("@types").InequalitySnapshot> {
  return invoke("campaign_get_inequality");
}

/** DLC 4 · the derived typed Pops of one hub (Nations & POPs foundation). */
export async function campaignGetPops(hub: number): Promise<PopBrief[]> {
  return invoke("campaign_get_pops", { hub });
}

/** DLC 3 · the cached yearly speculation read (per-polis bubble risk + why). */
export async function campaignGetSpeculation(): Promise<SpecCenter[]> {
  return invoke("campaign_get_speculation");
}

/** DLC 3 · the poleis as actors (treasury / tariff / mint / council). */
export async function campaignGetPoleis(): Promise<PolisBrief[]> {
  return invoke("campaign_get_poleis");
}

/** Realized trade flows at a settlement (per-good volumes + history, routes,
 *  top partner cities) for the Trade ▸ Flows subtab. Null if no campaign. */
export async function campaignTradeFlows(id: number): Promise<TradeFlows | null> {
  return invoke("campaign_trade_flows", { id });
}

/** DLC 3.5 · the world's coinage ranked by reserve strength (trust × throughput). */
export async function campaignGetCurrencies(): Promise<CurrencyBrief[]> {
  return invoke("campaign_get_currencies");
}

/** v2.0 · every polis/mint fused (polis + coin) for the unified Coin & Mints tab. */
export async function campaignGetMints(): Promise<MintBrief[]> {
  return invoke("campaign_get_mints");
}

/** v2.0 · the monetary chronicle (mints, debasements, reforms, runs, crashes), newest first. */
export async function campaignMonetaryChronicle(): Promise<MonetaryEvent[]> {
  return invoke("campaign_monetary_chronicle");
}

/** A3 · one coin's yearly biography (fineness/trust/value/price series), oldest→newest. */
export async function campaignCoinHistory(hub: number): Promise<CoinSnapshot[]> {
  return invoke("campaign_coin_history", { hub });
}

/** v2.0 · currency reserves per holder (cities / banks / houses) for the Reserves donuts. */
export async function campaignReserves(): Promise<ReservesPayload> {
  return invoke("campaign_reserves");
}

/** Per-city coin usage: which coin each settlement settles its trade in + volume —
 *  for the coin-usage map overlay and the per-coin donut/bar breakdown. */
export async function campaignCoinUsage(): Promise<CoinUseCity[]> {
  return invoke("campaign_coin_usage");
}

/** DLC 3.5 · all chartered banks with their balance sheets. */
export async function campaignGetBanks(): Promise<BankBrief[]> {
  return invoke("campaign_get_banks");
}

/** DLC 3.5 · the log of regional financial crashes (newest first). */
export async function campaignGetCrashes(): Promise<CrashRecord[]> {
  return invoke("campaign_get_crashes");
}

/** DLC 3.5 · active economic wars + the concluded-war log. */
export async function campaignGetWars(): Promise<WarsPayload> {
  return invoke("campaign_get_wars");
}

/** Phase 6 · plagues & epidemics grouped into outbreaks (active-first, deadliest). */
export async function campaignGetEpidemics(): Promise<EpidemicBrief[]> {
  return invoke("campaign_get_epidemics");
}

/** Phase 6 · craft guilds (good, quality, output, strength, guildhall). */
export async function campaignGetGuilds(): Promise<GuildBrief[]> {
  return invoke("campaign_get_guilds");
}

/** Phase 6 · notable figures (Great Lives roster). */
export async function campaignGetFigures(): Promise<FigureBrief[]> {
  return invoke("campaign_get_figures");
}

/** Phase 6 · landmarks & sacred sites (wonders, holy cities, fairs, guildhalls). */
export async function campaignGetLandmarks(): Promise<LandmarkBrief[]> {
  return invoke("campaign_get_landmarks");
}

/** Phase 7 · dynasties: marriage alliances + feuds between houses. */
export async function campaignGetDynasties(): Promise<DynastiesPayload> {
  return invoke("campaign_get_dynasties");
}

/** DLC 4 · every good's quality rating + produced/traded totals (Goods window). */
export async function campaignGetGoods(): Promise<GoodMarketRow[]> {
  return invoke("campaign_get_goods");
}

/** Goods Atlas · everything about ONE good (index): quality distribution, trade volume
 *  with producers/consumers, control by house/guild, and the per-good yearly flow lanes. */
export async function campaignGoodAtlas(good: string): Promise<GoodAtlas> {
  return invoke("campaign_good_atlas", { good });
}

/** DLC 3.5 · per-city schematics (buildings / estates / banks / coin). */
export async function campaignGetSchematics(): Promise<CitySchematic[]> {
  return invoke("campaign_get_schematics");
}

/** A house/guild's yearly T-account ledger (Accountant view). */
export async function campaignHouseLedger(house: number): Promise<HouseLedger | null> {
  return invoke("campaign_house_ledger", { house });
}

/** Active merchant routes (per family/guild, aggregated) for the map layer. */
export async function campaignMerchantRoutes(): Promise<MerchantRoute[]> {
  return invoke("campaign_merchant_routes");
}

/** Active futures contracts as directional supply lanes for the Futures map layer. */
export async function campaignFuturesLanes(): Promise<FuturesLane[]> {
  return invoke("campaign_futures_lanes");
}

/** All house/guild warehouses (largest stock first) for the Warehouses panel. */
export async function campaignWarehouses(): Promise<WarehouseInfo[]> {
  return invoke("campaign_warehouses");
}

/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.3 · the CITY's own warehouse (D17) —
 *  null for an estate or an unknown hub. */
export async function campaignCityWarehouse(hub: number): Promise<CityWarehouseInfo | null> {
  return invoke("campaign_city_warehouse", { hub });
}

/** ESTATES_SHARES_AND_WAREHOUSE_PLAN.md 4.6 · one works card (D15/D16/D2) —
 *  null for a non-estate or unknown hub. */
export async function campaignWorksCard(hub: number): Promise<WorksCardInfo | null> {
  return invoke("campaign_works_card", { hub });
}

/** Live richest-cities ranking with each city's share of world trade. */
export async function campaignCityRanking(): Promise<CityRank[]> {
  return invoke("campaign_city_ranking");
}

/** One house's full chronicle (timeline) by name. */
export async function campaignGetHouseHistory(name: string): Promise<HouseHistory | null> {
  return invoke("campaign_get_house_history", { name });
}

/** "Is trade actually moving?" snapshot — null when no sim is running. */
export async function campaignDiagnostics(): Promise<CampaignDiagnostics | null> {
  return invoke("campaign_diagnostics");
}

// ── House Dossier: stability gauges + the feud board ────────────────────────

/** The five stability gauges + liability breakdown for one house. */
export async function campaignHouseStability(idx: number): Promise<HouseStability | null> {
  return invoke("campaign_house_stability", { idx });
}

/** Feuds, live first then settled. `house` < 0 = every feud in the world. */
export async function campaignGetFeuds(house = -1): Promise<FeudRow[]> {
  return invoke("campaign_get_feuds", { house });
}

/** This house's kin roster (Phase 2.1) — empty for a guild or a house whose roster
 *  was never generated. */
export async function campaignGetHouseKin(idx: number): Promise<KinBrief[]> {
  return invoke("campaign_get_house_kin", { idx });
}

/** This house's ambitions, active and historical (Phase 3.1). */
export async function campaignGetHouseGoals(idx: number): Promise<GoalsBrief> {
  return invoke("campaign_get_house_goals", { idx });
}

/** This house's succession crisis — the live struggle (if any) plus its permanent
 *  record of past risings (Phase 3.2-3.6). */
export async function campaignGetHouseCrisis(idx: number): Promise<CrisisBrief> {
  return invoke("campaign_get_house_crisis", { idx });
}

/** This house's lineage — the chain it descends from, and what split off it directly. */
export async function campaignGetHouseLineage(idx: number): Promise<HouseLineage> {
  return invoke("campaign_get_house_lineage", { idx });
}

// ── Province land state (FIX_PLAN B1) + the holder's control verbs ──────────

/** One province's live land state, or null with no campaign / no province layer. */
export async function campaignProvinceLand(id: number): Promise<ProvinceLand | null> {
  return invoke("campaign_province_land", { id });
}

/** Every province's land state — what the browser sorts and filters on. */
export async function campaignProvinceLandAll(): Promise<ProvinceLand[]> {
  return invoke("campaign_province_land_all");
}

/** Set a province's rural tax rate. Rejects a province nobody administers. */
export async function campaignSetProvinceTax(id: number, rate: number): Promise<number> {
  return invoke("campaign_set_province_tax", { id, rate });
}

/** Begin a multi-year land improvement, funded by a city treasury or a house. */
export async function campaignStartProvinceWork(
  id: number, kind: number, funderHub = -1, funderHouse = -1,
): Promise<string> {
  return invoke("campaign_start_province_work",
    { id, kind, funderHub, funderHouse });
}

/** Abandon a work in progress. What has been paid is sunk. */
export async function campaignCancelProvinceWork(id: number, kind: number): Promise<void> {
  return invoke("campaign_cancel_province_work", { id, kind });
}

/** §2.5 · every good this province actually produces, with its exploitation
 *  reading (potential/actual/exploitation/depletion/market↔local split). Empty
 *  with no campaign / no province layer / nothing produced here. */
export async function campaignProvinceGoods(id: number): Promise<import("@types").ProvinceGoodExploit[]> {
  return invoke("campaign_province_goods", { id });
}

/** Province trade: who commands this province's commerce (by house/guild and by
 *  city) and what crosses its border (per-good exports/imports, last full year).
 *  Feeds the province view's circular diagrams. Empty/zero with no campaign, no
 *  province layer, or a province where nothing has traded yet. */
export async function campaignProvinceTrade(id: number): Promise<import("@types").ProvinceTrade> {
  return invoke("campaign_province_trade", { id });
}

/** #9 · The full goods PICTURE for a province: every good the land COULD yield (with
 *  belt richness) whether or not it is worked today, plus the individual ore workings
 *  and their grade/depth — so a province producing nothing still shows its potential
 *  and its mineral deposits rather than "no notable produce". */
export async function campaignProvincePotential(id: number): Promise<import("@types").ProvincePotential> {
  return invoke("campaign_province_potential", { id });
}

/** REALM_AND_GOVERNMENT_PLAN.md R1 · every realm currently standing. Reads real
 *  persisted state (`sim.realms`), not a derivation — empty with no campaign, no
 *  province layer, or no realm yet proclaimed (possible only from year 50). */
export async function computeStates(): Promise<import("@types").StateRegion[]> {
  return invoke("compute_states");
}

/** R2 · one realm's family, in insertion order (the founder first). Empty for an
 *  unknown realm id or one whose family hasn't been seeded (should not happen post-
 *  coronation, but degrades rather than erroring). */
export async function campaignGetRealmFamily(realmId: number): Promise<import("@types").PersonBrief[]> {
  return invoke("campaign_get_realm_family", { realmId });
}

// ── The campaign LIBRARY: a real folder of .campaign saves, listed with each
//    save's year so a run can be resumed without hunting through a file dialog. ──

/** Absolute path of the campaigns folder (created on first call). */
export async function campaignLibraryDir(): Promise<string> {
  return invoke("campaign_library_dir");
}

/** Point the library at a different folder; "" restores the default. Returns the
 *  folder now in use. */
export async function setCampaignLibraryDir(path: string): Promise<string> {
  return invoke("set_campaign_library_dir", { path });
}

/** Open the campaigns folder in the OS file manager. */
export async function revealCampaignLibrary(): Promise<void> {
  return invoke("reveal_campaign_library");
}

/** Every .campaign save in the library, newest first. Reads only each file's small
 *  header — never its simulation blob — so a folder of long runs lists instantly. */
export async function listCampaigns(): Promise<CampaignFileInfo[]> {
  return invoke("list_campaigns");
}

/** Save the running campaign into the library folder. Returns the path written. */
export async function saveCampaignToLibrary(name?: string): Promise<string> {
  return invoke("save_campaign_to_library", { name: name ?? null });
}

/** Delete a save from the library (refused for any path outside that folder). */
export async function deleteCampaignFile(path: string): Promise<void> {
  return invoke("delete_campaign_file", { path });
}

/** What the currently open world can do: whether a campaign can start right now,
 *  and if not, what a rebuild has to work with. */
export async function worldHumanLayerStatus(): Promise<WorldHumanLayerStatus> {
  return invoke("world_human_layer_status");
}

/** Re-attach settlements to the EXISTING province partition (membership + seat only;
 *  no geometry, no id changes), after a rebuild changed the settlement set. */
export async function repairProvinceSettlements(
  settlements: { id: string; x: number; y: number; population: number }[],
): Promise<ProvinceRepairReport> {
  return invoke("repair_province_settlements", {
    settlementsJson: JSON.stringify(settlements),
  });
}
