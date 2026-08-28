<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Secret-Detection Truth (Coverage, Catalogue, Calibration)

| ID  | Owner | Status   | Progress |
| --- | ----- | -------- | -------- |
| SDT | —     | In Progress | 2/6   |

**Last reviewed:** 2026-08-28 (SDT-001 and SDT-002 Merged via
[#4185](https://github.com/eddacraft/anvil-001/pull/4185), reviewed against
`src/secret/check.rs`, `src/secret/scanner.rs`,
`anvil-intercept-rules/src/secret.rs` and `tests/secret_calibration.rs`. The
measured baseline is recorded under SDT-002 and the module's third motivating
defect — an unmeasured detector — is now closed. Defect 1 is closed at line
granularity only: SDT-006 was filed for the whole-file half. Defect 2, the thin
catalogue, is unchanged and now quantified rather than estimated. Originally
created 2026-08-15 on operator direction after a
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

- **Status:** Merged — via
  [#4185](https://github.com/eddacraft/anvil-001/pull/4185) (rebase-merge
  2026-08-27; ancestor of `main` proven by content at `817e2849f`).
  Operator-promoted 2026-08-27 (Proposed → Ready → In Progress in one step,
  on direction).
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
- **As built (2026-08-27):** Every reason a scan could not see all of its
  input now collects in one place (`coverage_notes`), so `passed` is blocked
  by an oversize skip exactly as it is by a history-scan error, and the two
  compose instead of swallowing each other. `score` is 0 on any oversize
  skip. Message: `N line(s) too long to scan, so this result cannot prove
  them clean: raise` `max_line_bytes` `to cover them, or suppress with a
  documented reason (ADR-029)`. The intercept routes through
  `scan_content_with_limit_and_stats` and emits a `Severity::Warning`
  diagnostic; `evaluate()` still returns `Allow`, so an unscanned line never
  interrupts a save — an admission about coverage is not a detection, and
  blocking saves on minified files would violate warnings-over-blocks.
  History skips needed no separate handling: `merge_git_history_scan`
  already folded them into the same counter, verified end-to-end by
  `history_oversize_skip_blocks_a_clean_pass`.
- **Deviations from the written Expected Outcome (both deliberate):**
  1. *Score is zeroed, not capped, and the history precedent is not mirrored
     exactly.* The precedent zeroes only when `findings.is_empty()`, so
     "finding + history error" scores 90 while "finding + oversize skip"
     scores 0. Implemented per operator decision (score 0). Blind review
     called the asymmetry "internally incoherent" but not a contract breach,
     and recommends a one-line follow-up zeroing on either condition — the
     history half is pre-existing behaviour outside this item's scope.
  2. *No test was inverted, because none existed to invert.* The written
     outcome assumed a test asserting "0 findings, passed" on an oversize
     skip. Verified against the base revision: the only oversize tests are
     in `scanner.rs` and `git_scanner.rs` and assert scanner-level facts
     that remain correct. The missing check-level assertion was added
     instead.
- **Dogfood measurement:** across every tracked file minus default
  `skip_extensions`, exactly 3 have lines over the 4096-byte default — all
  `plans/**` markdown with long table rows (`completed-index.aps.md`,
  `index.aps.md`, `DECISION-LOG.md`), no source file. No workflow runs
  `anvil gate` against this repo, so CI is unaffected. The recorded Risk
  landed smaller than feared.
- **Known gap (not this item):** whole-*file* skips remain silent —
  `skip_extensions`, the file-size limit, an unreadable file, and the
  SCAN-001 `catch_unwind` panic arm all drop a file with no counter, so a
  panicking custom regex still yields "No secrets detected", passed, score
  100. That is the same defect class as this item's, one level up. Named by
  blind review; candidate follow-up.
- **Validation evidence (2026-08-27):** `cargo test -p eddacraft-anvil-checks`
  exit 0 (597 lib + 26 `secret_detection`, 18 targets, 0 failed);
  `cargo test -p eddacraft-anvil-intercept-rules` exit 0 (104 passed);
  `cargo clippy --workspace --all-targets -- -D warnings` exit 0;
  `cargo fmt --check` exit 0 both crates. Guard tests proven RED by
  reverting the production predicates: 5 of 8 fail on reversion, the other 3
  pin the rejected alternative rather than the change.

---

### SDT-002: Calibration corpus and measured detection rate

- **Status:** Merged — via
  [#4185](https://github.com/eddacraft/anvil-001/pull/4185) (rebase-merge
  2026-08-27; ancestor of `main` proven by content at `817e2849f`).
  Operator-promoted 2026-08-27 (Proposed → Ready → In Progress in one step,
  on direction).
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

#### SDT-002 baseline — 2026-08-27, 21-pattern catalogue

Reproduce with `pnpm secret:calibrate`. Corpus:
`crates/anvil-checks/tests/corpus/secret/` (55 cases); runner:
`crates/anvil-checks/tests/secret_calibration.rs`.

| Measure                                | Figure         |
| -------------------------------------- | -------------- |
| Detection — catalogue rules            | 21/21 = 100.0% |
| Detection — providers outside catalogue | 8/20 = 40.0%   |
| Detection — all planted secrets         | 29/41 = 70.7%  |
| False-positive rate                     | 1/13 = 7.7%    |
| Per-rule misses among the 21 built-ins  | none           |
| Built-in rules with no test case        | none           |

Reported separately and excluded from the rates: an oversize probe (a `ghp_`
canary past `max_line_bytes`) is **not** detected, `lines_skipped_oversize=1`
— the SDT-001 false-clean, measured. Folded in, the planted figure would read
29/42 = 69.0%. The single false positive is the CIB-080 Google-Drive `id=`
residual, which that review left deliberately unsuppressed.

**Decomposition — what the beta "~50%" is made of.** Catalogue gaps dominate
and nothing else comes close:

- **Catalogue gaps: all 12 out-of-catalogue misses are "no rule matched."**
  Not one is a suppression. GitLab, OpenSSH, EC private key, DigitalOcean,
  Mailgun, Shopify, Discord, Slack webhook URL, `https://user:pass@`, Azure
  Storage, Postman, New Relic.
- **CIB-080 over-suppression: zero.** No true positive is suppressed by any
  allowlist tier. Every benign case's suppression carries provenance
  (`BuiltinBenignFixture` / `BuiltinKeyword` / `BuiltinShape`) and 12 of 13
  have a non-vacuity control that fires, so the suppressions are doing work
  rather than the scanner never having cared. The standing suspicion that
  CIB-080 traded detection for quiet is not supported.
- **Oversize skips:** real (measured above) but not a volume driver here.

**Read the 40% carefully.** Of the 8 out-of-catalogue detections, only 3 come
from a pattern rule (Datadog via the `DD_API_KEY=` keyword; JDBC and age via
`Generic Secret`). The other 5 fire on `High Entropy String` — a
low-precision backstop, not provider knowledge. **Zero** are caught by a
provider-specific rule. Hex-alphabet providers (Datadog, Mailgun, Shopify,
DigitalOcean, Postman) can never be entropy-rescued: hex caps at 4.0 bits
against a 4.5 threshold. SDT-004's detection gain must be measured against
that, not against the headline.

**What this baseline cannot say.** It does not reproduce the beta ~50%. That
was field data on unknown planted shapes; 70.7% is a property of *this*
corpus composition (21 canaries + 20 chosen providers) and moves when the
composition moves — the runner prints that caveat on every run. The
composition-independent figures are the 21/21 catalogue coverage and the
named miss list. It also measures the pattern/entropy engine only: the runner
calls the scanner directly, so `skip_extensions`, the file-size limit and the
SCAN-001 panic arm are outside its reach — which is exactly where SDT-001's
known gap lives.

- **As built (2026-08-27):** One case per file so radius-2, path-sensitive
  CIB-080 rules see faithful context. The `.corpus` extension keeps the
  repo's own walkers out, while the manifest declares a realistic scan path
  (`src/payments/stripe.ts`, `packages/zod/.../string.test.ts`) that is
  handed to the scanner as `file_path` — so nothing about the real scan path
  is faked. Canaries are literal and full-shape but built from a `CANARY`
  marker plus fixed filler, which breaks provider checksums by construction;
  `PROVENANCE.md` records that as load-bearing so nobody later "fixes" one
  into a valid key. The gate fails on drift in **either** direction, so an
  improvement ships measured too, and it rides the required `Test` check
  (confirmed: `Test` is a required status check on `main`, path-gated on
  `crates/**`, running the affected Rust project — so a rules change cannot
  avoid it).
- **The corpus nearly poisoned the repo.** `manifest.json` and
  `PROVENANCE.md` both tripped anvil's own scanner on first write (high
  entropy; AWS/STS/credit-card literals quoted in prose). Both fixed, and now
  guarded by `corpus_metadata_stays_clean_under_the_scanner`, which scans the
  corpus's own metadata and fails if it carries a credential shape.
- **Known gaps (named by blind review, not blocking):** the vacuity gate
  asserts that a *declared* control fires but does not require one, so a
  future benign case added with no control banks an unproven "false positive
  avoided"; and the before/after CI leg has never been executed end-to-end.
- **Validation evidence (2026-08-27):** `cargo test -p eddacraft-anvil-checks`
  exit 0 including `secret_calibration` (2 passed);
  `bash scripts/ci/workflow-contracts.test.sh` exit 0; `pnpm format:check`
  exit 0. Blind review stripped all four per-case config overrides and
  re-ran: false-positive rate unchanged at 1/13, no per-case drift — the
  overrides are inert, not measurement-weakening.

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

### SDT-006: Fail closed on unscanned files

- **Status:** In Progress — operator-promoted 2026-08-28 (Proposed → Ready →
  In Progress on direction). Implemented on `feat/sdt-006-unscanned-files`.
- **Intent:** SDT-001 made unscanned *lines* honest. Whole *files* are still
  dropped silently, so the same false-clean survives one level up: a clean
  result can still mean "we never read it".
- **Expected Outcome:** `run_secret_check` accounts for every file it
  declined to scan, and that accounting reaches the result the way
  `lines_skipped_oversize` now does — blocking a clean pass and naming the
  cause. Four paths drop a file today with no counter at all
  (`crates/anvil-checks/src/secret/check.rs`, the `par_iter().filter_map`
  body):
  1. `should_skip_file(...)` — a configured `skip_extensions` match;
  2. `file_exceeds_size_limit(file)` — the 1 MiB `MAX_FILE_SIZE` guard;
  3. `fs::read_to_string(file).ok()?` — an unreadable or non-UTF-8 file,
     swallowed by `.ok()?`;
  4. `Err(_) => None` — the SCAN-001 `catch_unwind` panic-containment arm.
  **Path 4 is the sharpest and should land first:** a panicking custom regex
  silently discards an entire file's scan and the result still reports "No
  secrets detected", `passed = true`, `score = 100`. That is precisely the
  defect class this module exists to eliminate, and SCAN-001 contained the
  panic without surfacing it.
  Not every skip deserves equal weight — a configured `.png` skip is a
  deliberate operator choice and a panic is a bug — so the outcome must
  distinguish *deliberate* exclusions from *failed* ones rather than
  flattening both into one red.
  **Operator decision, 2026-08-28:** genuine failures (unreadable file,
  SCAN-001 panic) **and** the file-size limit block a clean pass; a configured
  `skip_extensions` match is reported but never blocks. Blocking on
  `skip_extensions` is rejected outright — the defaults include `.png`,
  `.jpg` and `.lock`, which every repository has, so it would redden every
  clean pass everywhere. The size limit blocks because it is the direct
  analogue of `max_line_bytes`, which SDT-001 already made blocking, and
  because it hides a real credential path: measured on this repository, the
  three tracked files over the 1 MiB limit are two `.json` audits and
  **`pnpm-lock.yaml`** — all in gate-scanned extensions. The lockfile is
  supposed to receive the URL-credential scan (GH #2584) but is dropped whole
  before that scan can run, so anvil cannot currently prove its own lockfile
  free of credentials. The base test
  `check.rs::skips_files_exceeding_size_limit` currently asserts
  `result.passed` and will need revisiting under whatever distinction is
  chosen.
- **Non-scope / do not:** do not raise `MAX_FILE_SIZE` or `max_line_bytes` —
  both are legitimate resource bounds, and SDT-001 already established that
  the defect is the silence, not the skip. Do not remove the SCAN-001
  `catch_unwind`; containing the panic is correct, reporting nothing is not.
  Do not revisit the SDT-001 line-level machinery, which is delivered.
- **Validation:** `cargo test -p eddacraft-anvil-checks`;
  `cargo test -p eddacraft-anvil-intercept-rules`; a panicking custom pattern
  produces a non-passing result naming the file; `pnpm secret:calibrate`
  shows no regression. Note that the SDT-002 corpus **cannot** currently
  detect this class — its runner calls the scanner directly rather than
  `run_secret_check`, so the file-selection layer is outside its reach;
  extending the corpus to cover file selection is part of this item.
- **Files:** `crates/anvil-checks/src/secret/check.rs`,
  `crates/anvil-checks/src/secret/types.rs` (result shape),
  `crates/anvil-checks/tests/`
- **Dependencies:** SDT-001
- **Confidence:** high on the defect and its location — all four paths were
  read directly and the panic arm reproduced in review; medium on the right
  reporting shape, because deliberate exclusions and failed reads should not
  read identically to an operator.
- **Identified From:** blind verification of SDT-001/-002, 2026-08-27, rated
  MAJOR-advisory. SDT-001's Expected Outcome named `lines_skipped_oversize`
  specifically and was fully delivered; its *Intent* sentence ("every line
  was actually scanned") is broader than the delivery, and this item closes
  the difference.
- **As built (2026-08-28):** Every candidate file now returns *why* it was not
  scanned. Failures (unreadable, SCAN-001 panic) and the size limit block a
  clean pass and zero the score; a `skip_extensions` match increments an
  advisory counter and never blocks, guarded independently by a unit test and
  an integration test. Blocking causes carry normalised paths (same shape as
  finding paths), the advisory case carries only a count because that
  population is unbounded. Notes compose with SDT-001's line and history notes.
  Two things the work surfaced: `should_skip_file` returns early for lockfiles
  so they reach the GH #2584 URL-credential scan, which means `skip_extensions`
  *cannot* exclude an oversize lockfile — the note says so and names the file
  rather than offering a remedy that does nothing; and `anvil gate` rendered a
  coverage-only failure as "Potential secrets found in 0 location(s):" above an
  empty list, so coverage notes now get their own block there.
- **Validation deviation, accepted 2026-08-28:** this item's Validation says
  "extending the corpus to cover file selection is part of this item". That was
  **not** satisfied as written, deliberately. The SDT-002 corpus measures the
  pattern/entropy engine — it feeds byte strings to `scan_content_with_stats`
  and its cases are `.corpus` files intentionally invisible to path-based
  walkers, which is the opposite of what selection tests need (real paths, real
  extensions, real sizes, real read failures). Selection is also a pass/fail
  contract, not a detection *rate*, so modelling it as a corpus rate would add
  manifest machinery and assert less. Equivalent coverage is delivered in
  `crates/anvil-checks/tests/secret_file_coverage.rs`, and the calibration
  runner's docs now state the boundary so its "outside its reach" limitation is
  no longer an unqualified claim.
- **Known gap, NOT closed here:** `anvil gate` is the only surface that reports
  these coverage failures. `anvil audit` (`audit.rs:312`) reads
  `result.findings` only and exits 0 over an unread file. Planless `anvil check`
  is worse: it *pre-filters* oversize and extension-skipped files via
  `is_secret_scannable` before calling `run_secret_check`, so the accounting
  never arrives and wiring it up means undoing that pre-filter, not reading a
  new field. Pre-existing on both — SDT-001's `lines_skipped_oversize` is
  equally unread there — and neither surface is in this item's Files list.
  Blind review rated it major-advisory and asked that it be raised as a
  follow-up before the module closes.
- **Verification (2026-08-28):** blind review returned `pass-with-findings`,
  no blocking findings. It ran 13 independent production mutations; 11 turned
  the new tests red, including one that removes the SCAN-001 `catch_unwind`
  this item's Non-scope forbids removing — a guarantee no test previously
  enforced. The two survivors (unpinned `display_path` normalisation, and the
  lockfile caveat firing without naming its file past the 3-path cap) were
  repaired in this branch with tests proven RED against those exact mutations.

---

Ranking is deliberate: SDT-001 is the "claims protected when it fails"
complaint verbatim and touches nothing else; SDT-002 must exist before
SDT-004 so breadth lands measured; SDT-003 keeps the licence boundary a
decision rather than an accident; SDT-005 is the biggest FP lever but the
only item with an egress question, so it goes last. SDT-006 was added after
SDT-001 shipped, on blind-review evidence that the same false-clean survives
at file granularity; it ranks with SDT-001 in kind, and its panic-arm slice
is the highest-value part. Promoting any item to Ready is an operator
decision.
