# Verifiable Governance Attestation — Technical Design

**Date:** 2026-03-11\
**Status:** Draft\
**Owner:** aneki

## Overview

Every time Anvil enforces governance (a gate run), it produces an attestation
proving what policy ran, against what inputs, with what result. When a signing
key is configured, attestations are cryptographically signed; otherwise they are
produced unsigned (useful for development, insufficient for audit). Attestations
are aggregated per-PR for leadership visibility and exportable for audit.

## Design Decisions

| Decision            | Choice                                                                                     | Rationale                                                                        |
| ------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| Primary audience    | Engineering leadership / CTOs                                                              | Buyer persona; compliance is the upsell                                          |
| Unit of governance  | Per-gate-run (building block), per-PR (default view)                                       | Maps to existing Anvil concepts; clean aggregation                               |
| Trust model         | Customer-controlled keys (Phase 1) → Transparency log (Phase 2) → Co-attestation (Phase 3) | "Don't trust us, trust your own keys" — strongest developer-first positioning    |
| Attestation payload | Replayable by default, full trace opt-in                                                   | Replayability is the differentiator; trace is valuable but potentially sensitive |

## Architecture

```
┌─────────────────────────────────────────────┐
│  anvil gate / anvil validate                │
│  (existing pipeline — unchanged)            │
├─────────────────────────────────────────────┤
│  Attestation Engine (new)                   │
│  ┌────────────────────────────────────────┐ │
│  │ 1. Collect gate inputs:                │ │
│  │    - policy version hash               │ │
│  │    - policy definition snapshot        │ │
│  │    - input content hashes (per-file)   │ │
│  │    - config/env state hash             │ │
│  │                                        │ │
│  │ 2. Execute gates (as normal)           │ │
│  │                                        │ │
│  │ 3. Produce attestation envelope:       │ │
│  │    - all inputs from step 1            │ │
│  │    - per-gate pass/fail + rule matches │ │
│  │    - timestamp                         │ │
│  │    - PR/commit ref                     │ │
│  │    - deterministic replay flag         │ │
│  │                                        │ │
│  │ 4. Sign with customer-controlled key   │ │
│  │    (KMS/HSM via provider interface)    │ │
│  └────────────────────────────────────────┘ │
├─────────────────────────────────────────────┤
│  Attestation Store                          │
│  - Per-gate attestations (.anvil/evidence/attestations/) │
│  - PR-level aggregate proof                 │
│  - Export formats: JSON, SARIF, PDF summary │
└─────────────────────────────────────────────┘
```

## Attestation Envelope Schema (v1)

```jsonc
{
  "version": "1.0",
  "type": "gate-attestation",
  "timestamp": "2026-03-11T09:00:00Z",

  // What was governed
  "subject": {
    "repo": "org/repo",
    "commit": "abc123",
    "pr": 42,
    "files": ["src/foo.ts:sha256:...", "src/bar.ts:sha256:..."],
  },

  // What policy ran
  "policy": {
    "version": "sha256:...", // hash of full policy definition
    "definition": "...", // embedded or URI to policy snapshot
    "profile": "production",
  },

  // What happened
  "results": {
    "outcome": "pass", // pass | fail | warn
    "gates": [
      {
        "name": "architecture",
        "outcome": "pass",
        "deterministic": true, // can this be replayed?
        "rules_evaluated": 12,
        "rules_passed": 12,
      },
      {
        "name": "security-scan",
        "outcome": "pass",
        "deterministic": true,
        "rules_evaluated": 8,
        "rules_passed": 8,
      },
    ],
    // opt-in: full decision trace
    "trace": null, // or detailed rule-by-rule log when verbosity=full
  },

  // Replay support
  "replay": {
    "supported": true,
    "input_manifest": "sha256:...", // hash of all inputs needed to replay
    "anvil_version": "<current-version>",
  },

  // Trust — omitted when no signing key is configured
  "signature": {
    "algorithm": "ECDSA-P256-SHA256",
    "key_id": "arn:aws:kms:...", // customer's key
    "value": "base64:...",
  },
}
```

## PR-Level Aggregate

After all gates pass in CI, Anvil produces a PR attestation referencing
individual gate proofs:

```jsonc
{
  "type": "pr-attestation",
  "pr": 42,
  "gate_proofs": ["sha256:gate1...", "sha256:gate2..."],
  "all_passed": true,
  "merged_by": "josh",
  "merge_timestamp": "...",
  "signature": {
    "algorithm": "ECDSA-P256-SHA256",
    "key_id": "arn:aws:kms:...", // customer's key
    "value": "base64:...",
  },
}
```

This is the CTO dashboard view: a single verified ✅ per PR, drillable into
individual gate proofs.

## Signing Infrastructure

### Phase 1 — Customer-Controlled Keys

- Provider interface supporting AWS KMS, GCP Cloud KMS, Azure Key Vault, and
  local PKCS#11
- Customer configures key reference in `.anvilrc` or env var
- Anvil never sees the private key — sends a hash to be signed
- If no key configured, attestations are still produced unsigned — useful for
  dev, useless for audit

### Phase 2 — Transparency Log (Future)

- Optional append-only log (self-hosted or Anvil-hosted)
- Merkle tree structure — each attestation includes hash of previous
- Tamper detection: gaps or mutations in the chain are provable
- Export for external auditors

### Phase 3 — Co-Attestation (Future)

- Independent verifier re-evaluates deterministic gates
- Dual-signed attestations for high-stakes environments (healthcare, finance,
  government)

## Verification CLI

```bash
# Verify a single gate attestation
anvil evidence verify proof .anvil/proofs/gate-abc123.json

# Verify all attestations for a PR
anvil evidence verify pr 42

# Replay: re-run the same policy against the same inputs,
# confirm the result matches the attestation
anvil evidence replay .anvil/proofs/gate-abc123.json

# Audit export: bundle all proofs for a time range
anvil evidence export --from 2026-01-01 --to 2026-03-31 --format json
```

## Intentional Scope Cuts

- **No TEE/hardware dependency** — trust comes from customer keys +
  replayability, not hardware enclaves
- **No real-time interception** — attests gate runs, not individual LLM calls
  mid-session
- **No agent session governance** — future feature when Anvil's AI tool
  integration (Horizon 5) lands
- **Non-deterministic gates** (LLM-based) get attested but not replayed — proves
  they ran, not that they'd produce the same answer twice

## Prior Art & Inspiration

| Concept                        | Domain                | What we borrow                                                         |
| ------------------------------ | --------------------- | ---------------------------------------------------------------------- |
| Certificate Transparency       | TLS/PKI               | Append-only tamper-evident logs                                        |
| SLSA Provenance                | Software supply chain | Signed build attestations                                              |
| Sigstore / cosign              | Container images      | Keyless signing with transparency log                                  |
| Proof-of-Guardrail (Sahara AI) | AI agents             | Concept of verifiable guardrail execution (we skip the TEE dependency) |
| in-toto                        | Software supply chain | Layout + link metadata for each pipeline step                          |

## Open Questions

1. Should attestations be stored in-repo (`.anvil/proofs/`) or in an external
   store? In-repo is simpler and git-native; external scales better.
2. How to handle policy version transitions mid-PR? If policy changes between
   gate runs on the same PR, the aggregate needs to reflect which version
   applied when.
3. What's the right default for trace verbosity? Too little = useless for
   debugging. Too much = information leakage risk.
4. Pricing model: is verifiable governance a premium tier feature, or core to
   all plans?
