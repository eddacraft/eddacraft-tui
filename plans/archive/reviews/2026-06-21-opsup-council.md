# OPSUP module — milestone Council review (2026-06-21)

Full 5-reviewer pack (general quality, security/privacy, adversarial, operations,
pragmatic-lead) on the **completed** operational-supplement (OPSUP) module — all
7 work items merged to `main`. Session `council-f8982969`.

## Verdict: WARN

The module is real, coherent, and **shippable as merged**. Calling it
**Complete** (the release-tag closeout) should be gated on the MAJOR items below.

- **Privacy/air-gap contract HOLDS** (security chair, explicit): no plaintext
  path leak, source content fail-closed, zero network, salted + domain-separated
  hashing, air-gap harness covers `report-fp`. No CRITICAL findings; no
  correctness regressions.
- **Coherence is good:** OPSUP-001/002 and OPSUP-007 share one registry surface
  (`definition_by_name` + `closest_registered_id`) with no divergent lookup
  paths; OPSUP-003/004 form a clean versioning + migration story.

## Findings → disposition

### Fixed now (PR #2844 — quick-wins)

| Finding | Reviewer | Sev |
| --- | --- | --- |
| `closest_registered_id` unbounded Levenshtein from `.anvilrc#checks` (CPU-DoS from cloned config) — capped needle to 64 chars | adversarial | MAJOR |
| `--include-snippet` help didn't warn the line is stored verbatim/unredacted | security | NIT |
| Misleading overflow messages (`parse_location` line, `SchemaVersion::parse`) | adversarial | MINOR/NIT |
| ADR-089 cited `sha256_hex`; code ships stronger salted `hash_file_path` — doc corrected | security | NIT |
| OPSUP spec checkboxes (FP destination; legacy-name→ID mapping) left unticked | pragmatic | MINOR |

### Filed as backlog (this PR — CIB items)

| Finding | Reviewer | Sev | Item |
| --- | --- | --- | --- |
| FP off-machine egress deferred with no tracking anchor | pragmatic | MAJOR | CIB-086 |
| FP sidecar is write-only — no operator read/list path | operations | MAJOR | CIB-087 |
| `drift migrate`: `.bak` retention unenforced (unbounded) + partial-failure not reported (no skipped count / exit code) | operations | MAJOR×2 | CIB-088 |
| `.anvilrc#checks` warn-and-continue vs `--skip-checks` fatal — divergent unknown-ID semantics | general | MAJOR | CIB-089 |
| `append_observation_to` TOCTOU symlink race (no `O_NOFOLLOW`) — affects usage + FP sidecars | adversarial | MAJOR | CIB-090 |

### Accepted as-is / noted

- Windows file perms are `cfg(unix)`-only (salt + sidecars) — tracked under the
  existing DSV-010/011 Windows state-hardening gap.
- `expired_suppressions` hardcoded `0` (documented "not yet implemented"); the
  field is a v1.0.0 schema field so it cannot be `serde(skip)`-ed — left as-is.
- Wall-time budget is soft/report-only (by design, `_soft_` in the field name);
  not a hard pre-emption cap.

## Note on Complete

OPSUP is **In Progress 7/7** — all items merged, release-tag closeout pending.
Per the review, the *Complete* transition should resolve or explicitly accept
the CIB-086..090 items.
