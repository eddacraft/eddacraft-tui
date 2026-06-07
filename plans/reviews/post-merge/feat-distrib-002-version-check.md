# Post-merge: feat-distrib-002-version-check

PR: #NNN
Branch: `feat/distrib-002-version-check`
APS: DISTRIB-002 (in module DISTRIB)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [x] Step 1 — Move DISTRIB module progress 1/5 → 2/5 and mark
      DISTRIB-002 Merged once this PR lands. (agent: yes — read
      `plans/archive/modules/distribution-and-update.aps.md` and update the
      progress count + DISTRIB-002 Status to Merged.)
- [ ] Step 2 — When the first v0.7.0-beta-or-later release with an
      attached `Security-Advisory: GHSA-…` line in its release body
      ships, confirm `anvil version --check` surfaces the advisory
      ID in both human and `--json` output on a clean install.
      Owner: release maintainer.
- [ ] Step 3 — When a downstream user runs `anvil status` on a
      pre-release machine running a version older than the latest tag,
      confirm the one-line "Update available" hint appears and that a
      second invocation within 24h does not re-print it. Owner: QA
      participant from N9 Boring-Week gate.

## Notes

- `--check` is opt-in for `anvil version` (one extra HTTP round-trip
  beyond the existing latest-version probe). `anvil status` and the
  watch TUI invoke the probe automatically and rate-limit the hint to
  once per 24h per advertised version via
  `<state-dir>/anvil/update-hint.json`.
- Opt out of the ambient hint with `ANVIL_DISABLE_UPDATE_HINT=1` in
  the environment. Documented in `anvil version --check --help`.
- Advisory IDs are recognised only when they match the GHSA / CVE /
  RUSTSEC schemes — Council-blocked false-positive on prose lines.
- The rendered hint line is ASCII-only. A unit test pins this so
  future edits can't reintroduce a `→` or `—` that would mojibake on
  Windows cp1252 consoles.
- Deferred Council MINORs:
  - Race condition on the JSON state file (two simultaneous
    `anvil status` invocations could both fire the hint). Acceptable
    for a convenience hint; revisit if it shows up in user reports.
  - Watch TUI keeps the first-frame `update_hint` for the lifetime of
    the session — a >24h `anvil watch` run will not refresh the hint
    for a new release shipped mid-session. Same trade-off as above.
  - When `include_advisories: true`, `compute_update_hint` spins up
    two sequential single-threaded tokio runtimes. Parallelising into
    one runtime would halve the worst-case 6s cold-start cost; left
    for a follow-up once profiling shows it matters.

## Cross-references

- ADR-045 (DISTRIB-001 minisign signing) — `VerifiedArtefact::trusted_comment`
  carries the `tag=` field that DISTRIB-005 (`anvil migrate`) will consume.
- DISTRIB-005 inherits a clean handoff: the releases-feed contract
  (`releases/tags/{v}` JSON body) is exercised by `fetch_advisories_for_version`
  and the parser is reusable. The release-body advisory convention is
  informal (no schema / signature over the claim); good enough for a
  hint surface, not strong enough for gating. A note on this lives in
  `commands::version` next to the parser.
