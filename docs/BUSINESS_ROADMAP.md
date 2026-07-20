# WorldForge 2 — Business Roadmap (Idea → Sellable Product)

> A living checklist for turning WorldForge 2 from a mature engineering project into a
> product that earns revenue. This is a **go-to-market and packaging** plan, not a
> feature plan — the engine is already broadly feature-complete (~47k Rust + ~29k TS).
>
> **Owner legend:** 👤 = you (decisions, money, accounts, human judgement) ·
> 🤖 = Claude Code can do most/all of it · 💵 = costs real money.
>
> Work the phases in order. **Do not skip Phase 0** — everything downstream depends on
> what real strangers tell you.

---

## Strategic summary (the decisions this plan is built on)

| Question | Decision |
|---|---|
| **The hook** | "Draw two colours (land/sea) → get a plausible, *living* world." Input effort ~0, output depth huge. Demoable in 60 seconds. |
| **Second hook** | The living economy — houses rise & go defunct, banks fail, coinage, wars, a chronicle. Turns a "cool map" into "I want to watch my world live." |
| **Beachhead market** | Worldbuilding hobbyists **+** emergent-sim enthusiasts (Dwarf Fortress / Victoria / Rimworld crowd). Secondary: fantasy authors & GMs (served via export). |
| **Monetization** | **One-time purchase, ~$24.99**, with a **free limited demo** funnel. NOT subscription. |
| **Channel** | **itch.io first** (validate, ~$0 to start) → **Steam second** (bigger reach, 💵 $100 + more setup). |
| **Recurring revenue later** | Paid **expansions** — your code is already split into "DLC" layers; this maps 1:1 onto Steam DLC. |
| **Team** | You (ideas/decisions) + Claude Code (build/content/copy). Plan is written to that constraint. |

**Core risk:** engineering-rich, marketing-poor. ~90% of the remaining work is non-code.

---

## Phase 0 — Validate & build an audience (do this FIRST, ~1–3 weeks)

Cheap market research that doubles as audience-building. Goal: confirm the hook lands
with strangers and that some of them would pay, **before** investing in packaging.

- [ ] 👤🤖 Write the **one-sentence pitch** and a 3-bullet "why it's different" (draft with Claude, you approve the voice).
- [ ] 👤🤖 Produce the **magic-moment demo clip**: 60–90s screen capture — paint 2 colours → full world → press play → chronicle. This single asset does 80% of your marketing.
- [ ] 👤 Post the clip to **r/worldbuilding, r/proceduralgeneration, r/dwarffortress, r/RimWorld-adjacent sim subs, Cartographers' Guild forum**, Bluesky/Twitter #worldbuilding, YouTube Short/TikTok. Ask one question: *"Would you use this?"*
- [ ] 👤 Recruit **5–15 playtesters** from the responses. Hand them a build + a short structured feedback form (3 questions: where did you get stuck? what's the one thing you'd want? would you pay $X?).
- [ ] 🤖 Instrument a lightweight **feedback form** (Google Form / Tally) and a public **roadmap/idea board** (or a pinned GitHub Discussion).
- [ ] 👤 **Go / no-go + positioning refinement:** which market reacted hardest? Rewrite the pitch around whatever actually resonated. This decides everything below.

**Exit criteria:** ≥1 clip with real engagement, ≥5 outside people who drove the app, and a sharpened one-line pitch you're confident in.

---

## Phase 1 — Make it shippable to strangers (product hardening)

The maturity answer was "needs outside eyes." These items close the gap between "works
on my machine / only I can drive it" and "a stranger pays and succeeds unaided."

### Onboarding (highest leverage — this is the real blocker)
- [ ] 🤖 **First-run guided flow**: a "Quick Start — paint 2 colours, hit Generate" path that lands the magic moment in <2 minutes, with everything advanced hidden until asked for.
- [ ] 🤖 **Bundled sample worlds & templates** so a new user sees payoff instantly (and has something to play the campaign on without generating).
- [ ] 🤖 **Progressive disclosure**: gate the 19 layers / DLC panels behind a "Basic / Advanced" toggle so the depth doesn't drown newcomers.
- [ ] 🤖 In-app **tooltips, empty-states, and a Help panel**; a "What am I looking at?" for each major panel.

### Stability & trust
- [ ] 🤖 **Hardening pass**: hunt crashes/panics, replace `unwrap`-in-user-paths with graceful errors, add user-facing error toasts + a "Report a bug" link.
- [ ] 🤖 **Structured QA playtest script** (a repeatable "new user does X, Y, Z" checklist) run against a clean profile.
- [ ] 🤖 **Save-compat & file model surfaced in UI**: make `.worldforge` vs `.campaign` obvious; confirm old saves still load (the v2 blob rule already supports this).
- [ ] 🤖 **Performance guardrails**: world-size limits with clear messaging ("this size may be slow on your machine") instead of silent hangs.

### The "what do I take away?" problem (raises perceived value + virality)
- [ ] 🤖 Strengthen **export**: full-world image, a **readable chronicle / gazetteer** (PDF or HTML) of the campaign, and named-features export. People *share what they export* — this is free marketing.
- [ ] 🤖 One-click **"share my world" image** (map + a few stats) sized for social posts.

---

## Phase 2 — Packaging & distribution (the hard technical blockers)

A Tauri app that isn't signed throws scary OS warnings and won't sell. This phase is
mandatory before any store.

- [ ] 🤖 **Installers** for Windows / macOS / Linux via the Tauri bundler (MSI/NSIS, DMG, AppImage/deb).
- [ ] 👤💵 **Code signing**:
  - Windows: an OV/EV code-signing certificate (~$100–400/yr) — *you must purchase.*
  - macOS: Apple Developer Program (💵 $99/yr) + **notarization** — *you must enrol.*
  - 🤖 Claude wires the signing/notarization into the build once you have the credentials.
- [ ] 🤖 **Auto-update channel** via the Tauri updater (so you can ship fixes post-launch).
- [ ] 🤖 **Release pipeline**: GitHub Actions → build + sign + publish artifacts on tag.
- [ ] 🤖 *(optional)* **Opt-in, privacy-respecting telemetry**: crash reports + a basic funnel (did they reach the magic moment?). Disclose it clearly.

---

## Phase 3 — Legal, licensing & assets (do before money changes hands)

- [ ] 👤🤖 **Choose & add a LICENSE / EULA.** You're selling, so a **proprietary EULA** (not MIT) is likely right. Claude drafts; *you decide and, for anything binding, have a professional review it.*
- [ ] 🤖 **Dependency license audit**: confirm every Rust crate & npm package is commercial-friendly (Tauri/Pixi/rusqlite/zstd are permissive — verify the full tree, flag any GPL/AGPL/copyleft).
- [ ] 🤖 **Asset license audit**: bundled **fonts**, emoji/icon sets, any images — confirm each is licensed for commercial redistribution, or replace it.
- [ ] 👤 **Product identity**: finalize name (is "WorldForge 2" clear of trademark conflicts? — a quick search), bump version off `0.1.0`, set the app identifier, buy a **domain**. 💵
- [ ] 👤 Decide **refund policy** (Steam/itch have defaults — mostly you just accept them).
- [ ] 👤 Basic **privacy policy & terms** for the landing page (Claude drafts).

---

## Phase 4 — Storefront & marketing assets

- [ ] 🤖 **Landing page** (static, self-hostable): hero = the magic-moment clip, a feature tour, screenshots, "Buy / Download demo" buttons, FAQ. Claude builds it end to end.
- [ ] 🤖 **Screenshot gallery + GIFs**: worldgen sequence, layers, the campaign chronicle, an economy panel. Curate 6–10 that tell a story.
- [ ] 👤🤖 **Trailer** (90s): the hook, then the depth, then "watch it live." Claude storyboards + edits the capture; you record the raw footage.
- [ ] 🤖 **Press kit** (logo, screenshots, one-liner, longer blurb, contact) — presskit-style page.
- [ ] 👤🤖 **itch.io page**: store copy, tags, price, demo build vs full build, capsule art. *Live in a day; near-zero cost.*
- [ ] 👤💵🤖 **Steam page** (after itch validates): 💵 $100 Steamworks Direct fee, capsule art in Steam's exact sizes, store copy, build depots, ~2 week review lead time. Claude preps everything; you pay + submit.
- [ ] 👤🤖 Decide the **demo↔full gating** (e.g. demo = capped world size / N campaign years / watermark on export). Claude implements the entitlement split.

---

## Phase 5 — Launch & growth

- [ ] 👤🤖 **Launch sequence**: a devlog cadence in the communities from Phase 0 building up to release day; coordinate the trailer drop.
- [ ] 👤 **Community hub**: a **Discord** (support + word-of-mouth + playtest pipeline for expansions).
- [ ] 🤖 **Content engine**: short "watch this world live" clips from the chronicle/economy — this is your renewable, low-effort marketing (Dwarf-Fortress-style story posts).
- [ ] 👤🤖 **Feedback → roadmap loop**: publish what's next; let buyers vote.
- [ ] 👤🤖 **First paid expansion** using the existing DLC-layer architecture (recurring revenue without subscriptions).
- [ ] 👤 **Support & update rhythm**: a predictable patch cadence builds trust and reviews.

---

## Suggested order of operations (the critical path)

```
Phase 0 (validate)  ──►  decide GO/NO-GO
        │
        ▼
Phase 1 onboarding + hardening  ──►  a stranger can succeed unaided
        │
        ▼
Phase 2 installers + signing (👤 buy certs)  +  Phase 3 legal/licensing
        │
        ▼
Phase 4 itch.io launch (cheap, fast)  ──►  real sales + reviews
        │
        ▼
Phase 4 Steam page  +  Phase 5 growth & first expansion
```

**First revenue milestone:** Phase 0 → Phase 1 onboarding → itch.io demo + paid build.
Everything else (Steam, expansions) compounds after you've proven strangers will pay.

---

## Open questions to resolve as you go

- **Demo gating:** what exactly is free vs paid? (Leaning: demo = small worlds + short campaign; full = unlimited + all export + all DLC layers.)
- **Price point:** launch at $24.99, or lower ($14.99) to seed reviews then raise? Test on itch.
- **Name & trademark:** is "WorldForge 2" defensible/clear? Worth a 30-minute check before you print it on a store page.
- **How much campaign depth to expose first:** the economy is a moat *and* a complexity tax. Consider shipping the map generator as the front door and the living economy as the "wow, there's more" reveal.
