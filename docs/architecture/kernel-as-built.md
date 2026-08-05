# anvil-kernel — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                      |
| -------- | --------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | KERN  | Live   | Last reviewed 2026-07-02 (drift sweep against main `d1fded280`: language registry now 15 grammars, `anvil-graph-cache` module count, snapshot persistence, KERN archived-Complete). Prior: targeted delta 2026-06-10 (GV2-024 hot-read split/seal, ADR-077 depth cap) against `a1c41e284`; full review 2026-05-07 against `v0.6.0-beta` and `crates/anvil-kernel`, `crates/anvil-kernel-types` |

| Upstream                                                    | Downstream                                                                               |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `crates/anvil-kernel`, `crates/anvil-kernel-types`, ADR-030 | engine ports (RENG), TUI / RATS surfaces, watch CLI (LAUNCH), MCP shim embedded fallback |

> **Status:** Live (beta) **Last reviewed:** 2026-07-02 (drift sweep against
> main `d1fded280`: language registry, `anvil-graph-cache` modules, snapshot
> persistence, KERN archived-Complete); prior targeted delta 2026-06-10 (GV2-024
> hot-read split/seal, ADR-077 depth cap) against `a1c41e284`; full review
> 2026-05-07 against `v0.6.0-beta` slate (HEAD `97b61fd0`) **Crate / location:**
> `crates/anvil-kernel` (+ `crates/anvil-kernel-types`) **Module owner (APS):**
> KERN (kernel substrate, **Complete** — archived at
> `plans/archive/modules/rust-kernel.aps.md`; 22/25 items Done, 3 daemon-mode
> items KERN-050..052 superseded by INTD per ADR-030); downstream callers in
> RENG (engine ports), surfaces in RATS / TUI, watch CLI in LAUNCH **Used by:**
> `anvil watch`, `anvil check` / `gate` / `audit` (via embedded API),
> `anvil-checks` registry consumers, MCP shim's `LocalDaemonValidationClient`
> embedded fallback (validation routes through the kernel for parse + graph)

## 1. Overview

`anvil-kernel` is the substrate that watches files, parses with tree-sitter,
builds a semantic graph, evaluates policy invariants, and emits structured
events. It is consumed in two shapes:

- **Foreground watch** (`anvil watch`) — long-lived process; the kernel performs
  an initial scan, then drains debounced filesystem events into incremental
  graph patches and policy re-evaluation.
- **One-shot embedded library** (`anvil check`, `gate`, `audit`, MCP shim
  embedded fallback) — synchronous in-process API that walks the workspace,
  parses, builds the graph, evaluates the four H1 invariants, and returns
  violations + diagnostics.

The kernel does not own CLI argument parsing, TUI rendering, MCP/IPC transport,
or remote distribution. Surfaces talk to it through `EngineEvent` envelopes
(watch) or the `EmbeddedResult` struct (one-shot). The watcher subsystem and
parser are the only modules that touch the filesystem directly.

The kernel is workspace-`#![forbid(unsafe_code)]` (`Cargo.toml:90-91`) — every
`unsafe` block in the dependency closure lives in `nix`, `notify`,
`tree-sitter`, or `petgraph`.

## 2. Reconciliation with `rust-kernel-spec.md`

`docs/architecture/rust-kernel-spec.md` is the H1 design intent record —
"Proposed — H1 Implementation Target". The kernel has shipped through
`v0.4.0-beta` (the original native scanner cut), `v0.5.0-beta` / `v0.5.1-beta`
(incremental import correctness fixes), and is current in `v0.6.0-beta`. The
KERN APS module is **Complete** and archived at
`plans/archive/modules/rust-kernel.aps.md` (22/25 items Done; KERN-050..052
daemon-mode tasks superseded by INTD per ADR-030).

This as-built supersedes the spec for "what shipped". The spec stays as the
intent record. Where the two disagree:

- **Daemon-mode kernel transport (spec §9.3) is not built.** ADR-030 routes the
  daemon-mode IPC surface through `anvil-intercept`, which hosts the kernel
  in-process; the kernel-owned daemon (KERN-050..052) was retired rather than
  shipped.
- **Language coverage (spec §5.2) shipped far past the TS/JS anchor.** The spec
  named Rust as a dogfooding target; the registry now binds **15 grammars** in
  `crates/anvil-kernel/src/parser/languages.rs` — TypeScript, TSX, JavaScript,
  JSX, Rust, Python, Dart, Go, Java, Kotlin, C#, C, C++, Zig, and WebAssembly
  text (`.wat`/`.wast`). TS/JS remain the full-support anchors; the tail
  languages (Dart..C++ under LANGTAIL, Zig/Wat under LTW2 / ADR-093) ship at
  **T1 (Parsed)** — grammar wired, files detected, basic symbol extraction, but
  no per-language anti-pattern catalogues, suppression syntax, or policy hooks
  yet (that is T2/T3). See §6.1.
- **AST graph snapshot to disk (spec §6.4 fast-follow) is not built.** Cold
  rebuild on every start is the only path; the embedded mode does not attempt to
  persist or restore graph state.
- **`Heartbeat` event (spec §8.2 future) is defined nowhere.** The shipped
  `EventPayload` enum is `Progress | Snapshot | Violation | Error` only
  (`crates/anvil-kernel-types/src/events.rs:23-41`).
- **Invariants are Rust-only (spec §7.2).** The spec hinted at a future
  declarative DSL; the OPA/Rego work moved to a separate `anvil-policy` crate
  (`docs/architecture/rust-architecture-overview.md:85`) and is not reachable
  from kernel evaluation.

The spec's H1 invariants list and H1 performance targets remain accurate. See §6
(parser), §8 (policy), §14 (performance posture) for the reconciled state.

## 3. Architecture diagram

```text
                ┌──────────────────────────┐
                │ anvil watch  /  anvil    │
                │ check · gate · audit     │
                │ (CLI dispatchers)        │
                └──────┬─────────────┬─────┘
                       │             │
   include/exclude     │             │     synchronous
   globs, --patterns   │             │     EmbeddedConfig
                       ▼             ▼
        ┌──────────────────────┐   ┌────────────────────┐
        │ kernel::watch        │   │ kernel::embedded   │
        │ (run_watch +         │   │ (run_embedded      │
        │  WatchHandle)        │   │  +_cancellable)    │
        └──────────┬───────────┘   └─────────┬──────────┘
                   │                         │
        ┌──────────▼─────────┐               │
        │ watcher::          │               │
        │  start_watcher     │               │
        │  (notify-rs +      │               │
        │   FileFilter)      │               │
        └──────────┬─────────┘               │
                   │ FileChange / ChangeBatch│
                   ▼                         │
        ┌────────────────────┐               │
        │ watcher::Debouncer │               │
        │ (50 ms window,     │               │
        │  500 max pending)  │               │
        └──────────┬─────────┘               │
                   │                         │
                   ▼                         ▼
              ┌─────────────────────────────────┐
              │ parser (tree-sitter + cache)     │
              │  parse_bytes → ParseResult       │
              │  extract_symbols → FileSymbols   │
              └─────────────┬───────────────────┘
                            │ symbols + ImportEdge[]
                            ▼
              ┌─────────────────────────────────┐
              │ graph::                          │
              │  SymbolGraph (petgraph DiGraph) │
              │  update_file → GraphDelta       │
              │  re_resolve_imports             │
              │  annotate_trust                 │
              │  remove_file                    │
              └─────────────┬───────────────────┘
                            │ GraphDelta
                            ▼
              ┌─────────────────────────────────┐
              │ policy::PolicyEngine             │
              │  CrossLayerViolation             │
              │  NewDependencyIntroduction       │
              │  PublicApiExpansion              │
              │  PrivilegeExpansion              │
              │  (fingerprint dedupe)            │
              └─────────────┬───────────────────┘
                            │ Violation[]
                            ▼
              ┌─────────────────────────────────┐
              │ protocol::EventEmitter           │
              │  Progress · Snapshot · Violation│
              │  · Error                         │
              │  → mpsc<EngineEvent>             │
              └─────────────┬───────────────────┘
                            │
            ┌───────────────┴────────────────┐
            ▼                                ▼
   anvil-tui watch dashboard         anvil-cli plain JSON
   (EngineEvent → render)            stream / EmbeddedResult.events
```

## 4. Crate layout

`crates/anvil-kernel` ships a single library (`anvil_kernel`) and a sister types
crate.

| Module                | Role                                                                                                  |
| --------------------- | ----------------------------------------------------------------------------------------------------- |
| `watcher/`            | KERN-010 / KERN-013 — notify-rs integration, debouncer, internal denylist `FileFilter`, glob filter   |
| `parser/`             | KERN-011 / KERN-012 — tree-sitter parser, AST cache, language registry, symbol/import extraction      |
| `graph/`              | KERN-020..023 — petgraph-backed `SymbolGraph`, dependency graph, trust annotation, incremental update |
| `policy/`             | KERN-030..032 — `ArchitectureConfig` loader, `PolicyEngine`, four H1 invariants                       |
| `protocol/emitter.rs` | KERN-033 — `EventEmitter` over `mpsc<EngineEvent>`                                                    |
| `embedded.rs`         | KERN-040 — `run_embedded` / `run_embedded_cancellable` one-shot library entry                         |
| `watch.rs`            | KERN-041 — `run_watch` foreground watch loop with parallel initial scan and panic isolation           |
| `engine_mode.rs`      | `EngineMode` enum — Rust-only (Legacy / Dual were removed as unimplemented stubs)                     |
| `feature_flags/`      | Re-exports flag resolver/snapshot/telemetry from `anvil-kernel-types::feature_flags`                  |

`crates/anvil-kernel-types` carries the wire vocabulary:

| File               | Role                                                                                     |
| ------------------ | ---------------------------------------------------------------------------------------- |
| `lib.rs`           | Re-exports + `EngineId` (`Rust` / `Legacy`) — the engine discriminator on `EngineEvent`  |
| `events.rs`        | `EngineEvent`, `EventType`, `EventPayload`, `ErrorPayload`, `ErrorCode` — protocol types |
| `graph.rs`         | `SymbolNode`, `SymbolEdge`, `SymbolKind`, `Visibility`, `EdgeType` — graph wire types    |
| `trust.rs`         | `TrustLevel` enum (`Unknown` / `Internal` / `Boundary` / `External` / `Privileged`)      |
| `diagnostics.rs`   | Canonical `anvil.diagnostic.v1` envelope shared with AIGUARD, RTAI, INTD, DRVR           |
| `feature_flags.rs` | Feature-flag manifest, resolver context, evaluation result                               |
| `notifications.rs` | `Notification` envelope used by JSON CLI surfaces                                        |
| `hooks.rs`         | `ANVIL_CONFIG_HOOK_PATTERN` + `is_anvil_managed_command` — hook wiring contract          |

`crates/anvil-kernel/Cargo.toml:12-41` declares the dependency surface:
`anvil-kernel-types`, `anvil-graph-cache` (the graph now lives there, §7),
`anvil-rayon-init` (the shared pool-cap helper — `num_cpus` no longer sits
directly on the kernel), `tree-sitter` plus the 13 grammar crates
(`tree-sitter-typescript` / `-javascript` / `-rust` / `-python` / `-dart` /
`-go` / `-java` / `-kotlin-ng` / `-c-sharp` / `-c` / `-cpp` / `-zig` and the
vendored `anvil-grammar-wat`), `notify`, `rayon`, `globset`, `ignore`,
`walkdir`, `serde` / `serde_json` / `serde_yaml`, `thiserror`. `petgraph` is now
a transitive dep via `anvil-graph-cache`, not a direct kernel dep. There is no
async runtime — the crate is sync-only.

## 5. Watcher

The watcher subsystem owns OS file notifications, the internal denylist, and the
user-facing glob filter. It runs in two parallel layers: a hardcoded internal
denylist (`watcher::filter::FileFilter`) that prunes `node_modules` / `.git` /
`target` and similar at the OS-watch and event-pump layers, and an outer
user-facing glob filter (`watcher::pattern::WatchPatternFilter`) that applies
`--patterns` / `--exclude` from the CLI.

### 5.1 notify-rs integration

`watcher::start_watcher` (`crates/anvil-kernel/src/watcher/mod.rs:169-267`)
constructs a `RecommendedWatcher` and walks the workspace with the `ignore`
crate, registering each non-denied directory with `RecursiveMode::NonRecursive`.
Per-directory registration (rather than recursive at root) is what lets the
watcher skip `node_modules` / `.git` / `target` at the kernel-watch level —
otherwise inotify exhaustion is the default failure mode for any large JS/TS
repo.

Partial failure is surfaced rather than swallowed: `WatchSetupDiagnostics`
(`watcher/mod.rs:56-71`) records `registered`, `failed`, `sample_errors`,
`root_failed`, and `limit_exhausted`. The watch loop emits a recoverable `Error`
event up-front when `failed > 0` with an actionable hint about
`fs.inotify.max_user_watches` (`watch.rs:568-588`). Root-level registration
failure is the one catastrophic path that propagates as `WatcherError::Notify`.

A debounce / event-pump thread (`watcher/mod.rs:204-260`) drains notify's raw
channel into a `Debouncer` that coalesces the burst into a `ChangeBatch`. Newly
created directories are auto-registered against the shared
`Arc<Mutex<RecommendedWatcher>>` — symlinks are stripped via `symlink_metadata`
so a hostile symlink cannot escape the workspace root.

### 5.2 Debouncer

`watcher::debounce::Debouncer` (`crates/anvil-kernel/src/watcher/debounce.rs`)
is a `HashMap<PathBuf, (ChangeKind, Instant)>` keyed by path, so two saves to
the same file in a 50 ms window collapse to one event. The defaults are
`debounce_window: 50 ms`, `max_pending: 500`, `tick_interval: 20 ms`
(`watcher/mod.rs:31-40`); the CLI overrides `debounce_window` from `--debounce`
(default 300 ms in `crates/anvil-cli/src/commands/watch.rs:762`). When
`max_pending` is exceeded the debouncer flushes immediately, applying
backpressure (`debounce.rs:30-38`).

### 5.3 Internal denylist (`FileFilter`)

`watcher::filter::FileFilter`
(`crates/anvil-kernel/src/watcher/filter.rs:11-94`) carries a fixed
component-name denylist (`node_modules`, `.git`, `target`, `dist`, `build`,
`.next`, `.turbo`, `.nx`, `coverage`, `.anvil` — see `IGNORE_DIRS`) plus a
parseable-extension gate. The gate is **not** a hardcoded JS/TS list:
`is_parseable` delegates to `Language::from_path` (`filter.rs:95-104`), so it
admits every extension the parser registers — all 15 languages of §6.1 — and
stays in lockstep with the registry automatically. Two checks compose:

- `should_ignore` matches any path component against the denylist — catches
  absolute, relative, and trailing-slash forms (`filter.rs:83-91`); tests pinned
  at `filter.rs:135-169`.
- `should_process` combines the denylist with the parseable-extension gate. When
  the caller wires user-supplied globs at the outer layer (the `--patterns` /
  `--source` / `--plans` cases), it builds the filter via
  `with_respect_extensions(false)` — the registry-extension gate is bypassed but
  a "must look like a file" floor (extension is `Some(_)`) still rejects
  directories (`filter.rs:105-121`,
  `crates/anvil-cli/src/commands/watch.rs:738-749`).

### 5.4 User-facing glob filter (`WatchPatternFilter`, LAUNCH-001)

`watcher::pattern::WatchPatternFilter`
(`crates/anvil-kernel/src/watcher/pattern.rs`) is the load-bearing LAUNCH-001
surface — wired and consumed in `v0.6.0-beta`. It compiles include + exclude
pattern lists into `globset::GlobSet` instances. The contract:

- Empty include = match everything; empty exclude = exclude nothing. `is_noop()`
  short-circuits the match call when both are empty (`pattern.rs:46-51`).
- Repo-relative paths are required — globs like `src/**/*.ts` are written
  relative to the workspace root.
- Exclude wins over include (`pattern.rs:85-95`).
- Windows backslash separators are normalised to forward-slash before matching
  (`pattern.rs:69-77`).

The wiring lives at three points:

- `WatchConfig.include_patterns` / `exclude_patterns` (`watch.rs:50-57`) carry
  user input from the CLI.
- `run_watch` compiles them into a `WatchPatternFilter` once at the start
  (`watch.rs:540-541`).
- The filter is consumed both during the initial scan (`watch.rs:138`, via
  `pattern_matches`) and inside the steady-state watch loop (`watch.rs:315`).
  Removed events get a narrow exemption: a delete event for a file the graph
  already tracks always flows through so the graph cleans up, even if the path
  no longer matches user globs (`watch.rs:316-335`).

The CLI dispatcher (`crates/anvil-cli/src/commands/watch.rs:760-773`) builds
`WatchConfig` from `--patterns`, `--source`, `--plans`, `--all`, and
`--exclude`. `--all` clears includes (matches everything through the denylist);
the no-flag default is also "everything" — fixed in `v0.4.0-beta` after the
prior default of `DEFAULT_WATCH_PATTERNS` had silently scoped `anvil watch` to
planning docs only (`watch.rs:704-727`).

## 6. Parser (tree-sitter)

`parser::Parser` (`crates/anvil-kernel/src/parser/mod.rs:35-113`) holds a
`HashMap<Language, tree_sitter::Parser>` and an `AstCache`. Per-file parse goes
through `parse_bytes(path, content)`:

1. Resolve language from path extension via `Language::from_path`
   (`parser/languages.rs:14-22`).
2. Compute FNV-1a content hash; check the AST cache; return the cached tree on
   hit (`parser/mod.rs:65-76`).
3. Otherwise parse with the language-specific tree-sitter parser and cache the
   result.

### 6.1 Language coverage in `v0.6.0-beta`

Fifteen language entries register in `parser::languages::Language`
(`parser/languages.rs:5-35`), dispatched to a tree-sitter grammar in
`ts_language` (`parser/languages.rs:66-86`):

| Variant      | Extensions                                                          | tree-sitter grammar            | Tier          |
| ------------ | ------------------------------------------------------------------- | ------------------------------ | ------------- |
| `TypeScript` | `.ts`                                                               | `tree-sitter-typescript` (TS)  | Anchor        |
| `Tsx`        | `.tsx`                                                              | `tree-sitter-typescript` (TSX) | Anchor        |
| `JavaScript` | `.js` / `.mjs` / `.cjs`                                             | `tree-sitter-javascript`       | Anchor        |
| `Jsx`        | `.jsx`                                                              | `tree-sitter-javascript`       | Anchor        |
| `Rust`       | `.rs`                                                               | `tree-sitter-rust`             | RSTLAN        |
| `Python`     | `.py` / `.pyi`                                                      | `tree-sitter-python`           | Parsed        |
| `Dart`       | `.dart`                                                             | `tree-sitter-dart`             | T1 (LANGTAIL) |
| `Go`         | `.go`                                                               | `tree-sitter-go`               | T1 (LANGTAIL) |
| `Java`       | `.java`                                                             | `tree-sitter-java`             | T1 (LANGTAIL) |
| `Kotlin`     | `.kt` / `.kts`                                                      | `tree-sitter-kotlin-ng`        | T1 (LANGTAIL) |
| `CSharp`     | `.cs`                                                               | `tree-sitter-c-sharp`          | T1 (LANGTAIL) |
| `C`          | `.c` / `.h`                                                         | `tree-sitter-c`                | T1 (LANGTAIL) |
| `Cpp`        | `.cpp` / `.cc` / `.cxx` / `.c++` / `.hpp` / `.hh` / `.hxx` / `.h++` | `tree-sitter-cpp`              | T1 (LANGTAIL) |
| `Zig`        | `.zig`                                                              | `tree-sitter-zig`              | T1 (LTW2)     |
| `Wat`        | `.wat` / `.wast`                                                    | `anvil-grammar-wat` (vendored) | T1 (LTW2)     |

TS/JS are the full-support anchors. Rust and Python have dedicated extractors
(`parser/extract/rust.rs`, `parser/extract/python.rs`); Rust symbol extraction
is owned by RSTLAN. The remaining tail languages ship at **T1 (Parsed)**: the
grammar binds, files are detected via `Language::from_path`
(`parser/languages.rs:39-63`), and basic symbol extraction runs, but no
per-language anti-pattern catalogues, suppression syntax, or policy hooks exist
yet (T2/T3). Two deliberate non-mappings: `.zon` (`build.zig.zon`) is a Zig data
manifest, not source, and the binary `.wasm` format is not source (ADR-093) — a
grammar-version fingerprint (`grammar_version`, `languages.rs:102-123`) folds
into the AST-cache key so a grammar bump can never serve a stale tree.

### 6.2 AST cache

`parser::cache::AstCache` (`parser/cache.rs`) is a
`HashMap<PathBuf, CacheEntry>` keyed by absolute path; entries store the FNV-1a
content hash plus the parsed tree. Cache hit on identical content; miss on any
byte-change (`parser/cache.rs:30-41`). The cache is single-process — there's no
on-disk warm-start (spec §6.4 fast-follow not built).

### 6.3 Symbol extraction

`parser::extract::extract_symbols`
(`crates/anvil-kernel/src/parser/extract/mod.rs:64-94`) walks the tree-sitter
AST and produces `FileSymbols { file, symbols, imports }`. The extraction
adapter handles function declarations, class declarations, exported variables,
ES module imports, CommonJS `require`, and re-export forms. The id allocator is
**0-based per file** with an `id_offset` parameter — both `watch.rs` and
`embedded.rs` rebase per-file ids onto a global allocator after the parallel
parse phase (see §9.3 + §10).

### 6.4 Parser-error reporting

Parse errors do not abort the run. Both watch and embedded paths catch
`ParseError` at the call site and emit a structured `EventType::Error` with
`ErrorCode::ParseError`, the offending file, and `recoverable: true`
(`watch.rs:208-216`, `embedded.rs:240-247`). Surfaces continue processing other
files. This is the council finding pinned in
`embedded.rs::tests::parse_errors_surface_as_events_not_silent_drops`
(`embedded.rs:482-516`).

### 6.5 Performance budget

Per-file tree-sitter parse benches **< 1 ms** in
`crates/anvil-kernel/benches/kernel.rs`; the symbol-extraction adapter adds a
microsecond-scale tail. See §14 and
`docs/architecture/kernel-benchmarking-spec.md` for the full envelope.

## 7. Semantic graph (KERN-020..023)

The semantic graph is its own crate, `anvil-graph-cache`, re-exported by the
kernel as `crate::graph` (`crates/anvil-kernel/src/lib.rs:8`). The crate ships
**ten modules** (`crates/anvil-graph-cache/src/lib.rs:10-19`):

- `symbol_graph` — the petgraph-backed `SymbolGraph` (§7.1).
- `dependency` — derived file-level `DependencyGraph` with cycle detection
  (§7.2).
- `trust` — the whole-graph trust-annotation pass (§7.3).
- `incremental` — the `update_file` hot path plus import/call/re-export
  resolution (§7.4).
- `certify` — bounded reverse-impact certifiability for the save-time daemon
  (ADR-061 / ADR-064, depth-capped under ADR-077 — §7.6).
- `hot_index` — the sealed save-time hot-read surface (GV2-022 / ADR-063 —
  §7.5).
- `call_graph` — reverse caller lookup (`callers_of`, `CallersReport`,
  `MAX_CALLERS_WALK`), the call-edge analogue of the dependency graph.
- `registry` — `GraphRegistry`, the multi-graph registry the daemon uses to hold
  several workspaces' warm graphs.
- `snapshot` — sealed allowlist-only snapshot DTO + `postcard` codec for warm
  graph persistence / warm-restart (GV2-030 / ADR-069 — §7.7).
- `tokens` — the GCTX token estimator (`estimate_gctx_tokens`,
  `GCTX_TOKEN_ESTIMATOR_VERSION`) used by the GCTX egress surface.

Its `lib.rs` re-exports the graph core (`SymbolGraph`, `DependencyGraph`,
`GraphDelta`, `update_file`, `re_resolve_imports`, `re_resolve_calls`,
`re_resolve_reexports`, `remove_file`, `annotate_trust`), the certifiability
surface (`certify`, `CertifyStale`, `Certifiability`, `export_surface_diff`),
the trust contract (`TrustGraph`, `TrustPostureChange`, `policy_profiles`), the
hot-read surface (`HotReadApi`, `BackgroundReadApi`, `HotPathSurface`,
`HotRead`, `HotReadMiss`, `MAX_REVERSE_IMPACT_DEPTH`), the call graph
(`callers_of`, `CallersReport`), the registry (`GraphRegistry`), the snapshot
codec (`SnapshotPayload`, `snapshot_filename`, `persist_graph_enabled`), and the
token estimator (`estimate_gctx_tokens`, `TokenEstimate`)
(`crates/anvil-graph-cache/src/lib.rs:21-45`). The kernel's own watch / embedded
paths use only the graph core, trust, and incremental surfaces; `certify`,
`hot_index`, `call_graph`, `registry`, `snapshot`, and `tokens` are the
daemon-facing (INTD / GV2 / GCTX) surfaces.

### 7.1 `symbol_graph.rs` — the symbol graph

`SymbolGraph` (`crates/anvil-graph-cache/src/symbol_graph.rs:22-33`) wraps a
`petgraph::DiGraph<SymbolNode, SymbolEdge>` plus three indexes:

- `index: HashMap<u64, NodeIndex>` — id → petgraph slot, O(1) lookup.
- `files: HashMap<String, Vec<u64>>` — file → contained symbol ids; drives
  `symbols_in_file` and `remove_file` in O(file size).
- `next_id: u64` — monotonic high-water mark; never decremented on removal (ids
  must stay unique across the lifetime of the graph — `symbol_graph.rs:26-32`).

`remove_file` is non-trivial: petgraph's `remove_node` swap-moves the last node
into the freed slot, so node-index validity has to be maintained explicitly. The
implementation collects all `(id, NodeIndex)` pairs, sorts by descending raw
index, and removes high-to-low so each swap-moved node has already been
processed (`symbol_graph.rs:100-126`). Test pinned at
`remove_file_with_interleaved_indices_preserves_other_files`
(`symbol_graph.rs:282-323`) — a regression that would silently corrupt the graph
for any repo where files alternate insert order.

Concurrency model: synchronous, single-writer. The watch loop holds
`SymbolGraph` directly on its dedicated thread; embedded mode constructs a fresh
`SymbolGraph` per call. `petgraph::DiGraph` is not `Sync` / `Send`-friendly
enough to share across threads, and the parallel parse path (§9.3, §10)
deliberately keeps graph mutation serial.

### 7.2 `dependency.rs` — derived module-level dependency graph

`DependencyGraph` (`crates/anvil-graph-cache/src/dependency.rs:8-13`) is a
file-to-file projection of the symbol graph's import edges.
`HashMap<String, HashSet<String>>` for forward edges, `reverse` for
who-imports-me lookup. Provides `find_cycle` via DFS (`dependency.rs:86-131`)
for cycle detection. It is not yet wired into a kernel-emitted policy event in
this release — `find_cycle` is a public API consumed elsewhere; the policy
engine's cross-layer invariant uses the symbol-graph edges directly.

### 7.3 `trust.rs` — trust annotation pass

`graph::annotate_trust` (`crates/anvil-graph-cache/src/trust.rs:29-83`) is a
whole-graph pass that computes `TrustLevel` for every node from imports plus
visibility:

- A file that imports a `node:` privileged module (`fs`, `child_process`, `net`,
  `http`, `https`, `crypto`) gets `Privileged` (`trust.rs:8, 19-24`).
- A file that imports any non-relative external module
  (`!source.starts_with('.') && !source.starts_with('/')`) gets `External` if
  not Privileged.
- A symbol with `Visibility::Public` gets `Boundary` if not Privileged /
  External.
- Everything else gets `Internal`.

Two correctness pins:

- **Synthetic external module nodes preserve `External` trust.** The resolver
  creates synthetic `SymbolKind::Module` placeholders for bare imports (`axios`,
  `node:fs`, etc., §7.4). `annotate_trust` skips these so the
  `NewDependencyIntroduction` invariant continues to fire (`trust.rs:51-61`,
  test `external_trust_preserved_for_synthetic_module_nodes`,
  `trust.rs:211-237`).
- **Module-name match is exact-token, not substring.** `fsevents` is External,
  not Privileged; `http-errors` is External, not Privileged (`trust.rs:20-23`,
  tests `:151-189`). The `node:fs/promises` subpath form is correctly Privileged
  (`:191-209`).

### 7.4 `incremental.rs` — incremental refresh path

`update_file` (`crates/anvil-graph-cache/src/incremental.rs:75-200`) is the hot
path:

1. **Capture pre-state for delta context.** Before removing the file, collect
   `previously_imported` (file paths) plus the `previously_public`,
   `previously_privileged`, and `previously_boundary` identity sets — keyed by
   `SymbolIdentity::for_file_symbols` so same-`(kind, name)` overloads stay
   distinct (GV2-002) — so the new-dependency, public-API, and
   privilege-expansion invariants can distinguish "newly introduced" from
   "re-added after edit" (`incremental.rs:78-114`).
2. **Remove the old file's symbols and their edges.**
3. **Insert new symbols.**
4. **Resolve imports** via `resolve_import` (`incremental.rs:208-293`):
   - Bare imports (`axios`, `node:fs`) — match an existing module node by name;
     otherwise create a synthetic `SymbolKind::Module` node with
     `TrustLevel::External` and `Visibility::Public`. The id is drawn from
     `graph.next_id()` so it cannot collide with a per-file allocator.
   - Relative imports (`./utils`) — normalise `.` / `..` components without
     filesystem access, then try seven extension candidates (`""`, `.ts`,
     `.tsx`, `.js`, `.jsx`, `/index.ts`, `/index.js`). Ambiguous matches are
     resolved deterministically: shortest known-file path wins
     (`incremental.rs:237-290`).
5. **Pick a source node for import edges.** First added symbol if any; otherwise
   create a synthetic Module node for the file (so a side-effect-only module
   like `polyfill.ts` still records its imports as graph edges —
   `incremental.rs:147-167`).
6. **Return a `GraphDelta`** carrying added/removed symbol ids, added/removed
   edges, errors, plus the four "previously" sets and the file path.

`re_resolve_imports` (`incremental.rs:297-331`) runs after a batch of
`update_file` calls to fix up edges that couldn't resolve when the target file
hadn't yet been parsed (file ordering during the initial scan). Idempotent:
skips edges that already exist.

#### `0.5.1-beta` import-id correctness fix

Two bugs were pinned and fixed in `0.5.1-beta` (CHANGELOG entry at line 159):

- **Synthetic-import id collision with the file allocator.** Earlier versions
  allocated synthetic external/module ids by scanning
  `graph.node_weights().map(|s| s.id).max() + 1`, independent of whatever
  per-file allocator the caller (`watch.rs`, `embedded.rs`) was using. When the
  caller's `state.next_id` then bumped past the symbol it just added, it could
  land on a slot the graph already owned via a synthetic external node, and
  every subsequent `add_symbol` returned `DuplicateSymbol`. Fix: ids flow
  through `SymbolGraph::next_id()`; callers take
  `(base + count).max(graph.next_id())` after each `update_file` /
  `re_resolve_imports` to stay ahead of synthetic allocations. Regression test
  pinned at `external_synthetic_does_not_collide_with_next_files_base_id`
  (`incremental.rs:752-846`).
- **Symbol id `0` treated as the "no source" sentinel.** `update_file` used
  `from_id == 0` as a sentinel for "no usable source node", which silently
  dropped every import edge for the very first file in a fresh watch session
  (whose first symbol takes id 0). Fix: the sentinel is `Option<u64>`, so id 0
  is a valid source. Test pinned at
  `id_zero_first_symbol_still_emits_import_edges` (`incremental.rs:848-893`).

Both fixes ride in every release after `0.5.1-beta`, including `v0.6.0-beta`.

### 7.5 `hot_index.rs` — sealed save-time hot-read surface (GV2-022 / ADR-063)

`HotReadApi` (`crates/anvil-graph-cache/src/hot_index.rs:153-156`) holds shared
references to the warm `(SymbolGraph, DependencyGraph)` pair and exposes only
the four ADR-063 hot-path-admissible reads (`hot_index.rs:16-26`): resident
per-file symbol lookup, known-edge existence, bounded reverse impact, and
precomputed boundary/trust membership. Every read returns
`HotRead<T> = Warm(T) | Stale(HotReadMiss)`, `#[must_use]`
(`hot_index.rs:112-119`). `HotReadMiss` — `WarmStateEvicted` /
`CrossFileResolutionNeeded` / `ImpactSetOverflow` (`hot_index.rs:86-105`) — is
graph-cache-local; the daemon maps it to the wire `StaleReason` at the boundary
(ADR-064 §2). On a miss the caller must degrade to its daemon-absent fallback —
never escalate to an on-hot-path parse, resolve, traversal, or I/O.

The seal (GV2-024): denylist reads live on `BackgroundReadApi`
(`hot_index.rs:353-355`), which hosts `impact_closure_unbounded`
(`hot_index.rs:368-370`); `HotPathSurface` is a sealed marker trait implemented
only by `HotReadApi` (`hot_index.rs:331-332`); two `compile_fail` doctests pin
the seal (`hot_index.rs:316-330`).

### 7.6 `certify.rs` — save-time certifiability, depth-capped (ADR-077)

`certify::certify` (`crates/anvil-graph-cache/src/certify.rs:565`, ADR-061 /
ADR-064) sizes its surface-change stale reason via
`bounded_impact_closure(dep, &delta.file, MAX_REVERSE_IMPACT_DEPTH, budget)`
(`certify.rs:617`); `MAX_REVERSE_IMPACT_DEPTH = 2` (`hot_index.rs:80`).
`bounded_impact_closure` truncates at the cap, with a `debug_assert` depth guard
(`certify.rs:463-495`). The closure only sizes the stale reason
(`ImpactSetOverflow` vs `ExportSurfaceChange`) and is never inline-validated, so
capping is verdict- and soundness-preserving; the only observable effect is that
some >2-hop over-budget graphs report `ExportSurfaceChange` (wire
`cross-file-resolution-needed`) instead of `impact-set-overflow`. The
pre-ADR-077 unbounded walk survives only as `impact_closure_unbounded`
(`certify.rs:503`), reachable solely through `BackgroundReadApi` — background
pool only (ADR-077, `plans/decisions/077-cert-closure-depth-cap.md`).

### 7.7 `snapshot.rs` — warm-graph persistence (GV2-030 / ADR-069)

`snapshot::SnapshotPayload` (`crates/anvil-graph-cache/src/snapshot.rs`) is a
sealed, allowlist-only DTO plus a `postcard` codec that persists the warm
`(SymbolGraph, DependencyGraph)` state so the **daemon** can warm-restart
instead of cold-rebuilding. The on-disk artefact carries _structural identity
only, never source text_ (ADR-069 §8): graph rows are projected field-by-field
from the live types, so adding a `String`/`Vec<u8>` field to `SymbolNode` /
`SymbolEdge` fails to compile here until someone decides whether it belongs in a
snapshot — there is no `serde(flatten)`, no `serde(other)` catch-all, and no
`GraphDelta` field. `to_bytes` frames the postcard body behind a fixed header
(magic, `format_version`, `backing_schema_version`, node/edge counts, CRC-32);
`from_bytes` validates magic → versions → checksum → decode → counts and returns
a typed `SnapshotLoadError` on any mismatch (the daemon discards and
cold-rebuilds — never accepts a partially-validated index). CRC-32 is a
corruption check, not an authenticity guarantee (CIB-092 / N6); the control is
the machine-local, owner-only, same-uid persistence boundary — snapshots are
**default-on** (graduated by GBASE-010 / ADR-105 §11; opt out with
`ANVIL_PERSIST_GRAPH=0`), `0600`, under a `0700` state dir
(`persist_graph_enabled`, `MAX_SNAPSHOT_BYTES`). This surface is daemon-only;
the kernel's own watch / embedded paths do not read or write it (see §2, G-05).

## 8. Policy engine (KERN-030..032)

The shipping policy engine is **in-process Rust invariants only**. Hybrid policy
framing (in-process for hot path + OPA/Rego for declarative policy) is
referenced in public docs but the OPA/Rego side lives in a separate
`anvil-policy` crate (`docs/architecture/rust-architecture-overview.md:85`;
historical end-state notes lived in
`docs/archive/architecture/rust-architecture-endstate.md`); the kernel does not
call into it. Inside the kernel, four invariants are registered.

### 8.1 `config.rs` — architecture config loader

`policy::config::ArchitectureConfig`
(`crates/anvil-kernel/src/policy/config.rs:3-39`) deserialises
`.anvil/architecture.yaml` (loaded in `watch.rs:528-537` and
`embedded.rs:75-84`). The shape is `layers: [{ name, paths, allowed_imports }]`.
`layer_for_file` matches a path against the layer's path patterns;
`is_import_allowed` checks the `allowed_imports` whitelist.

Pattern matching is deliberately simple (`config.rs:44-54`): trailing `*` =
prefix match, trailing `/` = directory prefix match, otherwise exact match. Path
separators are normalised to `/`. Cross-platform (Windows `\\` paths match
`src/domain/*` after normalisation — `config.rs:139-143`).

If `architecture_config` is `None`, `layers` is empty and the cross-layer
invariant becomes a no-op (`watch.rs:536`).

### 8.2 `engine.rs` — `PolicyEngine`

`policy::engine::PolicyEngine`
(`crates/anvil-kernel/src/policy/engine.rs:40-91`) is a registry of
`Box<dyn Invariant>` plus a `HashSet<ViolationFingerprint>` for deduplication.
Each invariant implements:

```rust
trait Invariant: Send {
    fn id(&self) -> &'static str;
    fn evaluate(
        &self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        config: &ArchitectureConfig,
    ) -> Vec<Violation>;
}
```

`evaluate` runs each registered invariant in order; violations are fingerprinted
by `(policy_id, file, symbol)` and the dedupe set suppresses repeats within the
engine's session (`engine.rs:57-80`). The watch loop calls `clear_seen()`
between cycles so a reintroduced violation re-emits each save (`watch.rs:454`);
the embedded path keeps the dedupe set for the duration of a single run.

### 8.3 `invariants/` — the four H1 invariants

Registered in both watch and embedded paths (`watch.rs:96-99`,
`embedded.rs:124-128`):

| Invariant                     | File                                             | Triggers when                                                                                                                      | Severity |
| ----------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | -------- |
| `cross-layer-violation`       | `policy/invariants/cross_layer.rs:10-61`         | A symbol added to a file in layer A has an `Imports` edge to a file in layer B, where B is not in A's `allowed_imports`.           | High     |
| `new-dependency-introduction` | `policy/invariants/new_dependency.rs:10-59`      | An added `Imports` edge points to a symbol with `TrustLevel::External`, AND the target file is not in `delta.previously_imported`. | Medium   |
| `public-api-expansion`        | `policy/invariants/public_api.rs:10-43`          | An added symbol has `Visibility::Public`, AND its name is not in `delta.previously_public`.                                        | Low      |
| `privilege-expansion`         | `policy/invariants/privilege_expansion.rs:10-47` | An added symbol has `TrustLevel::Privileged`, AND its name is not in `delta.previously_privileged`.                                | Critical |

All four operate on the `GraphDelta` plus a read-only view of the graph — no
full-graph rescans on the hot path.

### 8.4 Performance budget

Policy evaluation is sub-millisecond in the kernel benches: the "all four H1
invariants on one delta" benchmark records **~799 ns**
(`docs/architecture/rust-architecture-overview.md:184`). Incremental update
budget is ~10 µs per save (`overview.md:183`). See §14.

## 9. Event protocol (KERN-033)

`protocol::EventEmitter` (`crates/anvil-kernel/src/protocol/emitter.rs:9-83`) is
the only producer of `EngineEvent` in the kernel. It wraps an
`mpsc::Sender<EngineEvent>` plus an `AtomicU64` sequence counter.

### 9.1 `EngineEvent` envelope

Defined in `crates/anvil-kernel-types/src/events.rs:14-20`:

```rust
struct EngineEvent {
    event_type: EventType,    // Progress | Snapshot | Violation | Error
    seq: u64,                 // monotonic per emitter instance
    timestamp: String,        // ISO 8601, second precision, UTC
    engine: EngineId,         // Rust | Legacy
    payload: EventPayload,
}
```

JSON envelope shape (per the serde derives on
`crates/anvil-kernel-types/src/events.rs`):

```json
{
  "event_type": "Snapshot",
  "seq": 42,
  "timestamp": "2026-05-07T14:23:09Z",
  "engine": "Rust",
  "payload": {
    "Snapshot": { "node_count": 314, "edge_count": 521, "files_watched": 87 }
  }
}
```

`EventPayload` is one of four shapes (`events.rs:23-41`):

- `Progress { phase, current, total }` — emitted during the embedded scan and
  once per file during initial-scan parsing.
- `Snapshot { node_count, edge_count, files_watched }` — emitted after initial
  scan and after each successful file change.
- `Violation { policy_id, file, symbol, message }` — emitted per violation
  surviving dedupe.
- `Error(ErrorPayload { code, file, message, recoverable })` — emitted for parse
  failures, walk errors, IO failures, and (in watch mode) per-event panics.

`seq` increments monotonically per emitter instance (`emitter.rs:71-82`) so
consumers can detect dropped events. `EngineId::Rust` is the only value the
kernel emits; `Legacy` is reserved for the legacy TS engine, which has been
retired in this release.

### 9.2 Subscription / fan-out

Subscription is a single mpsc receiver — there is no fan-out, back-pressure, or
filtering inside the kernel. The CLI dispatcher
(`crates/anvil-cli/src/commands/watch.rs:775`) creates the channel, hands the
sender to `run_watch`, and pumps the receiver into either the TUI dashboard
(`crate::tui::run_watch`, `commands/watch.rs:867`) or a plain JSON stream
(`print_event_plain`, `commands/watch.rs:883`). The embedded path collects
events into `EmbeddedResult.events` for the caller to render.

### 9.3 Timestamp implementation

The emitter implements its own ISO 8601 formatter over `SystemTime`
(`emitter.rs:85-122`) — second precision, UTC. No `chrono` / `time` crate
dependency. Test-pinned format checks at `emitter.rs:264-284`.

## 10. Embedded API (KERN-040)

`embedded::run_embedded` / `embedded::run_embedded_cancellable`
(`crates/anvil-kernel/src/embedded.rs:56-152`) is the one-shot synchronous
library entry consumed by `anvil check`, `gate`, `audit`, and the MCP shim's
embedded-fallback validation client.

### 10.1 Input / output

`EmbeddedConfig` (`embedded.rs:40-47`):

```rust
pub struct EmbeddedConfig {
    pub root: PathBuf,
    pub architecture_config: Option<PathBuf>,
    pub filter: Option<FileFilter>,
    pub plan: Option<PathBuf>, // reserved; not yet consumed
}
```

`EmbeddedResult` (`embedded.rs:49-54`):

```rust
pub struct EmbeddedResult {
    pub violations: Vec<Violation>,
    pub stats: GraphStats,
    pub events: Vec<EngineEvent>, // collected from the internal emitter
    pub duration: Duration,
}
```

Errors flow through `EmbeddedError` (`embedded.rs:25-38`): `RootNotFound`,
`ConfigIo { path, source }`, `ConfigParse`, `Walk`.

### 10.2 Lifecycle

1. **Validate root** — early return with `RootNotFound` if missing
   (`embedded.rs:69-71`).
2. **Resolve filter** — caller-supplied `FileFilter` or default denylist +
   parseable-extension allowlist (`embedded.rs:73`).
3. **Load architecture config** — read + parse YAML, or empty layers
   (`embedded.rs:75-84`).
4. **Walk + collect parseable files** via `walkdir` with the filter's
   `should_ignore` pruning directories (`embedded.rs:268-286`); files are sorted
   for deterministic ordering.
5. **Emit `Progress { phase: "scanning" }`** with the file count
   (`embedded.rs:92`).
6. **Initialise the rayon pool** — one-time global init capped at
   `max(1, cpus / 2)` (`embedded.rs:107-113`).
7. **Parse + build graph** — see §10.3.
8. **Annotate trust + re-resolve imports.**
9. **Emit `Snapshot`** with the parsed-file count (not attempted — files that
   failed to parse don't show as covered).
10. **Evaluate policy** — register the four H1 invariants, walk each parsed
    file, build a per-file `GraphDelta`, evaluate, emit `Violation` events
    (`embedded.rs:124-137`).
11. **Drop the emitter, drain the channel, return `EmbeddedResult`.**

Cancellation: `run_embedded_cancellable(stop: Arc<AtomicBool>)`
(`embedded.rs:62-65`) is checked at the start of each rayon parse closure
(`embedded.rs:215-217`) and again before policy evaluation — the scan exits at
the next safe checkpoint without partial graph state.

### 10.3 Parallel parse + sequential apply

`parse_and_build_graph` (`embedded.rs:204-266`) is the same shape as the watch
initial scan:

- **Phase 1 — parallel parse via rayon.** Each closure reads a file, parses,
  runs `extract_symbols(.., id_offset = 0)`. Returns
  `Result<(rel_path, FileSymbols), (rel_path, error)>`. Errors are surfaced via
  `EventType::Error` after the parallel phase, never swallowed
  (`embedded.rs:240-247`).
- **Phase 2 — sequential apply.** Walks the results, rebases each file's 0-based
  ids by `base = next_id` (`embedded.rs:251-254`), calls `update_file`, then
  advances `next_id = graph.next_id()` to cover synthetic-allocator drift
  (`embedded.rs:259`). This is the invariant pinned by the `0.5.1-beta`
  collision fix (§7.4).
- **`re_resolve_imports` + `annotate_trust`** run once after the apply loop
  (`embedded.rs:262-263`).

The pool init is process-global (`POOL_INIT: std::sync::Once`,
`embedded.rs:23`). The same `POOL_INIT` is reused by `watch.rs:25`, so a process
that runs `anvil watch` first and then triggers an embedded scan reuses the
watch's pool rather than racing on re-init.

### 10.4 Diagnostic-envelope parity with daemon-backed validation

The MCP shim's `LocalDaemonValidationClient` routes between daemon-backed
`scan_buffer` and embedded fallback based on `DaemonValidationOutcome` (see
`docs/architecture/intercept-as-built.md` §12). The embedded path's
`anvil.diagnostic.v1` shape is byte-identical to the daemon-backed path on the
same fixture; the parity test lives on the intercept side
(`crates/anvil-cli/src/mcp/validation.rs`). Changing kernel-emitted diagnostics
without updating the daemon path breaks that test.

## 11. Watch loop (KERN-041)

`watch::run_watch` (`crates/anvil-kernel/src/watch.rs:520-615`) is the
foreground watch entry point. It returns a `WatchHandle` that owns the
`Arc<AtomicBool>` shutdown flag, a thread-join handle, and the `WatcherHandle`
keeping notify-rs alive (`watch.rs:60-74`). Save-time daemon-vs-embedded routing
for `anvil watch` (`ANVIL_WATCH_DAEMON`, DSV-021) is not part of this loop — it
lives in the anvil-cli layer
(`crates/anvil-cli/src/commands/watch_save_time.rs:105-114`); see
`docs/architecture/intercept-as-built.md`.

### 11.1 Startup

1. **Validate root + load architecture config** (`watch.rs:524-537`).
2. **Build filters.** Internal `FileFilter` from `WatcherConfig.filter` (default
   denylist), user-facing `WatchPatternFilter` from `include_patterns` /
   `exclude_patterns` (`watch.rs:539-541`).
3. **Canonicalise the watch root once.** Both notify-rs registration and
   downstream path comparisons use the same canonical form, avoiding the macOS
   `/tmp` vs `/private/tmp` symlink mismatch that otherwise forces per-event
   canonicalisation (`watch.rs:543-555`).
4. **Start watcher** via `start_watcher` — returns a handle, a
   `mpsc::Receiver<ChangeBatch>`, and `WatchSetupDiagnostics` (`watch.rs:556`).
5. **Spawn the watch thread** with the emitter, `WatchState`, and the shutdown
   flag (`watch.rs:561`).
6. **Surface partial-watch failure up-front** — emit a single recoverable Error
   event with the actionable inotify-limit hint if
   `setup_diagnostics.failed > 0` (`watch.rs:567-588`).

### 11.2 Initial scan

`initial_scan` (`watch.rs:113-243`) runs once before the steady-state loop:

1. **Walk** the workspace via `ignore::WalkBuilder` with denylist pruning and
   the user's pattern filter (`watch.rs:127-164`). Walk errors surface as
   recoverable Error events with the offending path (`watch.rs:151-160`).
2. **Initialise the rayon pool** capped at `max(1, cpus / 2)`
   (`watch.rs:175-181`). The cap is deliberate — saturating cores here would
   starve the VS Code extension host or the editor's tsserver. The same cap is
   applied in `embedded.rs:107-113`.
3. **Parse in parallel** (`watch.rs:183-200`). Each closure checks the cancel
   flag at the start, reads the file, parses, runs `extract_symbols(.., 0)`.
   Errors are returned as `Err((path, message))` for surfacing in the apply
   phase. `extract_symbols`'s 0-based id allocator is the load-bearing invariant
   the apply phase rebases against.
4. **Apply sequentially** (`watch.rs:204-236`). For each successful parse,
   rebase ids by `base_id = state.next_id`, call `update_file`, then bump
   `state.next_id = (base_id + symbol_count).max(graph.next_id())` so synthetic
   external nodes never collide with the next file's base id (the §7.4 fix).
5. **`re_resolve_imports` + `annotate_trust`** once after the apply loop
   (`watch.rs:238-239`).
6. **Run baseline policy evaluation** (`evaluate_baseline`, `watch.rs:246-284`)
   — synthesise a `GraphDelta` per file from its current symbols + outgoing
   edges and run the engine, so the first snapshot reflects real invariant
   results rather than an empty `0/0 checks` placeholder.
7. **Emit `Snapshot`.**

### 11.3 Steady-state loop

`watch_loop` (`watch.rs:286-355`):

1. Block on `batch_rx.recv_timeout(100ms)`. Timeout = continue; disconnect =
   exit.
2. For each `FileChange` in the batch:
   - Run the user pattern filter via `pattern_matches` (`watch.rs:315`). Removed
     events for graph-tracked files get the narrow exemption (§5.4) — a delete
     must always flow through so the graph cleans up.
   - **Panic isolation** — wrap the per-change work in
     `catch_unwind(AssertUnwindSafe(|| process_change(...)))`. A panic in parse,
     extract, or evaluate surfaces as an Error event with the file path; the
     loop keeps draining (`watch.rs:341-352`). `panic_message` extracts the
     payload (`&'static str`, `String`, or fallback "unknown panic" —
     `watch.rs:510-518`). Tests pin all three branches at `watch.rs:673-707`.

`process_change` (`watch.rs:357-472`) handles:

- **`Removed`** — `graph::remove_file`, drop the `tracked_files` entry (always —
  even if no symbols were removed, to handle the rename-then-recreate pattern
  editors use), retain imports for surviving files, re-annotate trust, emit
  Snapshot.
- **`Created` / `Modified`** — read the file (rename-style modify where the path
  is gone is treated as Removed and falls through to cleanup), parse, extract,
  `update_file`, re-resolve imports, re-annotate trust, **clear policy dedupe
  state** (`engine.clear_seen()`, `watch.rs:454`) so reintroduced violations
  re-emit each cycle, evaluate, emit violations, emit Snapshot. The id allocator
  is rebased after `update_file` AND `re_resolve_imports` (the latter can add
  synthetic externals too — `watch.rs:449`).

### 11.4 Shutdown

`WatchHandle::stop` (`watch.rs:67-73`) sets the atomic shutdown flag; the watch
thread exits at the next 100 ms tick boundary or mid-parse (rayon closures check
the flag at the start). The `WatcherHandle` is dropped, releasing the notify-rs
registrations.

## 12. CLI dispatcher (`commands/watch.rs`)

The kernel's CLI entry point lives in `crates/anvil-cli/src/commands/watch.rs`.
The dispatcher is the seam where user intent (`--patterns`, `--exclude`,
`--all`, `--source`, `--plans`, `--debounce`) is translated into a `WatchConfig`
and the internal `FileFilter` is built with the right `respect_extensions` flag.

Key seam points:

- `crates/anvil-cli/src/commands/watch.rs:737` — `warn_on_bare_exclude_patterns`
  calls out the `v0.4.0-beta` breaking change (bare names like `vendor` no
  longer match files inside the directory; users must pass `vendor/**`).
- `crates/anvil-cli/src/commands/watch.rs:748-749` — the dispatcher decides
  whether the kernel's `FileFilter` should keep or drop the registry-extension
  gate. `--patterns` / `--source` / `--plans` drop it (the user has their own
  scoping criterion); `--all` keeps it (the gate mirrors the parser registry via
  `Language::from_path`, so forwarding files the registry does not recognise
  would just generate `UnsupportedLanguage` errors).
- `crates/anvil-cli/src/commands/watch.rs:760-773` — `WatcherConfig`
  - `WatchConfig` construction. `architecture_config` is filled when
    `.anvil/architecture.yaml` exists; otherwise `None` and the cross-layer
    invariant is a no-op.
- `crates/anvil-cli/src/commands/watch.rs:775-778` — `mpsc` channel
  - `run_watch` call.
- `crates/anvil-cli/src/commands/watch.rs:780-785` — Ctrl-C handler toggles the
  shutdown flag the kernel watches. The dispatcher owns signal handling; the
  kernel does not.

## 13. Cross-cutting concerns

### 13.1 Determinism

Same files + same kernel version → same graph + same findings. The load-bearing
invariants:

- File walk produces a sorted file list (embedded: `embedded.rs:284`). Watch's
  initial scan parses in parallel but applies sequentially over the
  rayon-collected vector, which is walker-order; subtle non-determinism in walk
  order would surface as id-allocation drift but not as graph-content drift.
- Import resolution sorts ambiguous matches by shortest path
  (`incremental.rs:282`).
- Known-files iteration uses `BTreeSet` for stable ordering before resolution
  (`incremental.rs:134-140`, `:298-304`).
- Policy invariants iterate `delta.added_symbols` in insertion order; the dedupe
  set is content-keyed.

### 13.2 Concurrency model

- **One graph writer at a time.** `SymbolGraph` is held by a single thread (the
  watch thread, or the embedded caller's thread). Parse runs in parallel via
  rayon, but the apply phase is serial.
- **Thread-pool cap.** Both `watch.rs:175-181` and `embedded.rs:107-113` build
  the global rayon pool with `max(1, cpus / 2)` threads. The cap is
  process-global via a shared `std::sync::Once`.
- **Watcher event-pump** is its own thread (spawned in `start_watcher`,
  `watcher/mod.rs:204`) — it owns the `Arc<Mutex<RecommendedWatcher>>` so it can
  register newly created directories at runtime without contending with the
  watch loop.
- **Emitter** is `Send` + cheap to clone. Sequence numbers are atomic
  (`AtomicU64`).

### 13.3 Panic isolation

The watch loop wraps every `process_change` call in
`catch_unwind(AssertUnwindSafe(...))` (`watch.rs:341-352`). A panic surfaces as
an `Error` event with the file path; the loop keeps draining. Without this
guard, a single malformed file or a tree-sitter edge case would silently
terminate the watch thread and leave the user with an apparently working but
silently dead `anvil watch` — pinned by `v0.4.0-beta`'s "`anvil watch`
reliability" entry (CHANGELOG line 261).

The embedded path does not catch panics — it returns errors via `EmbeddedError`
and surfaces parse failures through the emitter.

### 13.4 `unsafe_code = "forbid"`

Workspace-level lint `unsafe_code = "forbid"` (`Cargo.toml:90-91`). Every
`unsafe` in the dependency closure lives in an external crate — `nix`, `notify`,
`tree-sitter`, `petgraph`. The kernel itself contains zero `unsafe` blocks.

### 13.5 ID allocator discipline

Symbol ids come from two interleaved allocators: a per-file allocator inside
`extract_symbols` (0-based, `id_offset` parameter), and the graph's own
`next_id` for synthetic external/module nodes. Callers must read
`graph.next_id()` after every `update_file` and `re_resolve_imports` to advance
their local allocator. Tests pin both the collision case
(`incremental.rs::external_synthetic_does_not_collide_with_next_files_base_id`,
`:752-846`) and the id-zero case (`:848-893`). Editing `update_file` without
preserving these invariants will reintroduce the `0.5.1-beta` cascade.

## 14. Performance posture

The shipping numbers (from
`docs/architecture/rust-architecture-overview.md:174-190`, measured via the
criterion benches at `crates/anvil-kernel/benches/kernel.rs`) substantially beat
the spec §8.3 targets:

| Metric                           | Spec target | Shipped (criterion)          |
| -------------------------------- | ----------- | ---------------------------- |
| Cold graph build (100 files)     | < 3 s       | **14.5 ms** (rayon parallel) |
| Cold graph build (1 000 files)   | < 3 s       | ~565 ms (estimated)          |
| Incremental update (single file) | < 100 ms    | **10 µs**                    |
| Policy evaluation (all H1)       | < 10 ms     | **799 ns**                   |
| Event emission (1 000 events)    | < 10 ms     | **408 µs**                   |
| tree-sitter parse (single file)  | < 1 ms      | **< 1 ms**                   |
| Concurrent burst (10 files)      | —           | **693 µs**                   |
| Concurrent burst (50 files)      | —           | **3.5 ms**                   |
| Memory footprint (medium repo)   | < 500 MB    | Not yet measured at scale    |

The "10–40x improvement over Node.js scanner" framing in
`docs/architecture/rust-architecture-overview.md:213-222` is a historical claim
from the cutover. Current targets and methodology live in
`docs/architecture/kernel-benchmarking-spec.md`. The criterion groups
(`cold_graph_build`, `incremental_update`, `policy_evaluation`,
`event_emission`) run on every PR via the benchmarks workflow; the extended
capacity benches (5k+ files, varied LOC, import density) are the open extension
surface that spec covers.

ADR-031 latency budgets cover the real-time validation paths (save-time,
mid-edit, gate); the kernel's policy evaluation budget sits comfortably inside
them.

## 15. Known gaps (refreshed 2026-07-02; originally dated 2026-05-07)

### G-01: Tail languages parse but have no per-language policy depth (T2/T3)

**Superseded — the "TS/JS only" gap has largely closed.** As of the 2026-07-02
sweep, `crates/anvil-kernel/src/parser/languages.rs:5-63` registers **15
grammars** (§6.1): TS/TSX/JS/JSX anchors, plus Rust, Python, Dart, Go, Java,
Kotlin, C#, C, C++, Zig, and WebAssembly text. A user pointing `anvil watch` at
a Rust, Python, or Go repo now gets a real parse, symbol extraction, and the
structural graph — not a no-op. The residual gap is **depth, not coverage**: the
tail languages (Dart..C++ under LANGTAIL, Zig/Wat under LTW2 / ADR-093) ship at
**T1 (Parsed)** — no per-language anti-pattern catalogues, suppression syntax,
or policy hooks yet (T2/T3). Rust owns its extractor under RSTLAN.

**Risk:** Low. The structural signal (symbol graph, trust, the four H1
invariants) is language-agnostic and fires on every registered language; only
per-language rule packs are outstanding. **Fix:** LANGTAIL/LTW2 T2/T3 tiers and
per-language rule catalogues.

### G-02: OPA / Rego policy is not reachable from kernel evaluation

`docs/architecture/rust-architecture-overview.md:85` declares an `anvil-policy`
crate for OPA / Rego evaluation. The kernel's `PolicyEngine` does not call into
it — only the four in-process Rust invariants (§8.3) ship today. Declarative
policy via OPA is an external-tool dependency from the kernel's perspective, not
a kernel feature.

**Risk:** Low for the H1 use case; users who want OPA-based governance run the
OPA gate command from the broader CLI. **Fix:** Tracked under OPAE
(`opa-enhancements.aps.md`, Draft).

### G-03: `embedded.rs::EmbeddedConfig.plan` declared but not consumed

`crates/anvil-kernel/src/embedded.rs:45-46` carries a `plan: Option<PathBuf>`
field documented as "passed through from the CLI for future plan-scoped
filtering (not yet consumed by `run_embedded`)". Plan-scoped filtering is a
hand-off to a follow-up; today the field is silently ignored.

**Risk:** Low — documented in the field's doc comment. **Fix:** plan-scoped
filtering work item under a future tag.

### G-04: Daemon-mode kernel transport not built (KERN-050..052)

Spec §9.3 names a daemon-mode IPC surface for the kernel. ADR-030 routes that
responsibility to `anvil-intercept`, which hosts the kernel in-process;
KERN-050..052 are marked superseded-into-INTD in `plans/index.aps.md:238`. The
historical end-state doc
(`docs/archive/architecture/rust-architecture-endstate.md`) showed
`daemon.rs [DEFERRED] Unix socket server` — that placeholder is not shipping
authority; daemon transport lives in INTD. This is by design.

**Risk:** None — design intent. Cross-link:
`docs/architecture/intercept-as-built.md`.

### G-05: The kernel's own watch/embedded paths cold-rebuild (snapshotting lives in the daemon)

Qualified since the 2026-07-02 sweep. The kernel's own `anvil watch` and
embedded (`anvil check` / `gate` / `audit`) paths still **cold-rebuild** on
every start — they neither read nor write a graph snapshot, and there is no
git-diff optimisation. For the medium-repo target (~100k LOC, ~2k files) cold
start is sub-second on rayon, so the cost is acceptable; for very large repos, a
fresh `anvil watch` re-parses everything.

Graph **snapshotting does exist**, but in `anvil-graph-cache::snapshot` (GV2-030
/ ADR-069 — §7.7) for the resident intercept **daemon**, which can persist and
warm-restart the warm `(SymbolGraph, DependencyGraph)` state. It is default-on
(graduated by GBASE-010 / ADR-105 §11; opt out with `ANVIL_PERSIST_GRAPH=0`),
machine-local, `0600`. The kernel's own entry points are not wired to it.

**Risk:** Low at H1 scale for the kernel paths, growing with repo size. **Fix:**
warm-restart for the kernel's own paths remains a spec §6.4 fast-follow; the
daemon side is shipped.

### G-06: `rust-kernel-spec.md` framing is "Proposed" (cross-link now resolved)

The spec doc carries `**Status:** Proposed — H1 Implementation Target` even
though the kernel has shipped through several beta tags. The intent has always
been that the spec is the design record and an as-built supersedes it for "what
shipped". This as-built closes that loop. The spec stays as the H1 design intent
reference; readers chasing the current state should land here first.

**Resolved (2026-07-02):** the spec→as-built cross-link is now in place —
`docs/architecture/rust-kernel-spec.md` names this doc as "the authoritative
record of current shipping state" and states it supersedes the spec wherever
they differ, and `docs/architecture/README.md` links here from the Kernel entry.
**Risk:** Documentation hygiene only, and now closed.

## 16. Source references

`crates/anvil-kernel/src/`:

- `lib.rs` — module re-exports; nothing else.
- `watch.rs` — KERN-041 foreground watch entry (`run_watch`, `WatchConfig`,
  `WatchHandle`, `initial_scan`, `watch_loop`, `process_change`,
  `pattern_matches`, panic isolation).
- `embedded.rs` — KERN-040 one-shot library entry (`run_embedded`,
  `run_embedded_cancellable`, `EmbeddedConfig`, `EmbeddedResult`,
  `parse_and_build_graph`, `evaluate_files`, `collect_files`).
- `engine_mode.rs` — `EngineMode::Rust` (Legacy / Dual removed as unimplemented
  stubs).
- `feature_flags/mod.rs` — re-exports the resolver / snapshot / telemetry
  surfaces from `anvil-kernel-types::feature_flags`.
- `feature_flags/resolver.rs`, `snapshot.rs`, `telemetry.rs` — the actual flag
  evaluation, snapshot loader, and event types.
- `watcher/mod.rs` — KERN-010 / KERN-013 notify-rs `start_watcher`
  - `WatcherConfig` + `WatchSetupDiagnostics`.
- `watcher/debounce.rs` — `Debouncer` (50 ms window, 500 max pending).
- `watcher/events.rs` — `ChangeBatch`, `FileChange`, `ChangeKind`.
- `watcher/filter.rs` — `FileFilter` (internal denylist + parseable-extension
  gate; `with_respect_extensions` for user-scoped pattern mode).
- `watcher/pattern.rs` — `WatchPatternFilter` (LAUNCH-001 user-glob filter via
  `globset`).
- `parser/mod.rs` — KERN-011 / KERN-012 `Parser` + `ParseResult` + AST-cache
  integration.
- `parser/cache.rs` — `AstCache`, `hash_content` (FNV-1a).
- `parser/languages.rs` — `Language` enum (15 grammars: TS / TSX / JS / JSX /
  Rust / Python / Dart / Go / Java / Kotlin / C# / C / C++ / Zig / Wat) +
  tree-sitter language bridge + `grammar_version` cache discriminator.
- `parser/extract/` — `extract_symbols` + `FileSymbols` + `ImportEdge`; the
  language-agnostic dispatch orchestrator (`mod.rs`, `LanguageExtractor` trait)
  plus per-language extractors: `typescript.rs`, `rust.rs`, `python.rs`,
  `dart.rs`, `go.rs`, `java.rs`, `kotlin.rs`, `csharp.rs`, `clike.rs` (C/C++),
  `zig.rs`, `wat.rs`, and the shared `tail_common.rs` helpers.
- `parser/queries/` — tree-sitter query (`.scm`) files for symbol extraction.
- The semantic graph subsystem now lives in the sibling `anvil-graph-cache`
  crate (re-exported as `crate::graph`; `lib.rs` re-exports). See §7. Modules:
  - `symbol_graph.rs` — KERN-020 `SymbolGraph` (petgraph `DiGraph` + indexes +
    monotonic `next_id`).
  - `dependency.rs` — KERN-021 file-level `DependencyGraph` with cycle
    detection.
  - `incremental.rs` — KERN-022 `update_file`, `resolve_import`,
    `re_resolve_imports`, `remove_file`, `GraphDelta`. The hot path. Hosts the
    `0.5.1-beta` synthetic-id / id-zero fixes.
  - `trust.rs` — KERN-023 `annotate_trust` (Privileged / External / Boundary /
    Internal classification).
  - `certify.rs` — bounded reverse-impact certifiability for the save-time
    daemon (ADR-061 / ADR-064): `certify`, `bounded_impact_closure` (ADR-077
    depth-capped), `impact_closure_unbounded` (background-only).
  - `hot_index.rs` — GV2-022 / ADR-063 sealed hot-read surface: `HotReadApi`
    four-read allowlist, `BackgroundReadApi` denylist, sealed `HotPathSurface`
    marker, `HotRead` / `HotReadMiss`, `MAX_REVERSE_IMPACT_DEPTH` (the ADR-077
    cap).
  - `call_graph.rs` — reverse caller lookup (`callers_of`, `CallersReport`,
    `MAX_CALLERS_WALK`).
  - `registry.rs` — `GraphRegistry`, the daemon's multi-workspace warm-graph
    registry.
  - `snapshot.rs` — GV2-030 / ADR-069 sealed snapshot DTO + `postcard` codec for
    warm-graph persistence (`SnapshotPayload`, `persist_graph_enabled`,
    `MAX_SNAPSHOT_BYTES`); daemon-only (§7.7).
  - `tokens.rs` — GCTX token estimator (`estimate_gctx_tokens`,
    `GCTX_TOKEN_ESTIMATOR_VERSION`).
- `policy/mod.rs` — re-exports.
- `policy/config.rs` — KERN-030 `ArchitectureConfig` YAML loader,
  `layer_for_file`, `is_import_allowed`.
- `policy/engine.rs` — KERN-031 `PolicyEngine`, `Invariant` trait, `Violation`,
  `Severity`, `(policy_id, file, symbol)` dedupe set.
- `policy/invariants/cross_layer.rs` — H1 cross-layer-violation invariant.
- `policy/invariants/new_dependency.rs` — H1 new-dependency-introduction
  invariant.
- `policy/invariants/public_api.rs` — H1 public-api-expansion invariant.
- `policy/invariants/privilege_expansion.rs` — H1 privilege-expansion invariant.
- `protocol/mod.rs` — re-exports `emitter`.
- `protocol/emitter.rs` — KERN-033 `EventEmitter` (sequence counter, ISO-8601
  timestamp, `mpsc::Sender<EngineEvent>`).

`crates/anvil-kernel/benches/`:

- `kernel.rs` — KERN-043 criterion benches: `cold_graph_build`,
  `incremental_update`, `policy_evaluation`, `event_emission`, fixture
  generation.

`crates/anvil-kernel/tests/`:

- `architecture_parity.rs` — KERN-042 parity harness fixtures (legacy comparison
  was retired; structural assertions remain).
- `dual_run.rs` — KERN-042 dual-run harness placeholder; legacy engine retired.
- `watch_pattern_filter.rs` — integration tests for `WatchPatternFilter`
  end-to-end through `run_watch`.
- `watcher_integration.rs` — integration tests for the notify-rs bridge.

`crates/anvil-kernel-types/src/`:

- `lib.rs` — re-exports + `EngineId`.
- `events.rs` — `EngineEvent`, `EventType`, `EventPayload`, `ErrorPayload`,
  `ErrorCode`. The wire protocol.
- `graph.rs` — `SymbolNode`, `SymbolEdge`, `SymbolKind`, `Visibility`,
  `EdgeType`. The graph wire vocabulary.
- `trust.rs` — `TrustLevel` (`Unknown` / `Internal` / `Boundary` / `External` /
  `Privileged`).
- `diagnostics.rs` — canonical `anvil.diagnostic.v1` envelope (shared with
  AIGUARD / RTAI / INTD / DRVR).
- `feature_flags.rs` — flag manifest, evaluation context, resolver data types.
- `notifications.rs` — `Notification` envelope for JSON CLI surfaces.
- `hooks.rs` — `ANVIL_CONFIG_HOOK_PATTERN` + `is_anvil_managed_command` for
  git-hook config wiring.

## 17. Related docs

- `docs/architecture/rust-kernel-spec.md` — the H1 design spec this as-built
  supersedes for "what shipped"; spec stays as the intent reference for §1–§13
  H1 design choices.
- `docs/architecture/kernel-benchmarking-spec.md` — capacity and
  regression-detection methodology; the criterion groups currently shipped in
  `crates/anvil-kernel/benches/kernel.rs` are the regression-detection layer
  this spec extends.
- `docs/architecture/rust-architecture-overview.md` — crate-level layout (KERN /
  RENG / RATS / PORT / RSTLAN), shipped performance targets table, dependency
  surface.
- `docs/archive/architecture/rust-architecture-endstate.md` — archived
  aspirational end-state (2026-04-03). Historical deltas vs this as-built:
  daemon transport belongs to INTD (G-04); language coverage extends through
  RSTLAN / PYLAN.
- `docs/architecture/checks-as-built.md` — downstream consumer (RENG-ported
  checks; the secret / antipattern / command-safety scanners that compose with
  the kernel's structural signal).
- `docs/architecture/activation-as-built.md` — baseline writer + language
  profile; the activation diagnostic that surfaces "we have no kernel coverage
  for this language".
- `docs/architecture/intercept-as-built.md` — the daemon that hosts the kernel
  in-process for ADR-030 / KERN-050..052 supersession; diagnostic-envelope
  parity contract with the embedded path.
- `plans/archive/modules/rust-kernel.aps.md` — KERN module acceptance record.
- `plans/archive/modules/kernel-benchmarking.aps.md` — BENCH module acceptance
  record (16/16 complete).
- `CHANGELOG.md` — `0.4.0-beta` "Native Rust scanner" entry, `0.5.1-beta`
  "Incremental kernel imports" entry, the latest `0.6.0-beta` entry.
