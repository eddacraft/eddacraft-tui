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
pnpm -F @eddacraft/anvil-cli test -- --run
pnpm -F @eddacraft/anvil-cli build
```

CLI package smoke checks:

```bash
cd apps/anvil-cli
npm pack --json > /tmp/anvil-pack.json
TARBALL=$(node -e "console.log(JSON.parse(require('fs').readFileSync('/tmp/anvil-pack.json','utf8'))[0].filename)")

# Must run successfully from packed artifact
npx -y --package "./$TARBALL" anvil --help
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

For beta releases, tag as:

```bash
vX.Y.Z-beta.N
```

---

## 3) Monitor publish workflow

Watch run:

```bash
gh run list --repo EddaCraft/anvil-001 --limit 5
gh run view <run-id> --repo EddaCraft/anvil-001
```

Expected behaviour:

- Validation jobs pass (lint/typecheck/test/build).
- Publish step includes only `@eddacraft/anvil-cli` unless explicitly running
  manual workspace publish.

---

## 4) Post-release verification (required)

```bash
npm view @eddacraft/anvil-cli version
npx -y --package @eddacraft/anvil-cli anvil --help
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

---

## 6) Human comms template

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
