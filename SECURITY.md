# Security Policy

> **`eddacraft-tui` is mirrored from the Anvil monorepo.** The
> canonical source is at
> [`anvil-001:crates/eddacraft-tui/`](https://github.com/eddacraft/anvil-001/tree/main/crates/eddacraft-tui)
> (private). The reporting channels below route to the same
> maintainer team regardless of which surface you reach us through.

## Reporting a Vulnerability

If you discover a security vulnerability in `eddacraft-tui`, **please
do not open a public GitHub issue**. Public reports can expose
downstream users of the crate before a fix is available.

Instead, report it privately using one of the following:

1. **GitHub Security Advisory (preferred):**
   <https://github.com/eddacraft/eddacraft-tui/security/advisories/new>.
   This routes to the public-mirror security tab; maintainers
   monitor it alongside the canonical source.
2. **Email:** security@eddacraft.com.

Please include:

- A description of the issue and the affected component
- Steps to reproduce, or a proof-of-concept if available
- The crate version(s) affected
- Your assessment of the impact (e.g. RCE, DoS, information disclosure)

## What to expect

- **Acknowledgement** within 3 business days of receipt.
- **Initial assessment** within 7 business days, including whether we
  can reproduce the issue and our tentative severity rating.
- **Fix timeline** communicated after assessment. Critical issues are
  prioritised above all other work.
- **Coordinated disclosure.** We will agree a disclosure date with
  you and credit you in the advisory (unless you request otherwise).

## Supported Versions

Security fixes are backported only to the latest published minor
release on crates.io. Older minor versions are not patched — users
should upgrade.

| Version | Supported |
| ------- | --------- |
| 0.2.x   | ✔︎         |
| < 0.2   | ✘         |

## Scope

This policy covers vulnerabilities in the `eddacraft-tui` crate
itself. Vulnerabilities in direct dependencies should be reported
upstream; we will update our dependencies promptly once a fix is
released.

We treat dependencies in two trust tiers:

- **Mature, broadly-vetted ecosystem crates** — `ratatui`, `crossterm`,
  `unicode-width`, `textwrap`, `image`, `ratatui-image`,
  `tui-big-text`. Tracked via `cargo audit` and updated on release.
- **Lower-bus-factor or proc-macro crates** — currently `animate`
  (and its `animate-core` / `animate-macros` companion crates).
  Pinned to exact versions because patch updates can land
  build-time code execution. Bumps go through manual review and
  `cargo audit`. The animation API surface is shimmed via
  [`crate::animation`] so the underlying engine can be swapped
  without breaking downstream callers.

## Fix routing

Fixes land in the **canonical Anvil source first** (per
D-TUIR-009 / [`docs/policies/eddacraft-tui-mirror.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/policies/eddacraft-tui-mirror.md)),
propagate to the public mirror within one workflow run, and reach
crates.io via the publish workflow on a tag push. There is no "fix
on the mirror first" path — even under time pressure, that path
produces drift the daily mirror-drift-check job (D-TUIR-018) flags
within 24 hours.

## CI

`cargo deny check` and `cargo audit`-equivalent advisory gates run
on every push and PR to the canonical Anvil source via the
workspace-wide jobs in `rust.yml`; advisories against the
dependency graph fail the build. Full Anvil-side gate contracts
are documented in
[`docs/policies/eddacraft-tui-mirror.md`](https://github.com/eddacraft/anvil-001/blob/main/docs/policies/eddacraft-tui-mirror.md).
