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
//! The ~150-template expansion (02.6) is explicitly a SEPARATE SESSION per
//! the design doc; this file ships the ~40-template starter set only.
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
        let entry = LifeEntry { tick: self.tick, template_id: t.id, args: vec![hub.max(0) as u32] };
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
    pub(crate) fn render_life_entry_for(&self, name: &str, e: &LifeEntry) -> String {
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
