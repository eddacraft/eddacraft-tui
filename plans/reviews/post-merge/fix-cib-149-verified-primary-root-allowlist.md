# Post-merge test plan — CIB-149 fail-closed Allowlist confinement

Branch: `fix/cib-149-verified-primary-root-allowlist`
Item: CIB-149 — Stop treating an unverified first wire root as the confinement
primary
Merged: 2026-07-03 via PR #3117

> **Design note.** An earlier pass of this work explored a *verified primary
> root* (resolving the connection's primary worktree from the daemon
> `SessionRegistry` via the peer's PID lineage). That approach was **abandoned**:
> under ADR-061 §7 a client-declared worktree can never be daemon-attested
> across the editor/MCP path, so no implicit primary is sound. The shipped fix is
> **fail-closed by removal** — `Allowlist` mode admits exactly the operator's
> configured allow entries and nothing implicit. This document describes that
> final design; ignore any lingering references to a "verified primary" (see
> CIB-161 for the doc/CLI reconciliation follow-up).

## What / why

In Allowlist confinement mode the daemon previously admitted an implicit
"primary check-in root" seeded from the first `workspace_root` a wire request
named, letting a same-uid client self-declare its way past the operator allow
list.

The fix removes the implicit primary entirely. `Confinement::to_admitted_roots`
in `Allowlist` mode now builds the admitted set from the configured
`exact + prefixes` allow entries only and takes **no path argument**;
`verified_primary`, `set_verified_primary`, `Confinement::is_allowlist`, and
`ipc::seed_save_time_verified_primary` are removed. A registered or first-named
worktree is **not** admitted unless it has an explicit allow entry; an empty
allow-list admits nothing (fail-closed). Open-mode first-touch adoption is
unchanged.

Touched:

- `crates/anvil-intercept/src/confinement.rs` — `to_admitted_roots` builds the
  admitted set from allow entries only (no verified-primary parameter)
- `crates/anvil-intercept/src/ipc.rs` — implicit-primary seeding removed from
  the accept loop; `RegisterSession.worktree` retained for telemetry only
- `crates/anvil-intercept/src/save_time.rs` — `verified_primary` field removed;
  `authorise_root` admits allow entries only

## Gate commands run (pre-PR, all green)

```
cargo fmt --all --check
cargo clippy -p eddacraft-anvil-intercept --all-targets -- -D warnings
cargo test -p eddacraft-anvil-intercept
pnpm run format:check
```

The shipped regression tests assert the fail-closed contract and genuinely fail
on the pre-fix code:

- `confinement::tests::allowlist_empty_admits_nothing`
- `save_time::tests::allowlist_registered_session_worktree_is_not_admitted`
- `ipc::tests::registered_worktree_is_not_implicitly_admitted_in_allowlist`

## Post-merge verification

1. Confirm the three regression tests above run green in the standard CI suite
   on `main` (push-event `rust-tests` job).
2. Live-daemon spot check (Linux box, quiet environment): start the daemon in
   Allowlist confinement mode and register a worktree from an agent session
   (lineage-registered, MLP2-025 path). Issue a save-time verb naming that
   registered-but-unlisted worktree — it must be refused (`NotAdmitted`),
   because registration does not grant admission. Then `anvil workspace allow`
   that root and confirm the same verb now succeeds. An empty allow-list admits
   no root at all.
3. Confirm the default (open) posture shows no behaviour change.

## Follow-up (CIB-161)

The operator-facing surface, public docs, and this evidence artifact originally
described the abandoned verified-primary design. CIB-161 reconciles the CLI
strings, `docs/public/anvil/operations/config.md`, the CHANGELOG, the refusal
diagnostic, and the stale code doc comments with the shipped fail-closed
contract. The ADR-061 §7 amendment recording the fail-closed decision is a
separate governance PR needing owner sign-off.
