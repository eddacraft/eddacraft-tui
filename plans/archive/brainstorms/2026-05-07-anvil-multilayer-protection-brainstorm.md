# Anvil Multi-Layer Protection — Round-2 Planning Brainstorm

**Date:** 2026-05-07
**Status:** Brainstorm (companion to
[`2026-05-07-anvil-multilayer-protection-architecture.md`](../specs/2026-05-07-anvil-multilayer-protection-architecture.md)
spec).
**Round-1 predecessor:**
[`2026-05-07-daemon-sessions-surfaces-boundaries.md`](./2026-05-07-daemon-sessions-surfaces-boundaries.md)
— scoped to "where does the daemon live."
**Author:** Planning conversation between user (Josh) and assistant
(Claude Opus 4.7, 1M ctx). Assistant produced the round-1 strawman;
user redirected several times when the strawman over-baked
preferences or missed real-world ground truth.

---

## 0. What this document is

This brainstorm captures the **design journey** of round 2 — the
alternatives considered, the corrections from ground truth, the
principles that emerged as load-bearing, and the rationale behind the
decisions encoded in the spec. It is intentionally narrative-shaped;
the spec is the reference.

Round-1 ended with a 14-section spec recommending a per-user singleton
daemon with `info.json` discovery and OS-locality boundary detection.
The user explicitly redirected: "we didn't brainstorm anything." The
strawman had pre-baked the assistant's preferences without checking
whether they served the user's actual workflow. Round 2 worked
backward from "what we need to claim" rather than forward from "what
the architecture should be."

---

## 1. The pivotal redirects

Several user inputs reshaped the architecture significantly. Each is
worth recording because the design cascades from them.

### 1.1 "Walk backward from what we need to claim"

The first major pivot. Instead of designing an architecture and
deriving claims, we tabled candidate claims first and let
architecture fall out:

- Tier 1: "we validate MCP writes" — RMCP-only
- Tier 2: "+ visible surface state"
- Tier 3: "+ save-time daemon coverage"
- Tier 4: "+ project-level claim across all surfaces"
- Tier 5 (rejected): "cross-OS reach without bridge"

User picked: "I'd like to start at what we need to claim and then work
backward… defense in depth. 'Anvil protects this project'… 'regardless
of whether it is human or AI code'… 'doesn't matter whether you have
1 or 100 agents… across all possible surfaces, CLI, IDE, Web, WSL,
SSH, remote and local sessions.'"

This forced the architecture to be **multi-layer defense-in-depth**
rather than single-layer save-time enforcement. Each surface
contributes the strongest layer it can; layers compensate for one
another.

### 1.2 The four-toggle correction

Assistant initially modeled coverage as `surface × layer`. User
pointed out this collapsed surface and access mode incorrectly:
"Browser / Web AI making PRs will sometimes work with MCP so claude
code would be L0, L4, L5."

Coverage is actually four orthogonal toggles, not a 2D grid:

1. Anvil agent-side presence (MCP shim attached?)
2. File locality vs daemon locality
3. Path-to-git (local commit + push? direct API? web edit?)
4. Server-side gate presence

This reshaped the failure-mode analysis significantly.

### 1.3 "Daemon coverage stays primary"

User pushed back when the assistant started leaning on L4/L5 as the
universal fallback: "I want state that the daemon coverage while not
the only thing, still needs to be the primary. I don't want you
taking the easy route because you can now be post commit… our mantra
is still deterministic — pre-commit. Post commit is defense in depth."

This locked the doctrine: L0–L3 do the heavy deterministic lifting;
L4 catches what L0–L3 missed; L5 audits what slipped through. NOT
"L4 saves us so L0–L3 can be best-effort."

### 1.4 The 3-daemons-plus-remote-MCP reality

User ran the strawman's "per-user singleton" model into ground truth:
"I'd have 3 running and remote mcp right now for this project."

Detailed setup:

- Linux laptop with wezterm + tmux locally
- Desktop with its own tmux server, SSH'd from laptop
- 7 tmux panes, 4 active agents (mix of opencode + Claude Code)
- Sub-agents within agent panes
- Some agents in separate worktrees
- 4 Zed windows (some local, some SSH'd to desktop)
- "Morgan" assistant on a separate machine, SSH'd into desktop
- Claude Code Web running cloud-side
- Phone occasionally SSH'd in
- Cron / systemd timers
- 82 commits in one pane last night via parallel-worktree subagent waves

The strawman's "PID-file exclusive create" guard was just wrong. The
real model is **multi-daemon by design** — one per execution scope, not
a singleton.

User articulated what they couldn't initially: "we are saying per
checkout but that isn't 100% accurate I don't think. I think we are
actually more per machine (but I don't know how to articulate that)…
so is it maybe per executable surface?"

The right concept is **execution scope**: a place where one kernel
can directly observe writes via inotify-equivalent. Container
boundaries, VM/distro boundaries, sandbox boundaries, user-namespace
boundaries each create a new scope. Within a scope, one daemon. Across
scopes, multiple daemons. This precisely captures what "per executable
surface" was reaching for.

### 1.5 MCP is probabilistic

User: "MCP might be the ideal first class but it adds probabilistic
therefore unpredictable events but also that MCP servers fail all the
time, worktrees struggle with them sometimes."

This downgraded L0 from "the strongest deterministic catch" to "the
fastest catch when it fires, but best-effort." Doctrine sharpened:

- L0 = soft (LLM may not call the tool; MCP servers can fail; worktree
  + MCP interactions are fragile)
- L2 = first hard deterministic gate (kernel inotify guarantees daemon
  sees every write)
- L3 = second hard deterministic gate (git's commit path)
- L4 = cross-scope hard gate (deterministic at server side)

This actually strengthened the pitch — when MCP fails, L2 catches it.
When L2 fails (cross-boundary), L3 catches it. When L3 fails
(`--no-verify`), L4 catches it. Each layer's failure has a backstop.

### 1.6 The Serena rule

User: "what we must absolutely avoid is that in a failure state the
user's terminal is a wash of anvil error messages as they will just
turn anvil off (like I did with serena)."

This became a hard non-negotiable principle. Every Anvil surface must
follow noise discipline:

- Silent on success
- Single terse line on warning, with actionable pointer
- Single terse line on block decision, with actionable pointer
- Repeat-suppressed (don't re-emit same class+detail in session)
- Detail goes to log files, never to user's terminal
- No stack traces, no panic backtraces, no multi-line diagnostics

The cascade-failure failure mode (one Anvil error → user disables
Anvil → no protection at all) is structurally avoided by making
Anvil's noise tax bounded.

### 1.7 The witness file insight

User on hash-chained witness records: "what if at commit the hook
adds a file… that file just appends whatever we need it to. Every
commit does it… never a true conflict because we will take them all.
L4 looks at that file to see if this commit is on there."

This was a genuinely better design than the assistant had floated
(git notes, signed commits, sidecar service). Properties:

- Travels with the repo via standard git operations
- `merge=union` solves the merge conflict problem natively
- `--no-verify` becomes self-defeating (no line appended → L4 detects
  missing witness → reject)
- L4 verification is a tail-of-file read, no special tooling
- Hash chain provides tamper detection

Architecture pivoted to put the witness chain at the centre of the
deterministic claim. Kindling and the witness file are
**complementary, not competing**: Kindling = local rich SQLite
governance facts; witness file = minimal portable proof in repo.

### 1.8 Worktree dotfile propagation

User: "worth noting that .anvil will by default not get carried into
worktrees (they skip dotfiles by default) though I don't know if that
is all dotfiles or just root ones."

Investigation showed the real cause: `.anvil/` is conventionally
gitignored (it's local state); putting tracked content there inherits
the ignore. Plus some worktree-creation tools skip dotfiles.

Fix: split into two directories with deliberately-different
conventions:
- `anvil/` (no dot) — tracked metadata that must travel
- `.anvil/` (with dot) — local execution state, gitignored

This sidesteps the propagation issue cleanly and lines up with
established conventions (`.git/` vs `.github/`; `node_modules/` vs
`package.json`).

### 1.9 GitHub App breaks wow-start

After the assistant proposed a GitHub App as v1, user: "what if GH app
wasn't v1. it breaks the wow start requirement."

Correct. App requires sign-up + permission grant + branch-protection
config — all friction for a 60-second wow-start. Pivot:

- v1 = pre-push hook (client-side) + CI action (committed to repo) +
  pre-receive hook (self-hosted)
- v2 = GitHub App as the team-enforcement amplifier when "can't be
  bypassed" matters

v1 ships with **zero hosted Anvil infrastructure**. Wow-start
preserved.

User confirmed: "I'm happy to say gh only v1 for now. with the hooks
we just need to think about what else they will get used for."

### 1.10 Cleanup-then-PR clarification

Assistant initially over-narrowed retroactive witness scope to "merging
old branches." User clarified: "when I'm doing that I'll find old
commits that never made it to gh… maybe docs, maybe little bug fixes
etc so I then PR them and delete the WT."

This is a real flow. Maps to:

- Cherry-pick / rebase to current branch → fresh hooks fire on new
  commits → fresh witnesses
- Squash-merge → one new commit with one fresh witness
- Direct push of old branch → L4 `validate_at_l4` policy generates
  server-side witnesses

Existing design covers it via the L4 policy framework + retroactive
witness in `anvil hook bootstrap --witness-recent`. Scope corrected
to "commits about to be pushed for the first time" rather than
"commits about to be deleted."

### 1.11 `anvil start` already exists

User: "it exists now.. it shipped over past 24 hours. just need to
extend with what you need."

LAUNCH 18/18 shipped. Inspection confirmed `commands/start.rs` is a
thin wrapper over `activation::orchestrator`, with `--verify` /
`--watch` modes, stable JSON schema, and a closed-set
`ProtectionState` enum that is **exactly** the protection-claim policy
the assistant had designed from scratch earlier in the session. The
existing surface needs extension (project-id, witness genesis, hook
install, CI workflow), not replacement.

### 1.12 Config format flexibility + Rego custom rules

User: "Anvil gives you an option of yaml, json or toml. and custom
could maybe be covered by rego policy but I'm not precious on this."

Two corrections:
- The design must handle multi-format config (YAML / JSON / TOML)
  with auto-detection, defaulting to YAML.
- Rego (already in the architecture per ADR-006) is the natural fit
  for declarative custom rules — separate from built-in Rust pattern
  rules. Two-lane rule architecture: Rust scanner produces facts,
  Rego evaluates declarative policies over those facts.

---

## 2. Principles that emerged as load-bearing

These weren't stated upfront. They surfaced through pressure-testing
and became non-negotiable:

1. **Noise discipline (the Serena rule).** Failure must reduce noise,
   not increase it. User terminal stays clean even when Anvil is
   broken.
2. **Deterministic, pre-commit.** L0–L3 do the heavy lifting; L4 is
   defense in depth. Don't lean on post-commit.
3. **Hosted infrastructure is opt-in.** v1 wow-start works on a
   single laptop with no accounts.
4. **Honest claims only.** Closed-set protection states; never claim
   "protected" when one or more layers are unverified.
5. **Multi-daemon by design.** Per execution scope, not per user.
6. **Travels via git.** Witness chain in tree; rules in tree;
   project-id in tree. No side-channels for primary protection.
7. **Hard-pinned security classes.** Secrets and command-safety can't
   be config-disabled. Defense at type-system level.
8. **Branch-deletion-doesn't-need-witness, branch-merge-does.**
   Retroactive witness scoped to "commits about to be pushed."
9. **Existing tools are integrated, not replaced.** Anvil's hook
   slots into husky / lefthook / pcf / etc. without disrupting them.
10. **Provenance, not authentication.** `rules_sha` is auditable
    metadata; tampering defense is the chain hash + L4 revalidation,
    not cryptographic signing in v1.

---

## 3. Alternatives considered (and why rejected or deferred)

### 3.1 Per-checkout daemon

Considered before user articulated "per executable surface." Rejected
because:

- Three checkouts in same execution scope share kernel inotify; one
  daemon is enough
- Per-checkout would multiply parser pools, watcher trees, fence
  state, runtime dirs
- Discovery harder (which daemon for `cd ../other-checkout`?)

### 3.2 GitHub App as v1

Considered before user surfaced wow-start concern. Rejected because:

- Sign-up + permission grant + branch-protection config break the
  60-second wow-start
- Couples Anvil's pitch to Anvil's hosted infrastructure existing
- Day-one users have to evaluate two things at once

Deferred to v2 as the team-enforcement amplifier.

### 3.3 Notes-ref for primary witness storage

Considered (`refs/notes/anvil-l4`). Rejected as primary because:

- Notes refs aren't pushed by default
- Most users don't know about them
- Self-contained-in-tree property is lost
- Adversarial removal of notes is silent (no diff visibility)

Used at v2 for L4-server-generated witnesses (which CAN'T go in tree
at server-receive time without writing back to the push).

### 3.4 Pre-push as the only L4

Considered. Insufficient because:

- Bypassable via `git push --no-verify`
- Doesn't catch GitHub web/mobile/API edits
- Doesn't catch external contributor PRs from forks
- Doesn't catch bot commits (Dependabot, Renovate)

Solved by complementing pre-push with CI action (committed to repo,
runs in user's CI). Two layers within v1's L4.

### 3.5 Single-format config (YAML only)

Considered. Rejected because user offered multi-format support as a
shipped reality. Design now hashes parsed canonical-JSON, not raw
bytes, so reformatting doesn't invalidate witnesses.

### 3.6 Cross-Windows/WSL bridge in v1

Considered. Rejected because:

- Path translation has known footguns (case folding, symlinks, 9P
  inotify)
- Bridge requires authentication + audit + transport spec
- "Refuse and explain" is honest; "bridge with caveats" is false-
  confidence territory

vNext+ with its own ADR.

### 3.7 Strict witness-required at L4

Considered. Rejected because:

- Editor-first / `--skip-hooks` users have no L3 witnesses
- External contributors don't have Anvil installed
- Bot commits arrive without local hooks
- Web/mobile/API edits never had local hooks
- `--no-verify` is sometimes legitimate

Solved by per-branch policy framework with `validate_at_l4`
fallback that runs server-side validation when no L3 witness exists.

### 3.8 Strict rollover threshold

User offered a "crude" alternative: let the file overshoot to ~1002
lines, lazy rollover at next write check. Considered. Filed as v1
simplification path if lock contention proves problematic. v1 ships
with strict threshold (rollover decided inside the lock).

### 3.9 Auto-resolve fence cascade

Considered. Rejected because:

- 5 fences in 60s is a signal something is project-wide wrong (bad
  rule update? misbehaving agent fleet?)
- Auto-clearing turns invisible silent failures into visible silent
  failures
- Operator review is the right answer

Defaults to `degraded:fence-cascade` mode pending human action.

### 3.10 Rule pack distribution via Anvil cloud

Considered. Deferred to vNext because:

- v1 ships with zero hosted infra
- Per-repo `anvil/rules/*.rego` (tracked) covers most use cases
- Org/community packs need a distribution channel + signing story —
  not a v1 problem

---

## 4. What surprised the design

A few things surprised the assistant during round 2 and reshaped the
spec:

1. **The 82-commits-in-a-pane stress test isn't theoretical.** User
   ran it last night. Sub-agent waves are a real load profile, not
   an edge case. Per-task fence isolation (was DLIFE v1.5) had to
   promote to v1.
2. **Existing OPA work is a load-bearing pre-existing commitment.**
   ADR-006 (hybrid DC + OPA) plus opa-agent-orchestration plus
   opa-enhancements plus pack architecture plus contextual-policy-
   assertions — Rego custom rules isn't an addition, it's
   continuation.
3. **`anvil start` shipped 24 hours before this conversation.** The
   `ProtectionState` closed-set enum + JSON schema + `--verify`
   semantics + honesty contracts are already in the binary. The
   round-1 strawman's protection-claim policy was redesigning what
   existed. The actual work is extension, not reinvention.
4. **The witness file pattern emerged from user, not assistant.**
   The assistant had drafted three different approaches (notes refs,
   sidecar service, signed commits). User's "what if the hook just
   appends a file" was strictly better than any of them. Worth
   recording as a lesson — the user often has architectural intuition
   the assistant should defer to faster.
5. **Kindling's existing design is exactly right for governance facts
   provenance.** 11 observation kinds, 4 query scopes, write-once,
   secret-detection-redacted, BYO-AI read-only contract. The witness
   chain doesn't need to duplicate any of this — they're complementary
   stores at different layers.

---

## 5. The seven decisions, restated

These are the load-bearing decisions captured in the spec; restated
here in plain language:

1. **Daemon scope:** per-execution-scope, not per-user-singleton.
   Multi-daemon is the design, not a failure.
2. **Project identity:** `anvil/project-id` UUID + cross-checks
   (first-commit, origin-canonical). Forks inherit by default.
3. **Discovery:** `info.json` runtime sidecar with `ready` two-phase
   write, hardened `os_locality_token`, structured refusal codes.
4. **Lifecycle:** `anvil intercept ensure` lazy launcher with
   safe-spawn (env-clear) and sibling-resolved binary path.
5. **Cross-OS boundary:** detect and refuse in v1. Bridge is vNext+.
6. **Witness chain:** in-tree NDJSON with hash chain, active +
   archive + manifest, lock-protected rollover at 1000 lines / 1MB,
   merge=union via gitattributes, DAG-aware at merge commits.
7. **L4:** v1 = pre-push (client) + CI action (repo-committed) +
   pre-receive script (universal self-hosted). v2 = GitHub App as
   team-enforcement amplifier. Per-branch policy framework with
   `validate_at_l4` fallback.

---

## 6. What's still open

Tracked but not blocking the spec landing as Draft:

- L5 audit cadence and UX
- Privacy posture details when the GH App lands
- Rule pack distribution channel
- Editor driver L1 → witness chain handoff (currently L1 doesn't
  emit witnesses; should it emit Kindling precursors?)
- Anvil-on-Anvil dogfooding meta loops
- Air-gapped enterprise scenario story
- Migration path when `project_uuid` changes (explicit fork-out)
- `anvil config check` command for early `.anvil.yaml` validation

---

## 7. Council expectations on the spec

When the spec is reviewed:

- **Adversarial reviewer:** check that hard-pinned security classes
  can't be disabled via novel config encodings; verify the witness
  chain integrity model defeats the obvious tampering vectors;
  confirm `validate_at_l4` doesn't accidentally accept commits that
  bypass policy.
- **Operations reviewer:** verify noise discipline is mechanically
  enforced (panic catchers, repeat-suppression); validate degraded-
  mode observability is testable; check cross-platform startup paths.
- **Pragmatic lead:** validate the v1 / v1.5 / vNext split matches
  realistic shipping appetite; check that v1 doesn't require new
  infrastructure; confirm the per-task fence isolation promotion to
  v1 is justified.
- **Security analyst:** confirm same-UID trust model is preserved;
  verify env-clearing on spawn covers all reasonable injection
  vectors; check the WSL distro-name derivation is genuinely
  attacker-proof.
- **Runtime / platform reviewer:** verify cross-platform paths are
  correct (macOS App Sandbox, Windows DACL, WSL per-distro); check
  the husky / lefthook / pcf framework integration is non-destructive.
- **Product / activation lead:** verify the wow-start claim is real
  (60-second greenfield, ~30s baseline); check that the
  protection-claim policy is testable and tooling-friendly.

---

## 8. Lessons recorded for future planning sessions

For the assistant's future use (and any reviewer of these docs):

1. **Don't pre-bake preferences as if they were brainstorming.**
   Round 1's "20-row scenario table and six pre-baked council voices"
   was the assistant's own opinions structured as if they came from
   exploration. User correctly called this out: "we didn't
   brainstorm anything." Real brainstorming requires throwing out
   half-ideas, exposing assumptions, hearing pushback, and pivoting.
2. **Don't take the easy route.** When the assistant started leaning
   on L4/L5 as the universal fallback, user redirected: "I don't
   want you taking the easy route because you can now be post commit."
   The user's mantra (deterministic, pre-commit) is the architectural
   commitment; assistant's job is to defend it, not erode it.
3. **Defer to the user's architectural intuition faster.** Multiple
   times in round 2, the user offered designs that were strictly
   better than the assistant's drafts (witness file pattern, "per
   executable surface" framing, GH-App-not-v1). Assistant should be
   quicker to recognise these as load-bearing rather than treating
   them as suggestions to evaluate.
4. **Ground truth beats speculation.** The "3 daemons + remote MCP"
   reality reshaped the architecture more than any abstract analysis.
   Asking "what does your current setup actually look like" should
   come earlier in any planning session about local tooling.
5. **Existing code is a constraint and a gift.** `anvil start`
   shipping 24 hours before the conversation made the design
   simpler — the activation orchestrator + protection state enum +
   `--verify` semantics already existed in the right shape. Always
   inspect what exists before designing what should exist.
6. **The user explicitly invited pushback** ("don't just agree with
   me if I'm making bad calls"). That's a license to disagree
   substantively, not a courtesy. Use it.

---

## 9. Status

This brainstorm is a record, not a proposal. The proposal lives in the
spec
[`2026-05-07-anvil-multilayer-protection-architecture.md`](../specs/2026-05-07-anvil-multilayer-protection-architecture.md).
ADRs ADR-037 / -038 / -039 (witness chain, hook surface, baseline
policy) are still to be written.

Both round-1 and round-2 brainstorms remain in the brainstorm
directory as historical record of how the design evolved.
