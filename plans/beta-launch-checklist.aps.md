# Beta Launch Checklist — Anvil CLI v0.1.0

**Status:** Draft
**Owner:** Engineering
**Target Package:** `@eddacraft/anvil-cli` on npm
**Current Version:** 0.1.0 (in `apps/anvil-cli/package.json`)

---

## Part 1: Go / No-Go Criteria

These are the hard gates. Every item in **MUST SHIP** must be green before we
tag and publish. **SHOULD SHIP** items are strongly recommended but won't block
the release. **NICE TO HAVE** items can follow in 0.1.1.

### MUST SHIP (Release Blockers)

#### Security — Critical Findings (REVIEW.md)

- [ ] **C1 — MCP workspace root validation.** Validate `workspaceRoot` against
  a server-configured allowlist in all 4 MCP tools (`check.tool.ts`,
  `gate.tool.ts`, `query-boundary.tool.ts`, `status.tool.ts`). Prevents
  arbitrary directory access from malicious MCP clients.
- [ ] **C2 — MCP newline injection.** Strip `\r` and `\n` from `reason`
  parameter in `suppress.tool.ts:87` before interpolating into source comments.
  Prevents code injection.
- [ ] **C3 — MCP HTTP authentication.** Add API key or mutual TLS
  authentication plus CORS restrictions to
  `streamable-http.ts:41-105`. Prevents unauthenticated tool invocation.

#### Security — High Severity (Top 5)

- [ ] **H1-runtime — OPA binary path override.** Validate `ANVIL_OPA_PATH` is a
  regular file (not symlink) with expected permissions, or remove the env var
  override entirely (`opa-binary-manager.ts:95-102`).
- [ ] **H2-runtime — Policy directory traversal.** Validate `policyDir` from
  config cannot escape `workspaceRoot` (`policy-loader.ts:71-72`).
- [ ] **H1-storage — FileStorage path traversal.** Harden
  `FileStorage.resolvePath()` against `../` escapes (already has tests from
  TEST-002 — verify the fix is deployed).
- [ ] **H1-adapters — Path traversal via external adapters.** Validate adapter
  output paths are confined to the workspace.
- [ ] **H2-policy — Tar extraction path traversal.** Validate tar entry paths
  during bundle extraction to prevent writes outside target directory.

#### Core CLI Functionality

- [ ] **`anvil init` works end-to-end** on Linux, macOS, and Windows (Git Bash).
  TUI wizard completes, `.anvilrc` is created, hooks are installed (or skipped
  with a message on Windows without Git Bash).
- [ ] **`anvil check --all` returns correct results** on a real project. No
  crashes, no false positives from default config, exit codes are correct (0 =
  clean, 1 = violations found).
- [ ] **`anvil check --changed` and `--staged` work** with current git state.
- [ ] **`anvil watch --source` starts and stops cleanly.** Ctrl+C exits without
  orphan processes. File changes trigger re-analysis.
- [ ] **`anvil doctor` and `anvil status` run without errors.**
- [ ] **`anvil explain <id>` resolves** for all 7 built-in anti-pattern IDs.
- [ ] **`anvil tutorial` completes** the full scan-watch-fix flow without
  errors.

#### Build & Test

- [ ] **CI green on all 3 platforms.** `ci.yml` matrix passes on
  `ubuntu-latest`, `macos-latest`, `windows-latest` for Node 20 and 22.
- [ ] **All 3,982+ tests pass.** `pnpm test -- --run` exits 0.
- [ ] **Lint clean.** `pnpm run lint:check` exits 0.
- [ ] **Type check clean.** `pnpm run typecheck` exits 0.
- [ ] **Build succeeds.** `pnpm build` produces `apps/anvil-cli/dist/` with a
  working `index.js` entry point.

#### Packaging & Distribution

- [ ] **`npm publish --dry-run` succeeds** from `apps/anvil-cli/`. Verify the
  tarball contains `dist/`, `README.md`, and nothing else (no source, no tests,
  no `.env`).
- [ ] **`files` field in package.json is correct.** Currently `["dist",
  "README.md"]` — confirm no sensitive files leak.
- [ ] **`bin.anvil` resolves.** After `npm pack && npm install -g
  eddacraft-anvil-cli-0.1.0.tgz`, the `anvil` command is available and prints
  help.
- [ ] **`engines.node >= 20.0.0` is enforced.** Attempting install on Node 18
  produces a clear error (or at minimum a warning).
- [ ] **workspace:* dependencies resolve.** pnpm's `publishConfig` or the
  publish workflow correctly replaces `workspace:*` with real versions in the
  published tarball.
- [ ] **No private packages leak.** Only `@eddacraft/anvil-cli` is published.
  All other workspace packages have `"private": true` or are not in `files`.

#### Documentation

- [ ] **BETA-TESTER-QUICKSTART.md has real install instructions.** The current
  file has a `<!-- TODO: Finalise npm package availability -->` placeholder at
  line 16. Replace with `npm install -g @eddacraft/anvil-cli`.
- [ ] **BETA.md known limitations are accurate.** Review against current state.
- [ ] **CHANGELOG.md reflects shipped features.** Verify nothing is listed that
  doesn't actually work.
- [ ] **README.md install section works.** A new user can copy-paste commands
  and get a working install.

---

### SHOULD SHIP (Strongly Recommended)

#### Security — High Severity (Remaining)

- [ ] **H3-runtime — Cache integrity.** Add HMAC to cache entries in
  `file-cache.ts:142-166` to prevent injection of false results.
- [ ] **H4-runtime — OPA temp dir TOCTOU.** Replace `randomUUID()` temp dirs
  with `fs.mkdtemp()` in `opa-executor.ts:271,310`.
- [ ] **H5-runtime — Bundle verifier env var exfiltration.** Restrict allowed
  env var names in `bundle-verifier.ts:380-389` to an explicit allowlist.
- [ ] **H1-mcp — TOCTOU race in fix/suppress tools.** Add file locking or
  compare-and-swap to prevent concurrent modification.
- [ ] **H2-mcp — Prompt injection.** Escape user inputs in prompt templates
  (`fix-violation.prompt.ts:29`, `suppress-violation.prompt.ts:29`).

#### Test Coverage Gaps

- [ ] **TEST-007 — Config loader tests.** `packages/platform/config/src/
  loader.ts` has zero coverage.
- [ ] **TEST-008 — Init error path tests.** `apps/anvil-cli/src/commands/
  init.ts` only has happy-path coverage.

#### Cross-Platform

- [ ] **XPLAT-005 — Windows glob patterns.** Audit glob consumers for
  separator normalisation. Add a test with Windows-style paths.

#### Publish Workflow

- [ ] **publish.yml uses `actions/create-release@v1`** which is deprecated.
  Migrate to `softprops/action-gh-release@v2` or `gh release create`.
- [ ] **publish.yml should set `prerelease: true`** for beta tags. Currently
  creates a non-prerelease GitHub Release for all `v*` tags.

---

### NICE TO HAVE (0.1.1 Follow-Up)

- [ ] Changeset-based versioning (`@changesets/cli` is configured but unused)
- [ ] Provenance attestation on npm publish (`--provenance` flag)
- [ ] VS Code extension marketplace listing
- [ ] Docs site live on Vercel (currently configured but not verified)
- [ ] Website live on Vercel
- [ ] Remaining 15 HIGH security findings from REVIEW.md
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

- [ ] `NPM_TOKEN` secret is configured in GitHub repo settings
  (Settings > Secrets and variables > Actions)
- [ ] The npm token has publish permission for `@eddacraft` scope
- [ ] `GITHUB_TOKEN` has `contents: write` permission (already in workflow)
- [ ] Azure credentials are configured (or the `continue-on-error: true`
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
5.  Verify docs site is live (if deployed):
      curl -s https://docs.eddacraft.com | head -1
6.  Verify website is live (if deployed):
      curl -s https://anvil.eddacraft.com | head -1
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
4.  Monitor GitHub issues for the first 48 hours
```

---

## Part 3: Rollback Plan

If a critical issue is found post-publish:

```
1.  Unpublish (within 72 hours of publish):
      npm unpublish @eddacraft/anvil-cli@0.1.0
    OR deprecate (after 72 hours):
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
| `NPM_TOKEN`         | GitHub Actions secrets | [ ] Ready |
| `GITHUB_TOKEN`      | Automatic             | [x] Ready |
| `ARM_CLIENT_ID`     | GitHub Actions secrets | [ ] Ready |
| `ARM_CLIENT_SECRET` | GitHub Actions secrets | [ ] Ready |
| `ARM_SUBSCRIPTION_ID` | GitHub Actions secrets | [ ] Ready |
| `ARM_TENANT_ID`     | GitHub Actions secrets | [ ] Ready |

**Note:** Azure credentials are used during the build step. The workflow has
`continue-on-error: true` on Azure Login, so missing credentials won't block
the publish — but any Azure-dependent build steps will be skipped.

---

## Part 5: Post-Launch Monitoring (First 30 Days)

### Week 1

- [ ] Daily triage of new GitHub issues
- [ ] Respond to all bug reports within 24 hours
- [ ] Track install counts via `npm info @eddacraft/anvil-cli`
- [ ] Monitor for crash reports in issues

### Week 2–4

- [ ] Ship 0.1.1 patch with top reported bugs
- [ ] Collect feedback themes for 0.2.0 planning
- [ ] Update known limitations in BETA.md
- [ ] Decide on VS Code extension marketplace publish timing

### Success Metrics

- Installs: Track weekly via npm stats
- Issues: < 5 critical bugs in first 2 weeks
- Onboarding: > 50% of testers complete `anvil init` successfully
- Retention: Testers run `anvil check` more than once

---

## Summary: What Blocks the Release

| Category       | Blocking Items | Status |
| -------------- | -------------- | ------ |
| Security (C)   | 3 critical MCP fixes | [ ] |
| Security (H)   | 5 high-severity fixes | [ ] |
| Core CLI       | 7 functional checks | [ ] |
| Build & Test   | 5 CI/build checks | [ ] |
| Packaging      | 6 distribution checks | [ ] |
| Documentation  | 4 doc updates | [ ] |
| **Total**      | **30 blocking items** | |

No tag is cut until all 30 items are checked off.
