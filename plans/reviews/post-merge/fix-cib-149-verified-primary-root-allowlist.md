# Post-merge test plan — CIB-149 verified primary root for Allowlist confinement

Branch: `fix/cib-149-verified-primary-root-allowlist`
Item: CIB-149 — Stop treating an unverified first wire root as the confinement
primary
Council: blocking findings on the first pass (in-connection-only primary was
unreachable for one-shot production connections; no two-connection test)
remediated in the second commit.

## What / why

In Allowlist confinement mode the connection's implicitly-admitted primary root
was seeded from the first `workspace_root` a wire request named, letting a
same-uid client self-declare its way past the operator allow list.

The fix seeds a dedicated `SaveTimeConn::verified_primary` at connection setup
from the durable `SessionRegistry` via the authenticated peer's PID lineage
(`worktree_for_lineage(peer_pid)` — the same anti-PID-reuse anchor the
write-time spoof cross-check uses), threaded through
`Confinement::to_admitted_roots` into `save_time::authorise_root`. The
`/proc` lineage walk is gated on `Confinement::is_allowlist()`; no verified
primary means allow-entries-only (fail-closed). Open-mode first-touch seeding
is unchanged.

Touched:

- `crates/anvil-intercept/src/confinement.rs` — `to_admitted_roots` takes an
  optional verified primary; `is_allowlist()` helper
- `crates/anvil-intercept/src/ipc.rs` — `seed_save_time_verified_primary`
  called from the accept loop; registry-across-connections test
- `crates/anvil-intercept/src/save_time.rs` — `verified_primary` field,
  `authorise_root` seeding, unit tests

## Gate commands run (pre-PR, all green)

```
cargo fmt --all --check
cargo clippy -p eddacraft-anvil-intercept --all-targets -- -D warnings
cargo test -p eddacraft-anvil-intercept
pnpm run format:check
```

Results: fmt exit 0; clippy exit 0 (no warnings); full crate suite green
including the new cases
(`confinement::tests::allowlist_without_verified_primary_admits_only_allow_entries`,
`save_time::tests::allowlist_without_session_refuses_first_named_unlisted_root`,
`save_time::tests::allowlist_session_bound_worktree_is_primary_not_first_named_root`,
`ipc::tests::verified_primary_resolves_from_registry_across_connections`);
oxfmt clean on 1412 files.

## Post-merge verification

1. Confirm the four new tests run green in the standard CI suite on `main`
   (push-event `rust-tests` job).
2. Live-daemon spot check (Linux box, quiet environment): start the daemon in
   Allowlist confinement mode, register a worktree from an agent session
   (lineage-registered, MLP2-025 path), then issue a save-time verb from a
   fresh one-shot connection in that lineage — the registered worktree must be
   admitted as the implicit primary. From a shell **outside** that lineage,
   the same verb naming an unlisted root must be refused (`NotAdmitted`).
3. Confirm the default (open) posture shows no behaviour change and no `/proc`
   lineage walk at connection setup (gated on `is_allowlist()`).

## Known limitation (by design, logged as follow-up)

Activation-path registrations (`anvil workspace register`) send no PID
lineage, so `worktree_for_lineage` cannot resolve them; those worktrees need
explicit allow entries. Extending the implicit primary to non-lineage
registrations needs a peer→worktree binding that does not exist yet (separate
design/ADR if operators ask for it).
