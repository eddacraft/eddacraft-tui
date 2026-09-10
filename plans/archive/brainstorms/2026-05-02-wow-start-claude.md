# Wow-Start: making "just install and use" actually wow

**Date:** 2026-05-02
**Status:** Brainstorm — pitch, not yet a plan
**Author:** Claude (Aneki session)
**Trigger:** Senior dev/influencer feedback after testing Anvil — "give me a version that people like me can just install and use."
**Reading order:** §1 problem framing → §2 the pitch → §3 demo loop → §4 what's already there vs. new → §5 risks/tradeoffs → §6 alternatives → §7 next steps

---

## 1. Problem framing — install isn't the gap, first 60 seconds is

We already ship a one-liner install:

- `curl … | sh` from `install.eddacraft.ai`
- `brew install eddacraft/tap/anvil`
- `winget install eddacraft.anvil`
- PowerShell `irm | iex` for Windows

A senior dev who says "give me a version I can just install and use" almost certainly **already installed it**. What they're really saying is one of:

1. After install, I ran `anvil` and didn't see anything that made me say "oh shit."
2. I had to read docs / pick a command / write a config before I got value.
3. It looked like another linter — nothing in the first minute told me this was a *different category* (governance, not SAST).

This matches our standing memory: **onboarding/init/tutorial is the conversion moment, and it currently underwhelms.** The launch-blocker work (RTV — real-time AI-output validation) is the substrate that makes a wow-start possible, but RTV alone is plumbing — it has to be wrapped in a **zero-config first-run experience** that *demonstrates the category in under a minute*.

The bar isn't "is it installed." The bar is: **the influencer's screen recording of the first 60 seconds is shareable on its own.**

---

## 2. The pitch — `anvil` (no args) is the demo

One command, no flags, no plan, no policy file:

```bash
$ anvil
```

That command:

1. **Auto-detects the AI coding session already running in this repo** — Claude Code, Cursor, Copilot CLI, Codex CLI — by sniffing well-known sockets / process names / IDE state.
2. **Attaches a live watch surface** in the terminal (TUI) that shows file edits as they land, with RTV verdicts streaming inline. Hallucinated APIs flagged. Architecture drift highlighted. Fake imports caught. All without the user authoring a single rule.
3. **Seeds a tiny, repo-shaped invariant set on first launch** by inspecting the codebase (language, frameworks, public APIs, dependency graph) so even on a cold repo there are *some* assertions firing within seconds — not a blank canvas.
4. **Suggests one prompt to try.** A single "Try asking Claude to: rename `getUser` to `fetchUser` and update all callers" line at the bottom of the TUI — pre-engineered to almost guarantee a catch on first run. The user copies it, pastes it into their AI tool, and watches Anvil intercept something.

That's the demo loop. It's **planless-first** (already a pillar). It's **deterministic** (same repo + same prompt = same catch). It's **warnings over blocks** (TUI annotates; doesn't gate). It aligns with every architectural constraint we've already locked in.

The pitch isn't "build new tech." The pitch is **collapse the install → wow path from N steps to one command** by making the default experience the demo.

---

## 3. The 60-second influencer demo loop

What the screen recording looks like:

| Second | On screen | What the viewer thinks |
|--------|-----------|-----------------------|
| 0–5    | `curl … \| sh` runs, finishes | "Standard install, fine." |
| 5–10   | `cd ~/Projects/some-repo && anvil` | "OK, what now?" |
| 10–15  | TUI opens. Header: "Watching `some-repo` · attached to Claude Code session · 14 invariants seeded from codebase" | "Wait — it found the AI session by itself?" |
| 15–25  | Bottom panel: "Try this prompt: *rename `getUser` to `fetchUser` and update callers*" — user copies, pastes into Claude Code | "Cute, a guided demo." |
| 25–45  | Files start changing. TUI flashes amber on a file: "⚠ `fetchUser` called in `auth.ts:42` but not exported from `users.ts` — Claude missed a barrel re-export." | "**Holy shit, it caught the AI mid-flight.**" |
| 45–60  | TUI shows verdict pinned, file path clickable, suggested fix shown. User hits `f` → fix applied. | "I need to install this." |

The catch in second 30 is the moment. Everything in the architecture has to serve that beat.

---

## 4. What's already there vs. what's new

**Already there (don't rebuild):**

- One-liner install across Linux/macOS/Windows.
- Watch surface TUI (recently got a zoom hotkey — see recent commits).
- Anvil kernel with sub-millisecond policy evaluation (10µs save-time, 800ns full).
- RTV in active development as launch-blocker (memory: A1 RTAI Spike Slice).
- Planless-first as a stated principle (CLAUDE.md → architecture rules).
- ADR-015 / ADR-030: Rust daemon with JSON-RPC IPC, surface drivers as thin clients — the substrate for "attach to any AI session" already exists conceptually.

**New work the wow-start needs:**

1. **AI-session auto-detect.** A small probe at startup that finds Claude Code / Cursor / Copilot CLI / Codex CLI sessions running against the current repo. Process name + lockfile + socket sniff is probably enough for v1; one detector per tool, fail-soft if nothing found.
2. **Cold-repo invariant seeder.** A "first run" pass that inspects the repo and generates a starter invariant set: public API exports, primary entrypoints, dependency surface, no-go imports. Has to run in <2s on a 100-file repo so it doesn't break the 60-second demo.
3. **Guided-prompt suggester.** One sentence in the TUI footer with a prompt engineered to land a catch given the seeded invariants. Pre-canned per language/framework family. This is the part that *guarantees* the demo doesn't fall flat on cold repos.
4. **Empty-state copy + visual polish.** The TUI in seconds 10–15 has to read like a product, not a debugger. "Watching X · attached to Y · N invariants seeded" — that line is positioning, not UI.
5. **Telemetry-free attribution beat.** The "attached to Claude Code session" line is the *signal* — the moment the user realises this is integrated, not a side-process. No actual telemetry needs to leave the box.

The split is roughly **80% already-built substrate, 20% new wrapper.** The wrapper is the product.

---

## 5. Risks and tradeoffs

**Risk 1: First-repo lottery.** If the repo the influencer points it at is too clean, too small, or in a language the seeder doesn't support well, RTV catches nothing in 30 seconds and the demo lands flat. **Mitigation:** the guided-prompt suggester is specifically designed to make a catch nearly deterministic. We pre-bake "this prompt + this seed set = guaranteed catch" pairs per language. Without that, the pitch is fragile.

**Risk 2: Auto-detect false positives.** If we claim "attached to Claude Code session" and it turns out we attached to the wrong process, the influencer's recording shows a lie. **Mitigation:** be conservative — only claim attachment when we have a high-confidence match; otherwise say "watching repo · no AI session detected." Honest is better than slick-but-wrong.

**Risk 3: Pulls eng away from RTV launch-blocker.** The wow-start is downstream of RTV being good. Building polish on top of half-finished RTV inverts the dependency. **Mitigation:** scope the wrapper as a *thin* layer — auto-detect + seeder + prompt-suggester + copy polish. Don't let it grow into a parallel product. Ship it as the *demo skin* of RTV, not as a separate workstream.

**Risk 4: "Planless" becomes "policyless."** Some users will leave Anvil in zero-config mode forever and never adopt explicit policies — undercutting the long-term governance pitch. **Mitigation:** the TUI has a persistent "Lock these invariants into a policy file? `anvil init`" prompt after N catches. Wow-start is a funnel into the real product, not a replacement for it.

**Risk 5: Influencer-bait optimisation.** Designing for screen-recording moments can produce theatre at the expense of daily-use ergonomics. **Mitigation:** every wow-start element has to also be the right default for the 100th run, not just the first. The auto-attach, the seeder, the TUI polish — all of these benefit returning users too. If something is *only* good for the demo, cut it.

---

## 6. Smaller / safer alternatives

If the wow-start as pitched is too much for current capacity, fallbacks in descending order of investment:

**A. `anvil tutorial` interactive walkthrough.** A guided 3-minute tour the user opts into. Less viral (requires a command they don't know), but much easier to ship — no auto-detect, no seeder, just a scripted demo against a sample repo we ship inline. Good 2-week deliverable.

**B. Pre-recorded `anvil demo` mode.** `anvil demo` runs against a bundled fixture repo and replays a canned RTV catch sequence in the TUI. Zero new substrate. Pure marketing artefact. Not as honest, but shareable.

**C. Onboarding doc + curated `anvil try-this`.** Just a docs page with "five prompts that will make Anvil flag something" and a thin `anvil try-this <name>` runner. Cheapest possible. Probably the right move if we're inside two weeks of a launch.

**D. Status quo + better README first-run section.** Acknowledge the install is fine and just write a stronger "your first 60 seconds" section. Effectively no eng cost. Won't change the influencer's verdict, but stops the bleeding.

The pitch in §2 is option **A++** — a real product investment that bets the first-run experience is the conversion moment. **C** is the conservative play. Recommend §2 if RTV is on track for the launch window; recommend **C** if not.

---

## 7. Suggested next steps

1. **Validate the framing with the influencer.** Quick reply: "Was the install painful, or was it the first 60 seconds after install? If it's the latter, we have a pitch — want to see it?" Don't build before confirming the gap is where we think it is.
2. **Spike auto-detect (1 day).** Prove we can reliably detect a Claude Code session running in a given repo from a separate process. If this is hard, the whole pitch shifts toward option C.
3. **Spike the cold-repo seeder (2 days).** Prove we can produce a useful invariant set on a 100-file unknown repo in <2s. RTV crew probably already has prior art.
4. **Prototype the TUI empty state.** Mockup the second-10–15 frame as static text and ship it past the founder for a gut check on the *category-defining* feel.
5. **Write the guided-prompt catalogue.** Five language families × three prompts each = fifteen pre-engineered "this will catch something" demos. This is content, not code, but it's the load-bearing piece.
6. **Decide: §2 pitch vs. §6.C fallback** based on RTV timing. Do not start §2 if RTV is more than two weeks from a demoable state.

---

## Open questions

- Does the auto-detect story need to span all four AI tools at v1, or is Claude Code + Cursor enough to claim "attached to your AI session"?
- Is the guided-prompt suggester acceptable as part of the influencer demo, or does it feel like a rigged demo? (Suspect: it's fine if framed as "Try this prompt to see what Anvil catches" rather than hidden as magic.)
- Does the wow-start go in the existing watch surface, or is it a new top-level surface (`anvil` with no subcommand)? Lean toward: `anvil` no-args = wow-start; `anvil watch` = power-user.
- Where does this slot against the A1 RTAI Spike Slice — is it a new work item under that module, or its own module?
