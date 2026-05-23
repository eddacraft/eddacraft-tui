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
    "manifest_uri": ".anvil/evidence/attestations/manifests/<id>.json", // retrievable manifest
    "anvil_version": "<current-version>",
  },

  // Trust — null when no signing key is configured, object when signed.
  // Unsigned: "signature": null
  // Signed:
  "signature": {
    "algorithm": "ECDSA-P256-SHA256",
    "key_id": "arn:aws:kms:...", // customer's key
    "value": "base64:...", // base64-encoded raw signature bytes
  },
}
```

**Replay manifests**: The `replay.input_manifest` hash is accompanied by a
`manifest_uri` pointing to a stored manifest file containing the full set of
retrievable inputs (policy snapshot, file content hashes with git blob refs,
config/env values). This ensures verifiers can reconstruct the exact inputs
needed to replay a gate run, even after local state has diverged. The manifest
is stored alongside attestations and is itself integrity-checked via its hash.

**Manifest security — config/env redaction**: Replay manifests MUST NOT store
raw secret values (API tokens, cloud keys, database credentials). Before
persisting, the manifest builder scrubs environment variables and config entries
against a deny-list of known secret patterns (e.g. `*_SECRET`, `*_TOKEN`,
`*_KEY`, `*_PASSWORD`) and any additional secret-detection heuristics. Redacted
values are replaced with an HMAC-SHA-256 computed using a project-scoped
redaction key managed by Anvil, formatted as `redacted:hmac-sha256:<digest>`.
This produces a stable, opaque identifier that does not permit offline guessing
of the original value (unlike plain SHA-256, which is vulnerable to dictionary
attacks on low-entropy secrets). Verifiers who do not possess the redaction key
treat these markers as non-reversible correlation tokens; the control plane can
independently recompute and verify them when needed.

Projects may additionally configure an explicit allow-list of non-sensitive keys
via `anvil.config.replay.allowedEnvKeys`. The deny-list (and secret detection)
ALWAYS takes precedence: if a key appears in the allow-list yet matches a secret
pattern or is otherwise classified as sensitive, its value MUST still be
redacted and MUST NOT be persisted in cleartext. Implementations SHOULD surface
such conflicts as warnings so that misconfigured allow-lists cannot silently
weaken manifest security.

**External dependency snapshots**: Gates that depend on mutable external data
sources (e.g. advisory databases used by `dependency.check`, registry metadata)
are inherently non-deterministic across time. To support meaningful replay, the
replay schema captures an `external_sources` array in the manifest:

```jsonc
"external_sources": [
  {
    "type": "advisory-db",
    "source": "npm-audit",
    "snapshot_timestamp": "2026-03-11T14:30:00Z",
    "version": "2026.03.11",           // when available
    "content_digest": "sha256:abc123"  // hash of downloaded snapshot data
  }
]
```

Gates that operate on purely local inputs (source files, config, policy
snapshots) are deterministic by default and do not need an `external_sources`
array. For gates that consume external mutable data, the determinism rule
applies: a gate is only considered `"deterministic": true` when **every** entry
in its `external_sources` array includes an immutable content reference
(`content_digest`). Gates that consume external data but declare an empty
`external_sources` array MUST be tagged as non-deterministic — an empty array
does not satisfy the requirement. Timestamp and version alone are insufficient
since upstream sources can change or reissue the same version identifier. If
even one source lacks a digest, the gate MUST be tagged as non-deterministic.
When all digests are present, replay consumers can fetch each snapshot artifact
and verify its integrity before re-evaluation.

When any external source lacks an immutable reference, the gate MUST be tagged
as `"deterministic": false` in its attestation, signalling to replay consumers
that results may legitimately differ over time. This prevents false-negative
integrity violations during audit replay.

## PR-Level Aggregate

After all gates pass in CI, Anvil produces a **pre-merge** PR attestation
referencing individual gate proofs. Merge metadata (`merged_by`,
`merge_timestamp`) is recorded in a separate **post-merge** attestation produced
by the merge hook so the pre-merge proof is never mutated after signing.

```jsonc
// Pre-merge — produced when all gates pass
{
  "type": "pr-attestation",
  "pr": 42,
  "head_commit": "abc123def456...", // exact commit SHA that was validated
  "gate_proofs": ["sha256:gate1...", "sha256:gate2..."],
  "all_passed": true,
  "signature": null, // or { "algorithm": "...", "key_id": "...", "value": "..." }
}

// Post-merge — produced by merge hook
{
  "type": "pr-merge-attestation",
  "pr": 42,
  "head_commit": "abc123def456...", // must match pre-merge head_commit
  "merge_commit": "789abc...", // the actual merge commit SHA
  "pre_merge_proof": "sha256:...", // references the pre-merge attestation
  "merged_by": "josh",
  "merge_timestamp": "...",
  "signature": null, // or { "algorithm": "...", "key_id": "...", "value": "..." }
}
```

This is the CTO dashboard view: a single verified pair per PR (governance
proof + merge record), drillable into individual gate proofs.

## Signing Infrastructure

### Phase 1 — Customer-Controlled Keys

- Provider interface supporting AWS KMS, GCP Cloud KMS, Azure Key Vault, and
  local PKCS#11
- Customer configures key reference in `.anvilrc` or env var
- Anvil never sees the private key — signing process:
  1. Remove the `signature` field from the attestation envelope
  2. Canonicalise the remaining JSON using
     [RFC 8785 JCS](https://www.rfc-editor.org/rfc/rfc8785)
  3. Compute SHA-256 over the canonical UTF-8 bytes
  4. Send only this hash to the configured key provider for signing (e.g.
     ECDSA-P256-SHA256)
  5. Store the raw signature as `signature.value` (base64-encoded);
     `signature.algorithm` and `signature.key_id` reflect the key actually used
  - Third-party verifiers repeat steps 1–3 and verify against the advertised
    public key
- If no key configured, `signature` is `null` — attestations are still produced
  (useful for development, insufficient for audit)

### Phase 2 — Transparency Log (Future)

- Optional append-only transparency log (self-hosted or Anvil-hosted)
- Merkle tree structure — each attestation is a leaf; the log maintains a
  rolling Merkle root and can produce compact inclusion/consistency proofs (cf.
  Certificate Transparency RFC 6962)
- Tamper detection: missing or mutated entries are provable via consistency
  proofs without downloading the full log
- Export for external auditors

### Phase 3 — Co-Attestation (Future)

- Independent verifier re-evaluates deterministic gates
- Dual-signed attestations for high-stakes environments (healthcare, finance,
  government)

## Verification CLI

```bash
# Verify a single gate attestation
anvil evidence verify proof .anvil/evidence/attestations/gate-abc123.json

# Verify all attestations for a PR
anvil evidence verify pr 42

# Replay: re-run the same policy against the same inputs,
# confirm the result matches the attestation
anvil evidence replay .anvil/evidence/attestations/gate-abc123.json

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

1. Should attestations be stored in-repo (`.anvil/evidence/attestations/`) or in
   an external store? In-repo is simpler and git-native; external scales better.
2. How to handle policy version transitions mid-PR? If policy changes between
   gate runs on the same PR, the aggregate needs to reflect which version
   applied when.
3. What's the right default for trace verbosity? Too little = useless for
   debugging. Too much = information leakage risk.
4. Pricing model: is verifiable governance a premium tier feature, or core to
   all plans?
