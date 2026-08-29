# FEFF-002: evidence-source and historical-replay feasibility audit

| Field | Value |
| --- | --- |
| Type | APS feasibility evidence |
| Work item | FEFF-002 |
| Date | 2026-08-27 |
| Branch | `feat/feff-001-002-field-evidence-decision` |
| Status | Complete — two historical snapshots replayed in isolation; three blocking source defects recorded |

This audit answers one question before FEFF-003/-004 build anything: **which of
the planned measures are actually reconstructable from anvil's current supported
surfaces, and which are placeholders?** Every "measured" verdict below is backed
by a command run in this branch; every "placeholder" verdict is backed by the
line of production code that hardcodes it.

Authority: ADR-133. Participant-data probes remain gated on that decision and
none were run — all probing used this repository's own operator-owned history.

## 1. Probe environment

| Item | Value |
| --- | --- |
| Binary | `anvil 0.9.7-beta`, built from this branch's tree, which is `main` at `29ce17dc6` plus a plans-only diff that cannot affect the binary |
| Invocation | `ANVIL_DEV=1`, `env -i PATH=/usr/bin:/bin`, per-snapshot `HOME`, `GIT_CEILING_DIRECTORIES` set to the probe root |
| Snapshot A | `5d5f99a8` (2026-08-17, `feat(checks): add shell-only command-safety rules`) |
| Snapshot B | `6e658892` (2026-08-06, `fix(tui): restore autoplay stash on tutorial reset`) |
| Materialisation | `git archive --format=tar` piped into `tar -x` — non-checkout plumbing per ADR-133 D-10 |
| Isolation | `--anvil-home <per-snapshot dir>`; the participant checkout was never switched, reset, or written |

Snapshot trees carry no `.git` after `git archive`, so no hook, filter,
submodule, LFS smudge, or credential helper can run during materialisation.
Confirmed by inspection in both trees.

## 2. Source map — measured versus placeholder

`M` = measured from a real source. `P` = hardcoded placeholder. `X` = not
reconstructable on the current surface.

| Planned measure | Authoritative source today | Verdict | Evidence |
| --- | --- | --- | --- |
| Merge throughput (open-to-merge, review/rework) | GitHub PR timestamps; Git history | M | Standard Git/GitHub metadata; no anvil surface involved |
| Eligible activity unit (commits or merged PRs) | Git history | M | Standard Git plumbing |
| Historical architecture drift (`boundary_violations`) | `anvil drift snapshot --json` | **P (conditional)** | Section 4.2 — reports `0` when architecture config is absent |
| Historical anti-pattern / escape-hatch proxy | `anvil drift snapshot --json`, `antipattern_count` and `antipattern_breakdown` | M | Section 4.1 — 93 vs 86 across snapshots, deterministic |
| Historical suppression counts | `anvil drift snapshot --json`, `suppression_count` and `expired_suppressions` | M | Section 4.1 |
| Files analysed (scan-domain denominator) | `anvil drift snapshot --json`, `files_analysed` | M | Section 4.1 — 1728 vs 1652; **not** a replay-coverage figure, see 4.1 |
| Prospective save-time adoption proxy | witness chain at `<repo>/anvil/witness/` | M | Section 3.1 |
| Prospective warning disposition | usage sidecar `<state>/kindling/usage.ndjson` | **M but non-attributable** | Section 3.2 |
| Prospective suppressions applied/resolved | `anvil insights` weekly summary | **P** | Section 3.3 |
| Prospective baseline edges added | `anvil insights` weekly summary | **P** | Section 3.3 |
| Prospective total saves observed | `anvil insights` weekly summary | **P** | Section 3.3 |
| Findings raised (weekly) | `anvil insights` weekly summary | **P** | Section 3.3 |
| Daemon uptime | `anvil insights` weekly summary | **P** (schema-locked) | Section 3.3 |
| Historical adoption | — | X | Not reconstructable; module already says so |
| Workflow friction | Participant closeout questions | X (qualitative) | Module already says so |
| Governance observations (gate/fence/action/audit facts) | KFIT canonical store | X | Section 6 — KFIT-007/-009/-010 are all `Draft` |

## 3. Prospective source findings

### 3.1 Witness chain — usable, repository-scoped

`anvil_witness::witness_paths` resolves `<repo_root>/anvil/witness/active.ndjson`
plus `<repo_root>/anvil/witness/archive/*.ndjson`
(`crates/anvil-witness/src/paths.rs:29`). It is append-only and
**repository-scoped**, so counts attribute cleanly to one study repository.
`insights::cumulative::cumulative_value` derives genuine all-time, 30-day, and
90-day event counts from it, anchoring windows to the witness chain's own latest
event rather than a cross-stream maximum
(`crates/anvil-cli/src/insights/cumulative.rs`).

**Verdict:** this is the only currently-supported source fit to back a
prospective developer-day adoption proxy. FEFF-004 should build on it.

### 3.2 Usage sidecar — three blocking defects

`SaveTimeCounts` (evaluations observed, risky writes flagged, writes blocked,
secret findings caught, fences engaged) is computed from
`<state_dir>/kindling/usage.ndjson`. It is genuinely measured, but three
properties make it unfit as the primary prospective source without new work.

**D-1 (blocking): the sidecar is machine-wide, not repository-scoped.**
`GateEvaluatedObservation`
(`crates/anvil-intercept/src/kindling_observation.rs:155`) carries no repository
identifier. The only repository signal on a row is `inputs.changed_files`, raw
paths — which ADR-133 D-5 forbids exporting and which the `SidecarRow` reader in
`cumulative.rs` deliberately does not even deserialise. A participant working in
several repositories therefore produces save-time counts that **cannot be
attributed to the study repository**. The existing code already treats this as a
known hazard: witness windows are anchored per-stream precisely so machine-wide
save-time activity in another repository cannot shift this repository's witness
windows (council-797f142a major 1).

**D-2 (blocking): retention is 7 days.** `USAGE_SIDECAR_MAX_AGE` is
`7 * 86_400` seconds and `USAGE_SIDECAR_MAX_BYTES` is 64 MiB
(`crates/anvil-cli/src/usage.rs:438`, `:445`); `trim_usage_sidecar` drops older
rows on append. A 10-to-20 working-day prospective window **cannot** be
reconstructed from the sidecar at closeout — the first half is already gone.

**D-3: trimming is lossy under concurrency.** The read, write-tmp, rename
rewrite races with concurrent `O_APPEND` writers and can drop a row; the code
documents this as accepted for best-effort observability
(`crates/anvil-cli/src/usage.rs:474-483`). Acceptable for a counter, not for a
denominator.

**Consequence for FEFF-004:** it must maintain **its own** bounded daily study
ledger, written continuously during the active period and scoped to the study
repository. It must not read the usage sidecar retrospectively at closeout, and
must not treat sidecar counts as repository-attributable. Capture health must be
recorded daily so a gap is visible as missing data rather than a low count.

### 3.3 `anvil insights` weekly summary — placeholder, do not consume

`insights::aggregator::weekly_summary` measures exactly one field.
`total_saves_observed`, `findings_raised`, `suppressions_applied`,
`suppressions_resolved`, `baseline_edges_added`, and `daemon_uptime_percentage`
are **hardcoded to `0`** in the constructor
(`crates/anvil-cli/src/insights/aggregator.rs:65-77`).
`daemon_uptime_percentage` is schema-locked at `0` by
`schemas/anvil-insights.v1.json` and is documented as such. The human surface is
honest about all six — `placeholder_metric_line` and `uptime_line` render them
as "not yet measured" — but the **JSON wire value is an indistinguishable `0`**,
because v1 pins every field as a required integer.

**Consequence:** a study consuming `anvil insights --json` would record **six**
fabricated zeros as measured results. Of the summary's seven metric fields,
exactly one — `witness_events_observed` — is real. The defect is confined to the
machine-readable surface: the plain-text output already tells the truth.

**Correction (2026-08-29):** an earlier revision of this section claimed the five
non-uptime placeholders "have no honest render at all". That was wrong — all six
go through `placeholder_metric_line` / `uptime_line` and render as "not yet
measured". Only the JSON surface was affected, and CIB-367 is scoped to it. FEFF-004 must not read this surface for any
study metric; section 3.1's cumulative path is the better route to the one real
field.

### 3.4 Privacy-control precedence — implementation gap

Two separate gaps, both narrower than the ADR-133 D-7 precedence order assumes.

**Value sensitivity.** `usage_collection_disabled`
(`crates/anvil-cli/src/usage.rs:789`) honours `ANVIL_USAGE_DISABLE`,
`ANVIL_INTERCEPT_DISABLE_OBSERVATION`, and `DO_NOT_TRACK` **only at the exact
value `"1"`**. `DO_NOT_TRACK=true`, `DO_NOT_TRACK=yes`, or a bare exported
`DO_NOT_TRACK` does not disable collection, though the wider convention is
presence-based.

**Scope.** `usage_collection_disabled` gates only the CLI `command.invoked`
producer (its sole call site is `record_invocation`,
`crates/anvil-cli/src/usage.rs:805`). The save-time and fence observation
producers are gated separately by `daemon_observation_producers`
(`crates/anvil-cli/src/usage.rs:1258`), which checks
`ANVIL_INTERCEPT_DISABLE_OBSERVATION=1` alone and **never consults
`DO_NOT_TRACK` at any value**. So a participant who sets `DO_NOT_TRACK=1`
still has save-time and fence observations written to the sidecar.

Related: `ANVIL_OBSERVATION_INCLUDE_PATHS=1`
(`crates/anvil-cli/src/usage.rs:1287`) opts raw file paths **into** the
sidecar. A study machine must never have it set, and FEFF-004 must assert that
before collection.

This is recorded, not fixed, here — changing the product default is outside
FEFF-002's scope. ADR-133 D-7 already requires the study surface to treat **any
non-empty** `DO_NOT_TRACK` as a hard-off and to check applicable controls before
every read and write, which covers the scope gap for the study even though the
product gap remains.

## 4. Historical replay feasibility — demonstrated

Both snapshots replayed successfully in isolation. The active checkout was
verified clean (`git status --porcelain` empty) after every probe.

### 4.1 `anvil drift snapshot` is a viable retrospective analyser

Command, per snapshot tree:

```console
cd <tree> && env -i PATH=/usr/bin:/bin HOME=<isolated> ANVIL_DEV=1 \
  GIT_CEILING_DIRECTORIES=<probe-root> \
  anvil drift snapshot --json --no-tui --anvil-home <isolated> --touch-project-state
```

| Metric | Snapshot A `5d5f99a8` | Snapshot B `6e658892` |
| --- | --- | --- |
| `schema_version` | `1.1.0` | `1.1.0` |
| `files_analysed` | 1728 | 1652 |
| `antipattern_count` | 93 | 86 |
| `suppression_count` | 5 | 5 |
| `expired_suppressions` | 0 | 0 |
| `boundary_violations` | 0 — **skip artefact, see 4.2** | 0 — **skip artefact** |

The counts discriminate between historical points, so the analyser is doing real
work on the materialised tree rather than reading ambient state.

**`files_analysed` is a scan-domain denominator, not replay coverage.** Snapshot
A's tree holds 4102 files; 1728 were analysed, because the scanner's default
extension domain excludes the rest. Replay coverage in the module's sense —
the proportion of *selected transitions* successfully analysed — is a property
of the runner's own snapshot ledger, not of this field. FEFF-003 must compute it
from analysed-versus-selected transitions and record files and languages in
scope separately, per the module's replay-contract item 7.

**Determinism:** a repeat run over the unchanged tree produced byte-identical
`schema_version`, `metrics`, `antipattern_breakdown`, `violations`,
`antipatterns`, and `suppressions`. Only `created_at` differed. This satisfies
FEFF-003's "byte-stable apart from explicitly excluded local receipt timestamps"
— and names the exclusion: `created_at`.

**Export hazard:** the snapshot's `antipatterns` and `violations` arrays carry
raw repository file paths and line numbers (for example
`apps/anvil-api/src/lib/fleet-overview.ts` at line 179). These are legitimate
local receipt content but are forbidden in an export payload by ADR-133 D-5.
Only the `metrics` and `antipattern_breakdown` aggregates may cross the export
boundary.

### 4.2 Blocking defect: a skipped check reports as a measured zero

`anvil gate --only-checks import-boundaries` returned, on both snapshots:

```json
{ "overall": true, "score": 100.0,
  "checks": [ { "name": "import-boundaries", "passed": true, "score": 100.0,
    "message": "No architecture config found (architecture section or .anvil/architecture.yaml). Skipping." } ] }
```

The repository carries `.anvilrc` at both commits but no
`.anvil/architecture.yaml` and no `architecture` section, so the boundary check
never ran. It nevertheless reported `passed: true` at `score: 100.0`, and the
corresponding `drift snapshot` reported `boundary_violations: 0`.

**Neither field distinguishes "not measured" from "measured zero".** A
retrospective runner that consumed either at face value would record perfect
architectural health for a repository that was never analysed — the module's
stated top risk, present and reproducible on the shipped surface.

**Consequence for FEFF-003:** before consuming `boundary_violations` at any
snapshot, the runner must independently establish that a boundary definition was
present and loadable at that snapshot, and record the answer per snapshot. A
snapshot without one is an **evidence gap**, never a clean result. This is now
frozen as ADR-133 D-11.

### 4.3 Blocking defect: `anvil gate` executes participant tooling

An unrestricted `anvil gate --json` in the materialised tree ran the `lint`
check, which invoked the repository's package manager through corepack from
inside the snapshot:

```text
"name": "lint", "passed": false, "score": 0.0,
"message": "Lint errors found\n\nnode:internal/modules/esm/utils:272 ...
  at Object.<anonymous> (<isolated-home>/.cache/node/corepack/pnpm/11.9.0/bin/pnpm.cjs:3:1)"
```

This is repository-declared tooling executing under the replay — a direct
violation of the replay contract's "never install dependencies or execute
participant code", and a live remote-code-execution path if the snapshot were
hostile. The composite `score: 50.0` it produced is also not a drift measure.

**Consequence for FEFF-003:** the composite `anvil gate` command is **barred**
from replay. Only narrowly selected, non-executing checks may run, and the
selection must be pinned in the study manifest rather than left to the profile
default. Frozen as ADR-133 D-10, second paragraph.

### 4.4 Isolation-flag interaction

`--anvil-home <dir>` **without** `--touch-project-state` causes
`anvil drift snapshot` to refuse outright:

```json
{"error":"Refusing drift snapshot under a non-default ANVIL_HOME ... Re-run with --touch-project-state if you deliberately want this candidate to write the real project."}
```

The guard is correct for its designed purpose — protecting a real project from a
side-by-side candidate — but it means the two flags a replay runner wants are
mutually exclusive: there is **no read-only drift-snapshot mode**. In replay the
"project" is a disposable archive tree, so passing `--touch-project-state` is
safe there, and that is what the probes did. It is a footgun elsewhere: the same
flag pair against a participant's live checkout would write their baseline and
witness chain.

**Consequence for FEFF-003:** the runner must assert that its working directory
is inside its own temporary root before passing `--touch-project-state`, and must
refuse to run if the target resolves to a path the participant owns. A read-only
snapshot mode would remove the footgun; filed as a follow-up rather than assumed.

### 4.5 Repository-discovery containment

`/tmp/.git` exists on this machine. Any replay tree materialised beneath `/tmp`
would let Git repository discovery walk up and bind to it, silently giving the
analyser a foreign repository's history. The probes set
`GIT_CEILING_DIRECTORIES` to the probe root and materialised under `~/.cache`.

**Consequence for FEFF-003:** path containment must include a Git-discovery
ceiling, not only filesystem bounds. `GIT_CEILING_DIRECTORIES` requires a
*proper* ancestor of the tree, so the runner must set it to its temporary root
and verify that `git rev-parse --show-toplevel` from inside the tree either
fails or resolves within that root.

## 5. Implementation owner decision

| Component | Owner | Rationale |
| --- | --- | --- |
| Retrospective baseline runner (FEFF-003) | **`crates/anvil-bench`** | It is a study harness, not a user command. It needs temporary trees, pinned analyser invocation, resource budgets, and fixtures — all bench-shaped. Keeping it out of `anvil-cli` avoids adding a study surface to the shipped command set and avoids the six-point command-registration cost. |
| Bundle verifier (FEFF-005) | **`crates/anvil-bench`** | Same harness lifecycle; consumes FEFF-003 and FEFF-004 output. The module already pins its validation to `cargo test -p anvil-bench`. |
| Prospective ledger and reviewed export (FEFF-004) | **`crates/anvil-cli`** | It must be a supported, user-explicit surface a participant can start, check, review, export, and delete. It also needs the privacy-control precedence checks of ADR-133 D-7, which live in the `anvil-cli` usage layer. New-command registration cost applies here and is accepted. |

This confirms the module's provisional `Packages:` line rather than changing it.

## 6. Dependency disposition

| Dependency | Status today | Disposition |
| --- | --- | --- |
| KFIT-007 (typed governance sink) | `Draft` | **Not a prerequisite.** FEFF-004 owns its own study ledger. |
| KFIT-009 (retire parallel sidecars) | `Draft` | **Not a prerequisite.** FEFF-004 must not depend on the usage sidecar at all (section 3.2), so its retirement is neutral to the study. |
| KFIT-010 (governance queries) | `Draft` | **Not a prerequisite.** Governance observations stay out of the first study's metric set. |
| DPO-003 | Blocked on KFIT-007/-009/-010 | **Not a prerequisite**, and FEFF must not be added to its blocking set. |

None of the four is Ready, and blocking the study behind a Draft chain would
stall FEFF indefinitely for metrics the first study is not permitted to claim
anyway (ADR-133 D-13). The module's "governance observations **where supported**"
wording is therefore satisfied by recording them as unsupported for study one.

**Undocumented-storage rule:** no FEFF component may read KFIT, DPO, kindling,
or daemon storage directly. Access is through supported commands and documented
schemas only. When KFIT-010 later ships a supported query surface, a follow-up
item may add governance metrics under a renewed consent review per ADR-133 D-5.

## 7. Follow-up items raised

These are recorded here for triage; none is a blocker for FEFF-003/-004.

1. **Usage surface:** two distinct fixes (section 3.4). (a) Honour any
   non-empty `DO_NOT_TRACK`, not only `"1"`. (b) Make `DO_NOT_TRACK` gate the
   save-time and fence producers in `daemon_observation_producers`, which today
   it does not reach at any value — (a) alone would not close this.
2. **Drift surface:** distinguish "boundary check skipped, no architecture
   config" from "zero boundary violations" in both `drift snapshot` JSON and
   `gate` output (section 4.2). This is a correctness defect beyond FEFF's needs.
3. **Drift surface:** provide a read-only snapshot mode so isolation does not
   require granting durable-write authority (section 4.4).
4. **Insights surface:** the six hardcoded zeros in the weekly summary are
   indistinguishable from measurements on the JSON wire (section 3.3). Consider
   a `null` or `"not-measured"` representation at the next schema version.
   `daemon_uptime_percentage` is schema-locked at `0`, so this needs a schema
   version bump rather than a code change alone.

## 8. Verdict

Historical replay is **feasible**. Two operator-owned snapshots replayed
deterministically in isolation, with the active checkout untouched, using
non-checkout materialisation and a pinned analyser.

Three blocking source defects were found before any collection machinery was
built: six of the weekly insights summary's seven metrics are placeholder
zeros; the usage
sidecar is machine-wide and 7-day-retained; and both the boundary check and the
drift snapshot report an unmeasured boundary state as a clean zero. All three
are now constrained by ADR-133 (D-10, D-11) and by the owner decisions in
section 5.

FEFF-003 and FEFF-004 are unblocked, with the source constraints above binding.
