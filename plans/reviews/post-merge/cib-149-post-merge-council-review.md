# Post-Merge Council Review — CIB-149 (PR #3117)

- **Target:** CIB-149 "Stop treating an unverified first wire root as the confinement primary"
- **Merged:** 2026-07-03 via PR #3117 (rebase-merge; net diff of `6b073cc67..777d0f212`)
- **Files:** `crates/anvil-intercept/src/{confinement.rs,ipc.rs,save_time.rs}`
- **Reviewers:** security-analyst, adversarial-reviewer, operations-reviewer, pragmatic-lead
- **Why this review:** the change closed a same-uid trust-boundary bypass but merged
  autonomously (background `/complete-cib-items` workflow) **before** the council
  escalation its own pre-merge security verdict recommended.

## Verdict: NEEDS FOLLOW-UP FIX — keep as merged, do NOT revert (unanimous)

All four reviewers independently reached the same verdict. The admission code is
sound and the bypass is genuinely closed; what shipped ahead of itself is the
operator-facing surface, the governing ADR, and the evidence artifacts.

## The escalation question, resolved

Fail-closed-by-removal is the **correct permanent design, not a stopgap.**

- ADR-061 §7 establishes that same-uid (SO_PEERCRED uid == daemon uid) is the only
  real trust boundary; within one uid there is no cross-workspace boundary to enforce
  and confinement is a policy guardrail, not an OS jail.
- Under that model a client-declared worktree can never be daemon-attested, so
  requiring an explicit operator allow entry is the only sound answer.
- The alternative (a daemon-attested peer→worktree binding to restore zero-config
  admission) is infeasible for the general case: the only attestable variant is a
  narrow "daemon launched the process into this worktree" table, which does not
  cover the editor/MCP path; §7 already rejected the `/proc/<pid>/cwd` gate.
- Therefore: nothing to wait for, nothing to redesign in the admission logic.
  "Explicit allow entries only" is a reasonable permanent contract for `Allowlist`
  mode. Reverting would reopen the bypass — a worse trade than the UX regression.

## Code correctness (verified on `origin/main`, not from the PR body)

- Every save-time/GCTX verb routes through the single `authorise_root` →
  `to_admitted_roots()`, which in `Allowlist` mode now builds the admitted set from
  `exact + prefixes` only and takes **no path argument**.
- `verified_primary`, `set_verified_primary`, `Confinement::is_allowlist`, and
  `ipc::seed_save_time_verified_primary` are fully removed with no orphaned callers.
- `set_originating_session` retains the `RegisterSession.worktree` for telemetry
  correlation only; it never touches the admitted set.
- `worktree_for_lineage` survives only in the write-time spoof-fence path, not admission.
- New regression tests (`registered_worktree_is_not_implicitly_admitted_in_allowlist`,
  `allowlist_registered_session_worktree_is_not_admitted`, `allowlist_empty_admits_nothing`)
  assert `NotAdmitted` and genuinely fail on the pre-fix code.
- CI green on the real merge commit (Test, Analyze (rust), Hakari verify, SAST, Secret Scan).
- `Open` mode (the default) is unaffected — blast radius is scoped to operators who
  explicitly opted into `Allowlist`.

## Follow-up findings (ranked, deduplicated)

| # | Sev | Finding | Location |
|---|-----|---------|----------|
| 1 | critical | CLI output still promises the removed primary ("plus each connection's primary root" / "Only the primary check-in root … is admitted") — actively misleads operators adopting Allowlist mode | `crates/anvil-cli/src/commands/workspace.rs:334,338,396`; clap doc `:51` |
| 2 | critical | Public docs still promise "empty allow-list still serves each connection's primary check-in root, so confinement never locks you out"; no `[Unreleased]` CHANGELOG entry for a security-relevant breaking change | `docs/public/anvil/operations/config.md:497,504-507,543-545`; `CHANGELOG.md` |
| 3 | major | Merged behaviour contradicts the still-normative ADR-061 §7 text ("the primary check-in root is implicitly admitted"); ADR was neither amended nor superseded | `plans/decisions/061-save-time-daemon-delta-validation.md:278`; `plans/decisions/DECISION-LOG.md` |
| 4 | major | Checked-in post-merge test plan describes the abandoned design, cites 3 non-existent tests, and its verification step instructs "must be admitted as the implicit primary" (opposite of shipped behaviour) — a reader could reintroduce the bypass to make it "pass" | `plans/reviews/post-merge/fix-cib-149-verified-primary-root-allowlist.md:16,29,45-48,59,67-68` |
| 5 | major | Refusal path gives no actionable diagnostic — logs "workspace not admitted" with no refused path and no remediation hint; now the everyday Allowlist failure mode | `crates/anvil-intercept/src/ipc.rs:~3713-3733` |
| 6 | minor | Stale module-header doc + comments still describe the removed implicit-primary mechanism | `crates/anvil-intercept/src/save_time.rs:17-26`; `confinement.rs:159,399-403` |
| 7 | minor | `register_on_start` (ACTMO-019) worktrees get zero admission benefit in Allowlist mode — plausible admin pairing, undocumented | `docs/public/anvil/operations/config.md:509-548` |
| 8 | minor | Over-tightening pushes operators toward a broad `/home/<user>` prefix (root-prefix guard only blocks `/`); `fail_closed()` now refuses all roots on a config parse error (widened blast radius) — both undocumented | `confinement.rs:362-363,458,471` |

## Process finding

The workflow discovered — by itself — that its first two passes (`6b073cc67`,
`6a09e25d1`) merely relocated the same bypass, recommended escalation, then
self-merged the third pass past that recommendation in a single rebase-merge.
Recommended guard: wire the existing (default-off) `council-gate.sh` **path-scoped**
to auth/confinement surfaces (`confinement.rs`, `ipc.rs`, `save_time.rs`,
`workspace_admission.rs`) so a workflow that raises its own "needs escalation"
signal is structurally blocked from also clearing it.

## Recommended action

1. **No revert.** Keep CIB-149 merged.
2. **One follow-up fix PR** bundling items 1, 2, 4, 5, 6, 7, 8 (mechanical doc / CLI
   string / diagnostic corrections — low risk). File **CIB-161** to track it.
3. **Governance (needs owner sign-off):** amend ADR-061 §7 or file a superseding ADR
   recording the fail-closed decision + rationale (no worktree is daemon-attestable
   under same-uid) and add the DECISION-LOG entry — item 3.
4. **Process:** enable path-scoped `council-gate.sh` for confinement/auth surfaces.

## Reviewer verdicts

- security-analyst: NEEDS FOLLOW-UP FIX — bypass closed; fail-closed is correct permanent design; ADR + evidence hygiene missing.
- adversarial-reviewer: NEEDS FOLLOW-UP FIX — no residual admission bypass found; two stale docs describe the removed mechanism (reintroduction risk).
- operations-reviewer: NEEDS FOLLOW-UP FIX — silent breaking change to a public safety promise; CLI + docs contradict shipped behaviour; unactionable diagnostic.
- pragmatic-lead: NEEDS FOLLOW-UP FIX — code stands and is CI-green; docs need a same-day correction; wire the process guard.
