//! life_choices — character PROGRESSION for `Individual`s (2026-09-30 audit).
//!
//! Before this, a life event was a thing that HAPPENED to a person: a
//! template fired, a line was logged, and a handful of templates added a
//! trait. Nobody ever CHOSE anything — `decide()` (the 75% rule, 02.2) was
//! built and tested but no life event called it — so a person's character at
//! seventy was their debut roll plus noise, and the life log read as weather.
//!
//! Three additions, all bookkeeping about people (no hub, house, price or
//! population number is touched — `living_world_is_inert_at_zero` still
//! holds):
//!
//! 1. **Choice events.** A share of the life events a person is due
//!    (`CHOICE_EVENT_SHARE`) is a DILEMMA instead of a happening: a situation
//!    with two responses, weighed by `decide()` against the person's own
//!    traits and live modifiers. The chosen response is what shapes them —
//!    it can GAIN a trait, DEEPEN one already held (strength 1 → 2), or FLIP
//!    its opposite (a craven who stands on the walls is no longer craven), it
//!    moves their fame, can leave a modifier (emboldened, humiliated, owes a
//!    debt) that tips their NEXT decision, and nudges their personal ideology
//!    (who you give to, and who you side with, is where a politics comes
//!    from). The entry keeps the dominant reasons, so the UI can say WHY.
//! 2. **Milestones.** Career turns (entering public life, taking office,
//!    studying under a teacher, founding a school, setting down a doctrine,
//!    exile, a masterwork, becoming renowned, death) are logged as KEY entries
//!    — they are the spine of a life story, and they are never pruned ahead of
//!    ordinary chatter (the chronicle's own rule 20, applied to a person).
//! 3. **Renown.** A yearly fame accrual by the offices and deeds a person
//!    actually holds, scaled by the standing of their city — the ONLY way fame
//!    rose before this was an occasional +0.01..0.03 event against a
//!    0.03/year decay, so measured over 100 years the 40-seat notable roster
//!    promoted nobody at all (max fame 0.10 against a 0.65 threshold).
use super::*;
use super::individuals::living_world_salts as salts;

/// A reason code at or above this is a modifier (`code - base`).
pub const REASON_MODIFIER_BASE: u16 = 1000;
/// "the circumstances" — a choice's own situational context term.
pub const REASON_CONTEXT: u16 = 2000;

/// Share of fired life events that are dilemmas rather than happenings.
pub const CHOICE_EVENT_SHARE: f32 = 0.35;
/// A choice adds a brand-new trait only below this many traits.
pub const CHOICE_NEW_TRAIT_SOFT_CAP: usize = 4;
/// How long a modifier a choice leaves behind lasts.
const CHOICE_MODIFIER_TICKS: u32 = 2 * TICKS_PER_YEAR;

// ── Milestone template ids (900..999, never an `EVENT_TEMPLATES` id) ────────
pub const MS_DEBUT: u16 = 900;
pub const MS_OFFICE: u16 = 901;
pub const MS_STUDY: u16 = 902;
pub const MS_TEACH: u16 = 903;
pub const MS_SCHOOL: u16 = 904;
pub const MS_DOCTRINE: u16 = 905;
pub const MS_EXILE: u16 = 906;
pub const MS_RETURN: u16 = 907;
pub const MS_PATRON: u16 = 908;
pub const MS_MASTERWORK: u16 = 909;
pub const MS_RENOWN: u16 = 910;
pub const MS_DEATH: u16 = 911;
pub const MS_POLITICS: u16 = 912;
pub const MS_TRAIT_FLIP: u16 = 913;
/// Moved home for good: args `[to, from, reason]` (`MOVE_*`).
pub const MS_MOVE: u16 = 914;
/// A journey there and back: args `[to, from, reason]`.
pub const MS_VISIT: u16 = 915;

pub const MOVE_RELOCATE: u32 = 0;
pub const MOVE_COMMISSION: u32 = 1;
pub const MOVE_TOUR: u32 = 2;
pub const MOVE_EMBASSY: u32 = 3;
pub const MOVE_TRADE: u32 = 4;
pub const MOVE_EXPLORE: u32 = 5;
pub const MOVE_PILGRIMAGE: u32 = 6;

fn move_reason(r: u32) -> &'static str {
    match r {
        MOVE_COMMISSION => "to carry out a commission",
        MOVE_TOUR => "on tour",
        MOVE_EMBASSY => "on an embassy",
        MOVE_TRADE => "on the business of the counting-house",
        MOVE_EXPLORE => "to see what lies beyond",
        MOVE_PILGRIMAGE => "on a journey of the heart",
        _ => "to make a new life",
    }
}

/// Yearly chance a person of this role sets out (a VISIT), and how likely a
/// journey ends in them staying for good.
fn travel_propensity(role: u8) -> (f32, f32, u32) {
    match role {
        ROLE_EXPLORER => (0.35, 0.10, MOVE_EXPLORE),
        ROLE_DIPLOMAT => (0.30, 0.05, MOVE_EMBASSY),
        ROLE_PERFORMER => (0.25, 0.15, MOVE_TOUR),
        ROLE_MERCHANT_PRINCE | ROLE_BANKER => (0.15, 0.20, MOVE_TRADE),
        ROLE_ARTISAN => (0.08, 0.25, MOVE_COMMISSION),
        ROLE_DEMAGOGUE => (0.06, 0.40, MOVE_RELOCATE),
        _ => (0.03, 0.20, MOVE_PILGRIMAGE),
    }
}
/// A person living away from their home city goes back each year with this chance.
const HOMECOMING_CHANCE: f32 = 0.08;

pub struct ChoiceOption {
    /// A short verb for the UI ("gives", "walks on").
    pub label: &'static str,
    pub base: f32,
    /// (trait, weight per strength point) — added to this option's odds.
    pub traits: &'static [(u8, f32)],
    /// (modifier, weight) — a live modifier tips the decision.
    pub mods: &'static [(u8, f32)],
    pub gain: Option<u8>,
    pub fame: f32,
    pub modifier: Option<u8>,
    /// Personal ideology nudge (Authority · Tradition · Openness · Economy).
    pub ideo: [f32; 4],
    pub outcome: &'static str,
}

pub struct ChoiceTemplate {
    pub id: u16,
    pub requires: u32,
    pub weight: f32,
    pub prompt: &'static str,
    pub options: [ChoiceOption; 2],
}

const NO: [f32; 4] = [0.0; 4];

macro_rules! opt {
    ($label:expr, $base:expr, $traits:expr, $mods:expr, $gain:expr, $fame:expr, $modifier:expr, $ideo:expr, $out:expr) => {
        ChoiceOption { label: $label, base: $base, traits: $traits, mods: $mods, gain: $gain, fame: $fame, modifier: $modifier, ideo: $ideo, outcome: $out }
    };
}

/// ids 2000+. Text uses `{name}`/`{city}` only, and obeys the same geography
/// lint as `EVENT_TEMPLATES` (`life_event_templates_respect_geography`).
pub static CHOICE_TEMPLATES: &[ChoiceTemplate] = &[
    // ── Anyone ─────────────────────────────────────────────────────────
    ChoiceTemplate { id: 2001, requires: TAG_GENERIC, weight: 1.2,
        prompt: "A beggar child catches at {name}'s sleeve in the {city} market.",
        options: [
            opt!("gives", 0.5, &[(TRAIT_GENEROUS, 0.15), (TRAIT_KIND, 0.12)], &[(MOD_NEWLY_WEALTHY, 0.1)], Some(TRAIT_GENEROUS), 0.01, None, [0.1, 0.0, 0.1, -0.2], "{name} empties a purse into the child's hands."),
            opt!("walks on", 0.5, &[(TRAIT_GREEDY, 0.15), (TRAIT_CRUEL, 0.12)], &[(MOD_OWES_DEBT, 0.1)], None, 0.0, None, [0.0, 0.0, 0.0, 0.15], "{name} shakes the hand off and walks on."),
        ] },
    ChoiceTemplate { id: 2002, requires: TAG_GENERIC, weight: 1.0,
        prompt: "A rival insults {name}'s family before witnesses in {city}.",
        options: [
            opt!("demands satisfaction", 0.45, &[(TRAIT_PROUD, 0.15), (TRAIT_BRAVE, 0.12), (TRAIT_IMPULSIVE, 0.12)], &[(MOD_DRUNK, 0.15), (MOD_HUMILIATED, 0.1)], Some(TRAIT_BRAVE), 0.03, Some(MOD_EMBOLDENED), [-0.2, -0.1, 0.0, 0.0], "{name} demands satisfaction, and walks away from the meeting with honour and a fresh cut."),
            opt!("lets it pass", 0.55, &[(TRAIT_HUMBLE, 0.15), (TRAIT_TEMPERATE, 0.12), (TRAIT_CRAVEN, 0.15)], &[(MOD_THREATENED, 0.1)], Some(TRAIT_TEMPERATE), -0.01, Some(MOD_HUMILIATED), NO, "{name} lets the insult pass; the city talks of it for a season."),
        ] },
    ChoiceTemplate { id: 2003, requires: TAG_GENERIC, weight: 0.9,
        prompt: "An old friend begs {name} to stand surety for a debt in {city}.",
        options: [
            opt!("stands surety", 0.5, &[(TRAIT_LOYAL, 0.15), (TRAIT_GENEROUS, 0.12)], &[(MOD_REMEMBERED_HELP, 0.15)], Some(TRAIT_LOYAL), 0.01, Some(MOD_OWES_DEBT), NO, "{name} puts their name to the bond, whatever it may cost."),
            opt!("refuses", 0.5, &[(TRAIT_GREEDY, 0.12), (TRAIT_FICKLE, 0.15)], &[], Some(TRAIT_FICKLE), 0.0, None, [0.0, 0.0, 0.0, 0.1], "{name} refuses, and the two drift apart."),
        ] },
    ChoiceTemplate { id: 2004, requires: TAG_GENERIC, weight: 1.0,
        prompt: "A foreign teacher in {city} is preaching an unsettling new doctrine.",
        options: [
            opt!("listens", 0.5, &[(TRAIT_CURIOUS, 0.18), (TRAIT_SCHOLARLY, 0.12)], &[], Some(TRAIT_CURIOUS), 0.0, None, [0.1, 0.4, 0.3, 0.0], "{name} goes to hear the stranger out, and comes away changed."),
            opt!("denounces", 0.5, &[(TRAIT_CLOSED, 0.18), (TRAIT_PROUD, 0.08)], &[], Some(TRAIT_CLOSED), 0.01, None, [-0.1, -0.4, -0.3, 0.0], "{name} denounces the stranger's teaching as dangerous novelty."),
        ] },
    ChoiceTemplate { id: 2005, requires: TAG_GENERIC, weight: 0.8,
        prompt: "The wine is still flowing at a feast in {city} long past midnight.",
        options: [
            opt!("drinks on", 0.45, &[(TRAIT_IMPULSIVE, 0.15), (TRAIT_DRUNKARD, 0.2), (TRAIT_GLUTTON, 0.1)], &[(MOD_GRIEVING, 0.15), (MOD_DRUNK, 0.15)], Some(TRAIT_DRUNKARD), 0.0, Some(MOD_DRUNK), NO, "{name} drinks on until the lamps gutter."),
            opt!("goes home", 0.55, &[(TRAIT_TEMPERATE, 0.18), (TRAIT_ASCETIC, 0.15)], &[], Some(TRAIT_TEMPERATE), 0.0, None, NO, "{name} slips away while the night is young."),
        ] },
    ChoiceTemplate { id: 2006, requires: TAG_GENERIC, weight: 0.7,
        prompt: "Dice are cast for a fortune in a back room in {city}, and {name} is invited to stake everything.",
        options: [
            opt!("stakes it", 0.4, &[(TRAIT_GAMBLER, 0.2), (TRAIT_AMBITIOUS, 0.1), (TRAIT_IMPULSIVE, 0.12)], &[(MOD_EMBOLDENED, 0.1)], Some(TRAIT_GAMBLER), 0.01, Some(MOD_OWES_DEBT), NO, "{name} stakes everything on a single throw."),
            opt!("declines", 0.6, &[(TRAIT_CONTENT, 0.15), (TRAIT_TEMPERATE, 0.12)], &[], Some(TRAIT_CONTENT), 0.0, None, NO, "{name} laughs, and keeps their purse closed."),
        ] },
    ChoiceTemplate { id: 2007, requires: TAG_GENERIC, weight: 0.9,
        prompt: "A great family of {city} offers {name} its patronage, at a price in loyalty.",
        options: [
            opt!("accepts", 0.5, &[(TRAIT_AMBITIOUS, 0.18), (TRAIT_GREEDY, 0.08)], &[(MOD_FLATTERED, 0.15)], Some(TRAIT_AMBITIOUS), 0.03, Some(MOD_NEWLY_WEALTHY), [-0.3, 0.0, 0.0, 0.1], "{name} kneels to the patron and rises in the world."),
            opt!("stays free", 0.5, &[(TRAIT_PROUD, 0.12), (TRAIT_CONTENT, 0.15), (TRAIT_HONEST, 0.08)], &[], Some(TRAIT_CONTENT), 0.0, None, [0.3, 0.0, 0.0, 0.0], "{name} thanks the family and keeps their own counsel."),
        ] },
    ChoiceTemplate { id: 2008, requires: TAG_GENERIC, weight: 0.8,
        prompt: "{name} learns a rival's shameful secret in {city}.",
        options: [
            opt!("exposes it", 0.45, &[(TRAIT_HONEST, 0.12), (TRAIT_CRUEL, 0.15), (TRAIT_AMBITIOUS, 0.08)], &[(MOD_HUMILIATED, 0.15)], Some(TRAIT_CRUEL), 0.02, None, NO, "{name} lets the secret loose, and the rival is ruined."),
            opt!("keeps it", 0.55, &[(TRAIT_KIND, 0.15), (TRAIT_LOYAL, 0.08), (TRAIT_DECEITFUL, 0.1)], &[], Some(TRAIT_KIND), 0.0, None, NO, "{name} keeps the secret — for now."),
        ] },
    ChoiceTemplate { id: 2009, requires: TAG_GENERIC, weight: 0.7,
        prompt: "A merchant in {city} offers {name} a share in a venture too good to be true.",
        options: [
            opt!("buys in", 0.45, &[(TRAIT_GREEDY, 0.15), (TRAIT_GAMBLER, 0.12), (TRAIT_AMBITIOUS, 0.08)], &[], Some(TRAIT_GREEDY), 0.01, Some(MOD_NEWLY_WEALTHY), [0.0, 0.0, 0.1, 0.3], "{name} buys in, and the venture pays — this time."),
            opt!("suspects a fraud", 0.55, &[(TRAIT_TEMPERATE, 0.12), (TRAIT_CONTENT, 0.1), (TRAIT_HONEST, 0.08)], &[], None, 0.0, None, [0.0, 0.0, 0.0, -0.1], "{name} smells a fraud and keeps well clear."),
        ] },
    // ── Circumstance ────────────────────────────────────────────────────
    ChoiceTemplate { id: 2010, requires: TAG_AT_WAR, weight: 2.0,
        prompt: "The enemy is at the gates of {city} and men are wanted on the walls.",
        options: [
            opt!("takes up a spear", 0.5, &[(TRAIT_BRAVE, 0.2), (TRAIT_LOYAL, 0.1), (TRAIT_PROUD, 0.08)], &[(MOD_EMBOLDENED, 0.15)], Some(TRAIT_HERO), 0.06, Some(MOD_EMBOLDENED), [-0.3, 0.0, -0.2, 0.0], "{name} stands on the walls through the assault, and is cheered in the streets after."),
            opt!("flees to the country", 0.5, &[(TRAIT_CRAVEN, 0.2), (TRAIT_FICKLE, 0.1)], &[(MOD_THREATENED, 0.15)], Some(TRAIT_COWARD), -0.03, Some(MOD_HUMILIATED), NO, "{name} slips out before the gates close, and the city does not forget it."),
        ] },
    ChoiceTemplate { id: 2011, requires: TAG_FAMINE, weight: 2.0,
        prompt: "Hungry crowds gather outside {name}'s house in {city} as the dearth bites.",
        options: [
            opt!("shares the stores", 0.5, &[(TRAIT_GENEROUS, 0.18), (TRAIT_KIND, 0.15)], &[(MOD_REMEMBERED_HELP, 0.1)], Some(TRAIT_BENEFACTOR), 0.05, None, [0.3, 0.0, 0.0, -0.4], "{name} throws open the storerooms; the hungry of {city} remember the name."),
            opt!("bars the doors", 0.5, &[(TRAIT_GREEDY, 0.18), (TRAIT_CRUEL, 0.12), (TRAIT_CRAVEN, 0.08)], &[(MOD_THREATENED, 0.1)], Some(TRAIT_GREEDY), -0.02, Some(MOD_THREATENED), [-0.2, 0.0, -0.1, 0.3], "{name} bars the doors and waits for the hungry crowd to pass."),
        ] },
    ChoiceTemplate { id: 2012, requires: TAG_COAST, weight: 1.2,
        prompt: "A captain in the harbour of {city} offers {name} a berth on a voyage to the edge of the known world.",
        options: [
            opt!("signs on", 0.45, &[(TRAIT_CURIOUS, 0.18), (TRAIT_BRAVE, 0.1), (TRAIT_SEAFARER, 0.15)], &[(MOD_EMBOLDENED, 0.1)], Some(TRAIT_SEAFARER), 0.03, None, [0.0, 0.2, 0.3, 0.2], "{name} signs on, and comes home salt-burned with tales no one believes."),
            opt!("stays ashore", 0.55, &[(TRAIT_CONTENT, 0.15), (TRAIT_CRAVEN, 0.1), (TRAIT_CLOSED, 0.1)], &[(MOD_HOMESICK, 0.1)], None, 0.0, None, NO, "{name} watches the sails go and turns back to the town."),
        ] },
    ChoiceTemplate { id: 2013, requires: TAG_TRAVELLING, weight: 1.2,
        prompt: "Far from home in {city}, {name} is offered a place to stay for good.",
        options: [
            opt!("settles", 0.45, &[(TRAIT_CURIOUS, 0.15), (TRAIT_FICKLE, 0.1)], &[], Some(TRAIT_CURIOUS), 0.0, None, [0.0, 0.1, 0.4, 0.0], "{name} puts down roots among strangers."),
            opt!("longs for home", 0.55, &[(TRAIT_LOYAL, 0.15), (TRAIT_CLOSED, 0.1)], &[(MOD_HOMESICK, 0.2)], Some(TRAIT_LOYAL), 0.0, Some(MOD_HOMESICK), [0.0, -0.1, -0.2, 0.0], "{name} declines; home is home."),
        ] },
    // ── Scholars ────────────────────────────────────────────────────────
    ChoiceTemplate { id: 2020, requires: TAG_ROLE_SCHOLAR, weight: 2.5,
        prompt: "{name} has finished a treatise that will offend the powerful of {city}.",
        options: [
            opt!("publishes", 0.45, &[(TRAIT_BRAVE, 0.15), (TRAIT_PROUD, 0.12), (TRAIT_CURIOUS, 0.12), (TRAIT_HONEST, 0.08)], &[(MOD_EMBOLDENED, 0.15)], Some(TRAIT_SCHOLARLY), 0.08, Some(MOD_THREATENED), [0.3, 0.4, 0.2, 0.0], "{name} publishes, and the treatise is copied from one city to the next."),
            opt!("burns the draft", 0.55, &[(TRAIT_CRAVEN, 0.15), (TRAIT_LOYAL, 0.1), (TRAIT_CONTENT, 0.1)], &[(MOD_THREATENED, 0.2)], None, 0.0, Some(MOD_GRIEVING), NO, "{name} burns the draft one page at a time."),
        ] },
    ChoiceTemplate { id: 2021, requires: TAG_ROLE_SCHOLAR, weight: 2.0,
        prompt: "A rival thinker challenges {name} to a public disputation in {city}.",
        options: [
            opt!("accepts", 0.5, &[(TRAIT_ORATOR, 0.2), (TRAIT_PROUD, 0.12), (TRAIT_BRAVE, 0.08)], &[(MOD_EMBOLDENED, 0.1)], Some(TRAIT_ORATOR), 0.06, None, [0.2, 0.2, 0.0, 0.0], "{name} takes the rostrum and carries the crowd."),
            opt!("declines", 0.5, &[(TRAIT_HUMBLE, 0.12), (TRAIT_CRAVEN, 0.12), (TRAIT_CONTENT, 0.08)], &[], Some(TRAIT_HUMBLE), -0.01, None, NO, "{name} declines the challenge and goes back to the books."),
        ] },
    ChoiceTemplate { id: 2022, requires: TAG_ROLE_SCHOLAR, weight: 1.5,
        prompt: "A rich house of {city} offers to keep {name}, if the teaching flatters its interests.",
        options: [
            opt!("takes the purse", 0.45, &[(TRAIT_GREEDY, 0.15), (TRAIT_AMBITIOUS, 0.12), (TRAIT_DECEITFUL, 0.1)], &[(MOD_OWES_DEBT, 0.15)], Some(TRAIT_BOUGHT), 0.02, Some(MOD_NEWLY_WEALTHY), [-0.3, 0.0, 0.0, 0.3], "{name} takes the purse, and the teaching turns gentle toward the house."),
            opt!("stays independent", 0.55, &[(TRAIT_HONEST, 0.15), (TRAIT_PROUD, 0.1), (TRAIT_ASCETIC, 0.12)], &[], Some(TRAIT_HONEST), 0.03, None, [0.3, 0.1, 0.0, -0.1], "{name} refuses the gold, and teaches as before."),
        ] },
    ChoiceTemplate { id: 2023, requires: TAG_ROLE_SCHOLAR, weight: 1.5,
        prompt: "A poor but gifted student asks to study under {name} for nothing.",
        options: [
            opt!("takes the student", 0.55, &[(TRAIT_GENEROUS, 0.15), (TRAIT_KIND, 0.12)], &[], Some(TRAIT_GENEROUS), 0.03, None, [0.2, 0.1, 0.0, -0.1], "{name} takes the student in, and gains a disciple for life."),
            opt!("turns them away", 0.45, &[(TRAIT_GREEDY, 0.15), (TRAIT_PROUD, 0.1)], &[], None, 0.0, None, NO, "{name} turns the student away at the door."),
        ] },
    // ── Office holders ──────────────────────────────────────────────────
    ChoiceTemplate { id: 2030, requires: TAG_ROLE_SENATOR, weight: 2.5,
        prompt: "A powerful house quietly offers {name} gold for a vote in the council of {city}.",
        options: [
            opt!("takes the gold", 0.4, &[(TRAIT_GREEDY, 0.2), (TRAIT_DECEITFUL, 0.15)], &[(MOD_OWES_DEBT, 0.15), (MOD_BLACKMAILED, 0.2)], Some(TRAIT_BOUGHT), 0.0, Some(MOD_NEWLY_WEALTHY), [-0.3, 0.0, 0.0, 0.2], "{name} takes the gold and votes as told."),
            opt!("refuses", 0.6, &[(TRAIT_HONEST, 0.2), (TRAIT_PROUD, 0.08)], &[(MOD_THREATENED, -0.1)], Some(TRAIT_HONEST), 0.04, None, [0.3, 0.0, 0.0, 0.0], "{name} refuses the gold before witnesses."),
        ] },
    ChoiceTemplate { id: 2031, requires: TAG_ROLE_SENATOR, weight: 2.0,
        prompt: "A measure before the council of {city} would help the poor and hurt {name}'s own friends.",
        options: [
            opt!("speaks for it", 0.45, &[(TRAIT_KIND, 0.12), (TRAIT_ORATOR, 0.15), (TRAIT_BRAVE, 0.08), (TRAIT_GENEROUS, 0.1)], &[], Some(TRAIT_ORATOR), 0.05, None, [0.5, 0.1, 0.0, -0.3], "{name} speaks for the measure, and the commons carry {name} home on their shoulders."),
            opt!("sides with friends", 0.55, &[(TRAIT_LOYAL, 0.15), (TRAIT_GREEDY, 0.12), (TRAIT_PROUD, 0.08)], &[(MOD_OWES_DEBT, 0.1)], Some(TRAIT_LOYAL), 0.01, None, [-0.4, -0.1, 0.0, 0.3], "{name} sides with the old families, and the measure fails."),
        ] },
    ChoiceTemplate { id: 2032, requires: TAG_ROLE_SENATOR | TAG_FAMINE, weight: 3.0,
        prompt: "The public granary of {city} could feed the hungry or be held for the great houses.",
        options: [
            opt!("opens the granary", 0.5, &[(TRAIT_GENEROUS, 0.15), (TRAIT_KIND, 0.15), (TRAIT_ADMINISTRATOR, 0.08)], &[], Some(TRAIT_BENEFACTOR), 0.06, None, [0.4, 0.0, 0.0, -0.4], "{name} opens the granary doors to the hungry crowd."),
            opt!("keeps it shut", 0.5, &[(TRAIT_GREEDY, 0.15), (TRAIT_CRUEL, 0.12), (TRAIT_LOYAL, 0.05)], &[(MOD_THREATENED, 0.1)], Some(TRAIT_CRUEL), -0.03, None, [-0.4, 0.0, 0.0, 0.3], "{name} keeps the granary shut, and the hungry curse the name."),
        ] },
    // ── Commanders and admirals ─────────────────────────────────────────
    ChoiceTemplate { id: 2040, requires: TAG_ROLE_COMMANDER, weight: 2.5,
        prompt: "The battle line wavers before {city}, and {name} must choose where to stand.",
        options: [
            opt!("leads the charge", 0.45, &[(TRAIT_BRAVE, 0.2), (TRAIT_IMPULSIVE, 0.12), (TRAIT_PROUD, 0.08)], &[(MOD_EMBOLDENED, 0.15)], Some(TRAIT_SCARRED), 0.08, Some(MOD_EMBOLDENED), [-0.3, 0.0, 0.0, 0.0], "{name} leads the charge from the front and carries the day, scarred for life."),
            opt!("holds the reserve", 0.55, &[(TRAIT_STRATEGIST, 0.2), (TRAIT_TEMPERATE, 0.12), (TRAIT_CRAVEN, 0.08)], &[], Some(TRAIT_STRATEGIST), 0.04, None, NO, "{name} holds the reserve back until the decisive hour."),
        ] },
    ChoiceTemplate { id: 2041, requires: TAG_ROLE_COMMANDER, weight: 1.5,
        prompt: "A beaten town near {city} lies open to {name}'s soldiers.",
        options: [
            opt!("spares it", 0.5, &[(TRAIT_KIND, 0.18), (TRAIT_HONEST, 0.05)], &[], Some(TRAIT_KIND), 0.03, None, [0.1, 0.0, 0.2, 0.0], "{name} forbids the sack and hangs the one man who disobeys."),
            opt!("lets them loot", 0.5, &[(TRAIT_CRUEL, 0.18), (TRAIT_GREEDY, 0.12)], &[(MOD_EMBOLDENED, 0.1)], Some(TRAIT_CRUEL), 0.02, Some(MOD_NEWLY_WEALTHY), [-0.2, 0.0, -0.2, 0.0], "{name} turns the soldiers loose on the town."),
        ] },
    ChoiceTemplate { id: 2045, requires: TAG_ROLE_ADMIRAL, weight: 2.0,
        prompt: "A storm is building over the approaches to {city} as {name}'s fleet makes ready.",
        options: [
            opt!("puts out anyway", 0.4, &[(TRAIT_BRAVE, 0.15), (TRAIT_SEAFARER, 0.15), (TRAIT_IMPULSIVE, 0.12)], &[(MOD_RUSHED, 0.15)], Some(TRAIT_SEAFARER), 0.05, None, NO, "{name} puts out into the storm and brings every hull home."),
            opt!("waits it out", 0.6, &[(TRAIT_TEMPERATE, 0.15), (TRAIT_STRATEGIST, 0.1)], &[], Some(TRAIT_TEMPERATE), 0.01, None, NO, "{name} waits out the weather in the roads."),
        ] },
    // ── Artisans and performers ─────────────────────────────────────────
    ChoiceTemplate { id: 2050, requires: TAG_ROLE_ARTISAN, weight: 2.0,
        prompt: "{name} dreams of a work that would take years and might never be finished.",
        options: [
            opt!("attempts it", 0.45, &[(TRAIT_AMBITIOUS, 0.18), (TRAIT_PROUD, 0.12), (TRAIT_PATRON_OF_ARTS, 0.1)], &[(MOD_IN_LOVE, 0.1)], Some(TRAIT_AMBITIOUS), 0.05, None, NO, "{name} shuts the workshop door and begins the great work."),
            opt!("takes commissions", 0.55, &[(TRAIT_CONTENT, 0.15), (TRAIT_GREEDY, 0.12)], &[(MOD_OWES_DEBT, 0.15)], Some(TRAIT_CONTENT), 0.0, Some(MOD_NEWLY_WEALTHY), [0.0, 0.0, 0.0, 0.1], "{name} takes the paying commissions, and the dream waits."),
        ] },
    ChoiceTemplate { id: 2051, requires: TAG_ROLE_ARTISAN, weight: 1.5,
        prompt: "An apprentice in {city} begs {name} for the secret of the craft.",
        options: [
            opt!("teaches it", 0.45, &[(TRAIT_GENEROUS, 0.15), (TRAIT_KIND, 0.12)], &[], Some(TRAIT_GENEROUS), 0.02, None, [0.1, 0.1, 0.1, 0.0], "{name} teaches the apprentice everything."),
            opt!("guards it", 0.55, &[(TRAIT_GREEDY, 0.12), (TRAIT_CLOSED, 0.15), (TRAIT_PROUD, 0.08)], &[], Some(TRAIT_CLOSED), 0.0, None, [0.0, -0.1, -0.1, 0.0], "{name} guards the secret jealously."),
        ] },
    ChoiceTemplate { id: 2055, requires: TAG_ROLE_PERFORMER, weight: 2.0,
        prompt: "{name} is asked to perform before the rulers of {city}.",
        options: [
            opt!("flatters them", 0.5, &[(TRAIT_AMBITIOUS, 0.12), (TRAIT_DECEITFUL, 0.15), (TRAIT_CRAVEN, 0.08)], &[(MOD_FLATTERED, 0.1)], None, 0.03, Some(MOD_NEWLY_WEALTHY), [-0.2, 0.0, 0.0, 0.0], "{name} sings the rulers' praises and is showered with gifts."),
            opt!("satirises them", 0.5, &[(TRAIT_BRAVE, 0.15), (TRAIT_HONEST, 0.12), (TRAIT_PROUD, 0.08)], &[(MOD_EMBOLDENED, 0.1)], Some(TRAIT_BRAVE), 0.06, Some(MOD_THREATENED), [0.4, 0.1, 0.0, 0.0], "{name} mocks the rulers to their faces, and the whole city laughs."),
        ] },
];

/// The opposed personality/reputation pairs — gaining one clears the other,
/// so a character can genuinely CHANGE (a craven who stands on the walls).
pub fn opposite_trait(t: u8) -> Option<u8> {
    Some(match t {
        TRAIT_KIND => TRAIT_CRUEL, TRAIT_CRUEL => TRAIT_KIND,
        TRAIT_BRAVE => TRAIT_CRAVEN, TRAIT_CRAVEN => TRAIT_BRAVE,
        TRAIT_HONEST => TRAIT_DECEITFUL, TRAIT_DECEITFUL => TRAIT_HONEST,
        TRAIT_PROUD => TRAIT_HUMBLE, TRAIT_HUMBLE => TRAIT_PROUD,
        TRAIT_AMBITIOUS => TRAIT_CONTENT, TRAIT_CONTENT => TRAIT_AMBITIOUS,
        TRAIT_CURIOUS => TRAIT_CLOSED, TRAIT_CLOSED => TRAIT_CURIOUS,
        TRAIT_LOYAL => TRAIT_FICKLE, TRAIT_FICKLE => TRAIT_LOYAL,
        TRAIT_TEMPERATE => TRAIT_IMPULSIVE, TRAIT_IMPULSIVE => TRAIT_TEMPERATE,
        TRAIT_GENEROUS => TRAIT_GREEDY, TRAIT_GREEDY => TRAIT_GENEROUS,
        TRAIT_HERO => TRAIT_COWARD, TRAIT_COWARD => TRAIT_HERO,
        TRAIT_ROBUST => TRAIT_SICKLY, TRAIT_SICKLY => TRAIT_ROBUST,
        _ => return None,
    })
}

/// What a trait gain actually did to a person — for the log and the UI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TraitChange {
    None,
    Gained(u8),
    Deepened(u8),
    /// (new, lost)
    Replaced(u8, u8),
}

/// Apply a trait gain to a trait list: an opposite is REPLACED (strength 1 —
/// a new habit, not yet a nature), a held trait DEEPENS to 2, a fresh one is
/// added at 1 while under `TRAIT_CAP_MAX`. Pure.
pub fn apply_trait_gain(traits: &mut Vec<(u8, i8)>, t: u8) -> TraitChange {
    if let Some(slot) = traits.iter_mut().find(|(tt, _)| *tt == t) {
        if slot.1 < 2 { slot.1 = 2; return TraitChange::Deepened(t); }
        return TraitChange::None;
    }
    if let Some(opp) = opposite_trait(t) {
        if let Some(pos) = traits.iter().position(|(tt, _)| *tt == opp) {
            // A deeply held opposite only WEAKENS first; a light one flips.
            if traits[pos].1 >= 2 {
                traits[pos].1 = 1;
                return TraitChange::None;
            }
            traits[pos] = (t, 1);
            return TraitChange::Replaced(t, opp);
        }
    }
    if traits.len() < TRAIT_CAP_MAX {
        traits.push((t, 1));
        return TraitChange::Gained(t);
    }
    TraitChange::None
}

/// Is a stored life entry a KEY moment of the life (a choice, a milestone, or
/// a happening that changed the person — fame or a trait)?
pub fn is_key_life_entry(e: &IndividualLifeEntry) -> bool {
    if e.template_id >= MS_DEBUT { return true; }
    EVENT_TEMPLATES.iter().find(|t| t.id == e.template_id)
        .map(|t| t.fame_delta >= 0.02 || t.trait_gain.is_some() || t.feature_gain.is_some())
        .unwrap_or(false)
}

/// Ordinary people keep at most this many KEY entries (the story) on top of
/// `ORDINARY_LIFE_LOG_CAP` ordinary ones (the texture). Famous people keep all.
pub const ORDINARY_KEY_LOG_CAP: usize = 16;

/// Renown per year by what a person IS, before the city-standing multiplier.
fn role_renown(role: u8) -> f32 {
    match role {
        ROLE_RULER | ROLE_HORDE_LEADER => 0.07,
        ROLE_COMMANDER | ROLE_ADMIRAL => 0.04,
        ROLE_PHILOSOPHER | ROLE_IDEOLOGUE => 0.04,
        ROLE_MERCHANT_PRINCE | ROLE_BANKER => 0.04,
        ROLE_SCHOLAR => 0.015,
        ROLE_DEMAGOGUE => 0.035,
        ROLE_GUILDMASTER | ROLE_ARTISAN | ROLE_PERFORMER | ROLE_EXPLORER => 0.025,
        ROLE_ALDERMAN | ROLE_DIPLOMAT | ROLE_PHYSICIAN => 0.015,
        _ => 0.008,
    }
}

/// A great city's people are heard of further (tier 1 × 2.2 … untiered × 1).
fn tier_renown_mult(tier: u8) -> f32 {
    match tier { 1 => 2.0, 2 => 1.5, 3 => 1.1, _ => 0.8 }
}

impl CampaignSim {
    fn person_index(&self, id: u32) -> Option<usize> {
        self.people.iter().position(|p| p.id == id)
    }

    /// Append a life entry, pruning an ORDINARY person's log with the key
    /// entries protected: chatter is dropped oldest-first past
    /// `ORDINARY_LIFE_LOG_CAP`, key entries past `ORDINARY_KEY_LOG_CAP`.
    pub(crate) fn push_life_entry(&mut self, i: usize, e: IndividualLifeEntry) {
        let famous = self.people[i].famous;
        let log = &mut self.people[i].life_log;
        log.push(e);
        if famous { return; }
        for (want_key, cap) in [(false, ORDINARY_LIFE_LOG_CAP), (true, ORDINARY_KEY_LOG_CAP)] {
            let count = log.iter().filter(|x| is_key_life_entry(x) == want_key).count();
            if count > cap {
                let mut drop = count - cap;
                log.retain(|x| {
                    if drop > 0 && is_key_life_entry(x) == want_key { drop -= 1; false } else { true }
                });
            }
        }
    }

    /// Log a career milestone for person `id` (no-op if they are not alive in
    /// `people`). `args[0]` is always a hub or `u32::MAX`.
    pub(crate) fn log_milestone(&mut self, id: u32, ms: u16, args: Vec<u32>) {
        if let Some(i) = self.person_index(id) {
            let e = IndividualLifeEntry { tick: self.tick, template_id: ms, args, why: Vec::new() };
            self.push_life_entry(i, e);
        }
    }

    /// A trait change with its consequences: face feature for a health
    /// trait, and a `MS_TRAIT_FLIP` milestone when one nature REPLACES its
    /// opposite (the moment a character visibly changes).
    pub(crate) fn gain_trait_logged(&mut self, i: usize, t: u8) -> TraitChange {
        let change = apply_trait_gain(&mut self.people[i].traits, t);
        match change {
            TraitChange::Gained(g) | TraitChange::Replaced(g, _) => {
                let f = feature_for_trait(g);
                if f != 0 { self.people[i].features |= f; }
            }
            _ => {}
        }
        if let TraitChange::Replaced(new, lost) = change {
            let hub = self.people[i].current_hub;
            let hub_arg = if hub >= 0 { hub as u32 } else { u32::MAX };
            let e = IndividualLifeEntry { tick: self.tick, template_id: MS_TRAIT_FLIP, args: vec![hub_arg, new as u32, lost as u32], why: Vec::new() };
            self.push_life_entry(i, e);
        }
        change
    }

    /// One dilemma for `people[i]`, if any choice template matches their
    /// circumstances. Returns false when none does (the caller then fires an
    /// ordinary happening instead).
    pub(crate) fn fire_choice_event(&mut self, i: usize) -> bool {
        let tags = self.individual_context_tags(&self.people[i]);
        let candidates: Vec<&ChoiceTemplate> = CHOICE_TEMPLATES.iter()
            .filter(|t| t.requires == TAG_GENERIC || (t.requires & tags) == t.requires)
            .collect();
        if candidates.is_empty() { return false; }
        let total: f32 = candidates.iter().map(|t| t.weight).sum();
        let id = self.people[i].id;
        let r = hash01(self.seed, self.tick as u64, (id as u64) ^ salts::CHOICE_PICK) * total;
        let mut acc = 0.0f32;
        let mut t = candidates[candidates.len() - 1];
        for c in &candidates {
            acc += c.weight;
            if r < acc { t = c; break; }
        }
        let base = [t.options[0].base, t.options[1].base];
        let trait_terms: Vec<Vec<(u8, f32)>> = t.options.iter().map(|o| o.traits.to_vec()).collect();
        let mod_terms: Vec<Vec<(u8, f32)>> = t.options.iter().map(|o| o.mods.to_vec()).collect();
        let outcome = decide(
            self.seed, self.tick, id, salts::CHOICE_DECIDE ^ t.id as u64,
            &base, &trait_terms, &self.people[i].traits, &mod_terms, &self.people[i].modifiers, &[0.0, 0.0],
        );
        let choice = outcome.choice.min(1);
        let o = &t.options[choice];
        // Reasons as codes (trait ids / modifier kinds), resolved from the
        // chosen option's own terms so the UI can name them in words.
        let mut why: Vec<(u16, f32)> = Vec::new();
        for &(tk, w) in o.traits {
            if let Some(&(_, st)) = self.people[i].traits.iter().find(|&&(tt, _)| tt == tk) {
                why.push((tk as u16, w * st as f32));
            }
        }
        for &(mk, w) in o.mods {
            if self.people[i].modifiers.iter().any(|m| m.kind == mk) {
                why.push((REASON_MODIFIER_BASE + mk as u16, w));
            }
        }
        why.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
        why.truncate(3);
        let hub = self.people[i].current_hub;
        let hub_arg = if hub >= 0 { hub as u32 } else { u32::MAX };
        let entry = IndividualLifeEntry {
            tick: self.tick, template_id: t.id, args: vec![hub_arg, choice as u32],
            why: why.into_iter().map(|(c, _)| c).collect(),
        };
        let famous = self.people[i].famous;
        self.push_life_entry(i, entry.clone());
        if o.fame != 0.0 {
            self.people[i].fame = (self.people[i].fame + o.fame).clamp(0.0, 1.0);
        }
        if let Some(g) = o.gain {
            // A choice may always DEEPEN or FLIP a trait; it adds a NEW one
            // only while the person carries fewer than
            // `CHOICE_NEW_TRAIT_SOFT_CAP` (reputation traits excepted) —
            // measured, every character drifted to ~6 traits without it.
            let has = self.people[i].traits.iter().any(|&(t, _)| t == g)
                || opposite_trait(g).map(|o| self.people[i].traits.iter().any(|&(t, _)| t == o)).unwrap_or(false);
            let reputation = (TRAIT_HERO..=TRAIT_KIN_SLAYER).contains(&g);
            if has || reputation || self.people[i].traits.len() < CHOICE_NEW_TRAIT_SOFT_CAP {
                self.gain_trait_logged(i, g);
            }
        }
        if let Some(mk) = o.modifier {
            let p = &mut self.people[i];
            p.modifiers.retain(|m| m.kind != mk);
            if p.modifiers.len() < MODIFIER_CAP {
                p.modifiers.push(Modifier { kind: mk, value: MODIFIER_MAGNITUDE, expires_tick: self.tick + CHOICE_MODIFIER_TICKS, note: String::new() });
            }
        }
        if self.people[i].ideology_seeded {
            let mut pos = self.people[i].ideology;
            for k in 0..4 { pos[k] += o.ideo[k]; }
            self.people[i].ideology = super::ideology::clamp_ideology(pos);
        }
        if famous {
            let name = self.people[i].name.clone();
            let text = self.render_life_entry_for(&name, &entry);
            self.journal.push(JournalEntry { tick: self.tick, kind: "individual".into(), hub, good: -1, value: 0.0, text });
        }
        true
    }

    /// Yearly renown: fame by the roles and offices a person actually holds,
    /// scaled by their city's standing. Runs before `people_yearly_pass`'s
    /// decay/promotion, so a head of a great city can climb onto the roster.
    pub(crate) fn people_renown_pass(&mut self) {
        // Who sits where: individual id → (hub, seat role).
        let mut seated: std::collections::HashMap<i32, (usize, u8)> = std::collections::HashMap::new();
        for (h, hub) in self.hubs.iter().enumerate() {
            if hub.is_estate || hub.abandoned { continue; }
            for o in &hub.officials {
                if o.individual_id >= 0 {
                    let e = seated.entry(o.individual_id).or_insert((h, o.role));
                    if o.role < e.1 { *e = (h, o.role); }
                }
            }
        }
        for i in 0..self.people.len() {
            if !self.people[i].is_alive() { continue; }
            let mut gain = self.people[i].roles.iter().map(|&r| role_renown(r)).fold(0.0f32, f32::max);
            let mut hub = self.people[i].current_hub;
            if let Some(&(h, role)) = seated.get(&(self.people[i].id as i32)) {
                // The head of a government is a ruler in all but name.
                gain = gain.max(if role == 0 { role_renown(ROLE_RULER) } else { 0.012 });
                hub = h as i32;
            }
            // A teacher's renown grows with the city's schools.
            if self.people[i].scholar_stage == super::ideology::STAGE_TEACH {
                gain += 0.01;
            }
            let mult = self.hubs.get(hub.max(0) as usize).filter(|_| hub >= 0)
                .map(|h| tier_renown_mult(h.tier)).unwrap_or(1.0);
            let before = self.people[i].fame;
            self.people[i].fame = (before + gain * mult).min(1.0);
        }
    }

    /// 2026-09-30 · people travel. Before this only scholars (study, exile)
    /// and artisans (an untraced commission) ever changed city; a merchant
    /// prince, a performer or a diplomat lived and died where they debuted.
    /// Yearly, a person NOT holding a government seat may set out — how
    /// often by role (`travel_propensity`) — to a trade neighbour (the more
    /// standing, the likelier). Whether a journey becomes a new home is a
    /// `decide()` against their own character (curious/ambitious/fickle go;
    /// loyal/content/closed and homesick stay). Someone living away from home
    /// may go back (`HOMECOMING_CHANCE`, doubled when homesick). Every move
    /// and visit is a KEY milestone, which is the travel history the UI reads.
    /// Touches only `current_hub` of people — no hub, house or price.
    pub(crate) fn people_travel_pass(&mut self, yr: u32) {
        let seated: std::collections::HashSet<i32> = self.hubs.iter()
            .flat_map(|h| h.officials.iter().map(|o| o.individual_id)).collect();
        for i in 0..self.people.len() {
            let p = &self.people[i];
            if !p.is_alive() || seated.contains(&(p.id as i32)) { continue; }
            // Scholars have their own career moves (ideology.rs); a horde
            // leader moves with the horde.
            if p.roles.iter().any(|&r| matches!(r, ROLE_SCHOLAR | ROLE_PHILOSOPHER | ROLE_IDEOLOGUE | ROLE_HORDE_LEADER)) { continue; }
            let here = p.current_hub;
            if here < 0 || here as usize >= self.hubs.len() { continue; }
            let hu = here as usize;
            let id = p.id;
            let role = *p.roles.last().unwrap_or(&ROLE_OFFICIAL);
            let home = p.origin_hub;
            let homesick = p.modifiers.iter().any(|m| m.kind == MOD_HOMESICK);
            let key = (yr as u64) ^ (id as u64).wrapping_mul(0x9E3779B1);
            // Homecoming first.
            if home >= 0 && home != here && (home as usize) < self.hubs.len() && !self.hubs[home as usize].abandoned {
                let chance = HOMECOMING_CHANCE * if homesick { 2.0 } else { 1.0 };
                if hash01(self.seed, key, salts::TRAVEL_ROLL ^ 0x40) < chance {
                    self.people[i].current_hub = home;
                    self.people[i].modifiers.retain(|m| m.kind != MOD_HOMESICK);
                    self.log_milestone(id, MS_RETURN, vec![home as u32]);
                    continue;
                }
            }
            let (go, _stay, reason) = travel_propensity(role);
            let go = if self.people[i].famous { go * 1.5 } else { go };
            if hash01(self.seed, key, salts::TRAVEL_ROLL) >= go { continue; }
            // Destination: a live trade neighbour, weighted toward standing.
            let cands: Vec<usize> = self.neighbors.get(hu).cloned().unwrap_or_default().iter()
                .map(|&b| b as usize)
                .filter(|&b| b < self.hubs.len() && b != hu && !self.hubs[b].is_estate && !self.hubs[b].abandoned)
                .collect();
            if cands.is_empty() { continue; }
            let w: Vec<f32> = cands.iter().map(|&b| 1.0 + match self.hubs[b].tier { 1 => 3.0, 2 => 2.0, 3 => 1.0, _ => 0.0 }).collect();
            let total: f32 = w.iter().sum();
            let mut r = hash01(self.seed, key, salts::TRAVEL_ROLL ^ 0x41) * total;
            let mut dest = cands[0];
            for (k, &b) in cands.iter().enumerate() { if r < w[k] { dest = b; break; } r -= w[k]; }
            let (_, stay_base, _) = travel_propensity(role);
            let outcome = decide(
                self.seed, self.tick, id, salts::TRAVEL_ROLL ^ 0x42,
                &[stay_base, 1.0 - stay_base],
                &[vec![(TRAIT_CURIOUS, 0.08), (TRAIT_AMBITIOUS, 0.08), (TRAIT_FICKLE, 0.08)],
                  vec![(TRAIT_LOYAL, 0.08), (TRAIT_CONTENT, 0.08), (TRAIT_CLOSED, 0.08)]],
                &self.people[i].traits,
                &[vec![(MOD_NEWLY_WEALTHY, 0.05)], vec![(MOD_HOMESICK, 0.1), (MOD_GRIEVING, 0.05)]],
                &self.people[i].modifiers, &[0.0, 0.0],
            );
            if outcome.choice == 0 {
                self.people[i].current_hub = dest as i32;
                self.log_milestone(id, MS_MOVE, vec![dest as u32, hu as u32, reason]);
            } else {
                self.log_milestone(id, MS_VISIT, vec![dest as u32, hu as u32, reason]);
                if self.people[i].famous {
                    self.people[i].fame = (self.people[i].fame + 0.01).min(1.0);
                }
            }
        }
    }

    /// Render a milestone entry in words.
    pub(crate) fn render_milestone(&self, name: &str, e: &IndividualLifeEntry) -> Option<String> {
        let city = |k: usize| e.args.get(k).and_then(|&h| self.hubs.get(h as usize)).map(|h| h.name.clone())
            .unwrap_or_else(|| "a distant place".into());
        let person = |k: usize| e.args.get(k).map(|&pid| {
            self.people.iter().chain(self.hall_of_dead.iter()).find(|p| p.id == pid).map(|p| p.name.clone())
                .or_else(|| self.people_tombstones.iter().find(|t| t.id == pid).map(|t| t.name.clone()))
                .unwrap_or_else(|| "a forgotten master".into())
        }).unwrap_or_default();
        let ideology = |k: usize| e.args.get(k).and_then(|&id| self.ideologies.iter().find(|i| i.id == id))
            .map(|i| i.name.clone()).unwrap_or_else(|| "a doctrine now lost".into());
        Some(match e.template_id {
            MS_DEBUT => format!("{name} enters public life in {} as {}.", city(0),
                e.args.get(1).map(|&r| role_name(r as u8).to_lowercase()).map(|r| format!("a{} {r}", if r.starts_with(['a','e','i','o','u']) { "n" } else { "" })).unwrap_or_else(|| "a citizen".into())),
            MS_OFFICE => format!("{name} takes a seat in the government of {}.", city(0)),
            MS_STUDY => format!("{name} goes to {} to study under {}.", city(0), person(1)),
            MS_TEACH => format!("{name} opens a lecture hall in {} and begins to teach.", city(0)),
            MS_SCHOOL => format!("{name} founds a school in {}, teaching {}.", city(0), ideology(1)),
            MS_DOCTRINE => format!("{name} sets down a new doctrine — {}.", ideology(1)),
            MS_EXILE => format!("{name} is driven out of {} and finds refuge in {}.", city(1), city(0)),
            MS_RETURN => format!("{name} comes home to {}.", city(0)),
            MS_PATRON => format!("{name} enters the household of a patron in {}.", city(0)),
            MS_MASTERWORK => format!("{name} completes a masterwork in {}.", city(0)),
            MS_RENOWN => format!("{name}'s name is now known far beyond {}.", city(0)),
            MS_DEATH => format!("{name} dies of {} in {}.", e.args.get(1).map(|&c| death_cause_name(c as u8)).unwrap_or("old age"), city(0)),
            MS_POLITICS => format!("{name} turns from the lecture hall to politics in {}.", city(0)),
            MS_MOVE => format!("{name} leaves {} {} and settles in {}.", city(1), move_reason(e.args.get(2).copied().unwrap_or(0)), city(0)),
            MS_VISIT => format!("{name} travels from {} to {} {}.", city(1), city(0), move_reason(e.args.get(2).copied().unwrap_or(0))),
            MS_TRAIT_FLIP => format!("{name} is no longer {} — life has made them {}.",
                e.args.get(2).map(|&t| trait_name(t as u8).to_lowercase()).unwrap_or_default(),
                e.args.get(1).map(|&t| trait_name(t as u8).to_lowercase()).unwrap_or_default()),
            _ => return None,
        })
    }

    /// Render a choice entry: the situation and what they did.
    pub(crate) fn render_choice(&self, name: &str, e: &IndividualLifeEntry) -> Option<String> {
        let t = CHOICE_TEMPLATES.iter().find(|t| t.id == e.template_id)?;
        let choice = e.args.get(1).copied().unwrap_or(0).min(1) as usize;
        let city = e.args.first().and_then(|&h| self.hubs.get(h as usize)).map(|h| h.name.as_str()).unwrap_or("an unnamed place");
        let fill = |s: &str| s.replace("{name}", name).replace("{city}", city);
        Some(format!("{} {}", fill(t.prompt), fill(t.options[choice].outcome)))
    }

    /// Words for a choice entry's stored reasons.
    pub(crate) fn render_choice_reasons(&self, e: &IndividualLifeEntry) -> Vec<String> {
        e.why.iter().map(|&c| {
            if c >= REASON_CONTEXT { "the circumstances".to_string() }
            else if c >= REASON_MODIFIER_BASE { modifier_name((c - REASON_MODIFIER_BASE) as u8).to_string() }
            else { trait_name(c as u8).to_lowercase() }
        }).collect()
    }
}

/// The chosen option's short label, if `e` is a choice entry.
pub fn choice_label(e: &IndividualLifeEntry) -> Option<&'static str> {
    let t = CHOICE_TEMPLATES.iter().find(|t| t.id == e.template_id)?;
    Some(t.options[e.args.get(1).copied().unwrap_or(0).min(1) as usize].label)
}

/// The trait a choice entry's chosen option grants, if any.
pub fn choice_gain(e: &IndividualLifeEntry) -> Option<u8> {
    let t = CHOICE_TEMPLATES.iter().find(|t| t.id == e.template_id)?;
    t.options[e.args.get(1).copied().unwrap_or(0).min(1) as usize].gain
}
