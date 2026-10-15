# The verdict: GUILTY, 9 to 3

**The case:** Building both a physics-grade world generator AND a deep economic/dynasty/people campaign simulation in one app is the right scope for WorldForge 2, rather than splitting it into separate products or cutting features.

**The charge:** The case fails as stated: it should not go ahead the way it is described.

## How the jury voted

| Juror | Vote | Why |
|---|---|---|
| The Accountant | GUILTY | The scope is meant to serve selling the product, yet the record has no price, no buyer, no tester and no release. Meanwhile the cost keeps climbing: the Living World layer was added in Sept-Oct, and one person has to maintain about 189,000 lines. The defense's "most build cost already paid" is a sunk-cost argument, not a return, so calling this the "right scope" has no number behind it. |
| The First-Time Customer | GUILTY | As someone hearing about this for the first time, I can't buy or even install it this week, and the one feature that justifies keeping everything in one app (taking my world into a campaign) failed on every saved world, so I have no reason to believe the all-in-one scope is right instead of first shipping a generator I could actually use. |
| The Competitor | GUILTY | As the rival, I would fear a geography-driven economy if it were in buyers' hands and hard to copy, but I can safely ignore a product that keeps adding layers like face genomes, leisure and games while the core handoff from generator to campaign is unfinished and nothing is installable, so "this scope, without cutting" is what keeps it out of my market. |
| The Retired Judge | GUILTY | The claim calls this the "right scope" for a product to sell, but the record holds no buyer, tester or release, and the one place the two halves join has been shown broken and is still unfinished, so "depth is the selling point" is hope dressed up as fact. |
| The Teenager | NOT GUILTY | A map generator alone is something you use once and forget, but a world that keeps running with banks crashing and houses rising is the thing people would come back to next week and talk about, and what's actually stopping anyone from seeing it is the missing release stuff, not having too much in one app. |
| The Engineer | NOT GUILTY | As a builder, I see one causal pipeline from plates to market economy that already has a file boundary between its halves; splitting it would turn that seam into a cross-product contract and add no capability, and what actually blocks a sale is release work and the unfinished handoff, not the scope itself. |
| The Small-Business Owner | GUILTY | In twenty years of running a shop I learned you open the doors with the shelves you have and let the till tell you what to stock next, but this one man is still adding whole new product lines like face genomes, councils and leisure four months in, with no doors open, no cash coming in, and only him on Monday morning to keep about 189,000 lines running when a slow month or an unexplained bug hits. |
| The Statistician | GUILTY | "Right scope... rather than cutting features" is a claim about what buyers will want, and its sample size is zero (no users, testers, or price), while the only signals the record does have (scope still growing, a handoff that failed on every saved world, a metric out of band for reasons "not yet explained") point the other way; a scope that keeps growing with no release and no data cannot be called right. |
| The Parent | GUILTY | A single person's time is being spent growing the scope with no buyer, tester or release in sight, and I wouldn't trust a product to people who pay for it when the step joining its two halves wipes out the work they saved. |
| The Investor | GUILTY | I would not put my money into this as described, because "keep everything and don't cut" means the scope keeps growing with no buyer, tester or price in sight, and the defense's sunk-cost point gives me no return, only more code for one person to maintain. |
| The Designer | GUILTY | "Science-grounded map generator with a living economy" is a pitch a stranger could take in within five seconds, but the claim also protects face genomes, councils, leisure & games and masterworks from any cut, and with those the product no longer has one clear identity for a buyer to recognise. |
| The Lawyer | NOT GUILTY | The legal traps I can see in the record (no LICENSE, 728 of 988 commits written by an AI agent, which clouds who owns the copyright in a product meant for sale, and save files that broke) are real. But each one hits a split product or a cut-down product just as hard, so none of them shows that this scope is the wrong choice. |

## Charges that stuck

1. **No buyer, tester, price or release exists.** After 988 commits there is no LICENSE, no release tag and no named user, so "right scope for a product to sell" rests on a guess. All 9 guilty jurors leaned on this.
2. **The scope is still growing.** The Living World layer (face genomes, councils, leisure & games) was added in Sept-Oct, adding code for one person to maintain while nothing ships. 7 jurors.
3. **The join between the two halves is broken.** Begin Campaign "failed on every world file previously written", and slices 3-5 of the fix are unbuilt. 6 jurors.

## What the defense got right

1. Generator and economy form one causal chain. Geography drives goods and trade, and splitting would turn the `.worldforge`/`.campaign` seam into a contract between products. Even the Engineer and the Teenager voted on this.
2. What blocks a sale today is release work (LICENSE, installer). Splitting the app would not fix that and would double it.

## Conditions for acquittal

1. **Freeze the people layer.** Write down that no new Living World features land before a first release.
2. **Make saved worlds open into campaigns.** Build the missing handoff work (at least the sibling-file pairing from slices 3-5) and add a test that opens an existing `.worldforge` file and begins a campaign.
3. **Put a build in front of worldbuilders.** Add a LICENSE, tag a `tauri build` release and give it to a handful of testers. Ask them whether they want the campaign half inside the map tool.

## The sentence

The court finds the vision coherent but unproven. Stop adding layers and finish the joint between the halves. Ship an installable build to a few real worldbuilders, and let them tell you whether the campaign earns its place. Return with that evidence, not more features.

_Full trial transcripts were kept locally and are not in the repo._

_Made with [/court](https://github.com/alexyc9381/court-skill), a free Claude Code skill by Alex Chen ([@nocodealex](https://instagram.com/nocodealex))._
