<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Secret-Detection Truth (Coverage, Catalogue, Calibration)

| ID  | Owner | Status   | Progress |
| --- | ----- | -------- | -------- |
| SDT | —     | Proposed | 0/5      |

**Last reviewed:** 2026-08-15 (created on operator direction after a
cross-product review from `eddacraft/edda-scan`'s SEC-005 work surfaced the
fail-open path, and beta feedback reported ~50% detection of planted secrets
plus false-positive complaints. The assessment that produced this module —
including the licence survey and the rejected alternatives — is summarised in
each item's provenance below.)

## Purpose

Make the `secret-detection` check tell the truth about its own coverage, and
close the gap between what it claims and what beta testing measured.

Three defects motivate this module, in order of severity:

1. **A silent false-clean.** The SCAN-002 oversize-line guard skips any line
   over `max_line_bytes` before pattern *and* entropy evaluation run
   (`ScanStats` doc: "neither pattern matching nor entropy scanning ran for
   it"). The gate ignores the counter: `assemble_secret_check_result` carries
   `lines_skipped_oversize` on the result, but `passed`, `score`, and the
   message do not consult it, so a file whose only secret sits inside an
   oversize line reports **"No secrets detected", passed, score 100**. The
   save-time intercept is one step worse: `anvil-intercept-rules/src/secret.rs`
   calls `scan_content_with_limit`, the documented "legacy entry point that
   drops the SCAN-002 stats", so it structurally cannot warn. The repository
   already states the governing principle twice — the history-scan-error path
   ("otherwise a broken scan looks identical to 'clean'", score 0) and the
   gctx egress redactor (`save_time.rs`: "a line too long to scan cannot be
   proven clean", fail-closed) — and the gate applies it to neither.
2. **A thin catalogue.** The built-in pattern set is 21 real patterns.
   Dedicated scanners carry an order of magnitude more (gitleaks ~170 rules),
   which by itself predicts a miss rate in the region beta reported. Every
   product path — gate, save-time intercept, capsule, gctx redaction — funnels
   through this single catalogue; there is no second detector behind it.
3. **An unmeasured detector.** No committed corpus measures detection or
   false-positive rate, so the beta number cannot be decomposed into its
   causes (catalogue gaps vs oversize skips vs CIB-080 over-suppression), and
   no rule change can be shown to help rather than harm.

The strategic boundary this module holds: the **engine stays**. Anvil's
provenance-carrying suppressions (`AllowlistProvenance`), structured skip
accounting, and ADR-029 suppression authority are differentiators no candidate
third-party engine has. What gets vendored is detection **knowledge as data**;
what gets adopted from elsewhere is at most a clean-roomed concept.

## In Scope

- Fail-closed handling of unscanned surface in the gate result and the
  save-time intercept diagnostic.
- A committed calibration corpus (true positives in canary format per rule;
  known-benign vectors seeded from the CIB-080 fixtures) with a CI-visible
  detection / false-positive report on any rules change.
- An ADR fixing the ruleset acquisition posture: vendor the gitleaks ruleset
  (MIT) as data compiled into the existing engine; AGPL sources (TruffleHog)
  added to the licence deny list; refresh procedure documented.
- The vendored ruleset itself, staged in confidence tiers behind the corpus
  gate, with ruleset version recorded in finding provenance.
- A flag-gated, opt-in, clean-roomed live-verification concept (severity by
  verifiability) — shaped here, deliberately last.

## Out of Scope

- Replacing the scanner engine with gitleaks / TruffleHog / Nosey Parker /
  Kingfisher (loses suppression provenance; Go binary or C deps break the
  pure-Rust single-binary posture; AGPL contaminates the commercial binary).
- Taint analysis or SAST-class detection (ADR-087 boundary).
- Org-wide or remote-repository scanning — the product scope stays "this
  developer's machine, at save and at gate".
- Raising `max_line_bytes` as the "fix" — the guard is a legitimate ReDoS
  bound; the defect is the silence, not the skip.

## Interfaces

**Depends on:**

- `crates/anvil-checks/src/secret/` — scanner, patterns, entropy, git history.
- `crates/anvil-intercept-rules/src/secret.rs` — save-time rule.
- ADR-029 suppression authority; `attribution/` + `ACKNOWLEDGEMENTS.md`
  licence infrastructure (`cargo-about`, `deny.toml`).
- Flags manifest (`flags/manifest.json`) for SDT-005.

**Exposes:**

- An honest `secret-detection` result whose clean pass means "scanned and
  clean", never "skipped and silent".
- A published, reproducible detection-rate figure per ruleset version.

## Work Items

### SDT-001: Fail closed on unscanned lines

- **Status:** Proposed
- **Intent:** A clean secret-detection result must mean every line was
  actually scanned; unscanned surface blocks a clean pass and is named.
- **Expected Outcome:** `lines_skipped_oversize > 0` blocks `passed` and caps
  `score` in `assemble_secret_check_result`, with the message naming the count
  ("N line(s) too long to scan"), mirroring the existing history-scan-error
  precedent. The save-time intercept receives the stats (a stats-carrying
  variant replaces the `scan_content_with_limit` call) and emits a diagnostic
  for skipped lines rather than staying silent. The gctx egress redactor's
  existing fail-closed behaviour is unchanged. Git-history scan skips follow
  the same rule. Tests cover gate, save-time, and history paths; the
  oversize-skip test that today asserts "0 findings, passed" is inverted to
  assert the blocked pass.
- **Validation:** `cargo test -p eddacraft-anvil-checks secret::`,
  `cargo test -p eddacraft-anvil-checks --test secret_detection`,
  `cargo test -p eddacraft-anvil-intercept-rules`
- **Files:** `crates/anvil-checks/src/secret/check.rs`,
  `crates/anvil-checks/src/secret/scanner.rs`,
  `crates/anvil-intercept-rules/src/secret.rs`
- **Dependencies:** —
- **Confidence:** high
- **Risks:** A repo with legitimately long lines in scanned extensions would
  flip from silent-green to warn; the message must say what to do (raise
  `max_line_bytes`, or suppress per ADR-029 with a reason), or this trades a
  false-clean for an unactionable red.

---

### SDT-002: Calibration corpus and measured detection rate

- **Status:** Proposed
- **Intent:** No rules change ships unmeasured; the beta "~50% detection"
  becomes a decomposed, reproducible number instead of an anecdote.
- **Expected Outcome:** A committed corpus of true positives (canary-format
  credentials per built-in rule — never live values) and known-benign vectors
  (seeded from the CIB-080 zod/base64/KSUID fixtures) with a runner that
  reports detection rate, false-positive rate, and per-rule misses. CI prints
  the before/after on any change under `secret/`. The first run against the
  current 21-pattern catalogue is recorded in this module as the baseline,
  decomposing the beta figure into catalogue gaps, oversize skips, and any
  CIB-080 over-suppression it reveals.
- **Validation:** corpus runner green in CI; baseline figures recorded here.
- **Files:** `crates/anvil-checks/tests/`, corpus fixtures (location per
  implementation), CI workflow.
- **Dependencies:** —
- **Confidence:** high
- **Risks:** Canary keys in the tree will themselves trip secret scanners
  (including Anvil's own gate and GitHub push protection); the corpus format
  must be constructed to be recognisably synthetic and allowlisted once,
  deliberately, with provenance.

---

### SDT-003: ADR — ruleset acquisition posture (rules as data)

- **Status:** Proposed
- **Intent:** Fix the acquisition decision in the decision log before any
  vendoring: detection rules are vendored data, the engine is Anvil's, and
  the licence boundary is explicit.
- **Expected Outcome:** An accepted ADR recording: gitleaks ruleset (MIT)
  vendored as data and converted to `SecretPatternDef` form; engine
  replacement rejected with reasons (provenance loss, binary/deps posture,
  AGPL); TruffleHog/AGPL added to the licence deny list in `deny.toml` so the
  boundary is enforced, not remembered; live verification concept noted as
  clean-room-only with Kingfisher (Apache-2.0) as the permissible reference;
  an upstream refresh procedure named (script + cadence + attribution entry),
  the DELIV-002-style lesson that a vendored asset without a refresh
  procedure is stale in a year.
- **Validation:** ADR accepted in `plans/decisions/` + `DECISION-LOG.md`
  entry; `pnpm adr:check` green; `deny.toml` rejects an AGPL test entry.
- **Files:** `plans/decisions/`, `attribution/deny.toml`,
  `ACKNOWLEDGEMENTS.md`
- **Dependencies:** —
- **Confidence:** high

---

### SDT-004: Vendored ruleset, staged behind the corpus

- **Status:** Proposed
- **Intent:** Close the catalogue gap (21 → ~170 rules) as measured
  improvement, not a pattern dump.
- **Expected Outcome:** The converted ruleset lands in confidence tiers —
  high-confidence provider patterns first; generic/entropy-adjacent rules
  staged behind FP review — each tier shipping only with a corpus run showing
  detection gain and FP cost. Ruleset version appears in finding provenance
  and in the corpus report. Anvil's allowlist/suppression layer applies on
  top of vendored rules exactly as it does to built-ins. Attribution recorded
  per SDT-003.
- **Validation:** corpus before/after per tier; existing secret suites green;
  dogfood FP check on the anvil repo itself before default-enabling each
  tier.
- **Files:** `crates/anvil-checks/src/secret/patterns.rs` (or the data files
  the ADR chooses), `crates/anvil-checks/tests/`, `ACKNOWLEDGEMENTS.md`
- **Dependencies:** SDT-001, SDT-002, SDT-003
- **Confidence:** medium
- **Risks:** FP volume is the known cost of breadth — beta already complains
  about FPs, so a tier that raises FP rate beyond its detection gain parks
  rather than ships; the corpus makes that a measurement, not an argument.

---

### SDT-005: Opt-in live verification (severity by verifiability)

- **Status:** Proposed
- **Intent:** Convert "looks like a secret" into "is a secret" for checkable
  providers, as the strongest FP lever available — without making a
  local-first governance tool phone home by default.
- **Expected Outcome:** Behind a flag, off by default, loudly disclosed: a
  candidate credential for a supported provider (initial set of ~5–10:
  GitHub, AWS, Stripe, OpenAI, Slack) can be verified against the provider's
  check endpoint; verified-live escalates severity, verified-dead or
  unverifiable de-escalates with the verification state named in the finding.
  Clean-roomed concept only — no AGPL source consulted; Kingfisher
  (Apache-2.0) is the permissible reference per SDT-003. Network egress is
  per-provider, disclosed in the finding and the docs, and never buffers the
  credential anywhere beyond the verification request itself.
- **Validation:** contract tests with mocked provider endpoints; a live smoke
  is operator-run, not CI; flag off ⇒ byte-identical behaviour to today.
- **Files:** `crates/anvil-checks/src/secret/`, `flags/manifest.json`, docs.
- **Dependencies:** SDT-003, SDT-004
- **Confidence:** low — deliberately last; the egress policy question is the
  hard part, not the HTTP.

---

Ranking is deliberate: SDT-001 is the "claims protected when it fails"
complaint verbatim and touches nothing else; SDT-002 must exist before
SDT-004 so breadth lands measured; SDT-003 keeps the licence boundary a
decision rather than an accident; SDT-005 is the biggest FP lever but the
only item with an egress question, so it goes last. Promoting any item to
Ready is an operator decision.
