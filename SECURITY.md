# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in `eddacraft-tui`, **please
do not open a public GitHub issue**. Public reports can expose
downstream users of the crate before a fix is available.

Instead, report it privately using one of the following:

1. **GitHub Security Advisory (preferred):**
   <https://github.com/eddacraft/eddacraft-tui/security/advisories/new>
2. **Email:** security@eddacraft.com

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

CI runs `cargo audit` and `cargo deny check` on every push and
pull request; advisories against the dependency graph fail the
build.
