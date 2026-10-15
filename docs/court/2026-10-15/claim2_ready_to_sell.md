# The verdict: GUILTY, 12 to 0

**The case:** WorldForge 2 is ready (or close to ready) to release publicly as a paid worldbuilding tool that people would use and pay for.

**The charge:** The case fails as stated: it should not go ahead the way it is described.

## How the jury voted

| Juror | Vote | Why |
|---|---|---|
| The Accountant | GUILTY | The claim is about a paid product, and the record has no price, no paying customer, no tester and no demand figure, so the money half rests on nothing but assertion. |
| The First-Time Customer | GUILTY | As someone hearing of it for the first time, I couldn't buy it this week even if I wanted to: there's no price, no installer and no release, only `npm run tauri dev`, and nobody besides the developer has shown it works on their machine. |
| The Competitor | GUILTY | If I saw this tomorrow I would ignore it, because the simulation depth is impressive but a tool with no installer, no license, no users and a save-to-campaign path that only just stopped failing cannot take a single customer from me, and if it ever gets traction I can copy the pitch long before one solo developer can support 190,000 lines of mostly AI-written code. |
| The Retired Judge | GUILTY | The claim says people "would use and pay for" this tool. The defense itself concedes the record holds no evidence of that, and its other points ("small step" to an installer, "days of work" for a license and tags) are assertions with nothing in the record behind them. |
| The Teenager | GUILTY | Honestly it sounds kind of sick, but you can't call something "ready to sell that people would pay for" when literally zero people outside the dev have ever opened it, and it was still breaking people's saved worlds like five minutes ago. |
| The Engineer | GUILTY | As a builder I can believe the generator works on the developer's machine, but nothing in the record shows it running from a release build on anyone else's machine, its save format just broke every earlier world file, and the O(n^2) campaign scaling is only a written plan, so the first paying customers would be the first real test. |
| The Small-Business Owner | GUILTY | I've never opened a shop without first knowing who walks through the door and what they'll pay, and here one person would be selling, supporting and fixing about 190,000 lines of mostly AI-written code with no price, no customer, and no tester ever seen, so the "people would use and pay for" half of the claim has nothing under it. |
| The Statistician | GUILTY | The claim says "people would use and pay for" it, but the sample of users, testers or buyers behind that is zero, and 748 self-written tests measure only what the developer thought to check, so the claim has no data under it. |
| The Parent | GUILTY | I would not hand my family's money or a month of their evenings to a tool that no one but its maker has ever run, whose saved worlds recently could not start a campaign, and that has no license, release build or support plan behind it. |
| The Investor | GUILTY | I would not put my own money behind "people would use and pay for" when the record shows no user, tester, price or sales page at all, and the core path of opening a saved world to start a campaign only recently stopped failing, with no one outside the developer ever having run it. |
| The Designer | GUILTY | A stranger shown this today gets a stack list (Tauri, PixiJS, Budyko, 45 goods, poleis, face genomes) with no one-line promise of what it does for a worldbuilder, and the owner is still asking whether it is one product or two, so it is not something people could understand quickly enough to pay for. |
| The Lawyer | GUILTY | You cannot sell software when the record shows no licence, no end-user terms and no audit of who owns the code (728 of its 988 commits were written by an AI agent), and nobody at trial even raised this; a paid release would carry ownership and liability risks the owner has not started to address. |

## Charges that stuck

1. **No one outside the developer has used it, and nothing shows anyone would pay.** The record shows no users, testers, price or sales page. All 12 jurors leaned on this.
2. **It cannot be bought or installed yet.** There is no release build, no installer, no licence and only dev-mode run instructions. 6 jurors cited this.
3. **The core path broke recently and the fix is unfinished.** Opening a saved world and starting a campaign "failed on every world file previously written", and only slices 1-2 of the fix are built. 5 jurors cited this.

## What the defense got right

1. The core product works on the developer's machine. The full generation pipeline runs and exports heightmap and layer PNGs, and the prosecution conceded that map generation "plausibly" works.
2. The gaps are mostly packaging and evidence, not missing features. With 748 tests and a scoreboard, the project finds its own crashes, such as the year-30 out-of-memory crash, and fixes the cause.

## Conditions for acquittal

1. Run `tauri build` and give the installer to a handful of outside worldbuilders. Have them make a world, export it, reopen the save and start a campaign on their own machines, and write down what breaks.
2. Put up a waitlist or priced pre-order page with a named price, and count who signs up.
3. Decide what the product is (map tool, campaign game, or both with a clear split) and write a one-sentence pitch. Also pick a commercial licence and EULA, and check the licences of the dependencies.

## The sentence

Stop adding features. This week, get a built installer onto other people's machines, put a price in front of them, and finish the save-to-campaign fix. Come back to this court with what those testers did and said.

_Full trial transcripts were kept locally and are not in the repo._

_Made with [/court](https://github.com/alexyc9381/court-skill), a free Claude Code skill by Alex Chen ([@nocodealex](https://instagram.com/nocodealex))._
