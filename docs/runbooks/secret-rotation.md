# Secret Rotation — Operator Runbook

| Type    | Authority     | Owner | Status | Freshness                                         |
| ------- | ------------- | ----- | ------ | ------------------------------------------------- |
| Runbook | Authoritative | SEC   | Live   | First filed 2026-06-16 against `main` for SEC-002 |

| Upstream                                                                                                                                          | Downstream                                                                                                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| [`infra/`](../../infra/) (Pulumi), [`apps/anvil-api/SECURITY.md`](../../apps/anvil-api/SECURITY.md), [`release-signing.md`](./release-signing.md) | [`vulnerability-response.md`](./vulnerability-response.md), [`dependency-audit-posture.md`](../guides/dependency-audit-posture.md) |

## TL;DR

Every long-lived secret has a home, an owner, and a review-by date. Rotation is
a runbook step, not tribal knowledge: mint the new value at its source, update
the store (GitHub Actions secret, Pulumi config, or Vercel env), redeploy/re-run
what consumes it, then retire the old value. A secret exposed in an incident is
rotated immediately, out of cadence (see the
[vulnerability-response runbook](./vulnerability-response.md)).

> **Confidence note (SEC-002):** the inventory and stores below are grounded in
> the repo's workflows and `infra/`. The exact Pulumi/Vercel _apply_ steps for
> the runtime secrets should be confirmed against the live infra on first use
> and this runbook corrected if they differ.

## Inventory

Names only — never record a secret **value** here or in any committed file. The
review cadence is the default rotation window; rotate sooner on any suspected
exposure.

| Secret(s)                                                                                                                  | Where it lives                              | Review cadence / by    | Owner |
| -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ---------------------- | ----- |
| `ANVIL_MINISIGN_PRIVATE_KEY` (release signing); licence signing keypair                                                    | GitHub Actions secret; key material offline | 12 months / 2027-06-16 | SEC   |
| `NPM_TOKEN`, `CRATES_IO_EDDACRAFT_TUI_TOKEN`, `WINGET_TOKEN`, `ANVIL_RELEASES_TOKEN`, `MIRROR_PUSH_TOKEN`, `GH_AUTH_TOKEN` | GitHub Actions secrets                      | 6 months / 2026-12-16  | SEC   |
| `EDDACRAFT_MIRROR_BOT_APP_ID` + `EDDACRAFT_MIRROR_BOT_PRIVATE_KEY` (GitHub App)                                            | GitHub Actions secrets + the GitHub App     | 12 months / 2027-06-16 | SEC   |
| `ARM_CLIENT_ID/SECRET/SUBSCRIPTION_ID/TENANT_ID`, `AZURE_STORAGE_ACCOUNT/KEY`, `PULUMI_CONFIG_PASSPHRASE`                  | GitHub Actions secrets + Azure / Pulumi     | 6 months / 2026-12-16  | SEC   |
| `TOKEN_PEPPER`, `DATABASE_URL` (Neon), admin tokens, github-device-crypto keys                                             | Pulumi config → Vercel env (anvil-api)      | 6 months / 2026-12-16  | SEC   |

## General rotation procedure

1. **Mint** the new value at its source (the registry, cloud provider, GitHub
   App, or key generator).
2. **Store** it in the right place — a GitHub Actions repository/organisation
   secret, a Pulumi config secret (`infra/`, encrypted under
   `PULUMI_CONFIG_PASSPHRASE`), or the app environment.
3. **Roll out** — re-run or redeploy the consumer so it picks up the new value
   (a GitHub Actions secret is read on the next run; a Pulumi-managed env needs
   a `pulumi up` and a redeploy).
4. **Verify** the consumer works on the new value (a publish dry-run, a signed
   build, an API health check).
5. **Retire** the old value at its source so it can no longer authenticate.

## Per-secret notes

- **Release signing (`ANVIL_MINISIGN_PRIVATE_KEY`) + licence keypair.** Follow
  the existing [release-signing runbook](./release-signing.md); the licence
  keypair is minted by
  [`scripts/generate-licence-keypair.sh`](../../scripts/generate-licence-keypair.sh).
  Rotating a signing key means consumers must trust the new public key —
  coordinate the public-key distribution, do not just swap the private key.
- **Registry/publish tokens.** Mint a new token in the registry (npm, crates.io,
  winget) or GitHub, update the Actions secret, and confirm with a publish
  dry-run before deleting the old token.
- **Azure / Pulumi.** Rotate the Azure service-principal secret in Azure AD,
  update the `ARM_*` Actions secrets, and re-run the infra workflow. Rotating
  `PULUMI_CONFIG_PASSPHRASE` re-encrypts the Pulumi config and is a heavier
  operation — plan it and confirm the apply succeeds.
- **anvil-api runtime (`TOKEN_PEPPER`, DB, admin tokens).** `TOKEN_PEPPER` has a
  dedicated **zero-downtime dual-pepper** procedure in
  [`apps/anvil-api/SECURITY.md`](../../apps/anvil-api/SECURITY.md) — use it
  rather than a hard swap, which would invalidate live tokens. Rotate
  `DATABASE_URL` (Neon) and admin tokens through the Pulumi-managed env and
  redeploy.

## On suspected exposure

Skip the cadence. Rotate the secret immediately following the procedure above,
then drive the incident through the
[vulnerability-response runbook](./vulnerability-response.md) and confirm the
old value is fully retired everywhere it was stored.
