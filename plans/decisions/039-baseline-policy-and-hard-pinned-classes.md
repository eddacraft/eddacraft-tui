# ADR-039: Baseline Policy and Hard-Pinned Rule Classes

## Status

Accepted (2026-05-13 during Wave 0 carry-forward reconciliation — per-class
baseline defaults and hard-pinned `secrets` / `command-safety` enforcement
remain the v1 mechanism for MLP-007/-013)

## Date

2026-05-07

## Context

Anvil exists to catch violations *as they're created*. Users adopting
Anvil into existing repos with years of history face a friction
problem: pre-existing code may have hundreds or thousands of findings
that aren't realistically fixable in a single PR. Without a baseline
mechanism, adoption-day produces a wall of findings, the user disables
Anvil, no protection.

ADR-003 ("new edges only") established the architectural intent:
baseline existing state, only flag *new* violations. This ADR pins
the v1 mechanism — `anvil baseline` — and the per-rule-class default
behaviour that determines what gets grandfathered.

Critical tension: not all rules should be grandfathered.

- **Architecture / boundary / coupling rules:** existing structure
  was created consciously; new edges are what matter.
- **Style / formatting rules:** triggering 10,000 file edits at
  adoption is the wrong move.
- **Secret detection:** existing secrets in code are *still secrets*.
  Grandfathering them silently ships a security hole every time the
  repo is cloned.
- **Command safety:** dangerous code is still dangerous regardless
  of when it was written.

So baseline must be **per-class**, with security-class rules
explicitly **not** grandfathered.

## Decision

### D-1 — `anvil baseline` mechanism

One-shot at adoption. End-to-end:

1. Identity init (write `anvil/project-id` UUID; cross-check existing
   per ADR-036 §D-2).
2. Scan current tree against all rules. Record findings in
   `anvil/baseline.json` (always JSON; anvil-managed).
3. Pin `cutoff_commit: <HEAD-sha>` in `anvil/policy.yml`.
4. Write witness genesis line anchored at the baseline cutoff
   (`prev_line_hash: GENESIS-BASELINED`).
5. Install hooks (pre-commit, pre-push, post-commit, post-merge,
   post-rewrite) per ADR-038.
6. Stage everything; user commits "Adopt Anvil."

Performance budget: <60s for 12k files (medium repo); progress line
updates with carriage return, not spam. Bounded scan budget for huge
repos (>100k files); excess scanned async with `complete: false`
flag in baseline.json.

### D-2 — Per-rule-class default behaviour

| Rule class | Default | Why |
|---|---|---|
| `architecture` (boundaries, coupling, layering) | grandfather | "New edges only" per ADR-003 — existing structure was conscious |
| `ai-patterns` (AI-001 reasoning patterns, etc.) | grandfather | Existing comments are usually human-written; enforce going forward |
| `style` (formatting, naming) | grandfather | Adoption shouldn't trigger 10k file edits |
| `license` (header rules) | grandfather | Same |
| **`secrets` (secret detection)** | **DO NOT GRANDFATHER** | Secrets in existing code are still secrets — they need rotation |
| **`command-safety`** | **DO NOT GRANDFATHER** | Dangerous code is still dangerous |

Users override per-class in `anvil/baseline.json`'s
`rule_class_baseline` field if their context demands. Default is
opinionated and security-aware.

### D-3 — Hard-pinned classes (cannot be config-disabled)

`secrets` and `command-safety` are **hard-pinned at the parser level**.
A `.anvil.yaml` (or `.json` / `.toml`) that attempts to disable these
classes — for example by setting `enforcement.rules.secrets.enabled: false` —
is rejected at config-parse time with a structured error:

```
anvil: config error — `secrets` is a hard-pinned class and cannot be disabled.
       use `@anvil-ignore SECRET-XXX: <reason>` to suppress individual findings.
```

This is the same pattern as ADR-015 ambiguous-ownership hard-cap at
`fence`. Set in proto, enforced at the type system level — runtime
checks are belt-and-braces, not the primary enforcement.

Per-finding bypass via `@anvil-ignore[-until DATE] WARNING-ID:
reason` syntax (ADR-004) remains available — the hard-pin is on
class-level disable, not on individual findings.

### D-4 — Fingerprint-based legacy-finding tracking

Each baselined finding records a fingerprint:

```
fingerprint = sha256(rule_id + file_path_canonical + surrounding_context_normalised)
```

The context chunk is ~10 lines around the finding with whitespace
normalised. This produces:

- **Stable across line moves within a file:** fingerprint unchanged
- **Moves with file rename:** Git rename detection helps; same
  context_normalised → same fingerprint
- **Sensitive to surrounding-code changes:** fingerprint changes →
  finding re-flagged (right outcome — the context shifted)
- **Sensitive to fixes:** fingerprint disappears → flagged as
  resolved in re-baseline diff

Hook-time finding emission:

```
finding emitted →
  is rule_class in rule_class_baseline as "grandfather"?
    no → emit normally
    yes → compute fingerprint, lookup in legacy_findings
      match → suppress, log telemetry "legacy-finding-matched"
      no match → emit (this is new or moved-too-far)
```

### D-5 — `anvil baseline.json` shape

```jsonc
{
  "v": 1,
  "schema": "anvil.baseline.v1",
  "project_id": "01997e4a-1b2c-7345-8901-abcdef123456",
  "cutoff_commit": "a3b2ea4e...",
  "created_at": "2026-05-07T12:34:56Z",
  "created_by": {
    "scope": "linux:8d3f1a2c",
    "anvil_version": "0.6.0",
    "rules_sha": "<sha of rule set used for baseline>"
  },
  "scan_coverage": {
    "files_scanned": 12340,
    "files_skipped": [
      {"path": "node_modules/**", "reason": "gitignored"},
      {"path": "fixtures/binary.bin", "reason": "binary"}
    ],
    "duration_ms": 8420,
    "complete": true
  },
  "legacy_findings": {
    "by_rule": {
      "no-circular-deps": [
        {"file": "src/foo.ts", "line_range": [42, 47], "fingerprint": "sha256:abc..."}
      ],
      "ai-001-reasoning-pattern": [
        {"file": "docs/api.md", "line_range": [17, 17], "fingerprint": "sha256:def..."}
      ]
    }
  },
  "rule_class_baseline": {
    "architecture":    "grandfather",
    "ai-patterns":     "grandfather",
    "style":           "grandfather",
    "license":         "grandfather",
    "secrets":         "do-not-grandfather",
    "command-safety":  "do-not-grandfather"
  }
}
```

The file is **tracked**, lives at `anvil/baseline.json`, travels with
the repo via clones / worktrees. Reviewers see baseline changes in PR
diffs.

### D-6 — Re-baselining

`anvil baseline --refresh` replaces the baseline at current HEAD:

```
$ anvil baseline --refresh
anvil: re-scanning at HEAD (was: 6 months ago, 247 findings)
anvil: 89 findings resolved, 12 new since last baseline (would have been blocked)
anvil: refreshed at e8a91c33 (158 legacy findings)
```

Refresh is **auditable**:
- Resolved findings: just gone (no record needed)
- New findings since cutoff: would have been caught by L3/L4 if
  they reached protection; flagged in refresh summary so the user
  notices if validation was bypassed
- Remaining legacy: still grandfathered

Refresh writes a `baseline-refreshed` line to the witness chain. The
old baseline state remains accessible via git history (`git show
<previous-sha>:anvil/baseline.json`).

### D-7 — Adversarial baseline detection

Threats:

- **Suppress findings via baseline edits:** visible in PR diff as
  "+47 entries in baseline.json"; reviewer notices.
- **Replace baseline with permissive one:** same; visible diff.
- **Refresh that suddenly grandfathers many new findings:** L4 (or
  `anvil status`) flags as `degraded:baseline-suspicious` for human
  review. Doesn't auto-block — informs.
- **Forge baseline with `complete: true` / `duration_ms: 0`:**
  inconsistent metadata (no scan would take 0ms with 247 findings);
  `anvil baseline verify` detects.

`anvil baseline verify` re-scans and confirms the recorded findings
still exist in the tree. Recommended as a CI step periodically (e.g.,
weekly) for repos that want strong baseline integrity. v1 ships the
command; CI integration is a documentation concern.

### D-8 — L4 cutoff handling

L4 reads `anvil/policy.yml` at the commit being validated:

```
for each commit in push:
  if commit.timestamp < baseline.cutoff_commit's timestamp:
    accept as legacy (no witness required)
  else:
    require witness or run validate_at_l4 per policy
```

Pre-cutoff commits never validated, never witnessed, never blocked.
Clean line in time.

This is what makes Anvil adoptable on day one. Without it: users
with old repos can't ship anything until they fix all legacy findings.
With it: legacy is grandfathered; new violations from baseline
forward are caught.

## Rationale

### Why the security-class exception

The user-facing claim ("Anvil protects this project") would be a lie
if existing secrets in the repo were silently grandfathered. The
secret would still be in the codebase, still exfiltratable, still
exploitable. Anvil's pitch is integrity at the point of change
creation; the *creation* of an unsafe secret may have been months
ago, but the *protection* claim covers it as long as it's still
there.

Same logic for command-safety: a `rm -rf $UNSET_VAR` lurking in a
script doesn't get safer with age.

So these classes are non-negotiable on principle. The mechanism
(hard-pin at parser level, refuse `enabled: false` configs) is
defense-in-depth: even a buggy or malicious config edit cannot
disable them.

### Why per-class, not per-rule

Per-rule baselining (every individual rule has its own
"grandfather?" flag) is too granular. Most users won't tune it; the
rule-author knows the right default for the rule's class better than
the user does. Per-class gives users one knob ("don't grandfather
these classes") that's coarse enough to use and fine enough to
matter.

### Why fingerprint-based, not line-number-based

Pure line-number tracking breaks the moment the file is reformatted
(every other line shifts → every finding looks "new"). Fingerprint
tracking is the standard pattern from Snyk, SonarQube, Codacy, etc.
We're not inventing.

The 10-line context window is a heuristic — large enough to
distinguish unrelated findings, small enough that minor edits don't
invalidate it. Tunable per-rule if a class genuinely needs different
context.

### Alternatives Considered

| Option | Pros | Cons |
|---|---|---|
| **Per-class baseline with hard-pinned security** *(chosen)* | Adoptable on day 1; security can't be silently disabled; auditable in PR | Per-class default may not fit every team; requires class taxonomy upfront |
| Grandfather everything | Maximum frictionless adoption | Ships with security holes invisible to user |
| Grandfather nothing | Cleanest enforcement | Adoption blocked until all legacy fixed; users disable Anvil |
| Per-rule baseline (every rule independently) | Maximum flexibility | Too granular; users won't tune; rule-author knows better |
| Time-based grandfather (older than X) | Auto-decay legacy | Gives wrong incentive (just wait long enough); arbitrary cutoff |

## Consequences

- **Positive — Anvil adoptable on day one.** A 10-year-old repo with
  500 findings can `anvil baseline` and ship. Legacy grandfathered;
  new violations caught from cutoff forward.
- **Positive — Secrets and command-safety remain enforced.** Adoption
  doesn't ship a security hole.
- **Positive — Auditable.** Baseline diffs are visible in PR; `anvil
  baseline verify` confirms recorded findings still exist; suspicious
  refreshes trigger `degraded:baseline-suspicious`.
- **Positive — Aligns with ADR-003.** "New edges only" was always the
  intent; this ADR pins the mechanism.
- **Positive — Re-baselining is a positive signal.** Resolving legacy
  findings reduces baseline.json over time; it's a measurable health
  indicator.
- **Negative — Per-class taxonomy is now public contract.** Rule
  authors must classify their rules; the class taxonomy becomes
  versioned. Mitigation: small fixed set (5-7 classes); changes go
  through ADR.
- **Negative — Fingerprint false-negatives.** A finding that moves
  >10 lines away from any anchor in the context chunk gets
  re-flagged. Mostly the right outcome (it's effectively a new
  context); occasionally annoying.
- **Negative — `anvil baseline.json` size for repos with many
  findings.** A 10k-finding baseline is ~3 MB; tracked. Mitigation:
  most repos have <500 findings; large repos grandfather coarsely.
- **Risk — Adversarial silent baseline edits.** Mitigation:
  visible-in-PR; baseline-suspicious detection; `anvil baseline
  verify`; defense in depth.
- **Risk — Hard-pinned classes too rigid for niche use cases.**
  e.g., a research repo deliberately commits intentionally-unsafe
  code as fixtures. Mitigation: `@anvil-ignore` per-finding bypass
  works; whole-class disable doesn't (by design).
- **Risk — Future rule that doesn't fit existing classes.** Class
  taxonomy may need extension. Mitigation: classes versioned;
  taxonomy update is an ADR.

## References

- **Spec:** [`2026-05-07-anvil-multilayer-protection-architecture.md`](../specs/2026-05-07-anvil-multilayer-protection-architecture.md) §7.5 (baseline mechanism)
- **Brainstorm:** [`2026-05-07-anvil-multilayer-protection-brainstorm.md`](../brainstorms/2026-05-07-anvil-multilayer-protection-brainstorm.md)
- **Companion ADRs:**
  - ADR-036 — Daemon scope (parent: identity is established at baseline)
  - ADR-037 — Witness chain (companion: baseline writes genesis line)
  - ADR-038 — Hook surface (companion: hooks installed at baseline time)
- **APS modules:**
  - `plans/archive/modules/multilayer-protection.aps.md` — MLP-007 (`anvil baseline`)
- **Related ADRs:**
  - ADR-001 — Planless-first (baseline is opt-in via `anvil baseline`; not required)
  - ADR-003 — New edges only (parent doctrine; this ADR pins the mechanism)
  - ADR-004 — Suppression syntax (`@anvil-ignore` for per-finding bypass; complementary)
  - ADR-015 — Intercept loop enforcement (hard-pinned ambiguous-ownership pattern; same enforcement style)
- **Inspiration:** Snyk / SonarQube / Codacy fingerprint-based legacy tracking
