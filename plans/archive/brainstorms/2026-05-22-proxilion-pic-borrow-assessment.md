# Proxilion + Provenance (PIC) — Borrow Assessment

**Date:** 2026-05-22
**Status:** Brainstorm — assessment of two external projects nominated as
priority adopt/borrow candidates. Outcome: **decline adopt for both;
optionally borrow PIC's invariant model as conceptual input to Witness
Chain v3 / MLP2-071.**
**Sources:**

- https://github.com/clay-good/proxilion
- https://github.com/clay-good/provenance

---

## 1. Nomination summary

Two external projects were nominated as priority adopt/borrow candidates:

| Project        | What it is                                                                                                                                          | Stack                | Maturity              |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------- | --------------------- |
| Proxilion      | Self-hosted reverse proxy between managed AI agents (Claude / OpenAI Workspace Agents) and SaaS APIs (Google Workspace, Salesforce, Atlassian). Enforces capability chains, prompt-injection filtering, write-gating. | Rust (91%) + PG      | 3★ / 162 commits      |
| Provenance/PIC | "Provenance Identity Continuity" — cryptographic authority tracking via Proof of Causal Authority (PCA) chains. Trust Plane (CAT service), Federation Bridge, Keycloak SPI, TS SDKs. | Rust + Java + TS     | 1★ / 38 commits       |

Both target the **confused-deputy** problem. Both operate at the
**runtime API-call boundary** between AI agents and downstream services.

---

## 2. Scope-guard test

Per `docs/vision/anvil-scope-guard.md`, Anvil operates at the **point of
change creation** (pre-commit / pre-deploy), enforces deterministic
policy against artefacts, and captures provenance for **policy
decisions**. Out of scope: agent orchestration, runtime observability
that does not feed enforcement, CI/CD replacement.

Applied to the two candidates:

| Question                                                  | Proxilion                                                    | PIC                                                          |
| --------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| Increases prevention capability?                          | Yes — for SaaS API calls                                     | Yes — for service-to-service requests                        |
| Operates before/at execution time?                        | Runtime, not creation time                                   | Runtime, not creation time                                   |
| Strengthens deterministic control of code/config changes? | No — controls outbound traffic                               | No — controls request authority                              |
| Same threat model as Anvil's witness chain?               | No (Agent → SaaS auth)                                       | No (service → service auth)                                  |

Both fail decision-framework rule #2 when "execution time" is read as
Anvil's "moment of change creation". Both extend Anvil's surface into
runtime agent-traffic mediation — a different layer with its own
operational topology (proxy deployment, OAuth interception, PostgreSQL
audit store).

**Verdict: adopt = no, for both.**

---

## 3. Overlap with existing Anvil work

Anvil's provenance story is already substantial and lives in MLP / MLP2
/ INTL / INTD:

- Witness chain with `rules_sha` threaded onto every line (MLP2-014)
- DAG-aware `verify_chain_dag` walking merge-join graphs (MLP2-011)
- `GENESIS-BASELINED` / `GENESIS-FRESH` first-line semantics (MLP2-013)
- Typed `ProtectionClaim` rendered at status surfaces (MLP2-048 / -049 /
  -051f)
- Cross-session attribution (MLP2-071, design pass landed 2026-05-21)
- Daemon-side `evaluate_version_floor` + recognised-rules registry
  (MLP2-018 / -019)
- `anvil-run` wrapped-launch ingress for agent attribution (INTL, 9/9)

This is the layer where Anvil owns provenance. Proxilion and PIC sit at
a layer Anvil has explicitly deferred (runtime SaaS-call mediation).

---

## 4. The one borrow worth considering

PIC formalises three invariants on its PCA chains:

1. **Immutable origin principal** — the chain's root authority cannot
   change after creation.
2. **Monotonically shrinking operations** — each hop's authority is a
   subset of its predecessor's.
3. **Cryptographic continuity** — each hop is bound to its predecessor.

Anvil's witness chain already approximates (1) and (3). (2) — the
shrinking-authority invariant — is not currently formalised in our
chain model, but the concept is relevant to cross-session attribution
(MLP2-071) and to any future Witness Chain v3 that wants to model
"agent-derived edits inherit a strict subset of the parent session's
authority."

**Borrow shape (concept only, not code):**

- Cite PIC's invariant framing in the MLP2-071 design notes as prior
  art.
- Consider adding an "authority monotonicity" property to the
  cross-session attribution spec — does a child session's witness
  ever claim authority its parent did not have?
- Do **not** import the Rust crates, Keycloak SPI, or TS SDKs. The
  upstream is a single-author project with 1★ and 38 commits — wholesale
  dependency would import substantial risk at a layer Anvil does not
  operate at.

---

## 5. Risks of adopting either codebase

- **Scope drift.** Both pull Anvil into being a runtime agent-traffic
  proxy / SaaS gateway. Fails scope guard.
- **Operational topology change.** Proxilion requires a deployed proxy
  per tenant + PostgreSQL audit store. PIC requires a Trust Plane / CAT
  service + Keycloak. Neither matches Anvil's "deterministic CLI +
  daemon at the point of change" footprint.
- **Upstream maturity.** 3★ and 1★, single-author, <6 months old.
  Adopting either as a dependency means owning the fork in practice.
- **License + supply-chain attribution.** Both MIT — compatible, but
  every adopted Rust crate adds to the attribution-pipeline-v3 surface
  (`about.toml`, `deny.toml`, `ACKNOWLEDGEMENTS.md`).

---

## 6. Recommendation

1. **Decline adopt** for Proxilion and PIC as Anvil dependencies or
   subsystems. Wrong layer (runtime SaaS mediation vs. creation-time
   policy enforcement).
2. **Note PIC's invariants** in the MLP2-071 cross-session attribution
   spec and any Witness Chain v3 brainstorm — useful framing, not a
   code dependency.
3. **No APS module filed.** This brainstorm closes the question; if a
   follow-up emerges from MLP2-071's implementation, file it under
   MLP2 with an explicit `Source:` link back to this note.

---

## 7. Open question (defer)

If Anvil ever wants to ship guidance for "what to do at the SaaS-call
boundary downstream of an Anvil-witnessed agent edit", that would be a
new module (working title: *agent-runtime-edge-handoff*) and would
itself need a scope-guard pass — it sits adjacent to Anvil rather than
inside it. Not in scope today.
