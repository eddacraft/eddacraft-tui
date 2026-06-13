# GitHub Device-Flow Login Operator Runbook

| Type    | Authority     | Owner     | Status | Freshness                                                                          |
| ------- | ------------- | --------- | ------ | ---------------------------------------------------------------------------------- |
| Runbook | Authoritative | GHCLIAUTH | Live   | Last reviewed 2026-06-11 against `apps/anvil-api/src/routes/auth-github-device.ts` |

| Upstream                                                                                                                                                                                                                                                  | Downstream                                                  |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `apps/anvil-api/src/routes/auth-github-device.ts`, `apps/anvil-api/src/index.ts`, `infra/src/vercel.ts`, `crates/anvil-cli/src/auth/device_flow.rs`, `plans/archive/modules/github-cli-auth.aps.md`, `plans/decisions/066-github-device-flow-cli-auth.md` | Operator cutover and incident triage for `anvil auth login` |

`anvil auth login` is the headless GitHub Device Authorisation Grant (RFC 8628)
flow for the Anvil CLI. The CLI never holds a GitHub client secret: it talks
only to the Anvil API, which brokers the credentialed upstream calls to GitHub.
This runbook covers the topology, the credentials and health signals, the
pre-cutover smoke step, the structured operational logs, rate limits and session
semantics, and troubleshooting.

For the broader incident triage flow, see
[observability triage](./observability-triage.md). For the admin surface used to
approve users that the flow gates as `awaiting_approval`, see
[admin CLI](./admin-cli.md).

## Flow topology

```
anvil CLI  ──POST /api/v1/auth/github-device/start──▶  Anvil API ──▶ github.com/login/device/code
   │                                                       │
   │  (prints user_code + verification_uri,                │
   │   user authorises in a browser)                       │
   │                                                       │
   └──POST /api/v1/auth/github-device/poll (every N s)──▶  Anvil API ──▶ github.com/login/oauth/access_token
                                                           │                 (then api.github.com/user for identity,
                                                           │                  then revoke the GitHub token)
                                                           ▼
                                                  mints the Anvil licence
```

- `/start` requests a device/user code pair from GitHub, persists a session row
  (hashed `poll_token`, encrypted `device_code`, no user binding), and returns
  the `user_code`, `verification_uri`, `interval`, `expiresIn`, and an opaque
  `pollToken` to the CLI.
- `/poll` exchanges the stored `device_code` with GitHub, derives the user
  solely from the resulting GitHub token, revokes that token immediately, runs
  the active-status gate, and mints the Anvil licence exactly once.

The CLI client lives in `crates/anvil-cli/src/auth/device_flow.rs`; the broker
route is `apps/anvil-api/src/routes/auth-github-device.ts`.

## Credentials and configuration

The flow runs against a dedicated **Anvil CLI** GitHub OAuth app — distinct from
the website OAuth app used by the browser sign-in path. The app **must be owned
by the `eddacraft` GitHub organisation, not a personal account**: the device
authorisation consent screen GitHub shows the user is branded with the app
owner's name, so a personally owned app makes CLI login look like it belongs to
an individual rather than to eddacraft. See
[Migrating the app to the eddacraft organisation](#migrating-the-app-to-the-eddacraft-organisation)
below if the live app is still under a personal account. Its credentials are
held in Azure Key Vault and wired into the Vercel deployment by
`infra/src/vercel.ts`:

- Key Vault secret `github-cli-client-id` → env `GITHUB_CLI_CLIENT_ID`
- Key Vault secret `github-cli-client-secret` → env `GITHUB_CLI_CLIENT_SECRET`

The client secret is used **only** for the broker-side token revoke call
(`DELETE /applications/{client_id}/token`). The device-grant exchange itself is
a public-client flow and sends `client_id` only — the secret never travels to
GitHub's device or token endpoints.

When either credential is absent, both `/start` and `/poll` fail closed with
HTTP 503 `github_device_flow_unavailable` and never call GitHub.

## Migrating the app to the eddacraft organisation

GitHub **OAuth Apps cannot be transferred** between a personal account and an
organisation (only the newer GitHub Apps can be transferred). Re-homing the
Anvil CLI app under `eddacraft` therefore means registering a fresh app under
the org and rotating the Key Vault secrets to it. Because the broker revokes the
GitHub access token immediately after deriving identity (no standing user
authorisation is retained) and minted Anvil licences are independent of the
OAuth app, this is a clean swap — users simply see the eddacraft-branded consent
screen on their next login. No re-onboarding is required.

1. **Register the new app under the org.** GitHub → `eddacraft` org → Settings →
   Developer settings → OAuth Apps → **New OAuth App**. Name it **Anvil CLI**.
   Set the homepage/callback URLs to match the retiring app (the callback is
   unused by the device grant but is required by the form).
2. **Tick "Enable Device Flow"** on the new app — without it GitHub rejects
   `/login/device/code` and `/start` returns 502 `github_unavailable`. This is
   the same one-checkbox gate the pre-cutover smoke step guards.
3. **Generate a client secret** and copy the **client ID** + secret.
4. **Rotate the Key Vault secrets** to the new app's values (see
   [Rotating the credentials](#rotating-the-credentials) below).
5. **Run the infra apply** that syncs Key Vault into the managed Vercel env,
   then redeploy `anvil-api` and run the
   [pre-cutover smoke step](#pre-cutover-smoke-step) against the target
   environment.
6. **Delete the old personally owned app** only after the smoke login confirms
   end-to-end against the new org-owned app.

### Rotating the credentials

The client ID and secret are Vercel env vars populated from Azure Key Vault by
Pulumi (`infra/src/vercel.ts` via `infra/src/keyvault.ts`) — nothing is
hardcoded in the CLI or the broker, so a rotation is a Key Vault update plus an
infra apply/redeploy, with no code change:

```sh
az keyvault secret set --vault-name <vault> --name github-cli-client-id     --value <new-client-id>
az keyvault secret set --vault-name <vault> --name github-cli-client-secret --value <new-client-secret>
```

Then run the appropriate `pulumi up` for the target stack so the managed Vercel
env vars are updated from Key Vault. Redeploy `anvil-api` after the env sync and
confirm `/health` reports `"githubCliCreds":"ok"` before completing cutover.

## Health and boot signals

`apps/anvil-api/src/index.ts` runs a boot probe and exposes a `githubCliCreds`
field on `/health`:

- At boot, `verifyGitHubCliCredentials()` runs; a failure logs
  `[boot] github cli credentials unavailable: …` to stderr but does not abort
  boot, so the remaining surfaces stay up.
- `/health` reports `githubCliCreds: "ok"` or `"unavailable"`. Because the
  device-flow login is the CLI default, missing CLI OAuth credentials are
  user-impacting and **gate overall health**: the endpoint returns
  `status: "degraded"` with HTTP 503 when they are absent (added in PR #2546).
  This gives ops a pre-user-impact signal.

Probe health quickly:

```sh
curl -sS https://<api-host>/api/v1/health
```

A healthy response carries `"status":"ok"` and `"githubCliCreds":"ok"`. A `503`
with `"githubCliCreds":"unavailable"` means the Key Vault secrets are not
reaching the deployment — fix that before cutover.

## Pre-cutover smoke step

Run this before switching the CLI default to the device flow, and after any
rotation of the Anvil CLI OAuth app credentials.

1. **Verify "Device Flow" is enabled on the Anvil CLI GitHub OAuth app.** In the
   GitHub OAuth app settings for the **Anvil CLI** app, confirm the **Enable
   Device Flow** option is ticked. Without it, GitHub rejects the
   `/login/device/code` request and `/start` returns 502 `github_unavailable` —
   a failure that looks like an outage but is a one-checkbox configuration miss.
   While there, confirm the app is **owned by the `eddacraft` organisation**
   (not a personal account) so the consent screen is correctly branded — see
   [Migrating the app to the eddacraft organisation](#migrating-the-app-to-the-eddacraft-organisation).
2. **Confirm the health signal is green:**

   ```sh
   curl -sS https://<api-host>/api/v1/health | grep -o '"githubCliCreds":"[a-z]*"'
   ```

   Expect `"githubCliCreds":"ok"`.

3. **Run a real end-to-end login** against the target environment:

   ```sh
   ANVIL_API_URL="https://<api-host>" anvil auth login
   ```

   Complete the browser authorisation when prompted. A successful run prints the
   `user_code`, opens the verification URI, polls, and ends with a confirmed
   session. If it ends in `expired`, `declined`, or a 502, do **not** cut over —
   work the troubleshooting table below first.

Only proceed with the cutover once the smoke login confirms end-to-end.

## Rate limits and the cross-instance poll gate

- `/start` is rate limited per-IP (10 / 60s) and globally (60 / 60s).
- `/poll` is rate limited per-IP (60 / 60s) and globally (300 / 60s). A
  well-behaved CLI polls roughly 12 times per minute.
- The real per-token gate is the atomic DB claim `claimGithubDevicePoll`: at
  most one API instance exchanges with GitHub per `device_code` per interval
  window, even across instances. When the claim is lost to the interval gate,
  `/poll` answers 429 `slow_down` with `retryAfter` set to the session interval.
- GitHub's own RFC 8628 `slow_down` is relayed through as a 429 with a clamped
  `retryAfter` (bounded to 1–3600s) so a hostile or broken upstream interval
  cannot drive the CLI into a tight loop or an endless sleep.

## Session semantics

- The session expires at `expires_at` (GitHub's `expires_in`, typically ~900s);
  the upstream value is bounded to defend the Date arithmetic.
- The licence is **minted exactly once and is re-returnable within TTL**: a lost
  poll response must not turn a success into a false `expired`. A repeated poll
  with the same `pollToken` re-returns the stored minted session (decrypted
  under the client-held poll token) rather than re-minting. Past TTL, or when
  the payload will not decrypt, it fails closed to `expired`.
- A concurrent mint race is resolved by re-reading and re-returning the winner's
  stored session; only one mint is ever recorded.

## Structured operational logs

The route emits **ungated** structured `console.info` lines (one JSON object per
line) at every upstream-call outcome, so production can be triaged without
enabling debug. Each line carries `ts`, `ns` (`anvil:auth-github-device`),
`event`, and flat operational fields (`outcome`, `httpStatus`, `errorClass`,
`ms`, etc.). **No secret values are ever logged** — not `access_token`,
`device_code`, `poll_token`, the minted licence, the OAuth client secret, or
user emails. Only presence, latency, and class.

Event names and their outcomes:

- `device_code.upstream` (the `/start` call to GitHub):
  - `outcome: "ok"` — device code obtained (with `intervalS`, `expiresIn`, `ms`)
  - `outcome: "fetch_error"` — transport failure / timeout (`errorClass`, `ms`)
  - `outcome: "non_ok"` — GitHub returned a non-2xx (`httpStatus`, `ms`)
  - `outcome: "malformed_body"` — response failed schema validation
- `token_exchange.upstream` (the `/poll` exchange to GitHub):
  - `outcome: "ok"` — token obtained
  - `outcome: "slow_down"` — RFC 8628 `slow_down` (with `retryAfter`)
  - `outcome: "expired"` — RFC 8628 `expired_token`
  - `outcome: "declined"` — RFC 8628 `access_denied`
  - `outcome: "unrecognised_error"` — an unmapped RFC error body
  - `outcome: "fetch_error"` / `"non_ok"` / `"malformed_body"` — transport,
    HTTP, or schema failure
- `identity.upstream` (the `api.github.com/user` identity fetch — `ms` covers
  the fetch only, not the subsequent token revocation):
  - `outcome: "ok"` or `outcome: "fetch_error"`

RFC 8628 `authorization_pending` is deliberately **absent** from the info
stream: it is the normal state every ~5 s per session and would drown the
terminal-outcome signal. Per-poll pending granularity is on the gated debug
stream (`ANVIL_DEBUG=1`).

- `login.outcome` (the terminal `/poll` result):
  - `outcome: "minted"` — licence issued (with `isNewPending`, `didFirstLink`)
  - `outcome: "blocked"` — non-active user gated (with `userStatus`)
  - `outcome: "link_conflict"` — github_id link conflict, failed closed
  - `outcome: "link_error"` — account-linking failure

To read these in production, filter the platform log stream on
`anvil:auth-github-device` (the `ns` field) and inspect the `event` and
`outcome` fields. For the full per-step diagnostic trace (sanitised), set
`ANVIL_DEBUG=1` in the API environment to enable the gated `console.debug`
output as well — the info logs above are always on regardless.

## Troubleshooting

- **`/start` returns 502 `github_unavailable` consistently.** Most often the
  Anvil CLI OAuth app does not have **Device Flow enabled** — re-run the
  pre-cutover smoke step. Otherwise check `device_code.upstream` info lines for
  `non_ok` (GitHub-side) versus `fetch_error` (transport/timeout).
- **`/start` or `/poll` returns 503 `github_device_flow_unavailable`.** The
  `GITHUB_CLI_CLIENT_ID` / `GITHUB_CLI_CLIENT_SECRET` env vars are not reaching
  the deployment. Confirm the Key Vault secrets and `infra/src/vercel.ts`
  wiring, then check `/health` reports `"githubCliCreds":"ok"`.
- **`/health` is `degraded` with `"githubCliCreds":"unavailable"`.** Same root
  cause as above — fix the credentials before cutover.
- **CLI login ends in `expired`.** The user did not authorise within the device
  code TTL (~15 min), or the session row expired. Re-run `anvil auth login`.
- **CLI login ends in `declined`.** The user pressed cancel/deny in the GitHub
  browser prompt. Re-run and complete the authorisation.
- **CLI login ends in `awaiting_approval`.** The GitHub identity resolved to a
  non-active Anvil user. Approve them via the admin surface (see
  [admin CLI](./admin-cli.md)); the audit log records a `github_oauth_blocked`
  row.
- **CLI login fails with `github_authentication_failed` (401).** The identity
  fetch failed, the account has no verified primary email, or a `github_id` link
  conflict was hit. Check the `identity.upstream` and `login.outcome` info lines
  and the audit log for `github_oauth_link_conflict`.
- **Repeated 429 `slow_down`.** Expected back-off — the CLI honours `retryAfter`
  automatically. Sustained 429s with no progress indicate either the
  cross-instance gate firing under heavy concurrency or a misbehaving poller.
- **Repeated 429 `slow_down` with no `login.outcome` info line ever appearing.**
  The mint-race loser path: an instance lost the store-mint race and the re-read
  found no winner yet. Harmless if it resolves within a poll interval;
  persistent occurrences point at a stuck winner row — inspect the
  `github_device_sessions` row for the session's `minted_*` columns.

## `--otp` fallback

When the device flow is unavailable (no browser, GitHub outage, or an account
without GitHub linkage), operators and users can fall back to the email
one-time-passcode login:

```sh
anvil auth login --otp
```

This path uses `/api/v1/auth/otp/request` and `/api/v1/auth/otp/verify` instead
of the GitHub broker, and is independent of the Anvil CLI OAuth app and its Key
Vault secrets — so it remains available even when `githubCliCreds` is
`unavailable`.

## Related

- Module plan: `plans/archive/modules/github-cli-auth.aps.md`
- Decision record: `plans/decisions/066-github-device-flow-cli-auth.md`
- [Observability triage](./observability-triage.md)
- [Admin CLI operator runbook](./admin-cli.md)
- [CLI surface reference](./cli-surface.md)
