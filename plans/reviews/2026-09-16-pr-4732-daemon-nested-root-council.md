# Council Review — PR #4732 daemon nested GCTX root

**Status:** Converged
**Tier:** full
**Target:** `origin/main...HEAD` on `fix/cib-414-daemon-nested-root`
(protected: `crates/anvil-intercept/src/save_time.rs`,
`crates/anvil-intercept/src/workspace_admission.rs`)
**Date:** 2026-09-16
**PR:** https://github.com/eddacraft/anvil-001/pull/4732
**Contract:** GitHub issue #4727 (CIB-414: refuse a nested GCTX root at the
daemon socket). CIB-398 refused nested GCTX roots in MCP only; the daemon still
accepted them via `authorise_root` plus `WorktreeKey::from_canonical`, allowing
`<repo>/secrets` first-touch-adopt in Open mode and rebasing identities past
the CE-3 deny-list. The new gate must run before admission (no first-touch
adopt, no CIB-154 budget consume on refuse). Wire refusal stays static
path-free `workspace-not-admitted` (`SaveTimeError::NotAdmitted`). All nine
`anvil/gctx/*` call sites; leave `validate_paths` / `workspace_status` /
`request_full_scan` / `witness_append` on plain `authorise_root`. Feature PR
must not edit the shared CIB APS module. Residual foreign nested git / planted
`.git` directory is later_item unless an in-contract must_fix exists on the
live head.
**Session:** `council-b1ae836e` (assurance / full)
**Code head reviewed:** `f4c2ad852b20cf20a5b2a93a8d44681f88c60d74`. Later
commits on this branch, if any, must be this review only; re-council if the
protected files move after the label.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `workspace_admission.rs` | `is_graph_root` / `permits_graph_root` / git-layout readers; gate before admit | Nested `<repo>/secrets` cannot key a graph on first contact |
| `save_time.rs` | `authorise_gctx_root` + `AuthorisedGctxRoot`; nine GCTX handlers | Socket clients get the CIB-398 rule; TOCTOU-closed single canonical |
| intercept + architecture docs | Freshness only | Diagrams unchanged |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general / kernel-maintainer | approve | All nine GCTX handlers use `authorise_gctx_root`. Public `authorise_canonical_within_budget` is optional hardening, not a current skip. Git-layout reader is large but justified (no `git` subprocess; gitfile fail-closed). |
| adversarial | approve | First-contact refuse holds in Open and Allowlist (prefix allow still refused). Budget of 1 still admits the parent after a nested refuse. Forged gitfile and planted `worktrees/*/gitdir` without reciprocal gitfile do not authorise secrets. Handlers key `WorktreeKey` on the gate's canonical path. |
| security | approve | CE-3 rebase is closed for nested dirs without a planted `.git` directory. Wire mapping stays `{"reason": "workspace-not-admitted"}`. Server `tracing::warn!` logs the requested root only. `validate_paths` remaining on `authorise_root` is not a GCTX identity bypass. |
| operations | approve (GO) | Linked worktree, separate-git-dir, and `.git` symlink-to-directory parents still authorise; nested secrets stay refused. Descendant repo under an ancestor checkout still authorises. Unresolvable GCTX root maps to `NotAdmitted`, not `Io`. |
| pragmatic | approve | Scope is the nine graph verbs only. No CIB APS edit. Docs are freshness-only. Residual planted `.git` directory is correctly scoped out so `$HOME`/dotfiles descendants still work. |
| **judge** | **Ship** (decision PASS, gate PASS) | No remaining `must_fix`. |

## Findings

### Later item (not must-fix)

- **C-001** (security + adversarial + operations): a planted
  `<repo>/secrets/.git` **directory** (or symlink to a directory) is itself
  the nearest checkout, so first contact authorises it. A planted `.git`
  **file** that is not a registered linked worktree is ignored. This is the
  documented residual versus MCP `nested_untrusted_git_root`; `f4c2ad852`
  keeps nearest-checkout equality so a descendant repo under `$HOME`/dotfiles
  still authorises. An attacker who can write that `.git` directory already
  has write access to the tree they would rebase.

### Consider (not must-fix)

- **C-002** (kernel-maintainer): `permits_graph_root` and
  `authorise_canonical_within_budget` remain public. All nine current GCTX
  handlers go through `authorise_gctx_root`, so nothing skips the rule today.
  Fusing behind a private `authorise_graph_root_within_budget` is optional
  future-handler hardening.

## Evidence

- `cargo test -p eddacraft-anvil-intercept --lib --no-fail-fast` on
  `f4c2ad852` — 1224 passed, 0 failed.
- Named CIB-414 tests (Open/Allowlist first-contact refuse, admitted-parent
  refuse, linked worktree admit, forged gitfile refuse, descendant-repo
  authorise, handlers key on canonical, separate-git-dir, symlink `.git`,
  planted worktree gitdir) — all passed.
- `cargo clippy -p eddacraft-anvil-intercept --lib -- -D warnings` — exit 0.
- Diff `origin/main...HEAD` does not touch
  `plans/modules/continuous-improvement-backlog.aps.md`.
- `ipc.rs` `NotAdmitted` arm still emits
  `json!({"reason": "workspace-not-admitted"})` with no path on the wire.

## Label

Maintainer applies `council:reviewed` to the **live** PR head after this
review file lands. Do not label a stale SHA. The label is dismissed on
synchronize.
