# The verdict: GUILTY, 11 to 1

**The case:** WorldForge 2 is worth continuing to build in its current direction (science-grounded world generation plus a deep living-economy and people campaign), as a worldbuilding tool the owner eventually wants to sell.

**The charge:** The case fails as stated: it should not go ahead the way it is described.

## How the jury voted

| Juror | Vote | Why |
|---|---|---|
| The Accountant | GUILTY | The claim is that this is worth building toward a sale, but the record has no price, no buyer, no license and no release, so "worth" has no number behind it, and the current direction spends four months of effort on new people-layer features instead of on anything that would bring in the first dollar. |
| The First-Time Customer | GUILTY | As someone hearing of this today, I couldn't buy it or even install it this week, and the plan as described keeps adding face genomes and leisure games when it should be giving me a build I can run where my saved world actually opens into a campaign. |
| The Competitor | GUILTY | As the business this would compete with, I would just ignore it: the climate science is a real edge I could not copy quickly, but the current direction keeps adding face genomes and leisure games instead of shipping, so it never shows up in front of my customers. |
| The Retired Judge | GUILTY | The claim is that the project is worth building further in this direction as something to sell, and the record holds no evidence that anyone would buy it. Even the defense concedes the gaps call for "adding a release step", which means changing the direction, not continuing it as described. |
| The Teenager | GUILTY | Face genomes and leisure games sound cool, but nobody outside can even download this thing, and anyone who did would save their world, come back next week, and find that Begin Campaign doesn't work, so piling on more features just means more stuff nobody sees. |
| The Engineer | GUILTY | The tiled engine works at scale, but the direction as described keeps adding entity-heavy layers (people, councils, realms) on a campaign that already has O(n^2) growth with only a written plan, and on a save-to-campaign round trip that does not work; that breaks first, so it should not go ahead as described. |
| The Small-Business Owner | GUILTY | In twenty years I've never seen a shop pay its rent by adding more stock to shelves before the door opens, and four months in this one has no price, no license, no release and no customer, while the owner keeps building leisure games and face genomes. |
| The Statistician | GUILTY | The case asks me to back a direction toward a paid product when the sample of outside users, testers or buyers is zero, so every metric we have measures the code (748 tests, 411/411 passing, Köppen fit) and none measures demand; meanwhile the direction keeps adding features rather than collecting the first data point. |
| The Parent | GUILTY | If someone in my house spent four months on something they hope to sell, I'd want them to stop adding face genomes and leisure games and first make sure a buyer could save a world and start a campaign, then put it in front of one real person; the plan as described does neither, so it's spending the family's time on a bet nobody has checked. |
| The Investor | GUILTY | I would not put my money into this as described, because the plan keeps adding features to a product no outside person has been shown to use, while the core path is broken and nobody has checked whether anyone would pay, so all the risk is up front and none of it has been reduced. |
| The Designer | GUILTY | A stranger could not say in five seconds whether this is a map generator for worldbuilders or a merchant-and-dynasty strategy sim, and the current direction keeps adding layers (face genomes, leisure & games, masterworks) that blur that answer further instead of sharpening it. |
| The Lawyer | NOT GUILTY | The legal traps I see are real but none of them blocks the build. There is no LICENSE file, no terms of sale, and nothing about the licences of dependencies or bundled data. Copyright in code that is 728/988 commits AI-authored is also unsettled. All of these are paperwork owed at the sale step, which the record puts at "eventually" with no date, so they argue for a legal checklist before release, not for stopping the current direction. |

## Charges that stuck

1. **Features before finishing (11 jurors).** The Sept-Oct 2026 work went into face genomes, leisure games and masterworks, not into shipping.
2. **No path to a sale (9 jurors).** There is no license, release build, price, tester or buyer, so whether anyone will pay has never been measured.
3. **The core handoff is broken (9 jurors).** Starting a campaign from a saved world "failed on every world file previously written," and slices 3-5 of the fix are unbuilt.

## What the defense got right

1. The climate model is checked against Earth's real Köppen map, with a CI regression floor. Even the Competitor juror called it an edge that would be hard to copy.
2. The tiled rewrite fixed the memory wall that killed WorldForge 1. The Engineer juror agreed that the engine works at scale.

## Conditions for acquittal

1. **Freeze new simulation layers and fix save-to-campaign first.** Build slices 3-5, or at least show that freshly saved worlds start campaigns.
2. **Add a LICENSE and cut a tagged, installable release build** with install steps that do not need dev mode.
3. **Give that build to a handful of outside worldbuilders.** Write down whether they come back to it and whether they would pay.

## The sentence

The court finds the engine sound but the order of work wrong. Stop adding new people-layer features. Spend the next stretch on the save-to-campaign path, a license and a release build, then get that build to real worldbuilders. Before release, check that every dependency and dataset you rely on allows commercial sale. Let what those first users do decide which feature comes next.

_Full trial transcripts were kept locally and are not in the repo._

_Made with [/court](https://github.com/alexyc9381/court-skill), a free Claude Code skill by Alex Chen ([@nocodealex](https://instagram.com/nocodealex))._
