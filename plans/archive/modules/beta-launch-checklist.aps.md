<\!-- Archived: 2026-03-01 | Reason: All work items complete — v0.1.2-beta shipped -->

# Beta Launch Checklist — Anvil CLI v0.1.x

**Status:** Complete
**Owner:** Engineering
**Target Package:** `@eddacraft/anvil-cli` on npm
**Current Version:** 0.1.2-beta (in `apps/anvil-cli/package.json`)

---

## Part 1: Go / No-Go Criteria

These are the hard gates. Every item in **MUST SHIP** must be green before we
tag and publish. **SHOULD SHIP** items are strongly recommended but won't block
the release. **NICE TO HAVE** items can follow in 0.1.1.

### MUST SHIP (Release Blockers)

#### Security — Critical Findings (REVIEW.md)

- [x] **C1 — MCP workspace root validation.** Verify
  `validateWorkspaceRootAgainstServer()` in `validate-workspace.ts` is called in
  all 4 MCP tools (`check.tool.ts`, `gate.tool.ts`, `query-boundary.tool.ts`,
  `status.tool.ts`). Already implemented — needs verification testing.
- [x] **C2 — MCP newline injection.** Verify `\r`/`\n` stripping in
  `suppress.tool.ts:91` (`.replace(/[\r\n]+/g, ' ').trim()`). Already
  implemented — needs verification testing.
- [x] **C3 — MCP HTTP authentication.** Verify API key middleware via
  `ANVIL_MCP_API_KEY` in `streamable-http.ts:123-138`. Already implemented —
  needs verification testing.

#### Security — High Severity (Top 5)

- [x] **H1-runtime — OPA binary path override.** Verify `isFile()` +
  `accessSync` validation at `opa-binary-manager.ts:101-111`. Already
  implemented — needs verification testing.
- [x] **H2-runtime — Policy directory traversal.** Validate `policyDir` from
  config cannot escape `workspaceRoot` (`policy-loader.ts:71-72`).
- [x] **H1-storage — FileStorage path traversal.** Harden
  `FileStorage.resolvePath()` against `../` escapes (already has tests from
  TEST-002 — verify the fix is deployed).
- [x] **H1-adapters — Path traversal via external adapters.** Validate adapter
  output paths are confined to the workspace.
- [x] **H2-policy — Tar extraction path traversal.** Validate tar entry paths
  during bundle extraction to prevent writes outside target directory.

#### Core CLI Functionality

- [x] **`anvil init` works end-to-end** on Linux, macOS, and Windows (Git Bash).
  TUI wizard completes, `.anvilrc` is created, hooks are installed (or skipped
  with a message on Windows without Git Bash).
- [x] **`anvil check --all` returns correct results** on a real project. No
  crashes, no false positives from default config, exit codes are correct (0 =
  clean, 1 = violations found).
- [x] **`anvil check --changed` and `--staged` work** with current git state.
- [x] **`anvil watch --source` starts and stops cleanly.** Ctrl+C exits without
  orphan processes. File changes trigger re-analysis.
- [x] **`anvil doctor` and `anvil status` run without errors.**
- [x] **`anvil explain <id>` resolves** for all 7 built-in anti-pattern IDs.
- [x] **`anvil tutorial` completes** the full scan-watch-fix flow without
  errors.

#### Build & Test

- [x] **CI green on all 3 platforms.** `ci.yml` matrix passes: Node 20 + 22 on
  `ubuntu-latest`; Node 20 on `macos-latest` and `windows-latest`.
- [x] **All tests pass.** `pnpm test -- --run` exits 0.
- [x] **Lint clean.** `pnpm run lint:check` exits 0.
- [x] **Type check clean.** `pnpm run typecheck` exits 0.
- [x] **Build succeeds.** `pnpm build` produces `apps/anvil-cli/dist/` with a
  working `index.js` entry point.

#### Packaging & Distribution

- [x] **`npm publish --dry-run` succeeds** from `apps/anvil-cli/`. Verify the
  tarball contains `dist/`, `README.md`, and nothing else (no source, no tests,
  no `.env`).
- [x] **`files` field in package.json is correct.** Currently `["dist",
  "README.md"]` — confirm no sensitive files leak.
- [x] **`bin.anvil` resolves.** After `npm pack && npm install -g
  eddacraft-anvil-cli-0.1.0.tgz`, the `anvil` command is available and prints
  help.
- [x] **`engines.node >= 20.0.0` is enforced.** Attempting install on Node 18
  produces a clear error (or at minimum a warning).
- [x] **workspace:* dependencies resolve.** pnpm's `publishConfig` or the
  publish workflow correctly replaces `workspace:*` with real versions in the
  published tarball.
- [x] **No private packages leak.** Only `@eddacraft/anvil-cli` is published.
  All other workspace packages have `"private": true` or are not in `files`.

#### Documentation

- [x] **BETA-TESTER-QUICKSTART.md has real install instructions.** TODO
  placeholder replaced with `npm install -g @eddacraft/anvil-cli`.
- [x] **BETA.md known limitations are accurate.** Reviewed — version corrected,
  docs site marked live, broken links fixed.
- [x] **CHANGELOG.md reflects shipped features.** Reviewed — structure fixed,
  security hardening summary added, repo URLs corrected.
- [x] **README.md install section works.** "For Users (Future)" hedging removed,
  version updated to 0.1.0.

---

### SHOULD SHIP (Strongly Recommended)

#### Security — High Severity (Remaining)

- [x] **H3-runtime — Cache integrity.** Add HMAC to cache entries in
  `file-cache.ts:142-166` to prevent injection of false results.
- [x] **H4-runtime — OPA temp dir TOCTOU.** Replace `randomUUID()` temp dirs
  with `fs.mkdtemp()` in `opa-executor.ts:271,310`.
- [x] **H5-runtime — Bundle verifier env var exfiltration.** Restrict allowed
  env var names in `bundle-verifier.ts:380-389` to an explicit allowlist.
- [x] **H1-mcp — TOCTOU race in fix/suppress tools.** Add file locking or
  compare-and-swap to prevent concurrent modification.
- [x] **H2-mcp — Prompt injection.** Escape user inputs in prompt templates
  (`fix-violation.prompt.ts:29`, `suppress-violation.prompt.ts:29`).

#### Test Coverage Gaps

- [x] **TEST-007 — Config loader tests.** `packages/platform/config/src/
  loader.ts` has zero coverage.
- [x] **TEST-008 — Init error path tests.** `apps/anvil-cli/src/commands/
  init.ts` only has happy-path coverage.

#### Cross-Platform

- [x] **XPLAT-005 — Windows glob patterns.** Audit glob consumers for
  separator normalisation. Add a test with Windows-style paths.

#### Publish Workflow

- [x] **publish.yml uses `actions/create-release@v1`** which is deprecated.
  Migrate to `softprops/action-gh-release@v2` or `gh release create`.
- [x] **publish.yml should set `prerelease: true`** for beta tags. Currently
  creates a non-prerelease GitHub Release for all `v*` tags.

---

### NICE TO HAVE (0.1.1 Follow-Up)

- [x] Changeset-based versioning (`@changesets/cli` is configured but unused)
- [x] Provenance attestation on npm publish (`--provenance` flag)
- [ ] VS Code extension marketplace listing
- [x] Docs site live on Vercel (currently configured but not verified)
- [x] Website live on Vercel
- [x] CLI medium security findings (M1-M4) from Codex/OpenCode reviews
- [x] Remaining HIGH security findings from REVIEW.md (VS Code H1-H3 fixed;
  policy H1, adapters H2-H3, APS H2, runtime H6 still open)
- [ ] Branch protection rules on `main` requiring CI pass
- [ ] npm 2FA / automation token best practices documented

---

## Part 2: Release Process — Step by Step

### Phase 1: Pre-Release Validation

```
1.  Merge all security fixes to `main`
2.  Merge all release-blocking PRs to `main`
3.  Pull latest main locally:
      git checkout main && git pull origin main
4.  Full local validation:
      pnpm install --frozen-lockfile
      pnpm run lint:check
      pnpm run typecheck
      pnpm test -- --run
      pnpm build
5.  Smoke test the CLI locally:
      cd apps/anvil-cli
      node dist/index.js --version
      node dist/index.js --help
      node dist/index.js doctor
6.  Test npm pack:
      cd apps/anvil-cli
      npm pack --dry-run          # Review file list
      npm pack                    # Create tarball
      npm install -g ./eddacraft-anvil-cli-0.1.0.tgz
      anvil --version             # Should print 0.1.0
      anvil doctor                # Should run without errors
      npm uninstall -g @eddacraft/anvil-cli
7.  Verify CI is green on main (all 3 platforms, both Node versions)
8.  Review the go/no-go checklist above — every MUST SHIP item must be checked
```

### Phase 2: Version Bump & Tag

```
1.  Confirm version in apps/anvil-cli/package.json is correct:
      Should be "0.1.0" for initial beta (already set)
2.  Update CHANGELOG.md if needed:
      - Confirm [0.1.0] date is correct
      - Add any last-minute additions
3.  Commit version/changelog updates (if any):
      git add apps/anvil-cli/package.json CHANGELOG.md
      git commit -m "chore: prepare v0.1.0 release"
4.  Create annotated git tag:
      git tag -a v0.1.0 -m "v0.1.0 — initial beta release"
5.  Push commit and tag:
      git push origin main
      git push origin v0.1.0
```

### Phase 3: Automated Publish (CI)

Pushing the `v0.1.0` tag triggers `.github/workflows/publish.yml` which:

```
1.  Checks out the tagged commit
2.  Installs dependencies (pnpm install --frozen-lockfile)
3.  Runs lint, typecheck, and tests
4.  Builds all packages (pnpm build)
5.  Validates tag version matches apps/anvil-cli/package.json version
6.  Publishes @eddacraft/anvil-cli to npm (npm publish --access public)
7.  Creates a GitHub Release from the tag
```

**Prerequisites for this to work:**

- [x] `NPM_TOKEN` secret is configured in GitHub repo settings
  (Settings > Secrets and variables > Actions)
- [x] The npm token has publish permission for `@eddacraft` scope
- [x] `GITHUB_TOKEN` has `contents: write` permission (already in workflow)
- [x] Azure credentials are configured (or the `continue-on-error: true`
  fallback on Azure Login is acceptable)

### Phase 4: Post-Publish Verification

```
1.  Verify package is on npm:
      npm view @eddacraft/anvil-cli
      # Should show version 0.1.0, correct metadata
2.  Test install from registry:
      npm install -g @eddacraft/anvil-cli
      anvil --version    # 0.1.0
      anvil --help       # Shows all commands
      anvil doctor       # Runs without errors
3.  Test in a fresh project:
      mkdir /tmp/anvil-test && cd /tmp/anvil-test
      git init && npm init -y
      anvil init          # TUI wizard runs
      anvil check --all   # Returns results
4.  Verify GitHub Release exists:
      gh release view v0.1.0
      # Should show release notes and tag
5.  Verify docs site is live:
      curl -s https://docs.eddacraft.ai | head -1
6.  Verify website is live:
      curl -s https://anvil.eddacraft.ai | head -1
```

### Phase 5: Announcement & Distribution

```
1.  Update BETA-TESTER-QUICKSTART.md with real install command:
      npm install -g @eddacraft/anvil-cli
2.  Draft release announcement covering:
      - What Anvil does (one paragraph)
      - How to install (one command)
      - Link to quickstart guide
      - Link to GitHub issues for feedback
      - Known limitations (link to BETA.md)
3.  Distribute to beta testers via chosen channel(s)
```


---

## Part 3: Rollback Plan

If a critical issue is found post-publish:

```
1.  Attempt to unpublish (only guaranteed within 72 hours of publish
    AND if this version has no dependants):
      npm unpublish @eddacraft/anvil-cli@0.1.0
    If unpublish is blocked or after 72 hours, deprecate instead:
      npm deprecate @eddacraft/anvil-cli@0.1.0 "Critical issue found, use 0.1.1"

2.  Fix the issue on main

3.  Bump to 0.1.1:
      # Update apps/anvil-cli/package.json version to "0.1.1"
      git add apps/anvil-cli/package.json
      git commit -m "fix: <description of critical fix>"
      git tag -a v0.1.1 -m "v0.1.1 — hotfix for <issue>"
      git push origin main
      git push origin v0.1.1
    This triggers publish.yml again for the new version.
```

---

## Part 4: Secrets & Credentials Checklist

| Secret              | Where                 | Status    |
| ------------------- | --------------------- | --------- |
| `NPM_TOKEN`         | GitHub Actions secrets | [x] Ready |
| `GITHUB_TOKEN`      | Automatic             | [x] Ready |

**Note:** Azure credentials are used during the build step. The workflow has
`continue-on-error: true` on Azure Login, so missing credentials won't block
the publish — but any Azure-dependent build steps will be skipped.

---
