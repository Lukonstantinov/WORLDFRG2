# The verdict: NOT GUILTY, 9 to 3

**The case:** The Tauri + Rust tile/WorldBuffer + SQLite architecture is the right technical foundation for where WorldForge 2 is heading (a sellable worldbuilding tool with a growing campaign simulation).

**The charge:** The case fails as stated: it should not go ahead the way it is described.

## How the jury voted

| Juror | Vote | Why |
|---|---|---|
| The Accountant | GUILTY | The claim says this stack is "right" for a product that will be sold, but the only hard memory number in the record belongs to WF1 (~1.4 GB). WF2 has no matching figure, and nobody has costed a buyer's machine, a platform or a price, so the claim is an unaudited statement and not a balance sheet I can sign. |
| The First-Time Customer | GUILTY | As someone seeing this for the first time, the only parts of the foundation I would actually touch are my saved files and a long campaign on my own machine; the record shows the first broke and the second aborted, and gives me no numbers to believe either is now solid, so I would not trust it with my worlds yet. |
| The Competitor | NOT GUILTY | If I were selling a rival map tool, I would attack this product on shipping (no license, no releases, saves that broke) and not on its stack, because a desktop Rust engine that handles 3600x1800 worlds is the part I could not copy quickly and the part I would actually fear. |
| The Retired Judge | NOT GUILTY | The prosecution has to prove that the foundation is wrong, and every concrete failure it cites is traced in the record to application code (an uncapped journal, a preview that loaded every column, a save routine that deleted rows) rather than to Tauri, Rust, tiles or SQLite. Its other points (no licence, no releases, AI authorship, unbuilt plans) show the product is not ready, not that the stack is unsound, so suspicion has been offered where proof is needed. |
| The Teenager | NOT GUILTY | Honestly nobody's buying this yet and that's kind of sad, but the question is whether the stack is the problem, and every crash the prosecution waves around was a dumb app bug like an uncapped journal or a save that deleted rows, not Rust or tiles being the wrong pick. |
| The Engineer | NOT GUILTY | Every breakage in the record is in application code: an uncapped journal, a preview that loaded every column, a save that deleted rows, and O(n^2) loops. Each can be fixed inside this stack and none is caused by Tauri, Rust, tiles or SQLite, while the tile/WorldBuffer split is the one design choice shown to beat the ceiling that killed WF1, so as a foundation it holds even though the product is nowhere near shippable. |
| The Small-Business Owner | NOT GUILTY | I've seen plenty of shops fail with good shelving and none fail because of the shelving, and what's wrong here is the shop: no licence, no price and one person on the hook for a lot of code. The building it stands on (a desktop app, files the customer owns, a Rust core that got past the old memory wall) is the right one to keep. |
| The Statistician | NOT GUILTY | Neither side brings a single measurement after the fixes, so the evidence is thin both ways; but every recorded failure I can count (journal OOM, all-columns preview, deleted save rows, O(n^2) algorithms, tech-factor erosion, loan principal) traces to application code rather than to Tauri, Rust, tiles or SQLite, and with zero failures attributed to the stack itself I cannot conclude the foundation is wrong. |
| The Parent | NOT GUILTY | As someone guarding a family's time and money, I'd hate to see a solo builder throw away four months to switch stacks over failures the record traces to specific code, like an uncapped journal, a preview that loaded every column and a save that deleted rows, rather than to Tauri, Rust, tiles or SQLite; the base did fix the crash that killed version 1. |
| The Investor | NOT GUILTY | I would not fund the business yet, because it has no users, no license and no releases, but the money question here is the stack, and switching it would cost more than it saves: the stack fixed the problem that killed version 1, and every failure in the record traces to application code that can be fixed, not to Tauri, Rust, tiles or SQLite. |
| The Designer | GUILTY | The engine may be sound, but a stranger buying this could not even tell whether it is one app or two, and the one thing they would hold in their hands, the saved world file, is the part the record shows breaking, so "right foundation for a sellable tool" is asserted rather than shown. |
| The Lawyer | NOT GUILTY | The legal and liability traps I can find in this record (no LICENSE file, 728 of 988 commits by an AI agent with no statement of who owns that code, and a save break that would mean lost customer work in a paid product) are real blockers to selling, but none of them comes from choosing Tauri, Rust, tiles or SQLite, and the claim on trial is only about that technical foundation. |

## Charges that stuck

1. **No numbers after the fixes.** Nobody has measured WF2's memory or speed on a long campaign at full map size. The year-30 out-of-memory crash is marked "fixed" only in the owner's notes. 4 jurors leaned on this, and every acquitting juror's flip condition depends on it.
2. **The save file broke.** Old `.worldforge` files failed to start a campaign, and slices 3-5 of the fix are still unbuilt. 3 jurors.
3. **It is not ready to ship.** There is no licence, no release, no platform and no price. 7 jurors named this, though most said it is a business gap and not a flaw in the stack.

## What the defense got right

1. Every recorded failure has a named cause in application code (an uncapped journal, a preview that loaded every column, a save that deleted rows). None traces back to Tauri, Rust, tiles or SQLite.
2. The tile/WorldBuffer split is what got WF2 past the ~1.4 GB ceiling that killed WF1.

## Conditions for acquittal

1. Run one campaign well past year 30 on a 3600x1800 world on an ordinary machine. Record its peak memory and run time against a limit you state in advance.
2. Add a test that opens world files saved by older builds and starts a campaign from each.
3. Decide between one app and two, and pick a file story (a bundle or sibling pairing). Write the decision down.

## The sentence

The foundation stays. Before you add any more Living World features, measure it: profile one long campaign at full size and publish the numbers in SCOREBOARD.md. Then make sure old saves open. If the numbers come back bad, put this question back on trial.

_Full trial transcripts were kept locally and are not in the repo._

_Made with [/court](https://github.com/alexyc9381/court-skill), a free Claude Code skill by Alex Chen ([@nocodealex](https://instagram.com/nocodealex))._
