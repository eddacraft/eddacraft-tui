# ADR-067: Daemon Symbol Feed via a Dependency-Inverted Parse Hook

## Status

Accepted

## Date

2026-06-03

## Context

[ADR-064](064-intercept-graph-cache-crate-boundary.md) makes it binding for
Sub-phase A that the resident intercept daemon (`anvil-intercept`) does **not**
link tree-sitter: it depends on `anvil-graph-cache` (petgraph) + plain
`anvil-kernel-types` data structs only. But to certify a save-time edit the
verdict (`crates/anvil-intercept/src/validate_paths.rs`) needs the **parsed**
`FileSymbols` of the edited file, and the only producer of `FileSymbols` is a
tree-sitter parse (`anvil_kernel::parser`). ADR-064 §4 flagged this as "the one
wiring detail Task 7/8 must nail" and added the escape clause: *"if the
kernel-feed proves infeasible, ADR-064 must be revisited before adding any parser
dep to the daemon."*

The DSV sub-phase-A execution plan (Task 7) sketched the mechanism as "extend the
in-process kernel→daemon **watcher feed** to carry the parsed `FileSymbols` for
each changed path." Investigation during DSV-005 found:

- The watcher (`crates/anvil-intercept/src/watcher.rs`) is **not** wired into the
  production daemon (`run_foreground`); its receiver loop pends forever and the
  kernel↔daemon bridge does not exist.
- A push-based watcher store is parsed from the watcher's **own** read of the
  file (debounced over a coalescing window), which can disagree with the bytes
  the daemon read and hashed for the verdict. Certifying fresh bytes against
  stale symbols is a B2 false-attestation hazard.

A design council (architect, kernel-maintainer, pragmatic-lead) plus a survey of
the [enterprise integration patterns](https://www.enterpriseintegrationpatterns.com/patterns/messaging/)
catalogue evaluated three mechanisms: (A) a dependency-inverted parse hook,
(B) the async watcher feed, (C) a daemon-side contract with the producer
deferred.

## Decision

**The daemon obtains symbols through a dependency-inverted parse hook — the EIP
*Content Enricher behind a Messaging Gateway* — and parses the exact guarded
bytes synchronously on the verdict path.**

- `anvil-intercept` defines the `SymbolParser` trait (`save_time::SymbolParser`) —
  a **Messaging Gateway**: the daemon codes against the domain method
  `parse(path, bytes) -> Option<FileSymbols>` and links no parser.
- `validate_paths` enriches the change it already holds (the **Content
  Enricher**): it hands the parser the **exact** openat2-guarded bytes it read
  and hashed (`fed_symbols: Fn(&str, &[u8]) -> Option<FileSymbols>`), so the
  parsed symbols provably describe the attested bytes. There is no second read,
  hence no race with the editor.
- The kernel-backed impl (`KernelSymbolParser`) lives in **`anvil-cli`**, which
  already depends on both the kernel (tree-sitter) and the daemon, and is
  injected into the daemon via `ForegroundOpts::with_symbol_parser`. Tree-sitter
  therefore links into the **binary**, never the `anvil-intercept` crate — the
  `daemon_dep_boundary` guard stays green.
- The async watcher feed (Option B) is **reframed**: when it is eventually built
  it is an *advisory cache-warmer* (an Event Message that pre-warms the graph so
  a cold-file verdict avoids a from-scratch parse), and is **never** the
  attestation source. The synchronous parse-of-guarded-bytes is always
  authoritative.

## Rationale

- **Correctness is the deciding axis.** Only the parse-of-the-guarded-bytes
  design guarantees the symbols describe the bytes the daemon hashed (the
  Content Enricher "enrich the message you hold" property). The watcher-store
  design enriches from a different, earlier read and can false-attest (B2).
- **ADR-064 is honoured, not revisited.** The trait is dependency inversion, not
  a daemon parser dep; the tree-sitter link lands in `anvil-cli`. The escape
  clause is not triggered.
- **Contained.** It needs no kernel→daemon bridge thread and no `WatcherChangeBatch`
  change; the injection seam (`anvil-cli` constructs the daemon) already exists.
- **Legible + evolvable.** The Messaging Gateway lets a future *out-of-process*
  parser service sit behind the same trait (the strongest ADR-064 reading —
  parser in a separate process) without touching the daemon.

## Consequences

- The parse runs synchronously on the interactive verdict path. Offloading it to
  the interactive pool and the `4 agents + 1 scan` SLO bench are **DSV-006**
  (Task 16). `Parser::new()` is built per call (tree-sitter `Parser` is not
  `Sync`); a thread-local/pool is a later optimisation.
- The interim symbol-id base is path-derived (FNV-1a, a stable collision-resistant
  per-file tag) because the kernel's default 0-based per-file ids would collide
  across files in the warm graph. A residual hash collision is **safe** (it
  yields `DuplicateSymbol` → `certify` `UnreliableGraph` → conservative
  `Partial`), never a false `Certified`. GV2 graph identity (Sub-phase A′)
  supersedes this interim scheme.
- A non-UTF-8 file is not certifiable through this parser (it returns `None` →
  `Partial`) — the extractor's empty-name rendering of invalid UTF-8 would
  otherwise be a B2 edge.
- `ANVIL_INTERCEPT_DISABLE_SYMBOL_PARSER=1` withholds the parser as a break-glass
  (the daemon then returns safe `Partial` verdicts) without a redeploy.

## Relationship to other ADRs

- **Refines** [ADR-064](064-intercept-graph-cache-crate-boundary.md): this is the
  resolution of its "Task 7/8 must nail" wiring detail; it does not weaken the
  no-tree-sitter-in-the-daemon boundary.
- **Realised by** DSV-005 (`plans/modules/daemon-save-time-validation.aps.md`).
- Sub-phase A′ ([GV2](../modules/graph-v2-foundation.aps.md)) supersedes the
  interim id-base scheme with durable graph identity.
