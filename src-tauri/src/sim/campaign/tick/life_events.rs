//! life_events — `docs/living_world/02_PEOPLE.md` §`## Life events` (slices
//! 02.4/02.5). A layered template pool (role+surroundings → surroundings →
//! in-city generic) with a guaranteed-firing generic layer, fired at the
//! weekly cadence hook so a year's count is spread across ticks rather than
//! landing all on New Year's Day.
//!
//! **Scope cut, recorded per 00_INDEX.md rule 36**: turbulence covers war
//! (×3), famine (×1.5), a recent plague strike (×2, the closest signal
//! `tick/` carries — there is no "currently besieged"/"festival active"
//! per-hub flag today), travelling (×2) and holding office (×1.5). Siege and
//! festival multipliers are QUEUED (Q02.5) behind a producing signal for
//! either state; inventing one here would be exactly the fabrication
//! 00_INDEX's own rules forbid.
//!
//! The ~150-template expansion (02.6, a separate session per the design doc)
//! is now shipped alongside the ~40-template starter set, reviewed against
//! `life_event_templates_respect_geography`'s keyword lint and the role list.
use super::*;
use super::individuals::living_world_salts as salts;

/// One life-event template. `text` uses `{name}`/`{city}` placeholders,
/// resolved lazily at read time (`render_life_entry`) — never baked in at
/// fire time, so a renamed city or a translated UI could one day change the
/// rendering without touching the stored log.
pub struct EventTemplate {
    pub id: u16,
    /// Subset of `TAG_*` that must ALL be present for this template to match
    /// (0 = the in-city generic layer, always matches).
    pub requires: u32,
    pub weight: f32,
    /// Traits that make this template more likely to be PICKED once it
    /// qualifies (not a requirement — 00_INDEX "trait weights").
    pub trait_bonus: &'static [u8],
    pub fame_delta: f32,
    pub trait_gain: Option<u8>,
    pub feature_gain: Option<u32>,
    pub text: &'static str,
}

pub static EVENT_TEMPLATES: &[EventTemplate] = &[
    // ── In-city generic (requires = 0 — the guaranteed layer) ──────────
    EventTemplate { id: 1, requires: TAG_GENERIC, weight: 3.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} haggles half the morning over the price of bread in the {city} market." },
    EventTemplate { id: 2, requires: TAG_GENERIC, weight: 2.5, trait_bonus: &[TRAIT_DRUNKARD], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} drinks late into the night at a tavern in {city}." },
    EventTemplate { id: 3, requires: TAG_GENERIC, weight: 2.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches a workshop at its trade in {city}, and lingers." },
    EventTemplate { id: 4, requires: TAG_GENERIC, weight: 2.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} spends a quiet evening at home in {city}." },
    EventTemplate { id: 5, requires: TAG_GENERIC, weight: 1.5, trait_bonus: &[TRAIT_IMPULSIVE, TRAIT_PROUD], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} comes to blows in a street quarrel in {city}." },
    EventTemplate { id: 6, requires: TAG_GENERIC, weight: 1.5, trait_bonus: &[TRAIT_GAMBLER], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} takes on a debt at the gaming table in {city}." },
    EventTemplate { id: 7, requires: TAG_GENERIC, weight: 1.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} attends a neighbour's wedding in {city}." },
    EventTemplate { id: 8, requires: TAG_GENERIC, weight: 1.0, trait_bonus: &[TRAIT_GENEROUS], fame_delta: 0.02, trait_gain: Some(TRAIT_BENEFACTOR), feature_gain: None,
        text: "{name} pays a poor family's arrears out of their own purse in {city}." },
    EventTemplate { id: 9, requires: TAG_GENERIC, weight: 1.0, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} spends an afternoon among the stalls of {city}, asking travellers their news." },
    EventTemplate { id: 10, requires: TAG_GENERIC, weight: 0.8, trait_bonus: &[TRAIT_GLUTTON], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} feasts too well at a friend's table in {city}." },
    // ── Coast ────────────────────────────────────────────────────────
    EventTemplate { id: 20, requires: TAG_COAST, weight: 1.5, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.01, trait_gain: None, feature_gain: None,
        text: "{name} watches whales breach beyond the mole at {city}." },
    EventTemplate { id: 21, requires: TAG_COAST, weight: 1.2, trait_bonus: &[], fame_delta: 0.0, trait_gain: Some(TRAIT_SEAFARER), feature_gain: None,
        text: "{name} takes ship out of {city} and learns the set of a sail." },
    EventTemplate { id: 22, requires: TAG_COAST, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} haggles with a foreign captain newly landed at {city}." },
    EventTemplate { id: 23, requires: TAG_COAST | TAG_TRAVELLING, weight: 0.6, trait_bonus: &[], fame_delta: 0.03, trait_gain: Some(TRAIT_ROBUST), feature_gain: None,
        text: "{name} clings to a spar after their ship founders in sight of {city}, and lives." },
    EventTemplate { id: 24, requires: TAG_COAST, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} buys a share in a fishing boat out of {city}." },
    // ── River ────────────────────────────────────────────────────────
    EventTemplate { id: 30, requires: TAG_RIVER, weight: 1.2, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} crosses the ferry at {city} amid a crowd of drovers." },
    EventTemplate { id: 31, requires: TAG_RIVER, weight: 1.0, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches barges unload grain along the {city} wharves." },
    EventTemplate { id: 32, requires: TAG_RIVER, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is nearly swept off the {city} landing when the river runs high." },
    // ── Desert ───────────────────────────────────────────────────────
    EventTemplate { id: 40, requires: TAG_DESERT, weight: 1.2, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} joins a camel caravan arriving in {city} out of the dunes." },
    EventTemplate { id: 41, requires: TAG_DESERT, weight: 1.0, trait_bonus: &[TRAIT_HUNTER], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} rides out from {city} at dawn before the heat closes in." },
    EventTemplate { id: 42, requires: TAG_DESERT | TAG_TRAVELLING, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: Some(TRAIT_ROBUST), feature_gain: None,
        text: "{name} survives three waterless days on the road to {city}." },
    // ── Steppe ───────────────────────────────────────────────────────
    EventTemplate { id: 50, requires: TAG_STEPPE, weight: 1.2, trait_bonus: &[TRAIT_BRAVE], fame_delta: 0.01, trait_gain: None, feature_gain: None,
        text: "{name} breaks a wild horse on the plain outside {city}." },
    EventTemplate { id: 51, requires: TAG_STEPPE, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} rides with herders across the grass beyond {city}." },
    // ── Cold ─────────────────────────────────────────────────────────
    EventTemplate { id: 60, requires: TAG_COLD, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} waits out a hard frost by the hearth in {city}." },
    EventTemplate { id: 61, requires: TAG_COLD, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses two fingers to frostbite on the road into {city}." },
    // ── Tropical ─────────────────────────────────────────────────────
    EventTemplate { id: 70, requires: TAG_TROPICAL, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shelters from a downpour under the eaves of {city}." },
    EventTemplate { id: 71, requires: TAG_TROPICAL, weight: 0.7, trait_bonus: &[TRAIT_SICKLY], fame_delta: 0.0, trait_gain: Some(TRAIT_SICKLY), feature_gain: None,
        text: "{name} is laid low by a fever common to {city}'s wet season." },
    // ── War / famine ─────────────────────────────────────────────────
    EventTemplate { id: 80, requires: TAG_AT_WAR, weight: 1.5, trait_bonus: &[TRAIT_BRAVE], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} helps man the walls of {city} through a night alarm." },
    EventTemplate { id: 81, requires: TAG_AT_WAR, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches the levy march out of {city} to the war." },
    EventTemplate { id: 82, requires: TAG_AT_WAR | TAG_ROLE_COMMANDER, weight: 0.8, trait_bonus: &[TRAIT_STRATEGIST], fame_delta: 0.04, trait_gain: None, feature_gain: None,
        text: "{name} quells a mutiny in the ranks outside {city}." },
    EventTemplate { id: 83, requires: TAG_FAMINE, weight: 1.5, trait_bonus: &[TRAIT_KIND], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} shares their own store with hungry neighbours in {city}." },
    EventTemplate { id: 84, requires: TAG_FAMINE, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} stands in a bread riot outside the {city} granary." },
    // ── Role-specific ────────────────────────────────────────────────
    EventTemplate { id: 90, requires: TAG_ROLE_ADMIRAL, weight: 1.2, trait_bonus: &[TRAIT_SEAFARER], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} rides out a storm at sea bound for {city}." },
    EventTemplate { id: 91, requires: TAG_ROLE_ADMIRAL | TAG_COAST, weight: 0.7, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses a ship on a reef outside {city}." },
    EventTemplate { id: 92, requires: TAG_ROLE_SCHOLAR, weight: 1.2, trait_bonus: &[TRAIT_SCHOLARLY], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} loses a public debate in {city} before an unkind crowd." },
    EventTemplate { id: 93, requires: TAG_ROLE_SCHOLAR, weight: 1.0, trait_bonus: &[], fame_delta: 0.01, trait_gain: None, feature_gain: None,
        text: "{name} finds a forgotten scroll in a {city} archive." },
    EventTemplate { id: 94, requires: TAG_ROLE_ARTISAN, weight: 1.2, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name}'s patron refuses to pay for work delivered in {city}." },
    EventTemplate { id: 95, requires: TAG_ROLE_PERFORMER, weight: 1.2, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name}'s crowd turns unruly at a show in {city}." },
    EventTemplate { id: 96, requires: TAG_ROLE_SENATOR, weight: 1.2, trait_bonus: &[TRAIT_ORATOR], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name}'s speech turns the vote in {city}'s council." },
    EventTemplate { id: 97, requires: TAG_ROLE_COMMANDER, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name}'s column is ambushed on the march near {city}." },

    // ── 02.6: the ~150-template expansion ───────────────────────────────
    // ── More in-city generic ─────────────────────────────────────────────
    EventTemplate { id: 100, requires: TAG_GENERIC, weight: 1.2, trait_bonus: &[TRAIT_HONEST], fame_delta: 0.01, trait_gain: None, feature_gain: None,
        text: "{name} refuses a bribe offered by a rival in the {city} exchange." },
    EventTemplate { id: 101, requires: TAG_GENERIC, weight: 1.0, trait_bonus: &[TRAIT_DECEITFUL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is caught out in a small lie before the neighbours of {city}." },
    EventTemplate { id: 102, requires: TAG_GENERIC, weight: 1.0, trait_bonus: &[TRAIT_HONEST, TRAIT_KIND], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} mediates a dispute between two households in {city}." },
    EventTemplate { id: 103, requires: TAG_GENERIC, weight: 1.0, trait_bonus: &[TRAIT_GAMBLER], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses a wager they could not afford in {city}." },
    EventTemplate { id: 104, requires: TAG_GENERIC, weight: 0.8, trait_bonus: &[TRAIT_KIND], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} takes in a stray dog off the {city} streets." },
    EventTemplate { id: 105, requires: TAG_GENERIC, weight: 0.8, trait_bonus: &[TRAIT_PROUD, TRAIT_IMPULSIVE], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} insults a proud neighbour over a trifle in {city}." },
    EventTemplate { id: 106, requires: TAG_GENERIC, weight: 0.8, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} spends a season studying under a local tutor in {city}." },
    EventTemplate { id: 107, requires: TAG_GENERIC, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is robbed on a dark street in {city} and comes home shaken." },
    EventTemplate { id: 108, requires: TAG_GENERIC, weight: 0.8, trait_bonus: &[TRAIT_KIND], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} nurses a sick relative through a hard winter in {city}." },
    EventTemplate { id: 109, requires: TAG_GENERIC, weight: 0.8, trait_bonus: &[TRAIT_GREEDY], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} closes a shrewd bargain that leaves a rival poorer in {city}." },
    EventTemplate { id: 110, requires: TAG_GENERIC, weight: 0.7, trait_bonus: &[TRAIT_GENEROUS], fame_delta: 0.01, trait_gain: None, feature_gain: None,
        text: "{name} gives away a fortune at a festival meal in {city}, to everyone's astonishment." },
    EventTemplate { id: 111, requires: TAG_GENERIC, weight: 0.7, trait_bonus: &[TRAIT_PROUD], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is humiliated before the {city} crowd by a sharper tongue." },
    EventTemplate { id: 112, requires: TAG_GENERIC, weight: 0.6, trait_bonus: &[TRAIT_LOYAL, TRAIT_HONEST], fame_delta: 0.01, trait_gain: None, feature_gain: None,
        text: "{name} keeps a solemn vow made years before, to their own cost, in {city}." },
    EventTemplate { id: 113, requires: TAG_GENERIC, weight: 0.6, trait_bonus: &[TRAIT_FICKLE], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} breaks a promise to an old friend in {city}." },
    EventTemplate { id: 114, requires: TAG_GENERIC, weight: 0.6, trait_bonus: &[TRAIT_AMBITIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} spends lavishly on a new house in {city}, to the talk of the quarter." },
    EventTemplate { id: 115, requires: TAG_GENERIC, weight: 0.6, trait_bonus: &[TRAIT_GREEDY, TRAIT_CRUEL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} turns away a beggar at the {city} gate without a coin." },
    EventTemplate { id: 116, requires: TAG_GENERIC, weight: 0.6, trait_bonus: &[TRAIT_LOYAL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} sits up all night at a sick friend's bedside in {city}." },
    EventTemplate { id: 117, requires: TAG_GENERIC, weight: 0.5, trait_bonus: &[TRAIT_GAMBLER, TRAIT_DECEITFUL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is caught cheating at dice in a {city} back room." },
    EventTemplate { id: 118, requires: TAG_GENERIC, weight: 0.5, trait_bonus: &[TRAIT_TEMPERATE], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} settles an old grudge quietly rather than provoke a scandal in {city}." },
    EventTemplate { id: 119, requires: TAG_GENERIC, weight: 0.5, trait_bonus: &[TRAIT_IMPULSIVE, TRAIT_PROUD], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} takes a rash oath in anger before witnesses in {city}." },
    // ── More coast ────────────────────────────────────────────────────
    EventTemplate { id: 130, requires: TAG_COAST, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches the tide uncover the mudflats below {city}." },
    EventTemplate { id: 131, requires: TAG_COAST, weight: 1.0, trait_bonus: &[TRAIT_GREEDY], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} buys a string of pearls from a diver off {city}." },
    EventTemplate { id: 132, requires: TAG_COAST, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is drenched by salt spray crossing the {city} quay in a gale." },
    EventTemplate { id: 133, requires: TAG_COAST | TAG_ROLE_ADMIRAL, weight: 0.9, trait_bonus: &[TRAIT_SEAFARER], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} inspects a new keel laid down in the {city} yards." },
    EventTemplate { id: 134, requires: TAG_COAST, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades news with a foreign sailor newly arrived at {city}." },
    EventTemplate { id: 135, requires: TAG_COAST, weight: 0.7, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} climbs the {city} lighthouse to watch the horizon." },
    EventTemplate { id: 136, requires: TAG_COAST, weight: 0.6, trait_bonus: &[TRAIT_GAMBLER], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses a wager on which vessel would round the {city} headland first." },
    EventTemplate { id: 137, requires: TAG_COAST, weight: 0.8, trait_bonus: &[TRAIT_ROBUST], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} helps haul in a heavy catch on the {city} shore." },
    EventTemplate { id: 138, requires: TAG_COAST, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: Some(TRAIT_ROBUST), feature_gain: None,
        text: "{name} pulls a drowning child from the surf at {city}." },
    EventTemplate { id: 139, requires: TAG_COAST, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} barters for coral brought up by divers at {city}." },
    EventTemplate { id: 140, requires: TAG_COAST, weight: 0.7, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches a squall break over the {city} anchorage." },
    EventTemplate { id: 141, requires: TAG_COAST | TAG_TRAVELLING, weight: 0.6, trait_bonus: &[], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} is becalmed for a week within sight of {city}, and grows restless." },
    EventTemplate { id: 142, requires: TAG_COAST, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: Some(TRAIT_SCARRED), feature_gain: None,
        text: "{name} is scarred by a rope burn hauling lines into {city}." },
    EventTemplate { id: 143, requires: TAG_COAST, weight: 0.6, trait_bonus: &[TRAIT_AMBITIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} buys shares in a new hull being built at {city}." },
    EventTemplate { id: 144, requires: TAG_COAST, weight: 0.7, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} listens to old mariners' tales at a {city} tavern by the water." },
    // ── More river ───────────────────────────────────────────────────────
    EventTemplate { id: 150, requires: TAG_RIVER, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches millers argue over water rights on the {city} river." },
    EventTemplate { id: 151, requires: TAG_RIVER, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} rides the current downstream to {city} on a raft of timber." },
    EventTemplate { id: 152, requires: TAG_RIVER, weight: 0.7, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} helps drag a cart free of the mud on the {city} riverbank." },
    EventTemplate { id: 153, requires: TAG_RIVER, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades for fish fresh off the river at {city}." },
    EventTemplate { id: 154, requires: TAG_RIVER, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: Some(TRAIT_ROBUST), feature_gain: None,
        text: "{name} survives a capsized ferry crossing near {city}." },
    EventTemplate { id: 155, requires: TAG_RIVER, weight: 0.7, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches the spring flood swell the banks below {city}." },
    EventTemplate { id: 156, requires: TAG_RIVER, weight: 0.6, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} listens to boatmen's songs on the {city} landing at dusk." },
    EventTemplate { id: 157, requires: TAG_RIVER, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses a purse to a cutpurse amid the crowd at the {city} wharves." },
    EventTemplate { id: 158, requires: TAG_RIVER, weight: 0.6, trait_bonus: &[TRAIT_AMBITIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} bargains with a barge captain for passage out of {city}." },
    EventTemplate { id: 159, requires: TAG_RIVER, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches the millwheel turn below {city} on a still morning." },
    // ── More desert ──────────────────────────────────────────────────────
    EventTemplate { id: 165, requires: TAG_DESERT, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades salt for dates with a caravan resting outside {city}." },
    EventTemplate { id: 166, requires: TAG_DESERT, weight: 0.8, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} learns to read the stars to find the road to {city}." },
    EventTemplate { id: 167, requires: TAG_DESERT, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shelters from a sandstorm within sight of {city}'s gates." },
    EventTemplate { id: 168, requires: TAG_DESERT, weight: 0.7, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} barters for a string of camels bound for {city}." },
    EventTemplate { id: 169, requires: TAG_DESERT | TAG_TRAVELLING, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: Some(TRAIT_ROBUST), feature_gain: None,
        text: "{name} crosses a waterless stretch of dunes to reach {city}." },
    EventTemplate { id: 170, requires: TAG_DESERT, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} pays a guide to lead them through the dunes toward {city}." },
    EventTemplate { id: 171, requires: TAG_DESERT, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses a mule to heat on the road into {city}." },
    EventTemplate { id: 172, requires: TAG_DESERT, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades for a fine rug woven by nomads camped near {city}." },
    EventTemplate { id: 173, requires: TAG_DESERT, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is nearly lost in a sudden sandstorm approaching {city}." },
    EventTemplate { id: 174, requires: TAG_DESERT, weight: 0.6, trait_bonus: &[TRAIT_GENEROUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shares water with a stranger on the road to {city}." },
    // ── More steppe ──────────────────────────────────────────────────────
    EventTemplate { id: 180, requires: TAG_STEPPE, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades hides with herders camped beyond {city}." },
    EventTemplate { id: 181, requires: TAG_STEPPE, weight: 0.8, trait_bonus: &[TRAIT_BRAVE], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} learns to shoot from horseback on the plain outside {city}." },
    EventTemplate { id: 182, requires: TAG_STEPPE, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is thrown from a half-broken colt near {city}." },
    EventTemplate { id: 183, requires: TAG_STEPPE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches a great herd cross the grass beyond {city}." },
    EventTemplate { id: 184, requires: TAG_STEPPE, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shares a tent with wandering herders for a night near {city}." },
    EventTemplate { id: 185, requires: TAG_STEPPE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades a horse for three good milk cows outside {city}." },
    EventTemplate { id: 186, requires: TAG_STEPPE, weight: 0.6, trait_bonus: &[TRAIT_HUNTER], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} joins a hunt across the open grass beyond {city}." },
    EventTemplate { id: 187, requires: TAG_STEPPE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is caught out by an early chill on the plain near {city}." },
    EventTemplate { id: 188, requires: TAG_STEPPE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches riders race across the plain at a gathering near {city}." },
    EventTemplate { id: 189, requires: TAG_STEPPE, weight: 0.5, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} learns to read weather in the grass and sky beyond {city}." },
    // ── More cold ────────────────────────────────────────────────────────
    EventTemplate { id: 195, requires: TAG_COLD, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} chops firewood against a hard frost in {city}." },
    EventTemplate { id: 196, requires: TAG_COLD, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches the harbour freeze solid at {city}." },
    EventTemplate { id: 197, requires: TAG_COLD, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses toes to frostbite crossing an exposed pass near {city}." },
    EventTemplate { id: 198, requires: TAG_COLD, weight: 0.7, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shares a hearth with strangers stranded by snow near {city}." },
    EventTemplate { id: 199, requires: TAG_COLD, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches wolves circle the {city} outskirts on a hard winter night." },
    EventTemplate { id: 200, requires: TAG_COLD, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades furs for grain as the frost sets hard on {city}." },
    EventTemplate { id: 201, requires: TAG_COLD, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is snowed in for a week on the road to {city}." },
    EventTemplate { id: 202, requires: TAG_COLD, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} tends the frost-bitten fingers of a traveller newly arrived at {city}." },
    EventTemplate { id: 203, requires: TAG_COLD, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches the first snow settle over the roofs of {city}." },
    EventTemplate { id: 204, requires: TAG_COLD, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} breaks ice on the well outside {city} on a bitter morning." },
    // ── More tropical ────────────────────────────────────────────────────
    EventTemplate { id: 210, requires: TAG_TROPICAL, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shelters through the wet season under a neighbour's roof in {city}." },
    EventTemplate { id: 211, requires: TAG_TROPICAL, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades spices with a trader freshly arrived at {city}." },
    EventTemplate { id: 212, requires: TAG_TROPICAL, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is laid low by a fever that sweeps through {city} each wet season." },
    EventTemplate { id: 213, requires: TAG_TROPICAL, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: Some(TRAIT_SICKLY), feature_gain: None,
        text: "{name} survives a bout of jungle fever near {city}." },
    EventTemplate { id: 214, requires: TAG_TROPICAL, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches monsoon clouds gather over {city}." },
    EventTemplate { id: 215, requires: TAG_TROPICAL, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses a season's stores to mould in the damp air of {city}." },
    EventTemplate { id: 216, requires: TAG_TROPICAL, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades a parrot for a fine length of cloth in {city}." },
    EventTemplate { id: 217, requires: TAG_TROPICAL, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is bitten by a snake in the undergrowth near {city} and recovers slowly." },
    EventTemplate { id: 218, requires: TAG_TROPICAL, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} waits out the wet season mending nets in {city}." },
    EventTemplate { id: 219, requires: TAG_TROPICAL, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trades for a rare dye brought inland to {city}." },
    // ── More war ─────────────────────────────────────────────────────────
    EventTemplate { id: 225, requires: TAG_AT_WAR, weight: 1.0, trait_bonus: &[TRAIT_KIND], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} hides a family's silver before the enemy reaches {city}." },
    EventTemplate { id: 226, requires: TAG_AT_WAR, weight: 0.8, trait_bonus: &[TRAIT_BRAVE], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} carries messages between besieged quarters of {city}." },
    EventTemplate { id: 227, requires: TAG_AT_WAR, weight: 0.7, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses a brother to the fighting near {city}." },
    EventTemplate { id: 228, requires: TAG_AT_WAR, weight: 0.6, trait_bonus: &[TRAIT_GREEDY], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} hoards grain against the coming siege of {city}." },
    EventTemplate { id: 229, requires: TAG_AT_WAR | TAG_ROLE_COMMANDER, weight: 0.7, trait_bonus: &[TRAIT_BRAVE], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} rallies a wavering line before the gates of {city}." },
    EventTemplate { id: 230, requires: TAG_AT_WAR, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} flees {city} with what they can carry as the enemy nears." },
    EventTemplate { id: 231, requires: TAG_AT_WAR, weight: 0.5, trait_bonus: &[TRAIT_PHYSICIAN_TRAINED], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} tends the wounded brought back from the fighting near {city}." },
    EventTemplate { id: 232, requires: TAG_AT_WAR, weight: 0.5, trait_bonus: &[TRAIT_CRAVEN], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} refuses to take up arms for {city}, whatever is said of them." },
    EventTemplate { id: 233, requires: TAG_AT_WAR, weight: 0.5, trait_bonus: &[TRAIT_GREEDY], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} sells stores to the highest bidder as {city} braces for war." },
    EventTemplate { id: 234, requires: TAG_AT_WAR | TAG_ROLE_COMMANDER, weight: 0.5, trait_bonus: &[TRAIT_STRATEGIST], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} orders a hard retreat to save the garrison of {city}." },
    // ── More famine ──────────────────────────────────────────────────────
    EventTemplate { id: 240, requires: TAG_FAMINE, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} waits half the day in a line for bread in {city}." },
    EventTemplate { id: 241, requires: TAG_FAMINE, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} sells a family heirloom to buy grain in {city}." },
    EventTemplate { id: 242, requires: TAG_FAMINE, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches the {city} granary opened to the hungry." },
    EventTemplate { id: 243, requires: TAG_FAMINE, weight: 0.5, trait_bonus: &[TRAIT_KIND], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} takes in a hungry neighbour's children in {city}." },
    EventTemplate { id: 244, requires: TAG_FAMINE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is caught stealing bread in {city} and let go with a warning." },
    EventTemplate { id: 245, requires: TAG_FAMINE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches prices climb past reach in the {city} market." },
    EventTemplate { id: 246, requires: TAG_FAMINE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} buries a child lost to hunger in {city}." },
    EventTemplate { id: 247, requires: TAG_FAMINE, weight: 0.5, trait_bonus: &[TRAIT_GENEROUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shares the last of their own store with the {city} poor." },
    // ── Large city ───────────────────────────────────────────────────────
    EventTemplate { id: 250, requires: TAG_LARGE, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses their way in the crowded quarters of {city}." },
    EventTemplate { id: 251, requires: TAG_LARGE, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is pressed into a crowd watching a public spectacle in {city}." },
    EventTemplate { id: 252, requires: TAG_LARGE, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} hears a rumour spread through the {city} crowds within a single day." },
    EventTemplate { id: 253, requires: TAG_LARGE, weight: 0.5, trait_bonus: &[TRAIT_DECEITFUL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is cheated by a sharper in the crowded {city} bazaar." },
    EventTemplate { id: 254, requires: TAG_LARGE, weight: 0.6, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} attends a great spectacle staged in the heart of {city}." },
    EventTemplate { id: 255, requires: TAG_LARGE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is robbed in a crowded alley of {city} and thinks nothing more of it." },
    EventTemplate { id: 256, requires: TAG_LARGE, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} finds work among the thousand trades of {city}." },
    EventTemplate { id: 257, requires: TAG_LARGE, weight: 0.5, trait_bonus: &[TRAIT_CURIOUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} watches a foreign embassy paraded through {city}." },
    // ── More role: commander ─────────────────────────────────────────────
    EventTemplate { id: 260, requires: TAG_ROLE_COMMANDER, weight: 1.0, trait_bonus: &[TRAIT_STRATEGIST], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} drills raw recruits outside {city}." },
    EventTemplate { id: 261, requires: TAG_ROLE_COMMANDER, weight: 0.8, trait_bonus: &[TRAIT_BRAVE], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is nearly unhorsed in a skirmish near {city}." },
    EventTemplate { id: 262, requires: TAG_ROLE_COMMANDER, weight: 0.6, trait_bonus: &[TRAIT_CRUEL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} disciplines an officer for cowardice near {city}." },
    EventTemplate { id: 263, requires: TAG_ROLE_COMMANDER, weight: 0.6, trait_bonus: &[TRAIT_GENEROUS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} shares the common ration with the troops outside {city}." },
    EventTemplate { id: 264, requires: TAG_ROLE_COMMANDER, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} wins a small skirmish that steadies the line near {city}." },
    EventTemplate { id: 265, requires: TAG_ROLE_COMMANDER, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is blamed for a costly delay on the march to {city}." },
    // ── More role: admiral ───────────────────────────────────────────────
    EventTemplate { id: 268, requires: TAG_ROLE_ADMIRAL, weight: 1.0, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} inspects the fleet at anchor before {city}." },
    EventTemplate { id: 269, requires: TAG_ROLE_ADMIRAL, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} settles a dispute among captains over prize goods from {city}." },
    EventTemplate { id: 270, requires: TAG_ROLE_ADMIRAL, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is nearly lost overboard in a squall bound for {city}." },
    EventTemplate { id: 271, requires: TAG_ROLE_ADMIRAL, weight: 0.5, trait_bonus: &[TRAIT_CRUEL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} orders a captain flogged for cowardice before {city}." },
    EventTemplate { id: 272, requires: TAG_ROLE_ADMIRAL, weight: 0.5, trait_bonus: &[TRAIT_STRATEGIST], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} brings a crippled vessel safely into {city}." },
    EventTemplate { id: 273, requires: TAG_ROLE_ADMIRAL, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is blamed for a convoy's late arrival at {city}." },
    // ── More role: scholar ───────────────────────────────────────────────
    EventTemplate { id: 276, requires: TAG_ROLE_SCHOLAR, weight: 1.0, trait_bonus: &[TRAIT_SCHOLARLY], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} corresponds with a distant scholar over a point of dispute concerning {city}'s own histories." },
    EventTemplate { id: 277, requires: TAG_ROLE_SCHOLAR, weight: 0.8, trait_bonus: &[TRAIT_PATRON_OF_ARTS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} takes on a promising student in {city}." },
    EventTemplate { id: 278, requires: TAG_ROLE_SCHOLAR, weight: 0.6, trait_bonus: &[TRAIT_HONEST], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is accused of heresy by a rival in {city}." },
    EventTemplate { id: 279, requires: TAG_ROLE_SCHOLAR, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} copies a rare text long into the night in {city}." },
    EventTemplate { id: 280, requires: TAG_ROLE_SCHOLAR, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} publishes a work that stirs argument across {city}." },
    EventTemplate { id: 281, requires: TAG_ROLE_SCHOLAR, weight: 0.5, trait_bonus: &[TRAIT_PROUD], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} quarrels bitterly with a former student in {city}." },
    // ── More role: artisan ───────────────────────────────────────────────
    EventTemplate { id: 284, requires: TAG_ROLE_ARTISAN, weight: 1.0, trait_bonus: &[TRAIT_PATRON_OF_ARTS], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} trains a promising apprentice in {city}." },
    EventTemplate { id: 285, requires: TAG_ROLE_ARTISAN, weight: 0.8, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} argues with a guildmaster over the price of materials in {city}." },
    EventTemplate { id: 286, requires: TAG_ROLE_ARTISAN, weight: 0.6, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} ruins a costly commission through a moment's carelessness in {city}." },
    EventTemplate { id: 287, requires: TAG_ROLE_ARTISAN, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name}'s finest work draws admirers from across {city}." },
    EventTemplate { id: 288, requires: TAG_ROLE_ARTISAN, weight: 0.5, trait_bonus: &[TRAIT_DECEITFUL], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is accused of copying a rival's design in {city}." },
    EventTemplate { id: 289, requires: TAG_ROLE_ARTISAN, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: Some(TRAIT_MAIMED_HAND), feature_gain: None,
        text: "{name} loses a hand's steadiness after a long illness in {city}." },
    // ── More role: performer ─────────────────────────────────────────────
    EventTemplate { id: 292, requires: TAG_ROLE_PERFORMER, weight: 1.0, trait_bonus: &[], fame_delta: 0.01, trait_gain: None, feature_gain: None,
        text: "{name} draws a great crowd at a {city} festival." },
    EventTemplate { id: 293, requires: TAG_ROLE_PERFORMER, weight: 0.8, trait_bonus: &[TRAIT_PROUD], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is booed off a stage in {city}." },
    EventTemplate { id: 294, requires: TAG_ROLE_PERFORMER, weight: 0.6, trait_bonus: &[TRAIT_GREEDY], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} takes a patron's coin for a private performance in {city}." },
    EventTemplate { id: 295, requires: TAG_ROLE_PERFORMER, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} loses their voice for a season after a hard winter in {city}." },
    EventTemplate { id: 296, requires: TAG_ROLE_PERFORMER, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name}'s performance in {city} is still spoken of years later." },
    EventTemplate { id: 297, requires: TAG_ROLE_PERFORMER, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} quarrels with a rival troupe over a coveted stage in {city}." },
    // ── More role: senator ───────────────────────────────────────────────
    EventTemplate { id: 300, requires: TAG_ROLE_SENATOR, weight: 1.0, trait_bonus: &[TRAIT_ADMINISTRATOR], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} brokers a compromise between two feuding factions in {city}'s council." },
    EventTemplate { id: 301, requires: TAG_ROLE_SENATOR, weight: 0.8, trait_bonus: &[TRAIT_BOUGHT], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is accused of taking a bribe in {city}." },
    EventTemplate { id: 302, requires: TAG_ROLE_SENATOR, weight: 0.6, trait_bonus: &[TRAIT_HONEST], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} champions an unpopular measure before {city}'s council." },
    EventTemplate { id: 303, requires: TAG_ROLE_SENATOR, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is outmanoeuvred by a rival in {city}'s council chamber." },
    EventTemplate { id: 304, requires: TAG_ROLE_SENATOR, weight: 0.5, trait_bonus: &[], fame_delta: 0.02, trait_gain: None, feature_gain: None,
        text: "{name} carries a difficult reform through {city}'s council." },
    EventTemplate { id: 305, requires: TAG_ROLE_SENATOR, weight: 0.5, trait_bonus: &[], fame_delta: 0.0, trait_gain: None, feature_gain: None,
        text: "{name} is quietly dropped from consideration for high office in {city}." },
];

/// `requires == 0` marks the guaranteed in-city generic layer.
fn template_matches(t: &EventTemplate, tags: u32) -> bool {
    t.requires == TAG_GENERIC || (t.requires & tags) == t.requires
}

/// Knuth's algorithm over a hashed uniform stream — deterministic, capped at
/// 10 per 00_INDEX ("the count is a hashed Poisson draw, capped at 10").
fn hashed_poisson(mean: f32, seed: u64, tick: u32, id: u32, salt: u64) -> u32 {
    if mean <= 0.0 {
        return 0;
    }
    let l = (-mean).exp();
    let mut k = 0u32;
    let mut p = 1.0f32;
    loop {
        k += 1;
        let u = hash01(seed, tick as u64 ^ (id as u64).wrapping_mul(0x9E3779B1), salt ^ k as u64).max(1e-6);
        p *= u;
        if p <= l || k >= 20 {
            break;
        }
    }
    (k - 1).min(10)
}

impl CampaignSim {
    /// `role_base` per 00_INDEX: "ordinary individuals use a lower role base
    /// ... famous ones the full rate".
    fn role_event_base(&self, famous: bool) -> f32 {
        if famous { 3.0 } else { 0.4 }
    }

    /// War ×3, famine ×1.5, a recent plague strike ×2, travelling ×2, holding
    /// office ×1.5 — see this file's own doc comment for what is left out
    /// and why.
    pub(crate) fn event_turbulence(&self, p: &Individual) -> f32 {
        let mut t = 1.0f32;
        if let Some(h) = self.hubs.get(p.current_hub.max(0) as usize).filter(|_| p.current_hub >= 0) {
            if h.war_with >= 0 { t *= 3.0; }
            if h.starving > 0.3 { t *= 1.5; }
            if self.tick < h.plague_immune_until { t *= 2.0; }
        }
        if p.origin_hub != p.current_hub && p.formed_hub != p.current_hub { t *= 2.0; }
        if !p.roles.is_empty() { t *= 1.5; }
        t
    }

    /// Weekly: for each living person whose yearly quota hasn't been rolled
    /// yet this year, roll it; then probabilistically spend a share of the
    /// remaining quota across the remaining weeks of the year, so a due
    /// event lands at some point during the year rather than all on 1
    /// January (00_INDEX "spread across the year's ticks").
    pub(crate) fn fire_due_life_events(&mut self) {
        let tick = self.tick;
        let year = tick / TICKS_PER_YEAR;
        let doy = tick % TICKS_PER_YEAR;
        let weeks_left = ((TICKS_PER_YEAR - doy) / 7).max(1);
        for i in 0..self.people.len() {
            if self.people[i].events_year != year {
                let base = self.role_event_base(self.people[i].famous);
                let turb = self.event_turbulence(&self.people[i]);
                let count = hashed_poisson(base * turb, self.seed, tick, self.people[i].id, salts::EVENT_COUNT);
                self.people[i].events_year = year;
                self.people[i].events_this_year = count as u8;
            }
            if self.people[i].events_this_year == 0 {
                continue;
            }
            let fire_roll = hash01(self.seed, tick as u64, (self.people[i].id as u64) ^ salts::EVENT_SPREAD);
            let p_this_week = self.people[i].events_this_year as f32 / weeks_left as f32;
            if fire_roll >= p_this_week.min(1.0) {
                continue;
            }
            self.people[i].events_this_year -= 1;
            self.fire_one_event(i);
        }
    }

    /// Pick + apply ONE event template for `people[i]` — always finds one,
    /// since the in-city generic layer matches unconditionally
    /// (`a_due_event_always_finds_a_template`).
    pub(crate) fn fire_one_event(&mut self, i: usize) {
        let tags = self.individual_context_tags(&self.people[i]);
        let mut candidates: Vec<&EventTemplate> = EVENT_TEMPLATES.iter().filter(|t| template_matches(t, tags)).collect();
        if candidates.is_empty() {
            candidates = EVENT_TEMPLATES.iter().filter(|t| t.requires == TAG_GENERIC).collect();
        }
        let mut weights: Vec<f32> = candidates.iter().map(|t| {
            let mut w = t.weight;
            for &tb in t.trait_bonus {
                if self.people[i].traits.iter().any(|&(tt, _)| tt == tb) {
                    w *= 2.0;
                }
            }
            w
        }).collect();
        let total: f32 = weights.iter().sum();
        if total <= 0.0 {
            weights = vec![1.0; candidates.len()];
        }
        let total: f32 = weights.iter().sum();
        let r = hash01(self.seed, self.tick as u64, (self.people[i].id as u64) ^ salts::EVENT_PICK) * total;
        let mut acc = 0.0f32;
        let mut pick = 0usize;
        for (k, &w) in weights.iter().enumerate() {
            acc += w;
            if r < acc { pick = k; break; }
            pick = k;
        }
        let t = candidates[pick];
        let hub = self.people[i].current_hub;
        // `u32::MAX` marks "no hub" (an abroad/unknown person) so rendering
        // never mistakes it for hub 0 — every world has a real hub 0.
        let hub_arg = if hub >= 0 { hub as u32 } else { u32::MAX };
        let entry = IndividualLifeEntry { tick: self.tick, template_id: t.id, args: vec![hub_arg] };
        let famous = self.people[i].famous;
        self.people[i].life_log.push(entry.clone());
        if !famous && self.people[i].life_log.len() > ORDINARY_LIFE_LOG_CAP {
            let drop = self.people[i].life_log.len() - ORDINARY_LIFE_LOG_CAP;
            self.people[i].life_log.drain(0..drop);
        }
        if t.fame_delta != 0.0 {
            self.people[i].fame = (self.people[i].fame + t.fame_delta).clamp(0.0, 1.0);
        }
        if let Some(tg) = t.trait_gain {
            if !self.people[i].traits.iter().any(|&(tt, _)| tt == tg) && self.people[i].traits.len() < TRAIT_CAP_MAX {
                self.people[i].traits.push((tg, 1));
                let feat = feature_for_trait(tg);
                if feat != 0 {
                    self.people[i].features |= feat;
                }
            }
        }
        if let Some(f) = t.feature_gain {
            self.people[i].features |= f;
        }
        // Only a FAMOUS person's ordinary event reaches the world journal —
        // the same salience discipline SETTLEMENT_LIFE_PLAN.md's Life tab
        // and 00_INDEX's own "Chronicle salience" rule already use.
        if famous {
            let name = self.people[i].name.clone();
            let text = self.render_life_entry_for(&name, &entry);
            self.journal.push(JournalEntry {
                tick: self.tick, kind: "individual".into(), hub, good: -1, value: 0.0, text,
            });
        }
    }

    /// Resolve a stored `(template_id, args)` pair into readable text AT READ
    /// TIME (00_INDEX "text generated lazily at read time") — never baked
    /// into the log itself, so re-reading an old entry always uses the
    /// CURRENT city name etc.
    pub(crate) fn render_life_entry_for(&self, name: &str, e: &IndividualLifeEntry) -> String {
        let t = EVENT_TEMPLATES.iter().find(|t| t.id == e.template_id);
        let city = e.args.first()
            .and_then(|&h| self.hubs.get(h as usize))
            .map(|h| h.name.as_str())
            .unwrap_or("an unnamed place");
        match t {
            Some(t) => t.text.replace("{name}", name).replace("{city}", city),
            None => format!("{name} passes an ordinary day."),
        }
    }

}
