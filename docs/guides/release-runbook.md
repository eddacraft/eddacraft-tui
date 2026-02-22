# Anvil CLI Release Runbook

Purpose: ship `@eddacraft/anvil-cli` safely and consistently without
accidentally publishing internal workspace packages.

## Release policy (current)

- **Default:** publish **CLI only**.
- **Beta tags (`v*-beta*`):** forced **CLI only**.
- **Workspace packages:** publish only via manual workflow dispatch with
  `publish-workspace=true`.

Workflow source of truth: `.github/workflows/publish.yml`.

---

## 1) Preflight checklist (required)

From repo root:

```bash
pnpm install --frozen-lockfile
pnpm run lint:check
pnpm run typecheck
pnpm run test -- --run
pnpm build
```

Publish dry run (catches missing files, bad metadata):

```bash
pnpm -F @eddacraft/anvil-cli publish --dry-run --access public --no-git-checks
```

CLI package smoke checks:

```bash
# Run from repo root — uses subshell to avoid changing directory
(
  cd apps/anvil-cli
  npm pack --json > /tmp/anvil-pack.json
  TARBALL=$(node -e "console.log(JSON.parse(require('fs').readFileSync('/tmp/anvil-pack.json','utf8'))[0].filename)")
  npx -y --package "./$TARBALL" anvil --help
)
```

Sanity assertions before release:

- `apps/anvil-cli/package.json` version is correct.
- No `workspace:*` in published runtime metadata expectations.
- `CHANGELOG.md` has release notes.

---

## 2) Version + tag

1. Bump `apps/anvil-cli/package.json` version.
2. Update `CHANGELOG.md`.
3. Commit to `main`.
4. Create tag matching CLI version:

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

For beta releases, either format works (both are matched by the workflow):

```bash
vX.Y.Z-beta      # e.g. v0.1.2-beta
vX.Y.Z-beta.N    # e.g. v0.1.2-beta.0
```

---

## 3) Monitor publish workflow

Watch run in real time:

```bash
gh run list --repo EddaCraft/anvil-001 --limit 5
gh run watch <run-id> --repo EddaCraft/anvil-001
```

Or inspect a completed run:

```bash
gh run view <run-id> --repo EddaCraft/anvil-001 --log-failed
```

Expected behaviour:

- Validation jobs pass (lint/typecheck/test/build).
- Publish step includes only `@eddacraft/anvil-cli` unless explicitly running
  manual workspace publish.

---

## 4) Post-release verification (required)

Verify the specific version landed (replace with your version):

```bash
npm view @eddacraft/anvil-cli@X.Y.Z version
npx -y --package @eddacraft/anvil-cli@X.Y.Z anvil --help
```

Confirm internal packages were **not** published unintentionally:

```bash
for p in \
  @eddacraft/anvil-core \
  @eddacraft/anvil-aps \
  @eddacraft/anvil-policy \
  @eddacraft/anvil-runtime \
  @eddacraft/anvil-adapters \
  @eddacraft/anvil-kindling-integration; do
  printf "%s: " "$p"
  npm view --prefer-online "$p" version 2>/dev/null || echo "not found"
done
```

---

## 5) Fast incident playbook

### If login fails for testers

- Verify API health + auth endpoint.
- If needed as immediate fallback:

```bash
export ANVIL_API_URL=https://eddacraft-api.vercel.app
```

### If wrong package(s) were published

1. Deprecate immediately with warning text.
2. If within npm window and approved, unpublish (`--force`) intentionally.
3. Patch workflow/config before next tag.

### If a bad version needs to be retracted

1. Delete the git tag locally and remotely:

```bash
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
```

2. Deprecate or unpublish the npm version (see above).
3. Fix the issue, bump to a new version, and re-release.

---

## 6) Known gotchas

- **`--provenance` requires `id-token: write`** — The publish workflow uses
  `--provenance` for npm supply chain attestation. This requires the GitHub
  Actions `id-token: write` permission (already set in the workflow). If
  provenance fails, check that the repo/org settings allow OIDC token
  generation.

---

## 7) Human comms template

After successful release, send:

- version + npm link
- one-line install command
- one-line auth/login command
- known temporary workarounds (if any)

Example:

```text
Anvil CLI vX.Y.Z is live: https://www.npmjs.com/package/@eddacraft/anvil-cli
Install: npm i -g @eddacraft/anvil-cli@X.Y.Z
Login: anvil login --token <token>
```
