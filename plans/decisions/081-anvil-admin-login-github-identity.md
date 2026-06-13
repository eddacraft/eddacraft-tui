# ADR-081: `anvil admin login` — fold admin auth onto the GitHub identity

## Status

Proposed

## Date

2026-06-13

## Context

Admin authentication and user authentication are today two **completely
separate credential systems** with no overlap:

- **User auth** (ADR-066, GHCLIAUTH): `anvil auth login` runs the GitHub Device
  Authorization Grant, brokered through `anvil-api`, and mints an **ES256
  licence JWT** stored in `~/.config/anvil/credentials.json`. TTL 7 days, with a
  90-day rotating refresh token (`session.ts`), so it silently auto-refreshes —
  the operator never re-keys. Identity is the GitHub account
  (`beta_users.github_id`, `bigint UNIQUE`, linked on first login). The licence
  claims (`LicenceClaims`: `sub, email, identity, org, tier, scopes[], seats`)
  carry **no admin role**.
- **Admin auth** (ADMINCLIH): `anvil admin …` sends a **static bearer key** as
  `Authorization: Bearer`. The middleware (`admin-auth.ts`) validates it by
  HMAC-SHA-256(pepper, bearer) against the `admin_keys` table
  (`id, hashed_key UNIQUE, actor_email, note, created_at, revoked_at`;
  Pulumi-provisioned, append-only, audited in `admin_keys_audit`) or against a
  shared `ADMIN_KEY`. The key has **no TTL** — it is a long-lived static secret,
  killed only by setting `revoked_at` or rotating. CIB-070 added
  `anvil admin auth set key` so operators can store the key locally instead of
  re-`export`-ing it every shell, but it is still a static secret they must
  obtain and hold.

The operator pain that motivates this: managing a second, manual credential.
The desire is to **log in for admin the same way as for a user** — run a device
flow, prove the GitHub identity, get admin capability that auto-refreshes — with
**no static key to fetch, store, or remember**. ADR-076 deferred a general
"staff axis / RBAC"; this is the first concrete staff role and forces the
question of how a GitHub identity earns elevated authority.

A decision is needed now because (a) the GHCLIAUTH device-flow infrastructure
exists and is proven, making this cheap to build on; and (b) it changes the
admin security posture (what a leaked local credential can do), so the trade-off
must be recorded before code, per the ADR process.

## Decision

Add **`anvil admin login`**: a GitHub device-flow login, gated on a **staff
allowlist keyed on `github_id`**, that mints a **separate, short-lived,
admin-scoped licence**. `/admin/*` gains a third auth path that accepts this
licence; the static key remains as the break-glass / CI fallback.

Concretely:

1. **New CLI verb `anvil admin login`** (and `anvil admin logout` /
   `anvil admin whoami`). It runs the **same** device flow as `anvil auth login`
   against a **dedicated admin start/poll surface**
   (`POST /api/v1/auth/admin/github-device/{start,poll}`), so a normal user
   login can never accidentally yield admin. Credentials are written to a
   **separate** file, `~/.config/anvil/admin-credentials.json` (mode `0600`) —
   not the everyday `credentials.json`.

2. **Staff allowlist on `github_id`.** Extend the existing per-operator model
   rather than invent a parallel one: add a nullable `github_id bigint` column
   to `admin_keys` (an "admin identity" row may carry a `github_id` with no
   `hashed_key`), provisioned and revoked the same Pulumi-driven, append-only,
   audited way as today's keys. The admin poll endpoint mints **only** when the
   authenticating `github_id` matches an active (`revoked_at IS NULL`) admin
   identity.

3. **Admin-scoped licence.** Reuse the ES256 `signLicence` path with an explicit
   **admin claim** (`roles: ["admin"]`) and a **short TTL** (proposed: 12–24h
   access, 7-day refresh — deliberately shorter than the user licence so a
   leaked admin credential expires fast and re-login is routine). The refresh
   token uses the same rotating-family theft detection as user sessions.

4. **Middleware accepts the admin licence — with a request-time allowlist
   re-check.** `admin-auth.ts` gains a path: a valid ES256 licence bearing
   `roles:["admin"]` whose `github_id` is **still** an active admin identity at
   request time. The claim alone is not sufficient — re-checking the allowlist
   per request means **removing someone from the allowlist revokes even an
   unexpired licence immediately**, closing the static key's worst property
   (valid until manually found and revoked). On success,
   `adminActor = <github-identity email>`, `adminAuthMethod = 'github_identity'`;
   all existing audit attribution keeps working.

5. **Static key stays.** The shared key and per-operator HMAC keys are unchanged
   and remain the **CI / automation / break-glass** path. This is additive and
   non-breaking.

Out of scope (named so the boundary is explicit): step-up re-auth for
destructive actions; a general RBAC role system (ADR-076's deferred staff axis);
folding admin into the *everyday* user licence.

## Rationale

The recommended approach gives the operator exactly the ergonomics they asked
for — device-flow login, auto-refresh, no key to manage — while keeping admin
authority **out of the everyday user session** and **revocable in real time**.
Putting admin on a *separate, short-lived* licence with a *per-request allowlist
check* makes the blast radius of a leaked credential strictly smaller than
today's static key, not larger.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **A — separate `anvil admin login` → admin-scoped licence (chosen)** | Device-flow ergonomics + auto-refresh; admin absent from the daily `credentials.json`; per-request allowlist check = instant revocation; short TTL bounds leak; reuses GHCLIAUTH broker + ES256 signing + per-operator audit | Largest build (new endpoint surface, `github_id` allowlist, middleware path, new credential file); introduces an `admin` claim and a staff-allowlist concept ahead of full RBAC |
| **B — `admin` scope on the everyday user licence** | Smallest UX (one `anvil auth login`, admin always present) | **Biggest blast radius** — every `credentials.json` would grant admin; a leaked daily session is admin; conflates user/admin lifetimes; no separation or step-up. Rejected on security posture |
| **C — device flow *delivers the existing static key* into config** | Smallest change; admin validation path unchanged; security model identical to today | The server stores only the key **hash**, so it must mint+return a fresh static key — re-introducing a long-lived secret on disk with **no auto-expiry/refresh** (the very thing the operator wanted gone). Effectively CIB-070 + a login wrapper, not a session model. Rejected as not solving the durability/rotation goal |
| **D — keep them separate (status quo + CIB-070)** | Zero new surface; CIB-070 already removes the per-shell `export` chore | Admin remains a second credential the operator must obtain and hold; no GitHub-identity unification. Acceptable interim; this ADR supersedes it as the target |

## Consequences

- **Positive:** one mental model (GitHub identity) for both user and admin;
  no static admin key for humans to manage; auto-refreshing admin sessions;
  real-time revocation via allowlist; per-operator audit attribution by GitHub
  identity; reuses proven GHCLIAUTH infrastructure; static key preserved for CI.
- **Negative:** more moving parts (a second credential file, a new endpoint
  surface, a schema change, a middleware branch); an `admin` claim and a
  github_id allowlist exist before the broader RBAC design (ADR-076) is settled,
  so this is a point solution that RBAC must later subsume gracefully.
- **Risks:**
  - *Leaked `admin-credentials.json`* grants admin until TTL expiry or allowlist
    removal — but bounded by both, unlike the static key.
  - *Allowlist drift* — a github_id allowlist that isn't pruned (departed staff)
    is a standing risk; mitigated by the same Pulumi-provisioned, audited
    lifecycle as keys.
  - *Endpoint confusion* — if the admin poll path reused the user
    `/github-device/poll`, a normal login could yield admin; mitigated by a
    **distinct** admin start/poll surface and an explicit allowlist gate.
  - *Claim replay* — a normal user licence presented at `/admin/*`; mitigated by
    requiring the `roles:["admin"]` claim **and** the request-time allowlist
    check (a user licence carries neither).
- **Mitigations:** short admin TTL; per-request allowlist re-check (not
  claim-only trust); separate credential file + separate endpoint; keep the
  static key as break-glass; audit every admin licence mint via the existing
  `admin_keys_audit` lineage; defer destructive-action step-up as a fast follow.

## Open Questions (resolve in council / with the owner before Accepted)

1. **Admin TTL and step-up.** 12h vs 24h access? Require fresh re-login (or a
   second factor) for destructive ops (`revoke`, `broadcast`, `send-migration`)?
2. **Allowlist storage.** Extend `admin_keys` with `github_id` (chosen above)
   vs a dedicated `admin_identities` table — does mixing keyed and keyless rows
   in `admin_keys` muddy the model enough to warrant a separate table?
3. **Claim vs pure-allowlist.** Keep both the `roles` claim *and* the
   request-time check (recommended, defence in depth), or trust the claim and
   skip the per-request DB read (faster, but loses instant revocation)?
4. **RBAC alignment.** Should `roles:["admin"]` be the explicit seed of ADR-076's
   deferred staff axis, designed so a future role system subsumes it without a
   claim migration?
5. **Broker reuse.** Confirm the dedicated admin OAuth-app vs reusing the "Anvil
   CLI" app with an allowlist gate (leaning: reuse the app, gate server-side).

## References

- Related ADRs: [ADR-066](066-github-device-flow-cli-auth.md) (the device-flow
  broker + licence mint this builds on); [ADR-076](076-feature-catalogue-surface-registry.md)
  (deferred staff-axis / RBAC — `admin.credential` surface)
- APS / CIB: CIB-070 (`anvil admin auth set key` — the ergonomic precursor this
  supersedes for human operators); GHCLIAUTH module (complete, shipped
  v0.8.1-beta) for the broker, `mintLicenceForGitHubUser`, and `github_id`
  linking; ADMINCLIH (per-operator key model)
- Code: `apps/anvil-api/src/middleware/admin-auth.ts`,
  `apps/anvil-api/src/routes/auth-github-device.ts`,
  `apps/anvil-api/src/lib/licence.ts`,
  `apps/anvil-api/src/db/migrations/007-admin-keys.sql` +
  `008-admin-keys-audit.sql` (the `admin_keys` / `admin_keys_audit` tables —
  defined in migrations, not `schema.sql`; the `github_id` column this ADR adds
  lands as a new migration), `apps/anvil-api/src/db/schema.sql`
  (`beta_users.github_id`), `infra/src/admin-keys.ts`,
  `crates/anvil-cli/src/commands/admin.rs`
- External: [RFC 8628](https://www.rfc-editor.org/rfc/rfc8628) (Device
  Authorization Grant)
