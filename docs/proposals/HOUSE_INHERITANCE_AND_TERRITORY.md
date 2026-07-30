# Inheritance, women, and provinces as house territory

**Status: PLAN. Nothing built.** Amends `HOUSE_MASTER_PLAN.md` after maintainer pushback.
Read that document's Part 0 first — it carries the blocking finding.

---

# Part A — Conceding the "only decline" critique, and the better gate it produces

The maintainer's objection: *houses gain wealth very fast, the economy is a bull market,
and the crisis layer is what makes wealth real; the earlier problem was immense wealth.*

**That is correct, and it is a better framing than my critique.** The evidence:

| Fact | Source |
|---|---|
| `tech_factor *= 1.015^(1/365)` — ~1.5%/yr compounding, forever, with no counterweight | `mod.rs:4089`; FIX_PLAN Part C: "growth is exogenous" |
| Sustained richest house: **154 045** (was 297 748 before the feud rework) | dynamics run |
| Peak house wealth: **370 527** (was 632 796) | dynamics run |
| The project's own stated ideal: **"no 100k blow-ups"** | `CLAUDE.md` §2.1 |
| Wealth Gini 0.828, top-10% share 0.712 — in band, but at the top of it | econ scorecard |

So peak wealth is still **1.5–3.7× above the project's own ideal**, in an economy that
compounds with no brake. In that context the decline machinery is not a misery machine —
it is **the missing brake**, and I was reading it as narrative when the first-order
question is economic.

### What this changes concretely

My critique 2.2 splits into two claims, and only one survives intact:

- **As balance — withdrawn.** Vices, crises, courting-spend, schism and ruin are wealth
  sinks in an economy that needs them. They should be *judged as sinks*.
- **As narrative — stands, but weaker.** The chronicle still needs to be able to say a
  house *rose*. A fall from a golden age reads better than a fall from nothing, and ascent
  markers cost almost no state. Keep them — but they are polish, not a fix.

### The gate this replaces

`HOUSE_MASTER_PLAN` Phase 3 gated the crisis layer on "deposition rate sane". That is a
weak, unfalsifiable gate. Replace it:

> **Phase 3 gate (revised).** The politics layer must move **sustained richest house
> wealth measurably toward 100 000** without (a) breaking the Gini 0.60–0.85 band,
> (b) breaking the top-10% 0.60–0.90 band, or (c) pushing mean firm lifespan back out of
> 30–90 years.

That is four numbers, all already instrumented, and it makes the crisis layer answerable to
the economy oracle rather than to taste. It is also the honest version of the maintainer's
point: *if the crisis does not bite the wealth curve, it is not doing its job.*

### A hypothesis this generates for Phase 0.1

A bull market plus a hard one-year insolvency window suggests the 12-year lifespan is
**overextension**, not poverty: a compounding house buys fleet and warehouse capacity,
upkeep is charged even at zero wealth (`apply_wealth_sinks` does this deliberately), it
dips negative, and `update_solvency` kills it inside a year. If so, boom-and-bust is the
mechanism and the two anomalies — too-rich and too-short-lived — are **one bug, not two.**
Worth testing first because it would make Phase 0.2 much cheaper than it looks.

---

# Part B — The inheritance rule (variants, and how each pays for itself)

Factored on two axes, which is how the historical variation actually decomposes. Both live
on the **culture**, both are one small enum, and both are read at succession.

## B.1 `LineRule` — who is eligible

| Rule | Eligible | Real precedent |
|---|---|---|
| **Agnatic** | males only | Salic practice; most patrician republics |
| **Agnatic-cognatic** | males preferred, females if no male | English common law; Qur'anic shares (daughters take half) |
| **Absolute (cognatic)** | eldest regardless of sex | rarer; some Iberian and Italian practice |
| **Enatic** | **females only** | the matriarchal case — see B.4 |

## B.2 `InheritanceRule` — how the estate divides

| Rule | Division | Real precedent | Consequence in this model |
|---|---|---|---|
| **Partible** | equally among eligible children | Italian *fraterna*; Islamic shares; Sinitic equal division among sons; Russian | capital splits every generation; many heirs; **high fragmentation** |
| **Primogeniture** | eldest takes nearly all | English gentry; Yamato | concentration, plus a surplus of able landless spares who staff offices and lead ventures — and plot |
| **Ultimogeniture** | youngest takes the hearth | **Mongol *otchigin*** — genuinely the youngest son kept the hearth; Borough English | concentration, but a *young* heir → weak-head periods and regencies |
| **Seniority / elective** | the eldest **capable** of the lineage; the house chooses | Celtic **tanistry** (eldest capable of the *derbfine*); Rurikid *rota*; elected patrician heads | many claimants → **endemic succession disputes**, i.e. a crisis engine |
| **Matrilineal** | through the female line | see B.4 | the house cannot be split by sons; continuity runs through sisters |

## B.3 Assignment to the eighteen kits

Grounded where the record is clear, seeded where it isn't:

| Kit | Line | Inheritance | Note |
|---|---|---|---|
| Roman · Hellene · Punic | Agnatic | Partible | Roman *sui heredes* took equal shares |
| Persian · Arab | Agnatic-cognatic | Partible | Qur'anic fixed shares; daughters half a son's |
| Norse | Agnatic | Partible | *óðal* land divided |
| **Celtic** | Agnatic | **Seniority** | tanistry — the distinctive case |
| Slavic | Agnatic | Partible | |
| Sinitic · Indic | Agnatic | Partible | Confucian equal division; Mitakshara coparcenary |
| Yamato | Agnatic | Primogeniture | |
| **Turkic · Mongol** | Agnatic | **Ultimogeniture** | *otchigin*, the hearth-keeping youngest |
| Nahua · Quechua | Agnatic | Primogeniture | |
| Amazigh · Mande · Nilotic | Agnatic-cognatic | Partible | |

## B.4 Matriarchal cultures (explicitly requested)

Real, well-documented precedents: **Akan/Ashanti** (a man's heir is his *sister's son* —
the avunculate), **Minangkabau** (matrilineal inheritance of ancestral property, the largest
matrilineal society on earth), **Kerala Nair** *marumakkathayam*, **Haudenosaunee** clan
mothers.

Because this project's cultures are *inspired-by* rather than literal, assigning matriliny
to one named kit would be a factual claim I can't support. Instead:

> **A seeded minority of generated peoples — roughly 10–15% — receive
> `LineRule::Enatic` + `InheritanceRule::Matrilineal`,** biased toward cultures carrying
> the `Clannish` trait (kin-bound descent groups are the actual precondition for
> matriliny). Deterministic from the culture seed, so a given world always has the same
> matrilineal peoples.

Two variants inside it, both attested, and worth having both because they behave differently:

- **Eldest-daughter succession** — the straightforward case; the house reads like any other
  with the sexes reversed.
- **Avunculate** — the head's heir is his **sister's son**. This is the distinctive one: the
  house's continuity runs through its *sisters*, a head's own sons are not his heirs, and
  the resulting tension (a father who cannot pass to his son) is a genuinely different
  crisis cause than any currently in the design. Worth adding as `FEUD_/CRISIS_` cause
  "the sister's son".

## B.5 Why each rule must change the kin roster (or it is decoration)

The rule's whole payoff is that it changes the *composition* the politics layer runs on:

| Rule | Wealth at succession | Heir count | Idle able spares | Schism odds | Crisis flavour |
|---|---|---|---|---|---|
| Partible | ÷ n | many | few | **high** | brothers quarrel over the division |
| Primogeniture | ~all to one | 1 | **many** | low | the spares plot |
| Ultimogeniture | ~all to youngest | 1 (young) | many, older | medium | elder brothers resent the child |
| Seniority | ~all to eldest capable | many claimants | few | **high** | disputed election, every time |
| Matrilineal | through the line | 1–2 | moderate | low | brothers-in-law; the sister's son |

**Gate (per `HOUSE_MASTER_PLAN` 0.4):** two worlds identical but for the inheritance rule
must show **measurably different fragmentation** — different schism rate, different mean
kin count, different power concentration. If they don't differ, the rule isn't wired to
anything and is flavour text.

## B.6 Sequencing constraint — this must come AFTER the turnover fix

Partible inheritance and seniority both *increase* fragmentation. Today's turnover is
already 2.5–7× too fast and comes from **bankruptcy, not division** (the model has no
division at all yet). Landing inheritance first would push turnover further out of band and
make Phase 0.2 unmeasurable.

So: **0.1 diagnose → 0.2 fix turnover → 0.3 determinism → 0.4 inheritance**, and 0.4's gate
is checked against the *repaired* lifespan band, not today's.

---

# Part C — Women as agents

`Kin.sex` plus the `LineRule` above, and then three mechanics that all already have half
their machinery:

### C.1 The widow

In agnatic cultures a widow could hold and run the firm — often as guardian for a minor
heir, sometimes for decades, and the practice is well attested in Italian and Hanseatic
records. Model: if the head dies with no eligible adult heir, **the widow becomes head**
(role `regent`), with her own character and skill.

This is also the **positive mechanic the design was short of**: a capable widow who holds a
house together through a minority is an ascent story, not a decay one, and it costs one role
value.

### C.2 Dowry as capital, in both directions

`arrange_marriages` already moves a dowry — it just has no person attached. With `sex` and
kin:

- a **daughter marrying out** carries capital *out* of the house (and dowry inflation was a
  genuine fiscal problem that cities legislated against)
- a **son marrying in** brings capital *in*

So daughters become simultaneously an asset (the alliance) and a cost (the dowry) — a real
tension, and it makes the marriage system economic rather than decorative.

### C.3 Matriarchal houses read differently on the panel

Under `Enatic`, the figure in culture dress is female, the roster's stars are daughters and
sisters, and the succession line runs through them. `cultureFigureSVG` already draws both
sexes with per-kit garments, so this is free.

---

# Part D — Provinces as house territory (the Venetian case)

The maintainer's stated direction: *provinces should become house territories, as when
Venetian merchants controlled whole provinces and countries.*

**This is historically exact and architecturally aligned.** Precedents are not analogies —
they are the thing itself:

- the **Maona di Chio** — a joint-stock company of Genoese families that *governed Chios*
  for roughly two centuries
- the **Casa di San Giorgio** — a Genoese bank that administered **Corsica**
- Venice's **Stato da Mar** — a merchant republic's overseas provinces, run by *bailo* and
  *provveditore* (and this project already has `bailos` on `House`)
- later, the chartered companies governing territory outright

### Why it fits the code rather than fighting it

`prov_holder: Vec<i32>` already exists (`mod.rs:3269`) and currently names the **hub** whose
writ runs in a province. The extension is one sibling field:

```rust
/// House whose writ runs here instead of a city's (−1 = none). A merchant house
/// that holds a province collects its dues directly — the Stato da Mar case.
#[serde(default = "neg_one_i32")] pub prov_holder_house: Vec<i32>,
```

Everything else it needs is built or planned:

| Needed | Status |
|---|---|
| Per-province dues, surplus, unrest, works | **shipped** (B1 land pass) |
| A house with reach, bailos, standing | shipped / Phase 1 |
| A tier that territory should raise | Phase 1 |
| Inheritance acting on territory | Phase 0.4 |
| Revolt against a holder | shipped (`prov_unrest` → revolt); needs re-pointing at a house |

### What holding a province does

| Effect | Mechanism |
|---|---|
| Dues flow to the **house**, not the seat city | redirect `prov_revenue` |
| The house sets `prov_tax` | the control verb already exists — change the authority check |
| Standing rises steeply | territory is the strongest tier input there is |
| Unrest is directed at the **house** | revolt costs the house, not the council |
| Contestable | a polis or rival house may take it — war goals gain a territorial option, which finally makes a war change the **map** rather than a ledger |
| Inheritable | and now the inheritance rule bites on *territory*, which is where it historically mattered most |

### Why this is also the answer to the "no ascent" problem

A house acquiring a province is the **ascent event** the design lacked. The full arc becomes
tellable: a family rises from one estate, corners a trade, charters a bank, takes a bailo,
is granted a province, holds it for three generations, then loses it in a succession crisis
and fragments. That is the Genoese and Venetian story, and every piece of it is now either
shipped or planned.

### Placement

**Phase 5**, after politics — because a province held by a house whose politics don't work
is just a bigger number. But it should be designed knowing this is coming, specifically:
`prov_holder` must be treated as "the authority here" and not as "the seat city", so
Phase 3–4 code doesn't hard-code the assumption.

---

# Part E — Is the plan good enough? Honest answer

**Yes, with two reservations.**

### E.1 The design-to-code ratio is now bad

Seven documents, five phases, zero lines of the series built. That is the failure mode
`CLAUDE.md` §9 warns about — `docs/proposals/` becoming a backlog nobody can finish. My
recommendation is firm:

> **Stop designing after this document.** Build Phase 0 and Phase 1, look at them, and let
> what you learn revise Phases 2–5. Phase 0 is measurement and repair; Phase 1 is
> read-only and cannot regress either oracle. Together they are a complete, judgeable
> delivery.

### E.2 One real risk remains unaddressed

If Phase 0.1 finds that turnover is *load-bearing* — that houses dying every 12 years is
what currently keeps wealth from running away — then fixing lifespan will **raise** peak
wealth, possibly a lot. The two anomalies may be in tension rather than being one bug.

That is a genuinely uncertain outcome and it should be stated before starting rather than
discovered halfway:

- **If they are one bug** (overextension), 0.2 fixes both and the plan proceeds as written.
- **If they are in tension**, 0.2 must land *with* a replacement brake — and the natural
  brake is exactly the Phase 3 crisis layer. In that case Phase 0.2 and Phase 3 have to be
  co-tuned, and the phase boundary I drew is wrong.

I would rather record that now than treat the plan as safer than it is.

### E.3 Revised phase list

| Phase | Content | Judgeable output |
|---|---|---|
| **0** | Diagnose turnover · fix it · fix determinism · inheritance + `LineRule` | Four numbers move into band |
| **1** | Tiers · culture dress (both sexes) · expeditions tab · chronicle-first dossier · ascent markers | A panel you can look at |
| **2** | Kin (with sex, widows, dowry direction) · stewards · character · power shares | — |
| **3** | Goals · vices · crisis · civic intervention · record — **gated on moving peak wealth toward 100k** | — |
| **4** | Schism · bankruptcy aftermath · plague-as-lineage · foreign hand (if it fires) | — |
| **5** | **Provinces as house territory** — the Stato da Mar | The full ascent arc |

Seventh invariant, from Part D:

7. **`province_authority_is_not_assumed_to_be_a_city`** — every reader of `prov_holder`
   must tolerate a house holding a province. Written as a test now, before Phase 5, because
   it is a *cheap* guard against an assumption spreading through Phases 3–4 code.
