# Air-Gapped Operation Guarantee

| Type    | Authority     | Owner  | Status | Freshness                                                             |
| ------- | ------------- | ------ | ------ | --------------------------------------------------------------------- |
| Runbook | Authoritative | @aneki | Live   | First filed 2026-05-13 alongside Wave 1 (MLP-017 v1, RELEASE-PLAN.md) |

| Upstream                                                                                                                                                                                                                                          | Downstream                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [ADR-036 §D-3](../../plans/decisions/036-daemon-scope-discovery-and-boundaries.md), [RELEASE-PLAN.md Wave 1 — MLP-017](../../RELEASE-PLAN.md), [MLP-017 task in `multilayer-protection.aps.md`](../../plans/modules/multilayer-protection.aps.md) | [`tools/test-harness/network-blocked/run.sh`](../../tools/test-harness/network-blocked/run.sh), [`crates/anvil-cli/tests/air_gapped.rs`](../../crates/anvil-cli/tests/air_gapped.rs), MLP-009 release gate |

## Claim

Anvil's core protection loop runs without internet access. Every command on the
`v0.7.0-beta` slate that an operator might reach for — `anvil start`,
`anvil baseline`, `anvil intercept ensure`, all `anvil hook` subcommands, and
`anvil audit` — makes **zero network calls** in normal operation. Telemetry and
update checks are opt-in and off by default.

This guarantee is part of the daemon-working release gate. See
[`RELEASE-PLAN.md`](../../RELEASE-PLAN.md) Wave 1 (`MLP-017`) and the canonical
scope in
[`plans/modules/multilayer-protection.aps.md`](../../plans/modules/multilayer-protection.aps.md).

## How it's tested

The contract is enforced by a Linux network-namespace harness, not by auditing
the source for `reqwest` calls. Behavioural checks survive refactors; source
audits don't.

### Harness

[`tools/test-harness/network-blocked/run.sh`](../../tools/test-harness/network-blocked/run.sh)
runs an arbitrary command in a child process with the network namespace
stripped:

```bash
tools/test-harness/network-blocked/run.sh anvil --no-tui status --verify --json
```

Under the harness the child has **no `lo` interface, no routes, no resolver
path**. Any DNS, TCP, UDP, ICMP, or raw-socket attempt fails at the OS layer —
not at the application layer — so the test cannot miss a "we caught the
exception and pretended to succeed" path.

Implementation notes:

- Linux `unshare -n -r` (user + network namespaces) is the v1 primitive. No
  sudo, no root, no Docker required.
- The script probes the kernel for namespace support at start. Restricted
  kernels (some Docker / CI hosts) cause it to exit `77` (skip).
- macOS / BSD have no unprivileged equivalent and currently fall through to the
  same `77` skip. Linux CI is the source of truth for the gate.

### Test surface

The active assertion lives at
[`crates/anvil-cli/tests/air_gapped.rs`](../../crates/anvil-cli/tests/air_gapped.rs):

| Test                                                     | Asserts                                                                                         |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `anvil_version_offline_succeeds_with_no_network`         | `anvil version --offline` exits 0 with no network                                               |
| `anvil_status_verify_json_exits_cleanly_with_no_network` | `anvil status --verify --json` finishes cleanly (any exit code; never killed by signal)         |
| `harness_is_executable_and_checked_in`                   | The harness script is on disk and executable — guards against accidental `chmod -x` regressions |

Run the suite manually with:

```bash
cargo test -p eddacraft-anvil --test air_gapped -- --nocapture
```

Or under CI's normal `cargo test` matrix on Linux.

## How to extend the gate

When you ship a new command on the MLP / INTL slate:

1. Add a `#[test]` to `crates/anvil-cli/tests/air_gapped.rs` that invokes the
   command through the harness.
2. Use `run_air_gapped(&[…])` so the platform-skip path is honoured.
3. Assert at least:
   - The process exited (status code is `Some`, not killed by signal).
   - Where the command is expected to succeed regardless of state,
     `out.status.success()`.

Reviewers: a PR that adds a new MLP / INTL command without extending this file
is incomplete.

## What this guarantee does NOT cover (v1)

- **Telemetry and update checks** — opt-in, off by default. When a user enables
  them they go to controlled endpoints; the harness doesn't run with them on.
- **Pack distribution** (vNext, post-`v0.7.0-beta`) — will be constrained to
  git-based fetch per ADR-036, so package pulls fall under git's own air-gap
  story (a local mirror is sufficient).
- **macOS / BSD** — the harness skips with exit 77 there. Linux CI is the gate;
  cross-platform extension is a documented follow-up (see MLP-017 footnote in
  `plans/modules/multilayer-protection.aps.md`).

## Provenance

- Filed 2026-05-13 as MLP-017 v1 scaffold (Wave 1, `RELEASE-PLAN.md`).
- Doctrine commitment per user direction 2026-05-07 (recorded in ADR-036).
- Hard release gate component for the `v0.7.0-beta` daemon-working slate; tied
  to MLP-009 protection-claim contract suite.
