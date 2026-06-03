# ADR-068: Windows Read-Safety for Save-Time `validate_paths`

## Status

Proposed

## Date

2026-06-04

## Context

DSV-007 made `anvil watch` and `anvil status` thin clients of the resident
save-time daemon; DSV-010/011 (Sub-phase A-W) bring **Windows** to parity with
the Unix save-time path. macOS is already covered — the daemon and clients are
`cfg(unix)`, so macOS uses the same guard described below.

The blocker DSV-010 flagged is the **read-safety guard**. On Unix
(`crates/anvil-intercept/src/path_safety.rs`, per ADR-061 §7) every save-time
read is anchored at a workspace directory **fd opened once at admission** and
resolved with `openat2(RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH)` — with an
`O_NOFOLLOW` component-ladder fallback where `openat2` is unavailable. That guard
delivers four properties the verdict depends on:

- **C2 — anchored-handle identity.** The held fd *is* the workspace identity, so
  a root-directory retarget *after* admission cannot redirect reads: they hit the
  original inode or fail closed, never silently re-resolve against a swapped-in
  directory.
- **No symlink traversal** (`RESOLVE_NO_SYMLINKS` / per-hop `O_NOFOLLOW`).
- **Beneath-root** (`RESOLVE_BENEATH` + the structural `normalise_rel` that
  rejects absolute / `..` / NUL / empty before any read).
- **B2 — no false attestation.** The daemon reads the exact guarded bytes it
  hashes and certifies; an oversized file is **refused** (`FileTooLarge`), never
  truncated to a wrong, hashable prefix.

Together these close the canonicalise-then-open TOCTOU/symlink class within the
same-uid trust boundary (the only boundary claimed — ADR-061 §7).

**Windows has neither `openat2`/`RESOLVE_BENEATH` nor `openat`**, and a wider
path-hazard surface than Unix: reparse points (symlinks *and* directory
junctions), 8.3 short names, alternate data streams (`file:stream`),
trailing-dot/space stripping, reserved DOS device names (`CON`, `NUL`, `AUX`, …),
`\\?\` / `\\.\` device prefixes, drive-relative (`C:foo`) and UNC paths, and
case-insensitivity. A naive `CreateFileW(path)` would follow junctions/symlinks
and re-resolve the path string on every call — exactly the class the Unix guard
closes.

This ADR decides **how the Windows read path reproduces the §7 guarantees**, and
where it lives. It is *only* about read-safety; the Windows peer-SID
authorisation check (`GetNamedPipeClientProcessId → OpenProcessToken →
TokenUser` SID compare, the SO_PEERCRED analog — ADR-061 §7) is a separate
DSV-010 prerequisite, not this decision.

## Decision

Mirror the Unix guard structurally with the native NT path APIs, isolated in the
existing `anvil-intercept-win32` FFI crate and exposed to the daemon as a **safe**
`read_under`-equivalent. `anvil-intercept` stays `forbid(unsafe_code)` and
dispatches the guarded read per-platform (`path_safety.rs` on unix, the win32
crate on Windows).

The Windows guard is **`NtCreateFile` anchored at a held workspace directory
handle, with `OBJ_DONT_REPARSE`**, with a per-component `FILE_OPEN_REPARSE_POINT`
ladder fallback — the direct analog of `openat2 + O_PATH dirfd + O_NOFOLLOW
ladder`:

- **Anchor / identity (C2):** open the workspace root once at admission as a
  directory handle; pass it as `OBJECT_ATTRIBUTES.RootDirectory` for every
  per-path open. The held handle pins the directory object, so a post-admission
  retarget cannot redirect reads — the Windows analog of the held `O_PATH` dirfd.
- **No reparse traversal (symlink + junction; `RESOLVE_NO_SYMLINKS` analog):**
  set **`OBJ_DONT_REPARSE`**, which fails the open if *any* component is a reparse
  point. This is the production-proven mechanism Go adopted for `os.Root`
  (golang/go#73080).
- **Beneath-root (`RESOLVE_BENEATH` analog):** resolve relative to the
  `RootDirectory` handle (resolution starts at the root) and reject escapes
  structurally before any open (see the hardened normaliser below).
- **Fallback ladder:** where the whole-path `OBJ_DONT_REPARSE` open cannot be
  used, walk one component at a time anchored at the prior directory handle,
  opening each with `FILE_OPEN_REPARSE_POINT` and refusing a component whose
  attributes carry `FILE_ATTRIBUTE_REPARSE_POINT`. Same in-model same-uid TOCTOU
  window as the Unix ladder.
- **B2 — no false attestation:** read the opened handle with the same
  `MAX_GUARDED_READ_BYTES` (64 MiB) ceiling, refusing oversized input rather than
  truncating.
- **Windows-hardened structural normaliser (new, beyond Unix `normalise_rel`):**
  in addition to rejecting absolute / `..` / NUL / empty, the Windows path must
  reject backslash separators (the wire is slash-only), drive letters and
  drive-relative forms (`C:`, `C:foo`), UNC (`\\…`), the `\\?\` / `\\.\` device
  prefixes, alternate-data-stream colons (`file:stream`), trailing dots/spaces,
  and reserved DOS device names. These are guard concerns, not wire concerns.

## Rationale

- **Symmetric security posture.** Option A maps 1:1 onto the Unix guard's three
  load-bearing properties (anchored-handle identity, no-reparse traversal,
  beneath + read-then-certify), so Windows ships the *same* guarantee as Unix for
  the same verb — not a quietly weaker one. The DSV-009 cross-path parity gate can
  then assert identical behaviour across platforms.
- **Production precedent.** Go's `os.Root` — literally the "open beneath a root,
  no reparse traversal" abstraction — uses `OBJ_DONT_REPARSE` with
  handle-relative opens (golang/go#73080). Rust `std` and Microsoft's BuildXL
  sandbox use the same reparse-control primitives.
- **Boundary placement.** The unsafe FFI stays in `anvil-intercept-win32`, which
  already owns the Windows security-attribute surface (`CreateFileW`, `ReadFile`,
  owner-only pipe DACLs, Job Objects) and runs on Windows CI (MLP2-075). The
  daemon crate keeps `forbid(unsafe_code)`.

### Alternatives Considered

- **B — Win32-only `CreateFileW` + `FILE_FLAG_OPEN_REPARSE_POINT` +
  `GetFinalPathNameByHandle` verify-beneath.** Documented Win32 surface, no
  `ntdll`. **Rejected as the mechanism** because `CreateFileW` has no
  handle-relative (`openat`) form: it must re-resolve the full path *string* on
  every open, which **loses the anchored-handle identity (C2)** the Unix guard
  provides — a post-admission root retarget would redirect reads. The
  `GetFinalPathNameByHandle` post-open verify preserves B2 (refuse if the final
  path is not beneath the root) but is *verify-after-open*, not
  *deny-during-resolve*, and cannot anchor. Retained only as optional
  defence-in-depth layered on Option A, never as the primary guard.
- **C — documented weaker guarantee** (same-uid trust + best-effort reparse-open
  control, no anchored identity). **Rejected:** ships an asymmetric, weaker
  Windows posture for the same verb; the C2 retarget-redirection class the Unix
  path closes would stay open on Windows — a silent footgun and a DSV-009 parity
  divergence.
- **D — defer Windows save-time read-safety entirely.** **Rejected:** contradicts
  the short-term Windows-support decision behind DSV-010/011; the gap is real and
  users run Windows.

## Consequences

- DSV-010's open read-safety risk is resolved in principle; with this Accepted it
  can move toward Ready (its other prerequisite, the Windows peer-SID auth check,
  is tracked in DSV-010 itself).
- `anvil-intercept-win32` gains an `NtCreateFile` / `OBJECT_ATTRIBUTES` /
  `OBJ_DONT_REPARSE` / reparse-attribute surface (the `windows-sys` Wdk/Ntdll
  feature set), exposed as one safe guarded-read API. The daemon dispatches
  `read_under` per-platform; no new unsafe enters `anvil-intercept`.
- A Windows-hardened structural normaliser is required (backslash / drive / UNC /
  device-prefix / ADS / trailing-dot / reserved-name rejection) — strictly more
  than the Unix `normalise_rel`.
- New Windows fixture tests mirror `path_safety.rs`: symlink **and** junction
  escape refused, retarget-after-admission fails closed (C2), oversized refused
  not truncated (B2) — on a Windows runner (the win32 crate already runs there).
- The DSV-009 cross-path diagnostic-parity gate extends to a Windows delivery
  path with identical finding sets.
- **Residual (in-model, documented):** the per-component fallback carries the same
  same-uid TOCTOU window as the Unix ladder (the trust boundary is same-uid); the
  `Nt*` layer is more lightly documented than the Win32 surface, so its behaviour
  is pinned by the fixture tests rather than relied upon from docs.

## References

- ADR-061 §7 (authorisation, read-safety, confinement — the contract this
  mirrors); ADR-064 (graph-cache crate boundary); ADR-067 (daemon symbol feed).
- `crates/anvil-intercept/src/path_safety.rs` — the Unix guard
  (`openat2(RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH)` + `O_NOFOLLOW` ladder + the
  `MAX_GUARDED_READ_BYTES` refuse-don't-truncate ceiling).
- `crates/anvil-intercept-win32/src/lib.rs` — the FFI isolation crate this lands
  in.
- DSV-010 / DSV-011, `plans/modules/daemon-save-time-validation.aps.md`
  (Sub-phase A-W).
- Microsoft Learn: `NtCreateFile` / `OBJECT_ATTRIBUTES` (`RootDirectory`,
  `OBJ_DONT_REPARSE`); *Reparse Points and File Operations*; `CreateFileW`
  (`FILE_FLAG_OPEN_REPARSE_POINT`); `GetFinalPathNameByHandle`.
- Precedent: golang/go#73080 — `os.Root` adopts `OBJ_DONT_REPARSE`.
