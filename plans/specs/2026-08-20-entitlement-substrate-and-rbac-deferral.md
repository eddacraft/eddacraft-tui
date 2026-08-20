# Entitlement substrate correctness, and the deferral of RBAC

| Type | Authority   | Owner | Status | Freshness            |
| ---- | ----------- | ----- | ------ | -------------------- |
| Spec | Design      | SEC   | Draft  | Authored 2026-08-20  |

| Upstream                     | Downstream                                  |
| ---------------------------- | ------------------------------------------- |
| ADR-121, ADR-107, FLAGCAT    | SEC-012, SEC-013, CIB-141, CIB-143, next ADR |

## Origin

A request to implement RBAC and tie it to the feature-flag catalogue. Design
grilling redirected it twice, and both redirections are the substance of this
document:

1. The catalogue already models roles. `flags/audiences.json` declares a `role`
   axis (`role-admin`, `role-developer`) and a separate `staff` axis;
   `AudienceContext.user_role` / `userRole` is a resolvable targeting attribute
   in **both** the Rust (`crates/anvil-kernel-types/src/feature_flags.rs:98`,
   `crates/anvil-kernel/src/feature_flags/resolver.rs:352`) and TypeScript
   (`packages/anvil/contracts/src/schemas/feature-flags.schema.ts:70`,
   `packages/anvil/runtime/src/feature-flags/resolver.ts:216`) runtimes. Nothing
   populates it and no flag targets it. RBAC here was never "build the
   mechanism" — it was "populate an axis that already exists".
2. Every enforcement point a role would govern sits **downstream of a verifier
   that cannot currently deny**. Roles would have added expressiveness to a
   system that grants regardless. Fixing the substrate came first.

## Goal

Make entitlement decisions correct, deny-by-default, and consistent across the
two edge verifiers — before any principal attribute (role, org, staff) is
introduced that would multiply the blast radius of getting it wrong.

## Facts established

- **The private-docs gate does not discriminate between real accounts.**
  `apps/docs-shell/lib/jwt.ts` accepts `tier ∈ {beta, pro, enterprise}`.
  Post-BACT-013 `signLicence` writes `tier: claims.plan`, and `plan` is `beta`
  for every account — the only value `beta_users.plan`'s CHECK admits. Every
  validly-signed licence from a real account therefore passes. SEC-009 is
  recorded Done for this gate; it shipped the mechanism, and the mechanism has
  nothing to discriminate on while one plan exists. **Corrected 2026-08-20**
  after review: an earlier draft said the gate "does not discriminate" full
  stop and that docs-shell "inherits the API's permissive default". Neither is
  right. `docs.access` targets `plan-beta/pro/enterprise` with
  `defaultVariant: disabled`, and docs-shell fails **closed** on a missing
  `tier` with its own local set — it never consults the API's default. The
  defect is narrower: the *claim feeding* the gates was fabricated.
- **Two verifiers, two implementations.** `apps/docs-site` resolves through the
  catalogue — `lib/feature-flags.ts`'s `evaluateDocsAccess` calls
  `canonicalAccountTier(tier)` to map `beta` → `plan-beta` before resolving the
  flag, while `middleware.ts` only reads `payload.tier` and passes it in.
  `apps/docs-shell` hardcodes an equivalent `Set` instead of importing the
  catalogue. Both fail closed on an absent claim; they differ in whether the
  entitled-plan list can drift from `flags/manifest.json`.
- **Stale licences assert a plan no account holds.** Pre-BACT-013 licences carry
  `tier: 'pro'` and no `plan` (`apps/anvil-api/src/lib/session.ts:85,165` at
  `f94ba7fe5^`). `verifyLicence` resolves `plan = rawPlan ?? rawTier ?? 'beta'`,
  so those tokens read as `plan = 'pro'` → `plan-pro` → `docs.access` enabled.
  `/auth/verify` is unaffected (BACT-013 prefers the DB plan), but the two edge
  verifiers do no DB round trip and believe the claim. This — not a missing
  gate — is the actual over-entitlement.
- **The window is 90 days, not 7.** `LICENCE_TTL_DAYS = 90`; the 7 is
  `RC_AFTER_DAYS`, a recommended-refresh hint, not an expiry. Pre-BACT-013
  licences therefore live until roughly **2026-11-11**. SEC-007's original
  follow-up text said 7 days and was wrong.
- **Signup is not self-serve.** Waitlist → admin approve → invite. The
  `['beta']` fallback in `findActiveScopesForUser` is documented as a
  "self-signup entry point"; that comment is inaccurate.
- **`findActiveScopesForUser` runs at session mint** (`lib/session.ts:86,166`),
  so its fallback is what grants scopes on an approved user's *first*
  interactive login, when zero tokens legitimately exist. Returning `[]` there
  would break the golden path.

## Decisions

1. **Fix authentication and authorisation before introducing roles.** Not a
   sequencing preference — a correctness precondition. A role axis layered on
   fail-open verification produces a more expressive description of access that
   is still granted.

2. **`plan` is the entitlement axis; `scopes` are the capability axis; both
   fail closed.** This is ADR-121's three-axis model made real rather than a
   fourth vocabulary. Product access resolves from the account's plan through
   catalogue flags. API capability resolves from token scopes. Neither silently
   defaults to a permissive literal.

3. **Stale tokens downgrade rather than inherit.** An absent `plan` claim
   resolves to `beta`; a stale `tier` is never promoted into `plan`. This
   **deliberately overturns** the BACT-013 judgement recorded in
   `apps/anvil-api/src/lib/licence.ts` — *"never silently downgrade an in-flight
   session"* — because that comment optimises for session continuity in a world
   where all plans are equivalent, and becomes an over-entitlement vector the
   moment plans differentiate. The `tier` alias then retires by natural expiry
   (~2026-11-11) with no forced re-authentication.

4. **Default scopes derive from `plan` via the `api.scope.*` entitlement flags,
   not from token history.** CIB-141 asks how to distinguish "never issued a
   token" from "all tokens revoked". Under decision 2 the question dissolves:
   neither case decides capability, so neither needs distinguishing. There is no
   hardcoded `['beta']`, and no bare `[]` that breaks first login.

5. **RBAC is deferred in full.** No role column, no organisation concept, no
   population of `user_role`, no flag targeting `role-*`. The catalogue's role
   and staff axes stay declared and dormant — they are correct as specified
   (`plans/specs/2026-05-19-feature-gating-model.md:70-72` defines `role-*` as
   roles *within the customer's own organisation*) and cost nothing while unused.

## Why RBAC was deferred

Recorded so it is not re-litigated from scratch:

- The axis that exists is for customer organisations. **There is no organisation
  table, no membership model, and no invite-to-org flow.** Customer RBAC would
  require inventing multi-tenancy for an invite-only beta with one plan and one
  cohort.
- The surface with a demonstrated gap is the operator plane — `admin_keys` is
  `hashed_key / actor_email / note / revoked_at`, guarding 14 routes including
  `/broadcast`, `/email-send`, `/user/email-update` and `/revoke`, with no
  granularity. But that is *not* what the `role-*` axis was designed for, so
  closing it is admin-key scoping (CIB-318), not the role axis.
- Doing either well depends on decisions 2–4 already being true.

**Revisit when** a concrete driver appears: a multi-user customer, a commercial
plan beyond `beta`, an enterprise or compliance requirement, or an incident on
the admin plane. CIB-318 remains the tracked entry point for operator-plane
granularity and is unblocked independently of this deferral.

## Boundaries

- **In scope:** `apps/anvil-api` licence minting and verification, scope
  resolution, licence-authenticated route auth; `apps/docs-shell` and
  `apps/docs-site` entitlement reads.
- **Out of scope:** `jti` deny-lists and licence-TTL shortening (deferred by
  SEC-007 for the same reason and still deferred); admin-key granularity
  (CIB-318); FLEET (anonymous, ADR-107 — no identity join); any role, org or
  staff attribute.

## Risks

- **Decision 3 is a behaviour change for stale sessions.** A token that asserted
  `pro` reads as `beta`. Today both grant `docs.access`, so the change is
  invisible; that invisibility is exactly why it should land *before*
  differentiation, not during it.
- **Decision 4 moves scope resolution onto flag evaluation** at session mint,
  putting the catalogue on the login path. Evaluation is local and
  deterministic, but the failure mode of a malformed catalogue becomes a login
  failure and must fail closed loudly rather than silently.
- **Sequencing constraint:** no plan differentiation may ship while stale
  `tier: 'pro'` licences are still honoured. Decision 3 removes that constraint
  immediately; without it the constraint holds until ~2026-11-11.

## Downstream effects on tracked items

| Item    | Effect |
| ------- | ------ |
| SEC-012 | Gains decisions 2 and 3; the "which claim denotes entitlement" open question is answered. |
| SEC-013 | Unchanged in intent; severity restated — the window is 90 days, not 7. |
| CIB-141 | Open question dissolved by decision 4; rescope to "derive defaults from plan". |
| CIB-143 | Entitlement half absorbed into SEC-012; the callback-throttling half stays independent. |
| CIB-318 | Unblocked, unchanged, explicitly out of this scope. |

## Open questions

None blocking. An ADR should record decisions 2–5 — decision 3 in particular,
because it reverses a judgement written into the code by BACT-013.
