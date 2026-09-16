# Secret-detection calibration corpus — provenance

SDT-002. This directory is a deliberate, allowlisted collection of
credential-shaped strings inside a repository whose product scans for
credential-shaped strings. Read this file before you touch a case file.

## What lives here

| Path             | What it is                                                  |
| ---------------- | ----------------------------------------------------------- |
| `manifest.json`  | Every case, its scan path, its committed expected outcome.  |
| `cases/*.corpus` | One case per file: the literal bytes handed to the scanner. |
| `PROVENANCE.md`  | This file — the safety contract for the values in `cases/`. |

The runner is `crates/anvil-checks/tests/secret_calibration.rs`. Run it and see
exactly what CI prints:

```bash
pnpm secret:calibrate
# or, without pnpm:
cargo test -p eddacraft-anvil-checks --test secret_calibration -- --nocapture
```

## The canary contract

Every planted credential in `cases/` is **structurally full-shape for Anvil's
regex and deliberately invalid to the provider that issues it**. Both halves are
load-bearing:

- Full shape, because the corpus must exercise the real scan path byte-for-byte.
  The values are literal in the fixture files. They are not assembled at runtime
  and they do not use fake prefixes — either trick would measure a string the
  scanner never sees in production.
- Provider-invalid, because a credential-shaped literal in a public-facing
  repository must never be mistakable for a live one. Most major providers embed
  a checksum over the token body (GitHub's `ghp_…` carries a CRC over its body;
  npm, Stripe and others have comparable structure or partner validation). A
  body built from the literal marker `CANARY` plus a fixed filler run fails
  those checks by construction.

Concretely, the canaries are built from:

- the marker `CANARY`, present in every hand-built body, so a human reading the
  file cannot mistake it for a real key;
- a fixed filler `xK7mQ2pV9tR4wS6yE3zA8bN5cM1dF0gH`, truncated to whatever
  length the provider's shape requires. It is high-variety on purpose so the
  entropy backstop is measured under realistic conditions rather than
  artificially defeated by a run of zeroes;
- `0cafe000` repeated, where the provider's alphabet is hex-only and `CANARY`
  cannot be spelled. Hex tops out at 4.0 bits of entropy, below the 4.5
  threshold, so these cases can never be rescued by the entropy pass — that is a
  true property of hex-alphabet credentials, not an artefact;
- upstream-published examples where they exist and are safer than anything we
  could invent: AWS's own documented `AKIA…EXAMPLE` access key, its `ASIA…` STS
  twin and the matching `wJalrXUtnFEMI/…EXAMPLEKEY` secret (the first of which
  `src/secret/patterns.rs` already quotes), and the industry-published Visa test
  PAN beginning `4111`. The full values live in the case files, not in this
  document — see "Why this file spells nothing out" below;
- `.invalid` hosts (RFC 2606) wherever a canary carries a hostname.

`cases/bn-zod-base64url-vector.corpus` carries the first 30 bytes of
`SHA-256("anvil-sdt-002-base64url-8")` in base64url. It is opaque by design and
reproducible from that one line.

**Never replace a canary with a live value, and never "fix" one into a valid
key.** An invalid checksum here is the feature. If a future contributor or tool
reports these as broken keys, the correct response is to point at this file.

## Why this file spells nothing out

This document deliberately carries no full-shape credential literal. Anvil's own
gate scans `.ts/.tsx/.js/.jsx/.rs/.py/.go/.sh/.json/.yaml/.yml/.toml/.env`, and
the opt-in `discovery_repro` harness additionally walks `.md` — so a textbook
AWS key quoted here would surface as a finding in anvil's own dogfood runs, for
no benefit. Every literal lives in `cases/*.corpus`, which no walker selects.
`tests/secret_calibration.rs` enforces the boundary: it scans this file,
`manifest.json` and the runner itself, and fails if any of them carries a
credential-shaped value.

## If you need a canary for a new rule

1. Read the rule's regex in `src/secret/patterns.rs`. Do not guess the shape.
2. Build the shortest string that satisfies it end-to-end, using `CANARY` plus
   the filler above (or hex `0cafe000` when the alphabet forbids letters).
3. Never substitute a fake prefix to dodge a scanner. A canary that does not
   match the real rule measures nothing, and this corpus exists to stop exactly
   that kind of comfortable non-measurement.
4. Add the case to `manifest.json`, run the runner, and commit the measured
   outcome. The runner fails if any built-in rule has no true-positive case.

If a rule genuinely cannot be given a full-shape-but-invalid canary, leave it
uncovered and say so in the module rather than shipping a canary that does not
match. The runner names uncovered rules in its report.

## GitHub push protection

Push protection is enabled on this repository. The canaries are built to be
rejected by provider validation, which is what push protection's partner checks
consult for the shapes that support it. If a push is nevertheless blocked on one
of these files, the remedy is an explicit bypass citing this file — **not**
editing the canary until the detector stops noticing it, which would silently
destroy the measurement.

The repository's own `Secret Scan` job runs TruffleHog with `--only-verified`,
so unverifiable canaries do not trip it.

## Why the `.corpus` extension

The case files carry a `.corpus` extension rather than `.ts` / `.env` / `.pem`
so that repository-wide walkers which select files by source extension (the
`discovery_repro` harness, the welcome scan) do not re-report the corpus as
findings in the anvil repo itself. The scan path each case is _measured_ under
is declared in `manifest.json` (`scan_path`) and is a realistic source path —
the scanner's path- and context-sensitive rules see exactly what they would see
in a real project. The on-disk name changes nothing about what is scanned.

## The false-positive half

`bn-*` cases are known-benign vectors, seeded from the CIB-080 review
(`plans/reviews/2026-06-26-cib-080-secret-fp-tuning.md`) and from the negative
expectations already asserted in `tests/secret_detection.rs`. They are not
decoration: SDT-004 vendors roughly 150 more rules, and its gating question is
whether detection gain exceeded false-positive cost. This half is the cost side
of that comparison.

Each benign case declares a **non-vacuity control** — the same bytes moved out
of the context that is meant to suppress them, or with the suppressing token
substituted away. A benign case whose control stays silent proves nothing,
because the scanner was never going to flag it; the runner fails on such a case
rather than banking it as a false positive avoided.

`bn-google-drive-id-residual` is expected to be **flagged**. CIB-080 left two
Google Drive URL `id=` values unsuppressed on purpose, because a general
public-ID heuristic was wider than the Council-approved fixture scope. The
corpus records that decision as a known residual instead of pretending the
false-positive rate is zero. The case file is representative of the residual
class; it is not a verbatim copy of the upstream excalidraw fixture.

## Updating the baseline

The runner asserts the per-case outcomes and the aggregate counts in
`manifest.json`. A regression **and** an improvement both fail it, with the
committed and measured numbers printed side by side. That is deliberate: the
point of this corpus is that no rules change lands unmeasured. Update
`manifest.json` in the same change as the rule change, and record the
before/after in `plans/archive/modules/secret-detection-truth.aps.md` (SDT-002).
