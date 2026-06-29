# Heraldry, House Names & Guilds — "more variants" proposal

Concrete expansion of the three identity systems, ready to wire into the app.
Visual sheet: [`docs/mockups/heraldry-variants.svg`](mockups/heraldry-variants.svg).

## 1. Coat of arms (`src/ui/CoatOfArms.tsx`)

Today: 8 tinctures · **1** shield shape (heater) · **6** guild charges (geometric) ·
**16** house charges (emoji) · 8 divisions · 6 ordinaries.

Proposed additions (all deterministic from the existing name hash — pick more
bits off `h`):

| Axis | Today | Add |
|---|---|---|
| **Shield shape** | heater only | Iberian (round base), French (square+point), Swiss (curved), Cartouche (oval), Lozenge (guilds) — pick by `(h>>3)%6` |
| **Furs** | none | **ermine** & **vair** as field/division fills (rule-of-tincture neutral) — `(h>>9)` |
| **Divisions** | 8 | + Per chevron, Paly, Barry, Gyronny, Checky → 13 total |
| **Ordinaries** | 6 | + Saltire, Chief, Bordure, Pile → 10 total |
| **Charges** | 16 **emoji** | replace with an **18-strong VECTOR set** (crisp at any size, tintable to the rule of tincture, render in any environment — emoji don't): fleur-de-lis, lion, eagle, tower, crown, mullet, estoile, sun, crescent, rose, garb, escallop, sword, key, anchor, hammer, trefoil, boar. Extensible to stag, dolphin, swan, griffin, ship, axe, ram. |

Why vector over emoji: the current charges are emoji glyphs, which (a) don't
recolour for contrast, (b) render differently per-OS, and (c) vanish in
SVG/PNG export. The vector set fixes all three and roughly **doubles** the
visible variety once shapes × furs × divisions × ordinaries × charges combine.

Combinatorial variety (deterministic per name):
`6 shapes × (8 tinctures + 2 furs) × 13 divisions × 10 ordinaries × 18 charges`
≈ tens of thousands of distinct, reproducible arms (vs ~a few thousand today).

## 2. House names (`src-tauri/src/sim/cultures.rs` → `Kit.family`)

Today each of the 12 culture kits has ~12 surnames. Proposed: roughly **double**
each pool (append-only, so existing saves are unaffected). Examples to add:

- **Roman**: Sergii, Manlii, Caecilii, Sempronii, Postumii, Hortensii, Pinarii, Furii, Atilii, Verginii
- **Greek**: Alkmaionidai→ + Gephyraioi, Praxiergidai, Salaminioi, Theodoridai, Hippeis, Eteoboutadai
- **Punic**: + Hannonids, Adonids, Bodashtarids, Hannibaals, Carthalonids, Safotids
- **Persian**: + Ispahbudhan, Zik, Aspahbadh, Waraz, Andigan, Qaren-Pahlav
- **Norse**: + Magnusson, Steinarsson, Vagnsson, Skjoldung, Yngling, Hlathir
- **Celtic**: + Atrebates, Trinovantes, Ordovices, Silures, Durotriges, Coritani
- **Arabic**: + al-Tanukhi, al-Ghassani, al-Lakhmi, al-Azdi, al-Qurashi, al-Tamimi
- **Indic**: + Vardhana, Kadamba, Western Ganga, Maitraka, Pratihara, Chahamana
- **Sinitic**: + Lin, He, Gao, Luo, Song, Tang, Feng, Deng
- **Slavic**: + Izyaslavichi, Vseslavichi, Rostislavichi, Glebovichi, Yuryevichi
- **Nahua**: + Culhua, Xochimilca, Chalca, Cuitlahuac, Mixquica, Cohuixca
- **Turkic**: + Qarluq, Toquz, Onoq, Turgesh, Qangli, Yagma

## 3. Guilds (`src-tauri/src/sim/cultures.rs` → `Kit.guild`) — the thin pool

Today each culture has only **3** guild words, assembled as
`"{word} of {City} ({specialty})"` — so guild names repeat heavily. Proposed:
expand each to **6–7** words AND add optional name *patterns* for variety.

Expanded word pools (add to existing):

| Culture | Today | Add |
|---|---|---|
| Roman | Collegium, Societas, Corpus | Negotiatores, Mercatura, Argentaria, Officina |
| Greek | Emporion, Koinon, Synedrion | Thiasos, Eranos, Symmoria, Naukleroi |
| Punic | Beth, Sokim, Miqdash | Mahanet, Tarsis, Kothon, Suffetim |
| Persian | Karwan, Anjoman, Bazaar | Rasta, Sarai, Kalantar, Ostandar |
| Norse | Felag, Kaupang, Lag | Gildi, Stafnbui, Bryggja, Varda |
| Celtic | Comann, Margad, Tuath | Nemeton, Cuallacht, Aonach, Ceard |
| Arabic | Suq, Funduq, Tujjar | Qaysariyya, Wakala, Hisba, Sinf |
| Indic | Shreni, Nigama, Sangha | Puga, Gana, Vanik, Mahajana |
| Sinitic | Hang, Hui, Shanghui | Gongsuo, Bang, Zihao, Piaohao |
| Slavic | Bratstvo, Torg, Druzhina | Artel, Sotnya, Ryad, Gostiny |
| Nahua | Pochteca, Calpolli, Tianquiztli | Pochtlan, Oztomeca, Tealtianime |
| Turkic | Orda, Kervan, Lonca | Esnaf, Ahi, Bazirgan, Tamga |

Optional name patterns (pick by hash) to break the single `"X of City"` mould:
- `"{word} of {City}"` (today)
- `"{City} {word}"` (e.g. "Aquentia Mercatura")
- `"{word} of the {specialty}"` (e.g. "Hanse of the Salt")
- `"{Family} {word}"` (house-affiliated guild, e.g. "Valerii Argentaria")

## 4. Suggested wiring order
1. Heraldry: add shapes + furs + the vector charge map (biggest visual win, pure
   front-end in `CoatOfArms.tsx`); keep emoji as a fallback flag during rollout.
2. Names: append the new `family` surnames (one-line edits per kit).
3. Guilds: append `guild` words + add the pattern selector in
   `cultures.rs::guild_name`.

Each step ships with a refreshed `heraldry-variants.svg` (or a names sample sheet)
per the project's standing visual-report rule.
