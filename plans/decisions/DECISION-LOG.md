# Decision Log

Condensed index of all Architecture Decision Records (ADRs). Read the linked
document for full context and trade-off analysis.

For the ADR process and when to write one, see
[docs/guides/adr-process.md](../../docs/guides/adr-process.md). For a live
integrity report against this directory (no duplicate numbers, no missing
entries, next available ADR number) run `pnpm adr:check`. The fixture tests
for the script itself live at `pnpm test:adr-integrity`.

## Core Philosophy

These define how Anvil behaves by default. All features must align.

| ADR | Decision | Status |
|-----|----------|--------|
| [001](001-planless-first.md) | Deliver value without requiring plans or config; codebase is the baseline truth | Accepted |
| [002](002-warnings-over-blocks.md) | Warnings don't block (exit 0) by default; opt-in `fail-on-warnings` for enforcement | Accepted |
| [003](003-new-edges-only.md) | Baseline existing architecture on first run; only warn on new violations | Accepted |
| [004](004-suppression-syntax.md) | `@anvil-ignore[-until DATE] WARNING-ID: reason` for targeted, explained suppressions | Accepted |

## Technology Stack

| ADR | Decision | Status |
|-----|----------|--------|
| [000](000-use-zod-for-aps-schema-definition.md) | Zod for APS schema definition, export to JSON Schema when needed | Accepted |
| [005](005-ink-over-opentui.md) | Ink (React-based Node.js) for TUI components | Accepted |
| [006](006-hybrid-dc-opa.md) | Dependency-Cruiser for static analysis + OPA for policy evaluation | Accepted (engine half amended by ADR-040) |
| [007](007-pulumi-iac.md) | Pulumi (TypeScript) for infrastructure as code | Accepted |
| [014](014-language-allocation-tree-ts-vs-rust.md) | TypeScript for orchestration/UX; Rust for CPU-bound hot paths | Proposed |
| [051](051-cli-panic-unwind-for-untrusted-input.md) | `anvil` CLI builds with `panic = "unwind"` (was `abort`) so untrusted-input panics (e.g. a `regorus` internal panic during `anvil policy eval`) unwind to a `catch_unwind` guard at the policy-engine facade → structured error + non-zero exit, not a no-diagnostics abort. Generalises the `release-napi` FFI-boundary discipline to the CLI | Accepted |

## Rust Migration

| ADR | Decision | Status |
|-----|----------|--------|
| [011a](011a-rust-core-engine.md) | Rust core engine for performance-critical paths (watch mode, analysis, policy) | Superseded |
| [012](012-rust-cli-replacement.md) | Single `anvil` Rust binary replaces Node.js CLI; big bang, no hybrid period | Accepted |
| [017](017-crates-io-naming.md) | Publish crates under `eddacraft-anvil-*` namespace to avoid collisions | Accepted |
| [026](026-rust-scanner-authoritative.md) | Rust scanner is authoritative; `patterns/compiled/registry.json` is the contract; TS scanner stays only for in-process IDE/MCP surfaces until a napi-rs migration retires it | Accepted (amended by ADR-033) |
| [033](033-park-ide-mcp-retire-ts-scanner.md) | Archive VSCode extension and TS MCP server (`archive/anvil-vscode-extension/`, `archive/anvil-mcp-server/`); archive TS scanner, TS suppression parser, and parity harness to `archive/anvil-ts-scanner/`; CI excluded via `'!archive/**'`; napi crate stays as build canary; surfaces return as new active packages via DRVR / RMCPF / future ADR | Proposed |
| [040](040-rust-policy-engine-regorus.md) | Adopt `regorus` as the embedded Rust policy engine behind `crates/anvil-policy-engine` facade; amends ADR-006 engine half (Dependency-Cruiser unaffected); POLENG-008 bench gate validates parity vs. Go OPA reference | Accepted |

## Product and Distribution

| ADR | Decision | Status |
|-----|----------|--------|
| [018](018-product-ip-architecture.md) | Free base tier, source-proprietary model; three foundational repos as OSS | Accepted |
| [020](020-versioning-strategy.md) | Lockstep versioning for Anvil core; independent versioning for separate products, OSS, and peripherals | Accepted |
| [025](025-package-manager-distribution.md) | Package manager distribution strategy across npm / crates.io / GitHub Releases | Accepted |
| [045](045-update-signing-scheme.md) | Minisign (Ed25519 + BLAKE2b) signs release artefacts; pure-Rust `minisign-verify` enforces the chain inside `anvil update`; long-lived key custody via GitHub Actions secret + offline backup | Proposed |
| [047](047-eddacraft-tui-canonical-source-mirror.md) | Move `eddacraft-tui` canonical source back into Anvil; keep the public repo as a read-only mirror and crates.io as external distribution | Accepted |
| [050](050-eddacraft-tui-runner-and-cli-policy.md) | `eddacraft-tui` ships an opt-in `runner` feature flag bundling a turn-key `launch_default(app)` entry point (lifecycle + minimal `lexopt` parser + `TerminalApp` trait); library-shaped consumers get a CLI from a 3-line `main.rs`; Anvil keeps its own `TerminalGuard` + `clap` tree (non-adopter); no `clap` in core, no `[[bin]]` in core, no sibling crate | Proposed |
| [054](054-json-render-tui-engine-home.md) | json-render TUI engine (spec parser, registry trait, tree walker, generic base catalogue) lives in `eddacraft-tui` behind a `json-render` feature; Anvil-domain catalogue + `.anvil/` data context + dashboard surface live in `anvil-tui`. No standalone `anvil-tui-render` crate; one-way dep `anvil-tui → eddacraft-tui`. Re-homes TUIDASH | Proposed |
| [055](055-aps-oss-carveout.md) | Narrow carve-out from ADR-018: read-only consumers of the public APS format — the `anvil plan dashboard` viewer AND the `scripts/aps/*` format tooling (drift-check, index-count derivation, active-lint) — may be published `Apache-2.0` in `anvil-plan-spec` as one-way forks, bounded by a three-test rule (public-format consumer, public-OSS-only deps, no product internals); product engines and product dashboards stay closed. Legal sign-off required before Accepted | Proposed |
| [056](056-format-flag-output-selector.md) | `--format <auto\|tui\|plain\|json\|sarif>` value-enum is the canonical output selector; `OutputMode` gains a `Sarif` variant with explicit precedence; `--json`/`--no-tui` retained as compatibility aliases (no deprecation); `--format sarif` valid only on finding-emitting commands and never TTY-auto-selected. Future machine formats are added as enum values, not booleans. **Amended 2026-05-29:** scoped per-command on `check`/`gate`/`audit` (not global) — `--format` already collides with `export`/`validate`'s domain flags | Accepted |
| [060](060-anvil-home-install-root-override.md) | Per-project state resolution under `ANVIL_HOME` (gates DISTRIB-006): re-root install/user-owned state (user dir, daemon socket/PID, cache/logs) under the prefix; keep per-project `.anvil/` (baseline/cache/witness) + `anvil/project-id` resolving to the **project root** (Option a, not re-rooted) so candidate tests run against the real repo with witness continuity — but **guard** durable project-state mutations (baseline/witness/cutoff) behind `--touch-project-state`, read-only/dry-run by default under a non-default `ANVIL_HOME`; `status --json` gains `installRoot` + write-gated fields; unset = byte-for-byte default. Rejected Option (b) (re-root project discovery) as defeating the side-by-side purpose. Cross-version chain *format* compat stays DISTRIB-005/`anvil migrate` | Accepted |
| [066](066-github-device-flow-cli-auth.md) | Adopt the **GitHub Device Authorization Grant** (RFC 8628) as the default `anvil auth login`, **brokered server-side through `anvil-api`**, replacing the broken homegrown device-code browser-confirm flow (`/device/confirm` was hardened by #1779 to require an `Authorization` header the website cannot send, so device login is un-completable). The CLI talks only to `anvil-api`; `anvil-api` calls `github.com/login/device/code` + the device-token poll endpoint. Brokering exists for **identity-brokering + server-side licence mint / active-status gate**, NOT secret custody — the device grant is a **public-client** flow (no `client_secret`). New dedicated "Anvil CLI" OAuth app (separate from `eddacraft Docs`, Device Flow ticked) + Key Vault `github-cli-client-id` / `github-cli-client-secret`; new versioned `POST /api/v1/auth/github-device/{start,poll}` (old `/device/*` retained until the new CLI ships); account-link by GitHub numeric `github_id` (first-link of an email-invited record via **any verified** GitHub email, then `github_id` authoritative); invitation stays **email-keyed** with GitHub as a linked auth method — funnel unchanged, invite email drops the retired activate URL/code and points at `anvil auth login`/`--otp`, vestigial interactive `/admin/invite` device code removed (`tokenOnly` unaffected); DB-backed `github_device_sessions` (Vercel serverless); single-use atomic `DELETE … RETURNING` mint, hashed `poll_token`/`device_code` at rest, GitHub token revoked after `fetchGitHubUser`. Reuses `auth-github.ts` mint path (extract `mintLicenceForGitHubUser`) + the ES256 `signLicence` (no licence schema change — `LicenceClaims` already supports `provider:'github'`). Email OTP (`--otp`) retained as the email-proof fallback. Activation page tombstoned + admin-invite path rebuilt. Supersedes BAUTH's device-confirm sub-flow; reuses DOCSAUTH's GitHub OAuth path. Rejected: fix the website-confirm flow (needs a website auth surface + a browser); CLI talks to GitHub directly (licence mint/gate/signing key cannot move client-side) | Accepted 2026-06-04 |

## Configuration and Structure

| ADR | Decision | Status |
|-----|----------|--------|
| [016](016-unified-config-format.md) | Consolidate three config files into single TOML with source delegation | Proposed |
| [046](046-yaml-parser-migration-deferral.md) | Defer `serde_yaml` → maintained-parser migration; byte-level pre-pass (size cap, alias reject, depth cap) is the actual defence; review 2026-08-15 | Accepted |
| [021](021-in-house-nx-rust-plugin.md) | In-house `@eddacraft/nx-rust` plugin; reject monodon (no licence) and cargo-make (not a substitute) (originally drafted as ADR-026; renumbered in DOCGOV-004 to resolve a duplicate-number conflict). Amended 2026-05-29 (DEVENV-003): plugin extracted to the standalone public repo `eddacraft/nxrust`, consumed from the registry as `@eddacraft/nxrust`; the in-repo `tools/nx-rust` vendored copy was dead code and removed | Proposed |
| [023](023-shared-packages-restructure.md) | Retire `packages/platform/`, consolidate into `packages/shared/` | Proposed |
| [044](044-mcp-entry-activation-owned.md) | `mcpServers.anvil` MCP entries are owned by the activation flow; backend swaps overwrite in place on next `anvil start` with a one-line notice; `--keep-mcp` opts out; protocol-shape changes still route through `anvil migrate` | Proposed |
| [049](049-cross-language-build-contract.md) | Cross-language `^build` contract: defer to nxrust D-009 (binding upstream); Anvil enforces the seam at the script layer (`pnpm test:js && pnpm test:rust`, PR #1729) until adopting nxrust generators that enforce at the generator boundary; no parallel Anvil recipe | Accepted |
| [057](057-dev-environment-hardening.md) | Local dev-environment hardening for the concurrent-agent box: layered Rust target relocation (committed `.cargo/config.toml` floor on `/home` + per-worktree `CARGO_TARGET_DIR` override), `[profile.dev]` debuginfo trim, nx-rust executor `CARGO_TARGET_DIR`-awareness + `.anvil-building` sentinel, disk-pressure eviction timer, Node version alignment, and `.config/wt.toml` worktree-lifecycle + parity fixes (wave 1); reproducible base (mise/devcontainer/Nix) + nx-cache/sccache dedup deferred to a go/no-go spike (DEVENV). Council `plan-6b3be127` | Accepted |
| [058](058-sarif-shared-emitter-no-finding-model.md) | SARIF output uses a thin shared emitter (`output/sarif.rs`) owning the pinned 2.1.0 document subset, fed by per-command adapters (SARIFOUT-003/004/005) that map each command's existing finding shape; deliberately **no** unified in-process finding model and **no** engine-crate refactor (SARIF itself is the shared target). Bundled upstream 2.1.0 schema, verbatim (no fork), is the validation gate. Second SARIFOUT candidate ADR alongside ADR-056 | Accepted |

## Intercept and Enforcement

| ADR | Decision | Status |
|-----|----------|--------|
| [015](015-intercept-loop-enforcement.md) | Rust daemon detects file changes from AI agents, evaluates policy, interrupts sessions | Proposed |
| [031](031-validation-latency-rubric.md) | Shared latency measurement rubric for intercept validation modes; standardises modes, timing boundaries, dimensions, and warm p95 SLOs so INTD / DRVR / RTAI cite one source | Proposed |
| [036](036-daemon-scope-discovery-and-boundaries.md) | Daemon scope, discovery, and OS-boundary policy: what the intercept daemon is allowed to see, watch, and act on | Accepted |
| [038](038-hook-surface-and-noise-discipline.md) | Hook surface contract and noise-discipline rules for the intercept hook system | Accepted |
| [043](043-ssh-remote-host-daemon.md) | SSH remote support runs Anvil on the remote host; local surfaces are display/control only and must not claim local daemon protection for remote files | Proposed |
| [061](061-save-time-daemon-delta-validation.md) | Save-time governance is daemon-mediated delta validation: `anvil watch` stops cold-spawning `check --all` per save and instead routes changed paths to the existing intercept daemon (`anvil/validate_paths` alongside `scan_buffer`) over warm Graph V2 hot-read state; watch/MCP/intercept become thin clients of one per-host daemon (one warm model, one work budget, per-host rayon); whole-repo scan becomes explicit/background with a `clean\|stale\|pending\|running\|unavailable` workspace-assurance state; daemon-absent degrades to a scoped (never `--all`) subprocess / in-process scan reporting `unavailable`, exit 0. Phase 1 (scope per-save check, RLB-007) ships independently of daemon/GV2. Sequences INTD/DRVR/RLB/GV2 behind one product contract | Accepted 2026-06-01 (council `plan-5768ae0c`) |

## Policy and Governance

| ADR | Decision | Status |
|-----|----------|--------|
| [019](019-flags-observability-alignment.md) | Align feature flag telemetry with OBS/Kindling before FLAGS work | Proposed |
| [022](022-opa-agent-orchestration.md) | OPA Agent orchestration for continuous policy intent translation and explainable guidance | Proposed |
| [035](035-three-pipe-observability-rule.md) | Three-pipe observability rule: Kindling = governance facts, Notification = user-visible state, tracing/OTEL = ephemeral debugging (never source-of-truth); `traceparent` is the cross-pipe correlation key | Accepted |
| [037](037-witness-chain-and-l4-policy.md) | Witness chain and L4 policy framework for cross-surface policy evidence | Accepted |
| [039](039-baseline-policy-and-hard-pinned-classes.md) | Baseline policy and hard-pinned rule classes; codifies which warning classes never get baselined | Accepted |
| [041](041-flag-snapshot-usage-join-contract.md) | Usage rows store resolved flag context inline; manifest `key` is the stable join key; ADR-019 stays gate-affecting-only for standalone Kindling flag facts | Accepted |
| [048](048-feature-group-architectural-model.md) | Feature Group is a defaults carrier (class + audiences + lifecycle) with per-flag override; hybrid taxonomy (`primaryGroup` surface + `tags` capability); kill-switch is a universal runtime channel via `FlagOverrides.emergency`, not a per-group class | Accepted |
| [052](052-automated-drift-snapshots.md) | Capture drift as an append-only **edge-delta event ledger** (`anvil/drift/edges.ndjson`, carrying `anvil_version` + `rules_sha`) appended on merge-to-`main` via the PR that introduces the edges — not periodic whole-state snapshots. Event-driven (no intra-week blindness), lossless, supports net + peak, team-shared + trunk-safe with no scheduler/extra-PR/bypass. Consumed by `anvil drift report` + INSIGHTS-003. Supersedes the original scheduled-CI-snapshot proposal; rejects daemon/hook/witness-chain/orphan-branch (planning council `plan-0e9c300c`) | Proposed |
| [059](059-production-tracing-sink.md) | Production tracing sink: OTLP-neutral (OTel) instrumentation → **Azure Monitor + Application Insights** (via the Azure Monitor OTel exporter), **dashboards hand-rolled (KQL/Workbook) first, Azure Managed Grafana later**. Exports **operator-hosted surfaces only** (`apps/anvil-api`) while the local-first Rust CLI/daemon stay formatter + local-file (never auto-export); redaction-wrapped (TRACE-003), config-gated off by default; alerting via uncapped Azure Monitor rules. Chosen for Azure stack consolidation over Honeycomb (best trace-debug query, but separate vendor + ~2-trigger free cap) and Grafana Cloud. Feature-flagged usage stays Kindling-of-record (USAGE), App Insights breadcrumbs only. Resolves EXPORT OQ1 | Accepted |
| [062](062-policy-evidence-drift-as-evidence.md) | Policy & evidence source-quality drift as first-class **advisory** evidence: stale / superseded / conflicting / unverified cited sources become deterministic (`AsOf`-injected), severity-graded `evidence_drift_findings` that **downgrade `evidence_strength`** (error→weak, warning→moderate) instead of blocking (ADR-002). Fields land on the CEWS `EvidenceRecord` (`policy_source_ref`, `policy_source_digest`, `policy_canonical`, `policy_review_due`, `policy_superseded_by`, `source_conflict_status`, `evidence_verification_status`, `code_anchor_status`); computation in MDGOV M2; defers to ADR-058 (no unified finding model — maps to SARIF per-command); clean-room from the DocGraph reference (MIT, no dependency); `code.*` beta-later via graph-v2-foundation | Proposed |
| [063](063-gv2-hot-path-boundary.md) | Graph V2 **hot-/non-hot-path read boundary** for save-time validation: a read is hot-path-admissible (inside `validate_paths` / driver mid-edit, against the ADR-031 budget) **iff** answerable from **resident warm indexes** in O(1)/O(bounded-fan-out) with no parse, no cross-file resolution, no transitive traversal beyond **1-hop reverse-impact**, and no I/O. Allowlist = resident extract lookup, known-edge existence, 1-hop `dependents_of()`, precomputed arch-index checks; everything else (parse/resolve/>1-hop/scan/rebuild/persist) is background-only. A warm miss returns a typed `stale` → `StaleReason` → fallback and **never** escalates on the hot path. Reverse-impact **depth is a hard-capped, feature-flagged lever** (default 1 hop, switchable to 2 without re-coding). One rule across INTD+DRVR+GV2 surfaces, behind the frozen ADR-061 wire; enforced by a GV2-022 type split + the ADR-031 benchmark. Closes the ADR-061 §9 boundary gate; clears GV2-022 freeze + sub-phase A′ | Accepted |
| [064](064-intercept-graph-cache-crate-boundary.md) | Resolves council blocker **B5** (predecessor to B1) for daemon save-time sub-phase A: extract a new internal crate **`eddacraft-anvil-graph-cache`** (`SymbolGraph`, `DependencyGraph`, incremental apply-delta, net-new `certify`) that both `anvil-kernel` and `anvil-intercept` depend on, instead of adding `anvil-kernel` to the daemon's deps. Cycle audit: kernel does **not** depend on intercept, so neither option cycles — the deciding factor is build-weight + the documented `watcher.rs:28` refusal. The graph layer is **already parse-free** (`update_file` takes parsed `FileSymbols`; only the relocatable plain structs `ImportEdge`/`FileSymbols` couple it to the parser — moved to `anvil-kernel-types::graph` beside `SymbolNode`). New crate deps are `petgraph`-only (already in-tree); **no `tree-sitter`/parser/`notify`/`walkdir`** enters the resident daemon. Parsing stays kernel-side; the daemon hot path only **reads** (`certify` over `dependents_of`, net-new — zero prod callers). Unblocks B1's `(SymbolGraph, DependencyGraph)` cache + `certify(sym, dep, change, delta, budget)` signature; gives sub-phase A′ GV2 hot-read its parser-free home. Rejected: add `anvil-kernel` to intercept deps (drags the parser surface into the daemon, reverses `watcher.rs:28`); host in `anvil-kernel-types` (turns the minimal type crate into a logic crate) | Accepted |
| [067](067-daemon-symbol-feed-parse-hook.md) | Resolves ADR-064's "Task 7/8 must nail" wiring detail for DSV-005: the daemon obtains parsed `FileSymbols` through a **dependency-inverted parse hook** (EIP **Content Enricher behind a Messaging Gateway**), not the spec's async watcher feed. `anvil-intercept` defines the `SymbolParser` trait (links no parser); `validate_paths` enriches the change it holds by parsing the **exact** openat2-guarded bytes it hashed (`fed_symbols(path, bytes)`) — no second read, so no B2 stale-symbol race. The tree-sitter `KernelSymbolParser` lives in `anvil-cli` (which deps both kernel + daemon) and is injected via `ForegroundOpts::with_symbol_parser`, so tree-sitter links into the **binary**, never the `anvil-intercept` crate (`daemon_dep_boundary` stays green) — ADR-064 honoured, not revisited. Async watcher feed reframed as a future *advisory cache-warmer*, never the attestation source. Interim path-stable (FNV-1a) symbol-id base; a collision degrades safely to `Partial` (never false `Certified`); GV2 (A′) supersedes it. Rejected: async watcher store as verdict source (B2 race); add a kernel dep to the daemon (reverses ADR-064) | Accepted 2026-06-03 (design council) |
| [068](068-windows-save-time-read-safety.md) | Resolves DSV-010's open Windows read-safety risk: the save-time verdict's Unix guard (`openat2(RESOLVE_NO_SYMLINKS \| RESOLVE_BENEATH)` against a held `O_PATH` dirfd, `path_safety.rs`) has no Windows analog. Mirror it structurally with **`NtCreateFile` anchored at a held workspace directory handle + `OBJ_DONT_REPARSE`** (per-component `FILE_OPEN_REPARSE_POINT` ladder fallback) — preserving C2 (held-handle identity; post-admission retarget can't redirect), no-reparse traversal (symlinks **and** junctions), beneath-root, and B2 (read-then-certify the exact bytes; refuse oversized, never truncate). Lives in `anvil-intercept-win32` (the FFI isolation crate) exposed as a safe `read_under`; `anvil-intercept` stays `forbid(unsafe_code)`. Adds a Windows-hardened structural normaliser (backslash/drive/UNC/device-prefix/ADS/trailing-dot/reserved-name rejection) beyond the Unix `normalise_rel`. Precedent: Go `os.Root` (golang/go#73080). Rejected: Win32-only `CreateFileW` + `GetFinalPathNameByHandle` verify-beneath (no handle-relative open ⇒ loses C2; verify-after-open not deny-during-resolve) — kept only as optional defence-in-depth; a documented weaker Windows guarantee (asymmetric posture + DSV-009 parity divergence); deferring Windows read-safety. Peer-SID auth check is a separate DSV-010 prerequisite, not this ADR | Accepted 2026-06-04 |

## Planning and Process

| ADR | Decision | Status |
|-----|----------|--------|
| [034](034-cross-cutting-modules-as-aps-primitive.md) | Promote cross-cutting modules to a first-class APS primitive in `aps-rules.md`; LAUNCH (first trial) and TRACE (second trial) cite the spec by anchor; `Blocks on:` callout type carried as provisional until exercised through a real close | Accepted |
| [042](042-closeout-enforcement-exit-codes.md) | Closeout-enforcement checks (`adr:check`, `aps:drift`, future `docs:check`) are a named carve-out from ADR-002 — they exit non-zero on violation by design. ADR-002 continues to govern runtime warnings on user code. | Proposed |
| [053](053-advisory-aps-index-counts.md) | Per-module APS index `N/M` counts are advisory-derived from per-item `Status:` lines, not PR-maintained; feature PRs never edit the count, a single-writer periodic reconcile refreshes it, and `aps:index:check` freshness is advisory (scoped exception to ADR-042). Post-merge regen bot recorded as the escalation. | Accepted |

## Agent Infrastructure

| ADR | Decision | Status |
|-----|----------|--------|
| [024](024-internal-agent-harness.md) | Thin agent runtime (weave, Apache-2.0) standalone at eddacraft/weave-rs; anvil-weave harness with zero-copy graph access | Proposed |

## Evaluation and Testing

| ADR | Decision | Status |
|-----|----------|--------|
| [013](013-eval-harness-adoption.md) | External eval harness behind `EvalHarnessPort` adapter boundary | Proposed |

## Edge and Infrastructure

| ADR | Decision | Status |
|-----|----------|--------|
| [032](032-edge-architecture-afd.md) | Azure Front Door as the canonical edge layer for eddacraft.ai; supersedes the implicit "Vercel as both origin and edge" assumption | Accepted |

## Language and Coverage

Decisions supporting the [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md).

| ADR | Decision | Status |
|-----|----------|--------|
| [065](065-rust-t3-architecture-enforcement.md) | Rust T3 architecture enforcement location is Rust-native (`crates/anvil-architecture` + kernel parser edges); no TS shim. Authoritative for layer/boundary, baseline, `architecture-validate` on Rust (and future anchors). TS analyser is legacy surface only. Realises council §16.5 #5 (C-019); unblocks RSTLAN Ready promotion and NBI re-eval completion. | Accepted |
| [027](027-pack-architecture.md) | Per-pack crate, kernel symbol-graph access, compiled-in activation; `crates/anvil-pack-{name}/` registered through `crates/anvil-packs/` | Accepted |
| [028](028-markdown-governance-crate.md) | Markdown governance lives in standalone Rust crate `crates/anvil-markdown-governance/` with `pulldown-cmark` — not the kernel | Accepted (rationale strengthened by ADR-033) |
| [029](029-suppression-parser-authority.md) | Rust suppression parser is authoritative for new surfaces; no new comment styles added to the TS parser | Accepted (amended by ADR-033 — TS parser retired) |
| [030](030-surface-drivers-supersede-napi-cutover.md) | Surface drivers (editor + MCP) on the intercept daemon supersede TSRET-003/-004; TSRET-005 retargeted; napi publication no longer required | Proposed (sequencing amended by ADR-033) |

## Superseded

| ADR | Replaced By | Reason |
|-----|-------------|--------|
| [008](008-ink-vs-ratatui-assessment.md) | ADR-011a, Rust kernel | TUI choice tied to language; Rust migration changed the calculus |
| [009](009-ink-vs-ratatui-watch-mode-performance.md) | ADR-011a, Rust kernel | Confirmed rendering isn't the bottleneck; check execution is |
| [010](010-pulumi-typescript-iac.md) | ADR-007 | Duplicate; ADR-007 is the canonical Pulumi decision |
| [011a](011a-rust-core-engine.md) | Architecture evolution docs | Rust kernel spec and architecture evolution are now authoritative |

### Module-level supersession (in-flight validation thesis)

The two real-time-validation modules predate the drivers-on-daemon architecture
and are superseded — recorded here (not as ADRs) per RTAI-009:

- **RTVS** (`real-time-validation-simplified`) — archived 2026-04-24; written
  against the retired Ink/TS stack.
- **RTVF** (`real-time-validation-full`) — superseded 2026-04-24; its "unified
  validation server" framing predates ADR-030.

Both are superseded by **RTAI**
([`realtime-ai-validation`](../modules/realtime-ai-validation.aps.md)), which is
the realisation of the in-flight validation thesis on the drivers → daemon
architecture (ADR-030): the intercept daemon's `scan_buffer` RPC (RTAI-002)
validates an unsaved buffer against the same INTR rule registry as the
save-time path, the MCP pre-write surface (RTAI-006) ships that semantics to
agents, and every mid-edit decision mirrors onto the INTD-013 notification lane
with a `mirror.path = "midEdit"` discriminator (RTAI-007). There is no separate
validation server.
