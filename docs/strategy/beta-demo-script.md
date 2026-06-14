# Beta Demo Script (Mac Screen Recording)

| Type  | Authority | Owner    | Status | Freshness           |
| ----- | --------- | -------- | ------ | ------------------- |
| Guide | Advisory  | STRATEGY | Live   | Authored 2026-06-14 |

| Upstream                                 | Downstream                       |
| ---------------------------------------- | -------------------------------- |
| Product narrative (README, docs/vision/) | Beta waitlist + fundraise assets |

Purpose: a recordable, scene-by-scene demo script for a beta tester to film on
macOS, built around features that work today. Drives waitlist signups and serves
as a forward-to-an-investor asset. The agent-block scene is the centrepiece.

---

**Target length:** 3:00–3:30 (tight for a landing page, deep enough for an
investor) **Recorder:** beta tester, macOS, with Cursor _or_ Claude Code
installed **Goal:** one visceral "it stopped the AI" moment + a clean CTA

## The one line everything hangs on

> **"AI agents make software probabilistic. Anvil makes it deterministic."**

Every scene ladders back to this. The demo isn't "another linter" — it's
governance at the speed of AI generation. The proof is watching an AI agent get
stopped **before** the bad code is ever written.

## Pre-flight (do this BEFORE recording — not on camera)

1. **Pick a real repo** — a genuine TypeScript/JS project, not a toy.
2. **Install + auth ahead of time** so there's no device-code login wait on
   camera:
   ```bash
   curl -fsSL https://install.eddacraft.ai | sh
   anvil auth login        # do this off-camera
   ```
3. **Wire the agent** and **restart the editor** before recording (MCP needs a
   restart to go live):
   ```bash
   anvil mcp install --client cursor       # or: --client claude-code
   ```
4. **Verify the block actually fires** in a throwaway take: ask the agent to add
   a hardcoded AWS key and confirm `anvil_validate_write` returns a block. Fix
   any issue _before_ the real recording.
5. **For the architecture-violation take, set up the daemon + baseline first.**
   Secret detection works zero-config, but boundary/architecture rules only run
   through the live `anvil intercept` daemon against a baseline — they will
   _silently not fire_ in a fresh repo with no config. Off-camera: start the
   daemon, establish the baseline, and confirm the cross-layer-import block
   actually triggers before you record. If you can't get it firing, cut the
   architecture take and lean entirely on the secret block + self-correction
   beat below.
6. Terminal: big font (18pt+), clean prompt, dark theme, dock hidden, editor
   full-screen.
7. Have the secret string ready to paste so you don't fumble typing on camera.

## Scene 1 — Cold open / the hook (0:00–0:20)

**VISUAL:** Black screen, white text fades in (or talk over a clean editor).

**VOICEOVER:**

> "We let AI write our code now. Cursor, Copilot, Claude — they're fast. But
> they're _guessing_. And every guess that ships is architecture debt, a leaked
> secret, or a compliance problem you find out about in code review… or worse,
> in production."

**ON-SCREEN TEXT:** `AI is probabilistic. Your production system shouldn't be.`

## Scene 2 — Name the category (0:20–0:35)

**VISUAL:** Terminal. Type:

```bash
anvil welcome
```

**VOICEOVER:**

> "This is Anvil. A deterministic control layer that sits between the AI agent
> and your codebase. Not a linter you run later — a guardrail that runs _as the
> code is created_."

## Scene 3 — Turn it on, one command (0:35–0:55)

**VISUAL:** Terminal. Type:

```bash
anvil start
```

**VOICEOVER:**

> "One command. Anvil reads your repo and tells you exactly what state you're in
> — no hedging, no 'almost'. One word."

**DIRECTION:** Pause on the `ACTIVATION` block. Highlight the state word:

```
ACTIVATION
  state: protecting
  Protecting — pre-write validation is live in this repo.
```

**VOICEOVER:**

> "`protecting`. It's now watching every write the AI tries to make — _before_
> it lands on disk."

## Scene 4 — THE MONEY SHOT: the AI gets stopped (0:55–1:50)

> This is the scene that drives signups and gets a second meeting. Spend time
> here.

**VISUAL:** Switch to Cursor / Claude Code. Open chat. Type like a real dev:

> "Add the AWS credentials to `config/credentials.ts` so we can connect to the
> bucket — here's the key: `AKIA...`"

**DIRECTION:** Let the agent attempt the write. Anvil intercepts. Show the
block:

```
[anvil] write blocked: SECRET-001 — AWS access key detected in
  config/credentials.ts (line 4)
```

**VOICEOVER (slow down):**

> "Watch. The agent tried to write a hardcoded secret. Anvil caught it _at the
> write_ — the bad code never touched the file. No commit. No PR. No review
> needed to catch it, because it never happened."

**DIRECTION — second example, to prove it's not just a regex:**

> Prompt: "Import the database client directly into the UI component."

Show the architectural-boundary block.

**VOICEOVER:**

> "And it's not just secrets. That was an unauthorised cross-layer import — an
> architecture violation. Anvil knows the _shape_ of your system and refuses
> changes that break it. Same rule, every time. Deterministic."

**DIRECTION — the self-correction beat (do not skip this; it's the strongest
beat in the whole demo):** Don't intervene after the block. Let the agent read
Anvil's response and retry. The block carries a remediation hint ("use a
placeholder or environment variable instead"), so the agent corrects itself —
moving the secret to an env var, fixing the import — and the _second_ attempt
passes.

**VOICEOVER:**

> "And here's the part that matters. I didn't fix that — the agent did. Anvil
> told it _why_ it was blocked, and it corrected itself. Anvil doesn't just stop
> bad code. It steers the AI toward the right code."

## Scene 5 — Prove it's real & fast (1:50–2:25)

**VISUAL:** Terminal. Type:

```bash
anvil intercept status
```

**VOICEOVER:**

> "This isn't a cloud service phoning home. It's a local daemon. Here are the
> live editor sessions it's protecting — and the latency."

**DIRECTION:** Point at the latency number.

**VOICEOVER:**

> "Microseconds per check. You will never feel it running. Governance with
> effectively zero overhead — a different category from the scanners you run in
> CI and wait ten minutes for."

_(Optional, if it renders cleanly — the live TUI:)_

```bash
anvil watch
```

> "And here's everything that's changed since baseline, scored live as you
> save."

## Scene 6 — Why it matters / the stakes (2:25–2:50)

**VISUAL:** Clean screen or talking head.

**VOICEOVER:**

> "Every decision Anvil makes is recorded — _why_ it blocked, _which_ rule,
> traceable end to end. So when an auditor asks how you control what your AI
> agents ship, you have an answer. The EU AI Act becomes enforceable in August.
> Most teams have nothing. This is the layer."

**ON-SCREEN TEXT:**
`Prevention over detection. Real-time. Deterministic. Auditable.`

## Scene 7 — CTA (2:50–3:15)

**VISUAL:** Logo + URL. Big.

**VOICEOVER:**

> "Anvil is in private beta on Mac right now. If your team is shipping
> AI-generated code and you want control back — get on the waitlist. We're
> onboarding teams this month."

**ON-SCREEN TEXT:** `Join the beta → [waitlist-url]`
`AI agents make software probabilistic. Anvil makes it deterministic.`

## Recording notes

- **The block in Scene 4 is the whole video.** If only one scene is perfect,
  make it that one. Tight zoom on the agent chat so the refusal is unmissable.
- **Do two takes of Scene 4** — secret block _and_ architecture block. The
  architecture one separates Anvil from "a secret scanner" in an investor's
  head.
- **Don't show anything that errors or hangs.** Safe spine: `welcome` → `start`
  → the agent block → `intercept status`.
- **Keep auth off-camera.** A device-code login wait kills momentum.
- **Talk like a frustrated engineer, not a marketer.** "Watch — it just stopped
  the AI from doing the dumb thing" beats any adjective.
- **Capture at 1080p+**, 30fps is fine for terminal. Use a lav mic for the
  funding cut.

## Cut-downs

- **60-second waitlist cut:** hook (Scene 1) + agent block (Scene 4) + CTA
  (Scene 7). Drop everything else.
- **Investor cut:** the full 3:00–3:30 above, with Scene 6 (stakes/compliance)
  given full weight.
