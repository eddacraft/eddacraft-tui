# Policy Capability Discovery — Design Spec

**Status:** Draft — pending Planning Council review (cross-boundary).
**Date:** 2026-05-24
**Owner:** TBD (proposed)
**APS module:** [`policy-capability-discovery`](../modules/policy-capability-discovery.aps.md) (`POLCAP`)
**ADR:** ADR-051 (to be drafted under POLCAP-001)
**Brainstorm provenance:** `plans/brainstorms/agent-security-package.md`
  (`@eddacraft/anvil-warden` concept), Warden repository analysis (2026-05-24).

## 1. Context

Governed agents operating inside Anvil today learn what they can do by trial:
they attempt an action, the daemon's gate evaluates it, and the response tells
them whether it was allowed. There is no protocol-level way for the agent to
ask "what can I do here, before I commit to a tool call?" The result is wasted
attempts, brittle planning, and audit rows that capture *what was tried*
rather than *what the agent believed it was authorised to try*.

Warden (`stephnangue/warden`, MPL-2.0) demonstrates a "discover then connect"
pattern for credential-mediated agent access: the agent calls `role list`,
`provider list`, and `skill read` against a gateway before performing any
upstream action, gets operator-prose descriptions back, matches its task to a
role and skill, then performs the call following the returned recipe. The
gateway re-evaluates every actual request — the discovery output is advisory.

This spec adapts that shape — discover, match, follow recipe, then act — into
Anvil's planless-first, gate-enforced model. The new surface, `anvil policy
capabilities`, returns a compact, signed, machine-readable capability view
that the agent can consult before attempting any governed action.

## 2. Goals

1. Give a governed agent a single deterministic call that returns: allowed
   action families, denied action families, escalation paths, evidence
   requirements, and operator-authored role/policy descriptions, scoped to the
   current session.
2. Make every capability row carry a stable identifier (`cap_id`), a `version`,
   and a `signing_epoch`. Subsequent gate decisions and witness-chain audit
   rows reference these IDs by value so intent and outcome can be reconciled.
3. Preserve `gate` and `watch` as the sole enforcement authorities. The
   capability view is **non-authoritative**: it predicts likely-allowed
   actions but never grants them.
4. Fail closed on unknown or stale `cap_id` references in downstream calls —
   the agent must refresh, never silently downgrade to broad permission.
5. Stay planless-first: a project with no `.anvil` config still gets a usable
   default capability view derived from the action taxonomy (ACTAX) + risk
   defaults (IORISK).

## 3. Non-goals

1. **Not** a credential broker. Anvil does not vend, proxy, or rotate upstream
   credentials. No equivalent of Warden's mount/provider proxying.
2. **Not** a provider catalogue. The 33-provider Warden surface is explicitly
   out of scope; the beta enumerates a narrow action set (§7) and grows by
   demand signal, not breadth-for-its-own-sake.
3. **Not** a runtime authorisation engine. The view describes *what gate is
   expected to permit*. Gate still evaluates each call.
4. **Not** a planning-mode tool for human developers. The CLI subcommand
   exists for diagnosis, but the primary consumer is the in-session agent.
5. **Not** a replacement for AGOV's capability manifest. AGOV-007 owns
   *agent-declared* capability bounds (what the AI tool claims it will do).
   POLCAP owns *Anvil-declared* capability bounds (what Anvil tells the
   agent it can do). The two are reconciled in policy evaluation, not merged.

## 4. Scope-guard fit (`docs/vision/anvil-scope-guard.md`)

1. **Prevention capability?** Yes — a conservative capability view prunes
   agent attempts before they reach the gate, and signed `cap_id` references
   strengthen the provenance chain for every later decision.
2. **Pre-execution?** Yes — discovery runs before the first governed call.
3. **Deterministic?** Yes — fail-closed on unknown IDs, version-pinned
   skill recipes, HMAC-signed envelopes, no probabilistic ranking.
4. **Enforces or just informs?** The *view* informs. The *system* enforces:
   gate still runs, witness chain still captures the decision, and unknown
   `cap_id` references at evaluation time are refused. POLCAP is in-scope
   **only** because the cap-ID is load-bearing for downstream enforcement
   and audit. If at any point cap-IDs become purely decorative, the surface
   ceases to fit the scope guard and is removed.

## 5. Authorities and prior art

- **ADR-001** Planless-first: a project without `.anvil` config returns a
  default capability view derived from action taxonomy + sensible deny
  defaults; no setup required.
- **ADR-002** Warnings over blocks: capability discovery itself is not a
  warning surface; it is a question the agent asks. Calls that proceed
  through `gate` continue to honour warn-by-default semantics.
- **ADR-037** Witness chain + L4 policy: signed `cap_id` references emitted
  on `gate` decisions feed the existing witness-chain row format (additive
  fields under `serde(default, skip_serializing_if)`); the L4 policy lane
  can later refuse a row whose `cap_id` is unknown to its registry.
- **ADR-040** `regorus` policy engine: capability view is generated from the
  policy engine's evaluation surface; the engine remains the source of
  truth for what *is* permitted, not what the view *claims* is permitted.
- **ADR-024** `weave-rs` agent harness: POLCAP becomes the first first-class
  protocol primitive that a weave-hosted agent calls during its bootstrap.

## 6. Surface

### 6.1 CLI

```bash
anvil policy capabilities [--scope <scope>] [--format json|yaml] [--raw]
```

- **`--scope`** narrows the response. Default scope is full
  `(repo, workspace, agent, principal, environment, time_window)`. Operators
  can probe a hypothetical scope for diagnosis.
- **`--format json`** is the default for agent consumers. `--raw` strips the
  signing envelope (diagnostic only; never use to drive an actual call).
- Exit codes match the structured taxonomy (§9). Exit 0 only when a signed
  view was returned.

### 6.2 IPC method (daemon)

```
capabilities/describe
  request:  { scope: ScopeTuple, since_epoch?: uint64 }
  response: SignedCapabilityView | ErrorEnvelope
```

The daemon is the authoritative source. The CLI is a thin wrapper that
prefers daemon-backed responses and falls back to an embedded
correctness-equivalent computation only when the daemon is genuinely
unavailable, matching the established pattern (RMCP-005, MLP2-051f).

### 6.3 MCP method

`anvil_capabilities` MCP tool, returning the same JSON shape. Initially
exposed only via the daemon-backed `anvil mcp serve`, never via embedded
fallback (the signing envelope is meaningless without a live key).

## 7. Response shape (beta — narrow set)

```jsonc
{
  "schema_version": "anvil.policy.capabilities.v1",
  "issued_at_unix": 1716480000,
  "scope": {
    "repo": "...", "workspace": "...", "agent": "...",
    "principal": "...", "environment": "...",
    "time_window": { "from_unix": 0, "to_unix": 0 }
  },
  "signing_epoch": 7,
  "view": {
    "allowed": [ { "cap_id": "cap_01h…", "family": "file.write",   "version": 1, "skill_ref": "file.write.v1",  "description": "...", "evidence": [...] }, ... ],
    "denied":  [ { "cap_id": "cap_01h…", "family": "secret.read",  "version": 1, "reason_ref": "policy.secret-deny.v1", "description": "..." }, ... ],
    "escalation": [ { "cap_id": "cap_01h…", "family": "repo.change", "to": "operator-review", "evidence_required": ["intent-statement","diff-summary"] } ]
  },
  "roles": [
    { "role_id": "read-only",        "description": "Read-only access to repo and config; never writes." },
    { "role_id": "code-author",      "description": "May write within tracked source paths; secrets always denied." },
    { "role_id": "release-helper",   "description": "May tag, push, and open PRs after operator handoff." }
  ],
  "envelope": {
    "alg": "HMAC-SHA256",
    "kid": "session-2026-05-24-<sessionId>",
    "mac": "..."
  }
}
```

Action families for the beta are deliberately narrow and map onto ACTAX
domain.verbs: `file.write`, `shell.run`, `repo.change`, `network.request`,
`secret.read`, plus selected MCP tools enumerated by their tool name
(`mcp.<server>.<tool>`). Adding a family is an ACTAX change, not a POLCAP
change.

## 8. Skill recipes

Each `skill_ref` points to a versioned recipe in
`docs/agent/skills/<family>.v<n>.md`. The recipe is markdown plus a
front-matter version integer. It carries:

- One-paragraph description (operator-authored; the agent matches its task
  against this prose, not against the `family` string).
- Required pre-call inputs (e.g., target path, intent statement, diff hash).
- Required evidence the agent must attach to the call (e.g., the
  intent-statement string, the planned diff hash).
- Exit-code interpretation (subset of §9).
- One worked example showing the exact tool-call shape.

Recipe authorship is governed by `docs/guides/documentation-governance.md`.
Adding or modifying a recipe triggers a `signing_epoch` bump for the
affected scope; agents holding a view with the prior epoch must refresh
before their next governed call.

## 9. Structured error taxonomy

| Code | Symbol | Meaning | Agent action |
|---|---|---|---|
| 0 | `ok` | Signed view returned | Proceed |
| 3 | `invalid_input` | Scope or version malformed | Read message hints, fix payload, retry once |
| 4 | `auth_required` | Session token missing or expired | Refresh session and retry |
| 5 | `forbidden` | Scope exists but agent has no view rights | Surface to operator; do not synthesise broader view |
| 6 | `not_found` | Scope refers to unknown repo/workspace | Re-discover; refuse to invent a scope |
| 7 | `network` | IPC or transport failure | Bounded retry with backoff (max 3) |
| 8 | `server` | Daemon internal error | Surface to operator with `cap_view_request_id` |
| 9 | `stale_epoch` | View's signing_epoch is below the daemon's current epoch | Refresh view; refuse to act on cached IDs |
| 10 | `unknown_cap_id` | A referenced `cap_id` is not in the daemon's registry | **Fail closed.** Refresh view. Never downgrade silently. |

Agents branch on the symbol, never on the human-readable `message`. This is
deliberately aligned with the Warden taxonomy structure — the codes
themselves are typed afresh for Anvil and live in
`crates/anvil-kernel-types`.

## 10. Signing envelope (beta)

- Symmetric HMAC-SHA256 keyed on a per-session secret derived from the
  daemon's owner-only IPC handshake. The session secret never leaves the
  daemon's address space; only the MAC and `kid` are returned to the agent.
- `signing_epoch` is a monotonic uint64 owned by the daemon. It increments
  on: policy reload, ACTAX taxonomy change, recipe version bump, or
  scheduled rotation.
- The envelope binds `(schema_version, scope, issued_at_unix,
  signing_epoch, view)` — modifying any field invalidates the MAC.
- Asymmetric signing (Ed25519, akin to ADR-045's minisign chain) is reserved
  for v2 when capability views are shared across agents in one session or
  exported off-host. The wire shape carries `alg` so the upgrade is
  additive.

## 11. Audit binding (load-bearing)

Every gate decision row in the witness chain (`anvil/witnessed.ndjson`)
gains an optional `cap_id` field. Rule:

- If the agent's call carried a `cap_id`, the daemon records it on the
  witness row alongside the decision (`allow`/`block`).
- If the recorded `cap_id` is unknown to the daemon's registry at write
  time, the row records `cap_id_status: "unknown"` and the call is
  refused — fail closed, per §9 code 10.
- L4 policy (ADR-037) can refuse to admit a rule set that lacks a recipe
  registry covering the cap-IDs the chain has observed.

This is the load-bearing tie between the advisory view and the
authoritative enforcement record. Without it, POLCAP is decorative and
out of scope.

## 12. "Agent never does" (the negative-space contract)

Adapted from Warden's enumerated agent prohibitions. Lands in `AGENTS.md`
as a normative list:

1. **Never** invent a `cap_id`, `role_id`, or `family` not returned by
   `capabilities/describe`. Unknown IDs are refused at gate time anyway,
   but the agent must not even attempt them.
2. **Never** act on a view whose `signing_epoch` is older than the daemon's
   current epoch. Refresh first.
3. **Never** match a task to a `family` directly — match against the
   operator-authored `description` field. Family names are not API
   contracts; the prose is.
4. **Never** treat a denied family as "ask harder" — surface to the
   operator. Escalation paths exist in `view.escalation` for a reason.

## 13. Out of scope (beta)

- Asymmetric signing of capability views (reserved for v2).
- Cross-host capability federation (single host, single daemon).
- Web-dashboard rendering of capability views (post-launch DASHOPS work).
- Operator UI for editing recipes — recipes are files, edited like any
  other governed doc.
- Capability views for non-governed sessions (raw shell, IDE without
  driver). These get no view and must not synthesise one.

## 14. Open questions

- **OQ-1:** Should the beta narrow set include or exclude `network.request`?
  It's the highest-value family for AI-coding agents but also the one most
  likely to leak credentials. Lean: include, with a default-deny policy and
  an explicit escalation path.
- **OQ-2:** Recipe versioning — is one version integer per recipe enough,
  or do we need a content hash on top? Lean: one integer plus a daemon-side
  content hash recorded in the registry, not exposed to the agent.
- **OQ-3:** Where does the recipe registry live on disk? Inline in the
  config crate, or its own `crates/anvil-recipes/`? Lean: own crate,
  parallel to `crates/anvil-rules/`.
- **OQ-4:** How does POLCAP interact with `weave-rs` (ADR-024) once the
  harness lands? Lean: `capabilities/describe` is one of the first
  bootstrap calls a weave session makes, and the returned `kid` becomes
  the session's audit identity.

## 15. Validation

- **V-1:** `cargo test -p eddacraft-anvil-policy` covering view shape,
  envelope round-trip, fail-closed on unknown cap-ID, fail-closed on
  stale epoch.
- **V-2:** Cross-language parity test (Rust ↔ TS driver client) for the
  signed-view JSON shape, byte-equal to a captured fixture.
- **V-3:** End-to-end test: agent calls `capabilities/describe`, attempts
  one allowed family (passes), one denied family (gate refuses), one
  unknown cap-ID (gate refuses with code 10). Witness chain row carries
  `cap_id` on the allow case and `cap_id_status: "unknown"` on the
  refusal.
- **V-4:** `pnpm docs:check && pnpm docs:index:check` green after recipe
  files land.

## 16. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Agents trust the view as authoritative and skip handling refusals | high | Spec explicitly forbids; gate refuses unknown IDs anyway; CI parity test asserts refusal handling in the TS driver client. |
| Operator-authored descriptions become a prompt-injection vector | medium | Treat descriptions as untrusted-by-default for any downstream LLM step; redact in tracing per ADR-035 + TRACE-003. |
| Signing-epoch rotation thrashes long-running agents | medium | Epoch bumps coalesce on a debounce; agents refresh on `stale_epoch` only, not proactively. |
| Capability families drift from ACTAX taxonomy | high | POLCAP consumes ACTAX as a typed dependency; `family` is `ActionId` from `crates/anvil-policy-action-taxonomy`. No parallel vocabulary. |
| Cap-IDs leak into long-lived caches and become attack tokens | medium | Cap-IDs are scope-bound and signing-epoch-bound; a leaked ID is useless against a different scope or after epoch rotation. |
| Surface inflates into Warden's 33-provider zoo | medium | Beta cap on action families enforced by ACTAX governance; new families require an APS work item, not a recipe drop. |

## 17. References

- `stephnangue/warden` repository analysis (in-session, 2026-05-24).
  Licence: MPL-2.0; no Warden source vendored. Patterns reused; code is
  clean-room.
- `docs/vision/anvil-scope-guard.md` (decision framework §4).
- `plans/decisions/001-planless-first.md`, `002-warnings-over-blocks.md`,
  `037-witness-chain-and-l4-policy.md`, `040-rust-policy-engine-regorus.md`,
  `024-internal-agent-harness.md`, `045-update-signing-scheme.md`.
- `plans/brainstorms/agent-security-package.md` (prior `anvil-warden`
  concept).
- `plans/modules/agent-governance-patterns.aps.md` (AGOV-007 capability
  manifest — declared intent surface).
- `plans/modules/policy-action-taxonomy.aps.md` (ACTAX domain.verbs).
- `plans/archive/modules/io-risk-controls.aps.md` (risk dimensions).
- `plans/modules/skill-discovery-observability.aps.md` (skill inventory).
