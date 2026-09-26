# 07 · Artisans and masterworks

**Status:** DONE (2026-09-26) — 07.1-07.7 all shipped · **Depends on:** 02, 03 · **Next:** 08

**Shipped, in one session.** `sim/campaign/tick/masterworks.rs`: a real
`Masterwork` record (title/kind/maker/year/owner/material/location/
condition/prestige/provenance), created either by a guild whose (now
CULTURALLY capped — `cultural_quality_cap_e`, blended into `run_craft_
guilds`' own cap at `CULTURAL_QUALITY_CAP_DOSE`) quality clears
`MASTERPIECE_QUALITY` at ideological level ≥ 3, or by a notable
`ROLE_ARTISAN` individual's talent alone, no ceiling. `loot_masterworks(hub,
to_hub)` is real and tested directly (row 09's sacks are the eventual
caller — and DO call it, see row 09's own entry, closing that loop this
same session). Theft (`maybe_steal_masterwork`) is a real yearly house
event resisted by the target city's own guild secrecy. Invitations
(`update_artisan_lives`) use `decide()`'s 75% rule for commission/relocate/
stay, reusing the exact "return home in the same call" pattern the gate
demands.

**Dosed at zero:** `MASTERWORK_DEV_BONUS_DOSE` (a masterwork's bounded
development-track bonus) and `MASTERWORK_MARKET_DOSE` (house purchases —
07.4's real wealth-moving market is NOT built this session, only its dose
constant and cap; see Queue). Both proven true no-ops.

Gates: `cultural_cap_is_a_noop_at_zero`, `guilds_need_culture_for_
masterpieces`, `talent_needs_no_threshold`, `masterwork_prestige_is_
bounded`, `masterwork_purchases_are_noops_at_zero`, `masterwork_dev_bonus_
is_a_noop_at_zero`, `provenance_records_every_move`, `commissioned_
artisans_return_home`. `tick::tests` (390/390), `npx tsc --noEmit`/`npx
vite build` (191 modules) both clean.

**UI:** a compact "Gallery" section in the Government panel (city
masterworks with maker/condition/provenance on hover) — not the doc's own
dedicated City Gallery / House Dossier tab / world "Great Works" list.

## Goal

Craft guilds improve over the centuries up to a ceiling set by how cultured their
city is; once the city is cultured enough they make masterpieces. Talented
individual artisans make masterpieces by talent alone. Every masterwork is a named
object with a history — commissioned, admired, bought, looted, stolen, destroyed —
and it lends its city prestige.

## Decisions (maintainer)

- Guild quality rises over time, **limited by the city's cultural development**
  (the Ideological track, row 03). Past a threshold a guild can produce
  **masterpieces**.
- **Notable artisans ignore that limit** — talent alone.
- Masterworks are **stored** in the city, add **a little prestige**, bounded, plus a
  **bounded development bonus**.
- They can be **looted or destroyed** by barbarians and armies, and **stolen** by a
  house (rare — like Venice taking St Mark's body from Alexandria in 828).
- **Houses or people abroad can buy** masterworks if the house has reach into the
  city; houses keep **their own gallery**; the house leader's traits shape buying.
- Artisans can be **invited** elsewhere — **variant C**: temporary commissions
  (they travel, make the work, return) + relocation when pushed (war, plague) or
  offered much more. Chance-based decisions (row 02).
- Artisans have **characters, appearance, lives and events** like everyone else,
  including funny ones, and a **works gallery**.

## What exists today (verified)

Per-(hub, good) quality with `TickHub.tradition` (years of practice), craft guilds
(`CraftGuild`, strength, secrecy, `signature` brand names), `maybe_steal_quality`,
`maybe_poach_master`, `GUILD_QUALITY_CAP`/`GUILD_MONOPOLY_QUALITY_CAP`, the
Master Craftsman figure kind, house galleries do not exist.

## Kinds of artisan

Sculptor · bronze-caster · painter · fresco artist · mosaicist · architect ·
goldsmith · gem-cutter · **die-engraver** (cuts coin dies — links to mints) ·
vase-painter · glassblower · tapestry-weaver · calligrapher · poet · playwright ·
musician · historian. Each culture's leisure family (row 08) and traits weight
which kinds appear (a calligrapher is likelier in a Chinese-like city, a mosaicist
in a Greco-Roman-like one).

## Quality ceiling

```
guild ceiling = min(existing tradition/guild cap, CULTURAL_CAP[ideological level])
masterpiece possible when ceiling ≥ MASTERPIECE_QUALITY and ideological level ≥ 3
```

Notable artisans: a talent value (0–1, at debut) and experience; a masterwork roll
each year scaled by talent, with no ceiling.

## The masterwork record

```rust
pub struct Masterwork {
    pub id: u32,
    pub title: String,          // generated: type + subject + place ("the Bronze Charioteer of Kedra")
    pub kind: u8,               // statue, fresco, mosaic, ode, epic, play, building, treatise, jewel…
    pub maker: u32,             // Individual id (tombstoned if an ordinary maker dies) (a guild's work credits the guildmaster)
    pub year: u32,
    pub patron: i32,            // house / realm / city (tagged)
    pub material: String,       // from the city's real goods where possible (marble, bronze, gold)
    pub location_hub: i32,
    pub owner: Owner,           // city / house gallery / realm
    pub condition: u8,          // intact, damaged, looted, destroyed
    pub prestige: f32,          // bounded
    pub provenance: Vec<(u32, String)>, // tick + "bought by House Varro", "carried off by the Horde of Kaan"…
}
```

Caps: masterworks per city (e.g. 30; the least prestigious drift into
"lost/forgotten" rather than being deleted from provenance). Prestige per city is
bounded and decays slowly unless the work is still there.

## Market, theft, looting

- **Buying**: a house with an office, bailo or strong trade tie in the city; a
  crown; a rich city. Masterwork prestige for its owner is **capped** per owner and
  decays unless the work is kept (rule 18). Price ∝ prestige × rarity. Appetite from the house head's
  traits and vices (Lavish buys, Miserly doesn't, Greedy resells, Patron of the
  Arts commissions). The seller is the owning city or house. **This moves real
  wealth** → dosed from zero, with a ceiling on spend per house per year.
- **Theft**: a rare house event, chance from the house's Deceitful/Greedy head and
  the target's weak watch; exposure risk; the victim city's relation to the thief's
  culture falls (row 05).
- **Looting**: a sack (row 09) carries off works to the victor's city or horde;
  some are destroyed. Provenance records it.

## Invitations (variant C)

- **Commission**: a patron abroad offers a fee for one work. The artisan decides
  (row 02 rule): fee vs home ties, traits (Ambitious, Curious), danger of the
  road, war on the route. They travel, make the work there, return.
- **Relocation**: yearly, the artisan compares cities — patronage, peace,
  prestige, the guild, their culture's tier there. They move on a push (war,
  plague, persecution) or when the gap is large.
- Michelangelo is the pattern: called to Rome by commission, repeatedly returning
  to Florence.

## Artisan events (a flavour)

Serious and funny, geography-checked (row 02 lint):
- "The statue slipped from the crane into the harbour; divers raised it, and the
  salt stain became its famous feature" (`coast`)
- "A patron refused to pay; he carved him an ass's ears"
- "Rivals bribed the judges; his ode lost, and his satire outlived them all"
- "The new tyrant had the old regime's statues recut with his own face"
  (after a change of government — Rome's *damnatio memoriae*)
- "A drunken poet wagered his manuscript at dice and lost it to a sailor"
- "The kiln exploded; the cracked vase became the fashion"
- "Commissioned for a victory statue, the war was lost before he finished"

## UI

- City **Gallery**: its masterworks, with maker, year, condition, provenance.
- House **Gallery** tab in the House Dossier.
- Artisan's person page: their **works**.
- A world "Great Works" list (most prestigious, by culture, lost works).

## Slices

| Slice | Content | Gate |
|---|---|---|
| 07.1 | Artisan kinds as roles; talent; culturally capped guild ceiling (dose 0 = the old cap) | `cultural_cap_is_a_noop_at_zero` |
| 07.2 | `Masterwork` records: creation by guild (threshold) and by talent | `guilds_need_culture_for_masterpieces`, `talent_needs_no_threshold` |
| 07.3 | Prestige and development bonus (bounded) | `masterwork_prestige_is_bounded` |
| 07.4 | Market: houses buy into galleries; appetite from traits; spend ceiling (dose 0) | `purchases_are_noops_at_zero` |
| 07.5 | Theft, provenance, and the **looting function** `loot_masterworks(hub, to)` — called by row 09's sacks (built later); tested here directly | `provenance_records_every_move` |
| 07.6 | Invitations: commissions and relocation (variant C) | `commissioned_artisans_return_home` |
| 07.7 | Galleries UI; end of row: dose the market; `tick::tests` + `econ_` | SCOREBOARD row |

## Queue
- Q07.1 — Masterworks as diplomatic gifts between cities (waits on row 09's
  full realm/diplomacy machinery — row 09 shipped hordes/barbarians only
  this session, not realm-to-realm relations).
- Q07.2 — Forgeries (a rare event), waits on 07.4 being live.
- Q07.3 — Real house purchases (07.4's actual market — money moving between
  a buyer house and a seller city/house, capped per house per year) — the
  dose constant and cap exist, the transfer mechanism does not yet.
- Q07.4 — Artisan life-event templates (the doc's own "a flavour" list) —
  needs authoring in row 02's `EVENT_TEMPLATES`, a content pass not a
  mechanism.
- Q07.5 — Dedicated Galleries UI (City Gallery, House Dossier tab, world
  "Great Works" list) — `campaign_get_city_gallery`/`campaign_get_house_
  gallery` already serve the data; only the windows wait on session budget.
- Q07.6 — Per-artisan "kind" (sculptor/painter/…) as a real trait rather than
  a hashed pick at creation time, weighted by row 08's leisure families.
