# GCTX Snippet-Egress Line — Deep Review (GCTX-021/022/023 + `gctx.egress` flag)

**Date:** 2026-06-24
**Branch:** `feat/gctx-021-snippet-extractor` (combined commit `ecaa1872e` + follow-ups, tip `92cd989ae`)
**Authors reviewed:** Morgan Brighthand (combined GCTX-021/022/023 + flag) over my GCTX-021 foundation.
**Reviewers:** security-analyst (privacy/CE-gates), adversarial-reviewer, code-reviewer (correctness/completeness) + maintainer read of `symbol_context.rs`/`slice.rs`/`save_time.rs`.

## Verdict

**Strong core, ship-blocking gaps in two privacy areas + several correctness/observability fixes.** Gate state is green: `cargo build --workspace` clean, `clippy --workspace --all-targets -D warnings` clean, `fmt --all --check` clean, gctx-types 49 / gctx-egress 81 / proto 81 passing, plus substantive socket-level integration tests (`gctx_symbol_context_wired.rs`) covering CE-1/CE-2/CE-3/CE-7/kill-switch. The CE spine (CE-1 flag+capability, CE-2 fail-closed redaction, CE-5 sealed DTO, CE-7 freshness, CE-8 `openat2(RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH)` anchor, CE-9 flag, CE-11 kill-switch) is correctly implemented end-to-end. No path was found where source text egresses without flag + capability + freshness + secret-scan.

## Fixed in this review pass

- **get_snippet byte-ceiling now observable** (`save_time.rs`): a per-session byte-ledger refusal previously returned `Ready{text:None}` indistinguishable from an identity-only response. Now sets `truncated=true` + `omitted_bytes` so the CE-6 suppression is visible (counts-only, CE-11-safe).
- **Leaked agent-session path** in `scripts/gctx-goal-verify.sh` (`/tmp/grok-goal-702d0af3ed1f/implementer`) → `${TMPDIR:-/tmp}/gctx-goal-verify`.
- APS honesty fixes (see below): GV2 index `Complete`→`In Progress` (Done≠Complete per lifecycle); GCTX-021 item text now states the CE-3 gitignore deferral + the tail-language span gap.

## Must-fix before merge (privacy / contract)

1. **CE-3 gitignore omission is unimplemented (MAJOR, 2 reviewers).** The PV-9 verdict + the GCTX-021 item text require gitignored files omitted *entirely* on the snippet path (the substrate scans with `standard_filters(false)`, so gitignored content is graph-resident). Only the static deny-list `is_sensitive_egress_path` exists — no `.gitignore` consultation. A gitignored, non-deny-listed, non-secret-shaped, fresh file would egress its source text under flag+capability. **Recommend:** inject a per-root `ignore::gitignore::Gitignore` matcher into the snippet path (mirror the ADR-064 redactor injection), drop gitignored candidates into `omitted_sensitive_paths`, add the missing fixture — OR, if deferred, file an ADR/verdict amendment; do not ship with the CE-3 claim unmet. Item text has been corrected to flag this as a pre-merge blocker.

2. **CE-2 line-based redaction misses multi-line secrets (medium, adversarial).** `redact_gctx_snippet` scans per `content.lines()`; a secret split across two physical lines (wrapped PEM body, `"AKIA" +\n"…"`) is not caught. Tests only cover single-line tokens. **Recommend:** add a full-text second pass (or document the limitation explicitly in the CE-2 gate comment) + a split-secret test. Also normalise CRLF→LF before scan/reconstruct (current code corrupts CRLF at redacted lines).

## Should-fix (defense-in-depth / correctness)

3. **Per-span byte ledger defeated by per-call connections (medium, adversarial).** The MCP `anvil_symbol_context` tool opens a fresh `UnixStream` per call → new `SaveTimeConn` → blank `SnippetByteLedger`. The intended per-`(file,ByteRange)` session dedup doesn't persist; the only real cap is the 8 MB process-level `GRAPH_EGRESS_SPENT`. **Recommend:** hold the daemon connection for the tool session, or move per-span accounting to the process-level accumulator, or track seen spans MCP-side.

4. **`gctx.egress` manifest flag is disconnected from the daemon (low).** The daemon gates on `ANVIL_GCTX_EGRESS` env only; the manifest flag/TS catalogue export is never read by Rust → decorative. **Recommend:** wire a Rust consumer (flag → env resolution) or note explicitly that the env var is the authoritative control and the manifest entry is the catalogue record only.

5. **CE-5 no-leak regression test missing for GCTX-023 (minor).** `SymbolContextOutcome`/`SymbolContextProjection`/`OmittedContext` lack the structural forbidden-name battery its sibling DTOs have. Mechanism is sound (text only via the audited `ContextSnippet.snippet` carve-out), but a future field-add wouldn't be caught. **Recommend:** add the battery test mirroring `snippet_outcome_non_ready_arms_have_no_forbidden_keys`.

6. **`is_sensitive_egress_path` deny-list gaps (low):** add `.npmrc`, `.netrc`, `*.tfvars`, `*.tfstate`, `docker/config.json`.

7. **`seal_symbol_context_outcome` outcome-label semantics (verify):** token-budget-only overflow maps to `Bounded`, byte-ceiling to `BudgetExceeded`. Likely intentional (soft vs hard limit) but lock the wire semantics before the format stabilises.

## Nits

- `same_symbol` is a dead one-line `PartialEq` wrapper in prod code — inline + delete.
- `symbol_context.rs` input schema uses `additionalProperties: true`; every other tool uses `false`.
- `RedactionSummary.fields_suppressed` is serialised but never populated (always 0).
- `gctx_symbol_context_wired.rs` is Linux-only (`cfg(all(unix, linux))`); ensure the macOS leg runs via dispatch before merge.

## Completeness

- **GCTX-021** — core complete (resolve/project, CE-1/2/5/7/8 gates, sealed DTO, TS/Rust/Python fixtures). Gaps: CE-3 gitignore (#1), tail-language gap now documented in item text.
- **GCTX-022** (`slice.rs`) — complete: budget property test (`estimate ≤ budget`), determinism golden, byte-ceiling + redact-before-budget, consumes GCTX-020 `estimate_gctx_tokens`.
- **GCTX-023** (`anvil_symbol_context`) — complete: registered in MCP TOOLS, RPC wired (proto→ipc→save_time), warm-daemon integration test. Gaps: #3, #5.
- **ADR-091** cursor (FNV, not HMAC) — by design; forged-cursor tests confirmed substantive (a forged cursor only reseeks within the caller's own authorised window).
