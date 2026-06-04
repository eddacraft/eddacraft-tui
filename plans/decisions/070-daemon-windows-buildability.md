# ADR-070: Daemon Windows-Buildability Strategy

## Status

Accepted

## Date

2026-06-04

## Context

DSV-010 (Windows save-time daemon) surfaced that `crates/anvil-intercept` — the
resident save-time daemon — **does not build on `x86_64-pc-windows-msvc` at all
today** (a pre-existing state; PR #2182 records the Windows workspace as
long-red). The ADR-068 read-safety guard
(`crates/anvil-intercept-win32/src/read_safety.rs`) is built and cross-compile-
verified, but it cannot be runtime-tested on the Windows CI matrix
(`cargo test --workspace`) while the daemon crate fails to compile, and DSV-010's
"lift `save_time.rs`" framing under-scoped the real prerequisite.

A full survey of the crate found the build blockers are a **small set of
unconditional imports from `#![cfg(unix)]` modules**, not a pervasive
Unix-entanglement:

- `assurance.rs` and `validate_paths.rs` import `crate::change_class::CanonicalChange`
  unconditionally — but `CanonicalChange` is a **platform-neutral enum**
  (`ContentModify`/`Create`/`Delete`/`Rename`); it merely *lives* in the
  unix-gated `change_class` module beside the inode-based `PathIdentity`.
- `workspace_admission.rs` imports `path_safety::open_workspace_dirfd`
  unconditionally — the Unix openat2 anchor.
- `confinement.rs` reaches `workspace_admission` (test-only).

Critically, **most heavy subsystems are already cross-platform**:

- IPC transport (`ipc.rs`) already dual-platforms Unix sockets / Windows named
  pipes via `anvil-intercept-win32`.
- The watcher (`watcher.rs`) consumes the kernel's `notify`-backed feed
  (`ReadDirectoryChangesW` on Windows) — neutral.
- The rayon pools (`workspace_pool.rs`) and the JobObject interrupt
  (`interrupt.rs`, already win32-pathed) are neutral.

So the genuinely Unix-specific, load-bearing pieces are exactly three, and each
has a real Windows counterpart:

1. The **read anchor** (`path_safety` openat2 / held `O_PATH` dirfd) — the
   ADR-068 guard (`read_safety::WorkspaceDir` + `read_under`) is the Windows
   counterpart, already built.
2. The **path identity** (`PathIdentity` inode triple via `MetadataExt`) — Windows
   has a faithful analogue: `GetFileInformationByHandleEx(FileIdInfo)` →
   `FILE_ID_INFO { VolumeSerialNumber, FileId(128-bit) }`.
3. The **peer-cred** check (`SO_PEERCRED`/`getpeereid`) — Windows: the owner-only
   pipe DACL (already enforced) plus an explicit
   `GetNamedPipeClientProcessId → OpenProcessToken → TokenUser` SID compare.

This ADR decides the strategy and sequencing for making the daemon
Windows-buildable and the save-time verbs Windows-functional.

## Decision

Make the daemon's **verdict spine platform-neutral**, parameterised over **three
platform primitives** (anchor, identity, peer-auth) that each have a real Windows
implementation — not a Unix-only crate with a stubbed Windows shell. Delivered in
**two stages**.

### The structure

1. **Decouple the neutral verdict logic from the Unix primitives.** Move the
   platform-neutral types out of the `#![cfg(unix)]` gates so they compile
   everywhere: `CanonicalChange` + the invalidation taxonomy
   (`change_class`/`assurance`), the verdict assembly (`validate_paths`),
   `AdmissionMode`/`AllowPolicy` (`workspace_admission`), and the `confinement`
   config. The inode-reading and openat2 primitives stay platform-split.

2. **Platform-abstracted read anchor.** Introduce a `WorkspaceAnchor` abstraction
   the verdict path codes against:
   - Unix: the held `O_PATH` dirfd + `path_safety::read_under` (openat2).
   - Windows: `read_safety::WorkspaceDir` + `read_under` (ADR-068 — built).
   `workspace_admission::AdmittedRoots` and `save_time` hold a `WorkspaceAnchor`,
   not a bare `BorrowedFd`.

3. **Platform-abstracted path identity.** `PathIdentity` keeps the inode triple
   on Unix; on Windows it reads `FILE_ID_INFO` (volume serial + 128-bit file id)
   via the handle the anchor already holds — a **real** identity, not a stub, so
   the atomic-save / rename classification (`change_class`) is as sound on
   Windows as on Unix.

4. **Peer auth = DACL + explicit peer-SID.** The owner-only named-pipe DACL is the
   boundary (kernel-enforced same-SID, mirroring the Unix owner-only socket
   perms); the explicit `GetNamedPipeClientProcessId → token SID` compare is the
   parity belt-and-suspenders for the Unix `SO_PEERCRED` check (defence in depth;
   closes the `peer_pid = None` gap, MLP2-028).

### Sequencing

- **Stage 1 — compile-clean (DSV-010a).** Decouple the neutral logic (step 1) and
  introduce the `WorkspaceAnchor` + `PathIdentity` abstractions wired with only
  the Unix impl. The daemon **builds on Windows**, with the save-time verbs
  returning the existing `Method not found / save-time not enabled` reply
  (`ipc.rs` already does this on non-unix). Mostly mechanical, low-risk; guarded
  against Unix regression by the existing unit tests + the DSV-009 parity gate.
  **This unblocks the whole Windows rust matrix** (not just save-time) and lets
  the ADR-068 guard's fixture tests finally run on Windows CI.

- **Stage 2 — functional Windows save-time (DSV-010b).** Wire the Windows anchor
  (the guard), the Windows `PathIdentity` (`FILE_ID_INFO`), and the peer-SID
  auth; enable the verbs on Windows; extend the DSV-009 cross-path parity gate to
  a Windows delivery path. This is the substantive DSV-010 deliverable.

- **Then DSV-011** — the Windows `watch`/`status` named-pipe clients.

## Rationale

- **Bounded, not a swamp.** The survey shows the Unix-specific surface is three
  primitives, each with a real Windows counterpart (one already built); the heavy
  machinery (IPC, watcher, pools, interrupt) is already cross-platform. A neutral
  spine over three swappable primitives is the least-code, least-drift shape.
- **No silent weakening.** Windows gets a *real* identity (`FILE_ID_INFO`) and a
  *real* anchor (the guard), so the save-time soundness contract (atomic-save
  classification, C2/B2 read-safety, same-uid auth) holds on Windows, and the
  DSV-009 parity gate can assert it.
- **Stage 1 is independently valuable.** Compile-cleaning the daemon fixes the
  pre-existing red Windows workspace build for the *entire* repo and turns on
  real Windows runtime testing for the win32 crate — value beyond DSV-010.

### Alternatives Considered

- **B — keep the daemon Unix-only; ship a separate thin Windows save-time
  daemon.** Rejected: duplicates the verdict spine, guarantees drift, and defeats
  the "one warm model / one wire" goal of ADR-061.
- **C — compile-clean only; leave Windows verbs permanently disabled
  (`Method not found`).** Rejected as the *end* state (it does not deliver
  Windows save-time, the point of DSV-010) — but **adopted as Stage 1**, the
  intermediate that unblocks the build.
- **D — gate the whole `anvil-intercept` crate `#![cfg(unix)]`.** Rejected: the
  `anvil` binary (CLI) links the daemon crate on Windows, so gating the crate
  away breaks the Windows binary build entirely.
- **Stub the Windows identity (path+mtime only, no file id).** Rejected: a
  stubbed identity mis-classifies atomic-saves/renames (the exact bug
  `change_class` exists to prevent), silently weakening Windows soundness;
  `FILE_ID_INFO` is the faithful analogue and is cheap.

## Consequences

- **Stage 1 turns the Windows rust matrix green** (fixes a pre-existing repo-wide
  red) and enables Windows runtime tests for `anvil-intercept-win32` (the guard).
- The neutral-spine refactor touches `change_class` / `assurance` /
  `validate_paths` / `workspace_admission` / `confinement` — it must not regress
  the Unix path; the existing unit tests + the DSV-009 parity gate are the guard
  rails, and Stage 1 changes no Unix behaviour (pure decoupling + an anchor
  abstraction with one impl).
- `anvil-intercept-win32` gains a `FILE_ID_INFO` identity read and a peer-SID
  helper (small, beside the existing token/SID code).
- DSV-010 is reshaped into **DSV-010a (compile-clean)** + **DSV-010b (functional
  Windows save-time)**; DSV-011 (clients) follows. The module's DSV-010 item is
  updated to this breakdown.
- Out of scope (unchanged): macOS already works (`cfg(unix)`); Windows GA
  *hardening* (signing, service autostart) stays out per the module. The Windows
  guard's per-component ladder carries the same in-model same-uid TOCTOU window
  as the Unix ladder (ADR-068), accepted.

## References

- ADR-061 §5 (change classification / invalidation taxonomy), §7 (read-safety,
  peer-auth); ADR-068 (the Windows read-safety guard); ADR-064 (graph-cache crate
  boundary).
- `crates/anvil-intercept/src/{change_class,path_safety,save_time,assurance,
  validate_paths,workspace_admission,confinement,ipc}.rs`;
  `crates/anvil-intercept-win32/src/{lib,read_safety}.rs`.
- DSV-010 / DSV-011, `plans/modules/daemon-save-time-validation.aps.md`.
- Microsoft Learn: `GetFileInformationByHandleEx` / `FILE_ID_INFO`;
  `GetNamedPipeClientProcessId`; `OpenProcessToken` / `TokenUser`.
- MLP2-028 (Windows peer-PID extraction — the `peer_pid = None` gap).
