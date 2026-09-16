# Settings Class A safe-write policy

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative for SETPREF persistence | [SETPREF](../modules/settings-safe-preferences.aps.md) | Accepted | 2026-09-16 — operator Ready-checklist item |

| Upstream | Downstream |
| -------- | ---------- |
| [`2026-08-06-settings-truth-surface.md`](./2026-08-06-settings-truth-surface.md) §14–15; ADR-132; catalogue `writers_by_scope` / `PersistenceTarget::UserConfig`; ADR-120 path-safe delegation; `anvil-baseline` atomic replace | SETPREF-004, SETPREF-005, SETPREF-006; `crates/anvil-config` settings writer |

This specification is the SETPREF v0.1 safe-write policy. It does not authorise
implementation by itself. SETPREF-004 implements atomic persistence;
SETPREF-006 proves this policy with tests.

## 1. Scope

Applies only to Class A persistence through the settings service to the
catalogue target `user-config` at **user** scope.

Does **not** apply to:

- `anvil config set`, `anvil init`, or `anvil start` bootstrap writes
- project-config, org, environment, or session scopes (no Class A writer)
- SETGOV multi-file transactions or recovery

The settings service is the only writer process. Surfaces never open the
destination file themselves.

## 2. Destination

- The write path is composed by the service from the catalogue writer, not from
  a caller-supplied path fragment.
- The destination must remain inside the user-config root after
  canonicalisation. That root is the same gated `ANVIL_HOME` / XDG state root
  used for other user-scoped Anvil files. The exact filename is SETPREF-004.
- Creating missing parent directories is allowed only when every created
  component stays inside that root.

## 3. Path traversal

Reject before any write:

- `..` components
- absolute paths, Windows drive letters, and UNC prefixes supplied as
  destination fragments
- NUL bytes and other non-path characters in composed names

After canonicalisation, the destination must still be a descendant of the
user-config root. This is the ADR-120 delegation containment rule applied to
the user-config root instead of the workspace.

## 4. Symbolic links

Inspect with `symlink_metadata` (or equivalent). Do not use `Path::exists` to
decide whether a symlink is present — a broken link looks absent if followed.

- **Leaf:** refuse if the destination is a symlink, live or broken. Do not
  follow it. Do not replace the link with a regular file.
- **Parents:** refuse if any parent used to reach the file is a symlink whose
  canonical target leaves the user-config root.
- **In-root parent symlinks** whose resolved path stays inside the root are
  allowed. The final leaf still must not be a symlink.

A refused symlink is a failed write, not a follow-and-continue.

## 5. Replace semantics

Never truncate the destination in place.

1. Refuse non-regular destinations (directory, FIFO, device, socket).
2. Write the complete payload to a uniquely named sibling temporary file in
   the same directory (same filesystem).
3. Preserve the existing destination mode when replacing. New files use Unix
   mode `0o600` (owner read/write only). Do not chmod an existing file wider.
4. `fsync` the temporary file (and, where the platform requires it, its
   directory) before publishing.
5. Publish with an atomic rename onto the destination.
   - POSIX: `rename(2)` replaces silently.
   - Windows: `std::fs::rename` may return `AlreadyExists`; then re-check
     `refuse if symlink`, remove, and rename. That remove-then-rename window
     is a known platform limit and must not truncate first.
6. On any failure, the original destination bytes must be unchanged. A unique
   leftover temporary file is allowed; a fixed temporary name is not.

Failed validation writes nothing. Persistence success is reported separately
from any claim that the new value is active.

## 6. Permission boundaries

- Unwritable destination or parent: fail closed; write nothing.
- Do not follow a permission error by writing to a fallback path.
- Do not create world-writable files.
- Interrupted writes leave either the previous complete file or a complete
  new file, never a truncated destination.

## 7. Concurrent writers

SETPREF-005 owns the source-revision check. This policy only requires that
two complete publishes never interleave bytes: the last successful atomic
rename wins. Mixed or partial content is a defect.

## 8. Diagnostics

Failures name a closed class: `symlink`, `traversal`, `permission`,
`not-regular`, `replace`, `interrupted`. They must not embed setting values
or unredacted source text. Path-shaped identifiers in errors follow the
settings resolver redaction rule.

## 9. Tests SETPREF-006 must cover

- destination is a symlink (live and broken)
- parent symlink that escapes the user-config root
- `..` and absolute/UNC destination fragments
- unwritable target
- interrupted write of the temporary file (destination unchanged)
- concurrent publishers (no mixed bytes)
- existing mode preserved; new file `0o600`

## 10. Precedents

- `crates/anvil-config/src/delegation.rs` — traversal, absolute/UNC, symlink
  escape, no-follow leaf
- `crates/anvil-baseline/src/io.rs` — `refuse_if_symlink` + atomic rename
- `crates/anvil-config/src/parse.rs` — `O_NOFOLLOW` bounded read

Reuse those mechanics. Do not invent a second path-safety dialect.
