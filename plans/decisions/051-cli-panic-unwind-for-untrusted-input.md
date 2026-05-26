# ADR-051: anvil CLI builds with `panic = "unwind"` so untrusted-input panics surface as structured errors

## Status

Accepted

## Date

2026-05-26

## Context

The `anvil` CLI binary built with `panic = "abort"` (`[profile.release]`, inherited
by `[profile.dist]` — the cargo-dist shipping profile). The rationale was recorded
inline in `Cargo.toml`: *"abort is correct there because the binary owns its own
process and a panic is acceptable to terminate it."* The `release-napi` profile was
the lone exception — it overrides to `panic = "unwind"` so a panic in the scanner
returns to the napi FFI boundary (where `catch_unwind` maps it to a JS exception)
instead of aborting the host Node process.

Two forces make the abort default the wrong fit for the CLI:

1. **`anvil` processes untrusted input.** Policies (Rego, evaluated by the
   single-vendor `regorus` 0.10 crate, which has internal `unwrap`/`expect`) and
   repository content are adversarial surfaces. A panic triggered by untrusted
   input is an *input-triggered crash*, not the internal-invariant-violation that
   the "a panic is a bug, die fast" philosophy targets.
2. **`anvil policy eval` is a machine-parsed CI gate.** Its `--json` output is
   consumed by pipelines. Aborting the process on a `regorus` panic leaves those
   parsers with no error envelope and only a `SIGABRT` — the gate fails with zero
   diagnostics.

CIB-018 (filed by the POLENG council, operations seat) asked for a `catch_unwind`
guard at the policy-engine facade so such panics become a structured
`EngineError` + non-zero exit. That guard is a **no-op under `panic = "abort"`** —
the panic runtime aborts before any unwinding begins — so the guard was inert in
exactly the shipped profile where it was needed. The decision must be made now
because shipping the guard without this change would claim a protection the binary
does not have.

## Decision

Build the `anvil` CLI (and all `[profile.release]` / `[profile.dist]` artifacts)
with **`panic = "unwind"`**. Pair it with a `catch_unwind` guard at the
`anvil-policy-engine` facade (`Engine::guard`, CIB-018) that converts a caught
panic into `EngineError::Regorus` and poisons the engine.

This generalises to the CLI the boundary discipline `release-napi` already applies
at the FFI edge: a panic at an untrusted-input boundary unwinds to a `catch_unwind`
that maps it to a structured error, rather than aborting the host.

## Rationale

Why unwind over the alternatives:

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **`panic = "unwind"` + facade `catch_unwind` (chosen)** | Idiomatic; `regorus` is pure Rust and unwinds cleanly; consistent with the existing `release-napi` reasoning; the gate emits a parseable `--json` error + non-zero exit instead of `SIGABRT`; minimal code | Unwind tables grow the binary modestly; panics no longer abort instantly; reverses a documented decision |
| **Keep `panic = "abort"`** | Smallest binary; "panic = bug = die" simplicity | Untrusted input can crash the CI gate with no diagnostics; `catch_unwind` is inert, so CIB-018 cannot be met at all |
| **Subprocess-isolate `anvil policy eval`** | Contains even an abort; isolates `regorus` fully | Heavy: spawn + IPC per eval, abort-signal handling; over-engineered for pure-Rust code that unwinds cleanly |

Trade-off accepted: a modestly larger binary and non-instant panics, in exchange
for a CLI that degrades gracefully on untrusted-input panics. `panic = "unwind"`
is the Rust default; `abort` was the opt-in optimisation.

## Consequences

- **Positive:** A `regorus` panic during `anvil policy eval` (or any guarded
  facade call) now yields a structured `EngineError::Regorus` → the CLI's existing
  `--json` error path emits a parseable envelope with a non-zero exit. The guard is
  effective in the shipped binary, not just under `cargo test`. The whole binary
  gains the option of a top-level panic handler in future.
- **Negative:** Unwind landing pads increase binary size modestly; panics unwind
  (running destructors) rather than aborting immediately.
- **Risks:** A `Drop` that panics during unwind still aborts (double-panic). The
  facade guard poisons the engine after a caught panic, so a panic that leaves
  `regorus` state inconsistent is never acted on again.
- **Mitigations:** Panics remain rare — `regorus` parse/validation errors are
  already `Err` (not panic), and POLENG-009 bounds eval resources. The guard's
  `AssertUnwindSafe` is sound because the poison flag prevents observing any broken
  invariant in `inner`.

## References

- Related ADRs: ADR-040 (regorus policy engine), ADR-030 (napi unwind boundary precedent)
- APS modules: CIB-018 (this guard), POLENG-009 (resource bounds)
- Code: `crates/anvil-policy-engine/src/lib.rs` (`Engine::guard`), `Cargo.toml` (`[profile.release]`)
