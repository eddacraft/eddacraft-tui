# Rust CLI Release Scope — v0.3.x

> Functional review areas for the Rust CLI release. Each slice has a clear
> boundary and can be reviewed/merged independently, respecting dependency
> order.

## Dependency Graph (merge order flows top to bottom)

```
                ┌─────────────────────┐
           1    │  anvil-kernel-types  │   shared enums, no logic
                └─────────┬───────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                  ▼
  ┌──────────┐    ┌─────────────┐    ┌────────────┐
2 │  checks  │  3 │   kernel    │  4 │    tui     │
  └──────────┘    └─────────────┘    └────────────┘
        │                 │                  │
        │    ┌────────────┤                  │
        ▼    ▼            ▼                  ▼
  ┌──────────┐    ┌─────────────┐    ┌────────────┐
2 │  policy  │  2 │architecture │  5 │    cli     │  <- depends on all
  └──────────┘    └─────────────┘    └────────────┘
```

---

## Slice 1 — Foundation Types

**Crate:** `anvil-kernel-types` (~580 LoC) **Risk:** Low — pure type
definitions, no logic **Review focus:** Serialisation stability, enum
exhaustiveness, cross-crate compatibility

| What         | Files           |
| ------------ | --------------- |
| Event types  | `src/events.rs` |
| Graph types  | `src/graph.rs`  |
| Trust levels | `src/trust.rs`  |
| Engine IDs   | `src/lib.rs`    |

**Merge first** — everything else depends on it.

---

## Slice 2 — Independent Engines (3 parallel reviews)

These crates have **zero inter-anvil dependencies** and can be reviewed and
merged in parallel.

### 2a — Quality Checks (`anvil-checks`)

**Surface:** Secret scanning, anti-pattern detection, command safety **Risk:**
Medium — regex patterns, entropy analysis **Review focus:** Pattern accuracy,
false positive rate, scan performance

| Module            | Key entry point              | ~LoC |
| ----------------- | ---------------------------- | ---- |
| `secret/`         | `run_secret_check()`         | 1.5k |
| `antipattern/`    | `run_antipattern_check()`    | 1.2k |
| `command_safety/` | `run_command_safety_check()` | 1k   |

### 2b — Policy Engine (`anvil-policy`)

**Surface:** OPA binary execution, policy loading, bundle management, exceptions
**Risk:** Medium — subprocess execution (OPA), YAML parsing **Review focus:**
OPA integration correctness, exception logic, bundle versioning

| Module          | What                   |
| --------------- | ---------------------- |
| `evaluator.rs`  | Policy evaluation      |
| `opa.rs`        | OPA binary execution   |
| `bundle.rs`     | Policy packaging       |
| `exceptions.rs` | Suppression management |
| `loader.rs`     | Policy file discovery  |

### 2c — Architecture Validation (`anvil-architecture`)

**Surface:** Layer definitions, boundary checking, drift baselines, YAML parsing
**Risk:** Low — deterministic validation logic **Review focus:** Template
schemas, BTreeMap ordering, baseline merge semantics

| Module           | What                                              |
| ---------------- | ------------------------------------------------- |
| `definition.rs`  | Schema + 8 templates (starter through serverless) |
| `validator.rs`   | `validate_with_edges()`                           |
| `baseline.rs`    | Drift tracking (create/load/merge/diff)           |
| `yaml_parser.rs` | `.architecture.yaml` handling                     |
| `types.rs`       | `Layers`, `Boundary`, `BoundaryViolation`         |

---

## Slice 3 — Kernel (Semantic Analysis)

**Crate:** `anvil-kernel` **Depends on:** kernel-types only **Risk:** High —
tree-sitter FFI, incremental graph algorithms, file watching **Review focus:**
Parser correctness, graph consistency under incremental updates, watcher
reliability, rayon parallelisation safety

| Module      | What it does               | Key concern                            |
| ----------- | -------------------------- | -------------------------------------- |
| `parser/`   | tree-sitter AST extraction | Language detection, incremental parse  |
| `watcher/`  | File system monitoring     | Debounce, backpressure, filtering      |
| `graph/`    | Symbol graph (petgraph)    | Incremental updates, trust annotation  |
| `policy/`   | Invariant evaluation       | Cross-layer rules, privilege expansion |
| `protocol/` | Event emission             | Structured output contract             |
| `watch/`    | Orchestration              | Parser to graph to policy loop         |

Recommend a dedicated reviewer with graph algorithm experience.

---

## Slice 4 — TUI Surfaces

**Crate:** `anvil-tui` **Depends on:** kernel-types, eddacraft-tui **Risk:** Low
— presentation only, no data mutation **Review focus:** Surface state machines,
keyboard handling, rendering correctness

| Surface     | What it renders          |
| ----------- | ------------------------ |
| `audit/`    | Violation list           |
| `browser/`  | File/pattern browser     |
| `doctor/`   | Diagnostic checklist     |
| `gate/`     | Gate check results       |
| `init/`     | Init wizard steps        |
| `status/`   | Project health dashboard |
| `tutorial/` | 4-path guided tutorial   |
| `watch/`    | Live watcher dashboard   |
| `welcome/`  | First-run menu           |
| `wizard/`   | APS scaffolding          |

---

## Slice 5 — CLI Commands (4 sub-reviews)

**Crate:** `anvil-cli` (456 KB, 20 command modules) **Depends on:** all crates

### 5a — Auth and Onboarding

Commands that require no authentication or handle authentication itself.

| Command    | File        | ~LoC | What                    |
| ---------- | ----------- | ---- | ----------------------- |
| `welcome`  | welcome.rs  | 8K   | First-run screen        |
| `init`     | init.rs     | 11K  | Project setup wizard    |
| `new`      | new.rs      | 26K  | Template scaffolding    |
| `wizard`   | wizard.rs   | 12K  | APS onboarding          |
| `tutorial` | tutorial.rs | 6K   | Interactive tutorial    |
| `doctor`   | doctor.rs   | 29K  | Environment diagnostics |
| `auth`     | auth.rs     | 6K   | Device-flow + OTP login |

**Plus:** `auth/` module (credential storage, device flow, OTP) **Review
focus:** Auth flow correctness, credential path migration, XDG compliance

### 5b — Core Analysis

The primary value commands — the ones users run most often.

| Command  | File      | ~LoC | What                   |
| -------- | --------- | ---- | ---------------------- |
| `gate`   | gate.rs   | 59K  | 7-check quality gate   |
| `check`  | check.rs  | 30K  | Planless file analysis |
| `audit`  | audit.rs  | 28K  | Full project scan      |
| `status` | status.rs | 27K  | Health dashboard       |

**Review focus:** Check orchestration, exit codes (0/1/2/3/4), workspace_root
caching, plan scoping

### 5c — Architecture and Policy Commands

| Command        | File            | ~LoC | What                    |
| -------------- | --------------- | ---- | ----------------------- |
| `architecture` | architecture.rs | 12K  | Boundary management     |
| `policy`       | policy.rs       | 19K  | Policy lifecycle        |
| `drift`        | drift.rs        | 36K  | Snapshot/compare/report |
| `validate`     | validate.rs     | 32K  | Plan format validation  |
| `gate-config`  | gate_config.rs  | 13K  | Check configuration     |

**Review focus:** Snapshot storage, comparison logic, policy subcommand
completeness

### 5d — Utilities and Real-time

| Command  | File      | ~LoC | What                  |
| -------- | --------- | ---- | --------------------- |
| `watch`  | watch.rs  | 26K  | Live file watcher     |
| `hooks`  | hooks.rs  | 20K  | Git hook management   |
| `export` | export.rs | 64K  | Constraint formatters |
| `admin`  | admin.rs  | 3K   | User approvals        |

**Plus:** `util.rs` (shared `workspace_root()`, `atomic_write()`) **Review
focus:** Watch concurrency (AtomicBool guard), export format correctness, hook
installation safety

---

## Slice 6 — Distribution Pipeline

Infrastructure review, no application code.

| Item                  | What                           | Status               | Reviewer |
| --------------------- | ------------------------------ | -------------------- | -------- |
| `release.yml`         | Cross-platform build + publish | Merged (PR #708)     | CI/CD    |
| `dist-workspace.toml` | cargo-dist (6 targets)         | In repo              | CI/CD    |
| `install.sh`          | Shell installer wrapper        | In repo              | Security |
| Homebrew tap          | `eddacraft/homebrew-tap`       | Needs repo creation  | Infra    |
| DNS                   | `install.eddacraft.ai` CNAME   | Pulumi code ready    | Infra    |
| crates.io             | Workspace dep publishing       | Blocked on dep order | Infra    |

---

## Recommended Merge Sequence

```
Week 1 — Foundations (parallel)
  ├── PR: Slice 1 (kernel-types)
  ├── PR: Slice 2a (checks)
  ├── PR: Slice 2b (policy)
  └── PR: Slice 2c (architecture)

Week 1–2 — Core engines
  ├── PR: Slice 3 (kernel)       — after slice 1
  └── PR: Slice 4 (tui)          — after slice 1

Week 2 — CLI (parallel sub-reviews)
  ├── PR: Slice 5a (auth and onboarding)
  ├── PR: Slice 5b (core analysis)
  ├── PR: Slice 5c (architecture and policy commands)
  └── PR: Slice 5d (utilities and real-time)

Week 2–3 — Distribution
  └── PR: Slice 6 (infra review + tag v0.3.0-beta)
```

**Total:** 10 review units, 6 parallelisable at the start, converging to the CLI
which ties everything together.

---

## Exit Codes

| Code | Meaning       |
| ---- | ------------- |
| 0    | Success       |
| 1    | General error |
| 2    | Gate failure  |
| 3    | Auth required |
| 4    | Config error  |

## Platform Targets

| Target                      | OS      | Arch   |
| --------------------------- | ------- | ------ |
| `x86_64-apple-darwin`       | macOS   | x86_64 |
| `aarch64-apple-darwin`      | macOS   | ARM64  |
| `x86_64-unknown-linux-gnu`  | Linux   | x86_64 |
| `aarch64-unknown-linux-gnu` | Linux   | ARM64  |
| `x86_64-pc-windows-msvc`    | Windows | x86_64 |
| `aarch64-pc-windows-msvc`   | Windows | ARM64  |

## Out of Scope (deferred)

- **RCLI Tier 2 Phase 3–4:** `policy-debug`, `policy-watch`, `pr-comment`,
  `exception` — blocked on OPAE items
- **RCLI Tier 3:** Subsystem commands — proposed, not started
- **Memory commands:** `edda`, `ember`, `stack` — no Rust port yet
- **Agent commands:** `agent list/status/cleanup` — no Rust port
- **26 polish items:** Phase 9–11 council findings — low priority, post-launch
- **crates.io publish:** Requires publishing 5 workspace deps first
