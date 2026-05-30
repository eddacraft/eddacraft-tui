# Post-merge: fix-lint-no-mutate-markdown

PR: #NNN
Branch: `fix/lint-no-mutate-markdown`
APS: CLAWP-009 (Clawpatch finding — tracked via GitHub issue #1743, not an
active APS module)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Confirm `scripts.lint` in `package.json` no longer carries
      the trailing `--fix` and is now byte-identical to `scripts.lint:check`.
      (agent: yes — `node -p "const p=require('./package.json').scripts;
      p.lint===p['lint:check'] && !p.lint.includes('--fix')"` returns `true`)
- [ ] Step 2 — Confirm the auto-fix path is still reachable via
      `scripts.lint:md:fix` (`markdownlint --ignore-path .markdownlintignore
      . --fix`). (agent: yes — assert `scripts['lint:md:fix']` contains
      `--fix`)
- [ ] Step 3 — Confirm `pnpm lint` is idempotent: running it twice leaves a
      clean working tree (no auto-reformatted `.md` files). (agent: no —
      requires a full `pnpm install` + lint toolchain run; verify
      opportunistically on a developer machine)
- [ ] Step 4 — Close issue #1743 once merged (auto-closed by the `Fixes
      #1743` trailer). (agent: yes — confirm issue state is CLOSED)

## Notes

Single-token fix: `package.json:30` dropped `--fix` from the final
`markdownlint` step of `scripts.lint`. CI runs `lint:check` (the already
non-mutating form), so CI behaviour is unchanged — this only affects local
developer workflows, which were left with a dirty working tree because
`pnpm lint` auto-reformatted `.md` files in place.

Auto-fixing is unchanged and still available on demand via
`pnpm run lint:md:fix`. No source code, no tests, no APS module status to
advance — CLAWP-009 lives in the archived/terminal
`plans/modules/clawpatch-pre-tag-v0.7.0-beta.aps.md` findings tracker, which
is exempt from the active-corpus lint gate.

Provenance: Clawpatch finding
`fnd_sig-feat-release-4862937c51-dcb1_18d9dd2f14`; release council pass 2
verdict 2026-05-20 (defer-with-issue, kernel-maintainer).
