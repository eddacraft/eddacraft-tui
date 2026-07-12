# Remaining Design Gaps — Future Session Input

**Date:** 2026-05-07
**Status:** Living document — gaps surfaced during the multi-layer
protection planning sessions that need their own focused work.
**Purpose:** Single entry point for future planning sessions. Each
gap below is sized for one focused session; pick by priority or
appetite.

---

## 0. How to use this doc

Each gap has:

- **Why it matters** — what's blocked without it
- **What's already settled** — pointers to existing decisions
- **What needs to be decided** — the open questions
- **Estimated session shape** — roughly what the work looks like
- **Priority** — Critical (blocks v1 ship), High (blocks v1.5),
  Medium / Low

Sessions can pick one gap and run it to closure. Don't try to
dispatch multiple gaps in one session — they have distinct shapes.

---

## 1. Per-rule-class taxonomy and rule × layer matrix

**Why it matters.** ADR-039 names hard-pinned classes (`secrets`,
`command-safety`) but doesn't define the full taxonomy. The spec's
§2.1 (rule × layer matrix) is opinion, not derived from the actual
rule catalogue. Without nailing this, `rule_class_baseline` defaults
in `anvil baseline.json` could grandfather rules that should never
be grandfathered.

**What's already settled:**
- ADR-039 names two hard-pinned classes
- Spec §2.1 sketches a 7-class taxonomy (architecture, ai-patterns,
  style, license, secrets, command-safety, anti-pattern)
- Per-class baseline default behaviour table (grandfather /
  do-not-grandfather)

**What needs to be decided:**
- The complete class set (small, fixed, future-extensible)
- For each existing rule in `anvil-checks` (built-in) and any planned
  ones, which class does it belong to?
- Rule-class metadata: how is it carried (`rule.class()` method on
  the rule trait? declaration in registry?)
- Migration: existing rules that don't have a class assigned yet —
  what happens at first L3 fire?
- New rule classes: when does a new class get introduced (ADR? PR
  review? founder gate?)
- Edge cases: rules that span multiple classes (e.g., a rule that
  detects both a secret and a command-safety pattern)

**Session shape:** Inventory existing rules in `anvil-checks`;
classify each; surface ambiguous cases; settle taxonomy; write
ADR-040 (rule class taxonomy); update rule trait + registry.

**Priority:** **Critical** — blocks `anvil baseline` shipping safely.

---

## 2. `anvil hook bootstrap` ergonomics + framework detection edge cases

**Why it matters.** ADR-038 + MLP-008 specify the bootstrap recovery
path, but the framework-detection algorithm and edge-case handling
are sketched, not specified. Real-world frameworks (husky, lefthook,
pcf, cargo-husky, `core.hooksPath`-pointed-elsewhere, Lefthook with
custom config paths, multiple frameworks layered) need explicit
handling.

**What's already settled:**
- Detection table in spec §6.2
- Bootstrap appends Anvil's hook to existing chain
- Self-contained binary; 3-line shell wrapper
- Husky `_/` regeneration outline

**What needs to be decided:**
- Exact detection algorithm (precedence when multiple frameworks
  detected; e.g., `.husky/` AND `.pre-commit-config.yaml` both
  present)
- Husky version coupling (different husky majors have different
  `_/` runtimes — Anvil ships vendored runtimes per version?)
- Lefthook integration mechanics (lefthook is YAML-config; we add
  an `anvil` step)
- pre-commit-framework integration (Python-ecosystem; we register
  as a hook)
- What if `core.hooksPath` is set to an unrecognised dir?
  (Gracefully append; don't second-guess)
- What if user has manually edited `.husky/_/pre-commit` (rare but
  possible)?
- `anvil hook bootstrap` without write access (sandboxed CI runner)
  — degraded mode?
- Conflict resolution when bootstrap and user simultaneously edit
  `.husky/pre-commit`

**Session shape:** Test-matrix-driven design. Spin up fixtures for
each framework + edge case. Pin behaviours. Write
`docs/runbooks/anvil-hook-bootstrap.md`.

**Priority:** High — blocks MLP-008 implementation. Not v1-ship-blocking
but soon after.

---

## 3. L5 audit drift detection and CI integration patterns

**Why it matters.** MLP-015 + spec §15.4 specify `anvil audit` exists
and ships an active nightly workflow, but "what counts as drift"
isn't pinned. Without it, the audit is just a re-scan with no
threshold or alerting story.

**What's already settled:**
- MLP-015 work item
- Nightly cron + on-demand modes (per user direction)
- `mode: audit` Kindling observation kind
- `degraded:audit-drift` mode triggered above threshold

**What needs to be decided:**
- What "drift" means concretely: new findings? removed findings?
  changed findings? all of the above with separate counts?
- Drift threshold: absolute count? percentage-of-baseline? per-class?
- Audit comparison baseline: last successful audit? `cutoff_commit`
  in `anvil/policy.yml`? both?
- CI integration: how does the workflow report drift (PR comment? new
  issue? slack hook?)? GitHub commit status? Failed check?
- Multi-machine audit reconciliation: laptop's audit vs CI's audit —
  same source-of-truth, different scopes — how to display?
- Audit runtime: scan-everything every night? incremental since last?
- Long-running repo: audit history retention (Kindling vs separate
  store)
- Author of `anvil audit --json` output schema (`anvil.audit.v1`)
  pinned per CLI coherence spec but contents not yet detailed

**Session shape:** Define drift semantics; pin threshold defaults;
sketch CI integration patterns (GitHub-first); design output
schema; write runbook.

**Priority:** High — needed for L5 to be testably useful. v1.5 is
acceptable; v1 ships the command + workflow, audit-drift detection
is an extension.

---

## 4. Editor driver L1 → witness chain handoff

**Why it matters.** MLP-016 / spec §10.3 specify L1 emits Kindling
mid-edit observations. But the driver-protocol message shape, the
sampling policy, and the relationship to the eventual L3 witness
need pinning.

**What's already settled:**
- MLP-016 work item
- `gate_evaluated` Kindling kind with `mode: midEdit` (RTAI-007)
- Sampled emission (only on findings; pass-no-finding silent)
- No witness file write at L1

**What needs to be decided:**
- The exact Kindling observation shape for mid-edit findings
  (richer than save-time? same?)
- Whether to emit `constraint_applied` for mid-edit warns/blocks
  (probably yes for blocks; uncertain for warns)
- How L3 witness incorporates L1 history (commit's witness line:
  does it list "L1 fired N times during the session"? or is L1
  forgotten once it's not the latest decision?)
- Sampling under burst (mid-edit at 60Hz on aggressive editors —
  bound emission to ~10/sec per session)
- Cross-session correlation: L1 in editor → L3 hook on commit →
  L4 in CI — `traceparent` per ADR-035?

**Session shape:** Coordinate with RTAI-007 owner. Pin the
mid-edit envelope. Update DRVR-002 protocol if needed. Write
the L1 → L2 → L3 forensic narrative as a runbook.

**Priority:** Medium — L1 already provides validation without
emission; emission adds forensic value, not protection. Can ship
L1 working at v1 with emission added at v1.5.

---

## 5. Anvil-on-Anvil dogfooding (when it's safe)

**Why it matters.** Anvil developers building Anvil should be the
first users of Anvil's protection. But the spec just shipped — code
to implement it doesn't exist yet. Dogfooding too early is theatre;
too late and the dev team misses real feedback.

**What's already settled:**
- User direction: defer until MLP-001 / -002 land (per design Q)
- Initial step: write `anvil/project-id` to this repo when MLP-001
  is implemented

**What needs to be decided:**
- The exact gate for "MLP-001 implemented enough to use" (binary
  shipped with `anvil start` extension? unit tests pass? integration
  test against this repo?)
- What protection layers anvil-on-anvil starts with: project-id
  only? + witness chain? + hooks?
- How to prevent footguns (this is the repo with the rules being
  built; bad rule changes could fence the daemon's own development)
- How to handle the meta-loop (the rule that flags AI-generated code
  catches Claude Code's own commits to this repo)

**Session shape:** When MLP-001 ships, allocate a session to:
1. Initialise `anvil/project-id` here
2. Write a "dogfooding policy" doc in `docs/internal/`
3. Set protection-claim mode to "warn-only" initially; promote to
   "block" as confidence grows
4. Document footgun avoidance

**Priority:** Medium — dogfooding starts when implementation does.

---

## 6. Privacy / data residency for vNext (GH App + Anvil cloud)

**Why it matters.** v1 is air-gapped (per direction). vNext brings
the GitHub App and optional Anvil cloud sidecar. These services see
diff content and witness data. Privacy posture needs explicit
design before product launch.

**What's already settled:**
- Air-gapped v1 doctrine (no cloud calls in normal v1 operation)
- vNext: GH App as team-enforcement amplifier
- Anvil cloud sidecar as opt-in queryable witness API

**What needs to be decided:**
- GH App data flow: what gets sent, what's retained, for how long?
- Self-hosted GH App option (Helm chart for enterprises that can't
  trust a managed service)
- Data residency options (EU / US / customer-region)
- Witness storage: in-tree (always) + side-channel (optional cloud)?
- Telemetry / usage analytics: opt-in only, or default-off?
- SOC2 / ISO compliance roadmap
- Threat model document: what does Anvil cloud commit to NOT do
- Per-customer vs shared-infra deployment

**Session shape:** Threat-model + product-decisions session. Closer
to security-and-business than architecture. Likely engages legal /
compliance side. Run when commercial direction for vNext is clearer.

**Priority:** Low for v1; Critical for vNext shipping.

---

## 7. Off-grid / air-gapped enterprise scenario story

**Why it matters.** v1 already commits to air-gapped operation. But
"air-gapped enterprise" is a specific deployment context with
additional needs (no-internet builds, internal package mirrors,
self-hosted everything).

**What's already settled:**
- v1 doctrine: no cloud calls in normal operation
- MLP-017 air-gapped test suite
- Pack distribution constrained to git-based (vNext)

**What needs to be decided:**
- Self-hosted Anvil cloud (Helm chart) — when does it land?
- Internal Rust crate registry for `cargo install` of Anvil
- Internal pack registry (git-based; documented patterns)
- License attribution for air-gapped (license files vendored)
- Telemetry policy in air-gapped: hard-disabled? config-disabled?
- Self-hosted GH App — same as GitHub Enterprise Server compatibility
- How updates ship (no internet → vendored package downloads)

**Session shape:** Operations-focused session. Document the
deployment story. Probably overlaps with SOC2 / enterprise compliance
conversations.

**Priority:** Low for v1; High for first-enterprise-customer.

---

## 8. Council review of the multi-layer protection spec

**Why it matters.** Spec §20 says promotion to **Accepted** requires
a council review. The spec captures the design but hasn't been
adversarially pressure-tested by an engineering-volunteer council.

**What's already settled:**
- Three rounds of brainstorm-shaped review during this session
  (assistant + user iteration)
- Two rounds of spec-shaped review (round-1 spec had council
  remediation §11)
- Self-review against ADR-038 / -039 lens

**What needs to be decided:**
- Council composition (security analyst, runtime/platform, adversarial,
  pragmatic, product, kernel-maintainer at minimum per existing
  Anvil council patterns)
- Whether council reviews the spec, the ADRs, or both
- What "Accepted" promotion looks like (PR with all green checks?
  founder approval after council pass?)
- Implementation-volunteer pass: someone who would actually build
  MLP-001 reviews to catch engineering blockers in the spec

**Session shape:** Run `/council` skill against the spec; address
findings; re-run if Major findings surface. Promotion to Accepted on
clean council pass.

**Priority:** High — gate for any MLP work item starting
implementation.

---

## 9. DLIFE module skeleton

**Why it matters.** ADR-036 + MLP module reference DLIFE; the
module file at `plans/archive/modules/daemon-lifecycle.aps.md` doesn't exist.
APS hygiene says the file should exist before any DLIFE work item is
counted.

**What's already settled:**
- DLIFE-001..-007 v1 items (info.json, os_locality_token, refusal
  codes, ensure launcher, runtime path, status output, doctor)
- DLIFE-010 (App Sandbox) v1.5
- DLIFE-011 (logging) v1
- DLIFE-012 (status JSON schema) v1
- DLIFE-008 → MLP-014 (consolidated)
- DLIFE-009 → MLP-009 (folded into MLP)

**What needs to be decided:**
- Whether DLIFE remains a separate module or folds entirely into
  INTD-extensions
- If separate: full module skeleton with the 7+1+1+1 items, deps,
  dependencies on MLP / INTD, etc.
- If folded: each remaining item gets a new INTD-NN id

**Session shape:** Small. Either write the module skeleton (~30
minutes equivalent of work) or write the rationale for folding into
INTD and update references.

**Priority:** Low — APS hygiene only; doesn't block architecture.

---

## 10. Smaller open items (not session-sized)

These are individual items that don't justify a full session each;
collect into a "next planning session miscellany" batch when
convenient:

### 10.1 Repeat-suppression state file
Per CLI coherence §6: `~/.local/state/anvil/suppressions.json` for
per-(uid, project) state to handle script-invoked Anvil seeing the
same condition many times.

### 10.2 `anvil show <commit-sha>` vs `anvil log`
Should there be an `anvil log` for chronological browsing of
witnesses? CLI coherence §12 question 3.

### 10.3 Help text generation single-source-of-truth
Manifest format (YAML?) + tooling to render to `--help` / man /
docs. CLI coherence §10.

### 10.4 Backward-compat window for renamed subcommands
`mcp-config` → `mcp config`; `gate-config` → `gate config`; `hooks`
→ `hook`. How long do aliases stay? CLI coherence §12 question 4.

### 10.5 Rule lint / `anvil config check` command
Pre-runtime validation of `.anvil.yaml`. Spec §19 open question 10.

### 10.6 Migration story when `project_uuid` changes
Spec §19 question 9 (deferred per direction; surface when real case
appears).

### 10.7 Rego runtime pinning
Which OPA version, where it lives (vendored vs linked), how it
updates. Spec §19 question 1.

### 10.8 `anvil/witnessed.ndjson` `merge=union` corner cases
Custom merge driver as escape hatch if `merge=union` produces
unexpected interleaving. MLP-002 risk.

### 10.9 Husky runtime version coupling
Vendoring known-good `_/` files per husky major version. Bootstrap
gap §2.

### 10.10 Founder-review gate for new ADRs
ADR-038 / -039 are Proposed; promotion-to-Accepted process
documented in spec §20. Existing Anvil pattern says PR + council
+ founder review; this is a process item, not architecture.

---

## How to consume this doc

Pick a section. Run a focused brainstorm session against it. Land
the artefacts (ADRs / runbooks / module updates) it produces. Cross
the section off here.

Sections #1, #2, #3, #8, #9 are all v1-relevant and could each be
their own session. Sections #4, #5, #6, #7 are vNext-relevant.
Section #10 is miscellany — pick one or two per session as filler.

The most v1-critical: **#1 (rule class taxonomy)**. Without it,
`anvil baseline` could ship grandfathering rules that shouldn't be.
Recommended next session.
