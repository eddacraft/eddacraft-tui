# ADR-136: Secret-Detection Ruleset Acquisition Posture (Rules as Data)

## Status

Accepted 2026-08-30 (operator)

## Date

2026-08-30

## Context

Anvil's `secret-detection` check carries 21 built-in patterns. Every product
path — `anvil gate`, the save-time intercept, the capsule surface, and gctx
egress redaction — funnels through that single catalogue; there is no second
detector behind it. SDT-004 proposes closing the gap by importing a mature
public ruleset. That import is a licensing and architecture commitment that is
expensive to unwind once rules are in the tree and shipping inside a commercial
binary, so the acquisition posture is fixed here, before any rule lands.

### The gap is missing provider knowledge, not a smaller number

The module's original framing was "21 rules versus gitleaks' ~170". SDT-002
replaced that estimate with a measurement (2026-08-27, reproducible via
`pnpm secret:calibrate`), and the measurement says something sharper than the
rule count does:

| Measure                                 | Figure         |
| --------------------------------------- | -------------- |
| Detection — catalogue rules             | 21/21 = 100.0% |
| Detection — providers outside catalogue | 8/20 = 40.0%   |
| Detection — all planted secrets         | 29/41 = 70.7%  |
| False-positive rate                     | 1/13 = 7.7%    |

The 40% row is the one that matters, and it does not mean what it looks like.
Of the 8 out-of-catalogue providers detected, **zero are caught by a
provider-specific rule**: 5 fire on `High Entropy String` — a low-precision
backstop — and 3 on keyword rules. All 12 misses are "no rule matched"; not one
is a suppression, so the standing suspicion that CIB-080 traded detection for
quiet is unsupported.

Worse, part of the miss set is structurally unreachable by the current backstop.
Hex-alphabet providers (Datadog, Mailgun, Shopify, DigitalOcean, Postman) can
never be entropy-rescued: hex caps at 4.0 bits against a 4.5 threshold. No
tuning of the existing engine reaches them. Only provider knowledge does.

So the deficiency is not that Anvil has fewer rules. It is that Anvil has **no
provider knowledge at all** for what it misses, and a whole class of providers
is unreachable without it. That is what a vendored ruleset buys, and the 40% /
70.7% figures are the baseline SDT-004's detection gain must be measured
against — not the headline rule count.

### What must not be traded away

Anvil's differentiators live in the engine, not the patterns: provenance-carrying
suppressions (`AllowlistProvenance`), structured skip accounting (SDT-001 /
SDT-006 / SDT-008 coverage notes), and ADR-029 suppression authority. No
candidate third-party engine carries an equivalent. Detection *knowledge* is a
commodity; the governance semantics around it are not.

### Why the licence question is load-bearing

Anvil ships as a proprietary single binary (ADR-018). The most capable
open-source secret scanner, TruffleHog, is AGPL-3.0. AGPL is not a licence a
commercial binary can absorb, and the temptation to consult it — for its engine,
its rules, or its live-verification logic — will recur every time someone
revisits this area. A boundary that exists only in somebody's memory is not a
boundary.

## Decision

### 1. Vendor the gitleaks ruleset as data; the engine stays Anvil's

SDT-004 vendors the [gitleaks](https://github.com/gitleaks/gitleaks) ruleset,
which is MIT-licensed, converting each upstream rule into a **repo-owned
compiled pattern alongside the built-in catalogue** — the same seam
`SECRET_PATTERNS` uses, so a vendored rule can carry the confidence its
prefix-anchored shape warrants. The operator-facing `SecretPatternDef` config
type is deliberately **not** the conversion target and is deliberately **not**
extended with a confidence field: it is `Serialize`/`Deserialize`, so such a
field would become a config switch for opting arbitrary `.anvilrc` regexes out
of the false-positive filters with no provenance behind the claim. The rules
become data compiled into the current scanner. No third-party engine, binary, or
runtime enters the product.

Anvil's allowlist and suppression layer applies to vendored rules exactly as it
does to built-ins. MIT is already on the licence allow-list, so vendoring
gitleaks rules requires no licence-policy change.

#### Amendment history — §1 conversion target (Accepted 2026-08-30, operator)

The paragraph above is the amended text. As originally accepted, §1 named
Anvil's existing `SecretPatternDef` form as the conversion target. SDT-004 hit
that before writing a single rule and it does not survive contact with a
confidence-bearing tier.

`SecretPatternDef` is `{ name, pattern }`; it carries no confidence, and
`compile_custom_patterns` hardcodes `high_confidence: false` because "the
scanner cannot know whether a hand-written regex is structurally unambiguous".
That reasoning is correct *about operator regexes* and false about a vendored,
prefix-anchored provider rule where the match **is** the credential. Routed
through `SecretPatternDef`, every tier-1 rule would have inherited the fuzzy
false-positive stack — the `example`/`test` keyword allowlist and
`looks_like_code` — which is precisely what defeated textbook credentials in
issue #1800 and eats camelCase bindings in CIB-363. Tier 1 is entirely
prefix-anchored, so it would have hit all of it.

Extending `SecretPatternDef` with a confidence field was considered and rejected
on a second, independent ground: it is `Serialize`/`Deserialize` and
operator-facing, so the field becomes a config switch letting any `.anvilrc`
custom pattern opt itself out of the false-positive filters with no provenance
behind the claim — a privilege escalation on exactly the boundary issue #1800
established. The fix is to stop treating vendored rules as operator regexes, not
to weaken the rule about operator regexes.

The rest of §1 stands unchanged, including the sentence about the allowlist and
suppression layer — which the shipped implementation makes true **by
construction** rather than by convention, because the vendored rules are
appended to the single `DEFAULT_COMPILED_PATTERNS` set that the file scan, the
git-history scan, the env-surface scan and the MCP pre-write redactor all
already consult.

Evidence: `crates/anvil-checks/tests/secret_vendored_tier1.rs` demonstrates the
fuzzy stack eating three tier-1 credential shapes rather than asserting it, and
`plans/archive/modules/secret-detection-truth.aps.md` (SDT-004) records the blocker as
it was found.

### 2. Engine replacement is rejected

Replacing the scanner with gitleaks, TruffleHog, Nosey Parker, or Kingfisher is
rejected for three independent reasons, any one of which is sufficient:

- **Suppression provenance is lost.** `AllowlistProvenance` records *why* a
  candidate was suppressed and under whose authority (ADR-029). SDT-002
  demonstrated the value concretely: it could show that zero true positives were
  suppressed and that 12 of 13 benign suppressions had a firing non-vacuity
  control. No third-party engine emits this, and a detector that cannot explain
  a suppression cannot be governed.
- **The pure-Rust single-binary posture breaks.** gitleaks and TruffleHog are Go
  programs; adopting either means shipping a second binary or an FFI boundary.
  Nosey Parker carries C dependencies (Hyperscan/Vectorscan). All of these
  contradict the single-binary distribution model and the lean-daemon posture.
- **AGPL contaminates the commercial binary.** TruffleHog is AGPL-3.0.
  Linking, embedding, or deriving from it is incompatible with a proprietary
  distribution (ADR-018).

### 3. The copyleft boundary is held by an allow-list and an executable test

The governing work item asked for AGPL "added to the licence deny list in
`deny.toml` so the boundary is enforced, not remembered". Implementing that
literally is **not possible, and attempting it disables the licence gate**.
Two facts, both verified against this repository:

- **`attribution/deny.toml` has no `deny` array, and must not gain one.** Under
  cargo-deny `[licenses] version = 2`, `deny` is a *removed* key (upstream
  PR 611). Adding it does not produce a redundant-but-harmless entry: cargo-deny
  reports `error[deprecated]: this key has been removed`, fails configuration
  validation, and exits **before checking any licence**. A commit intended to
  make the boundary explicit would therefore switch the entire licence gate off
  while appearing to strengthen it.
- **AGPL is already rejected, by omission.** Under `version = 2` an `allow`
  list denies everything absent from it. That is strictly stronger than a
  blocklist, because it also catches copyleft nobody thought to enumerate.

The boundary is therefore kept as deny-by-omission, and "enforced, not
remembered" is satisfied by a test rather than by a config entry:
`scripts/ci/licence-boundary-agpl.test.sh` builds a two-crate fixture and runs
the **real** `attribution/deny.toml` against it, asserting that `AGPL-3.0-only`,
`AGPL-3.0-or-later`, the deprecated `AGPL-3.0` (the identifier TruffleHog itself
declares), and `GPL-3.0-only` are all rejected, that an otherwise identical MIT
fixture is accepted, and that no `[licenses].deny` array has been reintroduced.

The test runs in the `cargo-deny` job of `.github/workflows/rust.yml`, which is
already path-gated on `attribution/**` (CIB-205) — so it fires on precisely the
edits that could remove the boundary.

`attribution/licences.toml` remains the single source of truth (ATTRIB-006) and
is **not** extended with a denial concept. Its `deny` field is a
consumer-inclusion flag meaning "emit into `deny.toml`'s allow array", not
"deny this licence"; the schema has no way to express a denial, and giving it
one would recreate the second policy surface ATTRIB-006 removed.

### 4. Live verification is clean-room only

SDT-005's live-verification concept is **clean-roomed**. TruffleHog source,
including its detector and verification code, is never consulted, quoted, or
adapted. The permissible reference is
[Kingfisher](https://github.com/mongodb/kingfisher) (Apache-2.0), and provider
verification endpoints are taken from each provider's own public API
documentation. This constraint binds SDT-005 and is not re-litigable inside it.

### 5. Refresh procedure

A vendored asset with no refresh procedure is stale within a year, and stale
detection rules are worse than absent ones because they are trusted. The
procedure is named here so SDT-004 delivers it alongside the rules rather than
promising it.

**Script — `scripts/secret/refresh-gitleaks-ruleset.sh`.** It follows the
established repository idiom for generated artefacts (`expand-licences.sh`,
`generate-acknowledgements.sh`): regenerate in place by default, verify in
`--check` mode.

- Reads a pin file beside the vendored data recording the upstream repository,
  release tag, exact commit SHA, and the SHA-256 of the upstream source config.
- Fetches the upstream ruleset at the pinned commit and verifies the digest,
  aborting on mismatch rather than silently accepting moved content.
- Converts upstream rules into `SecretPatternDef` form.
- Regenerates a `PROVENANCE.md` beside the data carrying tag, commit, retrieval
  date, digest, and the upstream MIT licence text.
- `--check` re-derives and diffs, exiting non-zero on drift, so a hand-edited
  vendored rule file fails CI instead of quietly diverging from upstream.
- `--upgrade <tag>` is the only way to move the pin, making a version bump an
  explicit reviewable act.
- On success it runs `pnpm secret:calibrate`, so no ruleset change can land
  unmeasured against the SDT-002 corpus.

**Cadence — monthly, scheduled, issue-opening.** A scheduled workflow compares
the pinned tag against the latest upstream release and opens an issue when the
pin is behind. It deliberately opens an issue rather than an automatic PR: a
ruleset change moves both the detection rate and the false-positive rate, and
SDT-002's corpus report has to be read by a person before it lands. Monthly
keeps it inside the repository's scheduled-workflow cost posture; per-PR
enforcement is already covered by `--check` on the path-gated leg.

**Attribution — hand-curated, and it will not happen by itself.** Verified
against this repository: `ACKNOWLEDGEMENTS.md` is generated from *dependency
manifests* — `cargo-about` over `crates/anvil-cli/Cargo.toml` and
`license-checker` over `tools/dev/package.json`. A vendored data file is not a
dependency of either, so it appears in **neither** generated block and no
existing gate will ever notice its absence. The gitleaks attribution therefore
belongs in the hand-curated `## Thanks` section, with the upstream LICENSE
copied into the vendor directory, and the refresh script owns keeping it
truthful. Because MIT is already allow-listed and the ruleset is not a
dependency, no `licences.toml` or generated-file change is required.

### 6. Scope of this decision

This ADR fixes posture only. It vendors nothing. SDT-004 executes the import in
confidence tiers behind the SDT-002 corpus gate, and SDT-005 is bound by §4.

## Rationale

Vendoring rules as data is the only option that closes the measured provider-
knowledge gap while keeping the governance semantics that make Anvil's secret
check different from a scanner. The engine is where the differentiation lives
and the rules are where the commodity knowledge lives, so the boundary is drawn
between them.

On the licence boundary, the trade accepted is legibility for correctness. An
explicit AGPL entry would be more obvious to a reader skimming `deny.toml` —
and that is exactly what the work item asked for — but it cannot be expressed
in cargo-deny v2 without disabling the gate, and enumerating denied licences
would also weaken the reader's model from "everything unlisted is denied" to
"the denied things are listed". The test recovers the legibility in a form that
fails when it stops being true, and an orienting comment in `deny.toml` points
a reader who goes looking for AGPL at this ADR and that test.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Vendor gitleaks rules as data (chosen)** | MIT; closes the provider-knowledge gap; engine, suppression provenance and coverage accounting all preserved; stages behind the SDT-002 corpus | Vendored asset needs a refresh procedure; conversion cost; FP volume rises and must be measured per tier |
| Replace engine with gitleaks | Largest immediate rule coverage | Go binary or FFI; loses `AllowlistProvenance` and coverage accounting; breaks single-binary posture |
| Replace engine with TruffleHog | Best-in-class detection plus live verification | AGPL-3.0 — incompatible with a proprietary binary (ADR-018); Go binary; loses suppression provenance |
| Replace engine with Nosey Parker | Very fast; Rust | Hyperscan/Vectorscan C dependencies break the pure-Rust posture; loses suppression provenance |
| Replace engine with Kingfisher | Apache-2.0; Rust; live verification | Still an engine swap: loses suppression provenance and coverage accounting, the differentiators this module exists to protect |
| Write ~170 rules from scratch | No vendoring, no attribution | Months of work reproducing commodity knowledge; no upstream refresh path; strictly worse than MIT reuse |
| Add explicit AGPL `deny` entry to `deny.toml` | Reads as an explicit boundary | **Impossible under cargo-deny v2** — removed key, fails config validation, silently disables the whole licence gate; also duplicates policy outside ATTRIB-006's single source of truth |
| Rely on deny-by-omission with no test | Zero new machinery | Unfalsifiable — the boundary is an absence, so nothing fails when it is removed; the item's "enforced, not remembered" requirement is unmet |

## Consequences

- **Positive:** the measured provider-knowledge gap becomes closable, and every
  tier of closure is measured against a committed corpus rather than argued.
  The copyleft boundary gains a test that goes red on the realistic removal
  path (adding the licence to `licences.toml`), and a second probe that blocks
  the specific well-intentioned edit that would disable the licence gate. SDT-004
  and SDT-005 both start with their licence question already answered.
- **Negative:** Anvil takes on a vendored asset with a maintenance obligation —
  a script, a scheduled cadence, and a hand-curated attribution entry that no
  generated gate will keep honest. Vendored breadth also raises false-positive
  volume, which beta already complains about.
- **Risks:** the refresh cadence is the part most likely to rot, because it is
  the part with no per-PR gate behind it. A tier that raises FP cost beyond its
  detection gain must park rather than ship; the corpus makes that a
  measurement, not an argument.
- **Mitigations:** `--check` on the path-gated CI leg catches hand-edited or
  drifted vendored data on every PR that touches it; the digest-verified pin
  makes a silently moved upstream fail loudly; `pnpm secret:calibrate` runs on
  every refresh so no ruleset change lands unmeasured.
- **Known limit, stated rather than glossed:** cargo-deny polices the
  *dependency graph*. It cannot see a data file. The test in §3 proves that an
  AGPL **dependency** is rejected; it does not and cannot prove that nobody
  pasted AGPL-derived rule text into a vendored data file. That half of the
  boundary is held by §1's MIT-only sourcing, §4's clean-room constraint, the
  digest-verified upstream pin, and code review — not by an automated gate.

## References

- Related ADRs: ADR-018 (source-proprietary product/IP model), ADR-029
  (suppression authority), ADR-002 (warnings over blocks), ADR-087
  (`insecure-construction` category — credentials stay with the `secret` check)
- APS module: SDT-003, in
  [`secret-detection-truth`](../archive/modules/secret-detection-truth.aps.md);
  binds SDT-004 (vendoring) and SDT-005 (live verification)
- Baseline measurement: SDT-002, `crates/anvil-checks/tests/secret_calibration.rs`,
  corpus at `crates/anvil-checks/tests/corpus/secret/`, reproduce with
  `pnpm secret:calibrate`
- Licence infrastructure: ATTRIB-006 (`attribution/licences.toml` as single
  source of truth), `tools/starters/acknowledgements/expand-licences.sh`,
  CIB-205 (`attribution/**` path gate on the cargo-deny job)
- Boundary test: `scripts/ci/licence-boundary-agpl.test.sh` (`pnpm
  test:licence-boundary`)
- External: [gitleaks](https://github.com/gitleaks/gitleaks) (MIT),
  [Kingfisher](https://github.com/mongodb/kingfisher) (Apache-2.0),
  [cargo-deny PR 611](https://github.com/EmbarkStudios/cargo-deny/pull/611)
  (removal of the `[licenses].deny` key)
