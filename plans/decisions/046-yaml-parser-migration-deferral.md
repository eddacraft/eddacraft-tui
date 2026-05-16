# ADR-046: YAML parser migration — defer with byte-level hardening

## Status

Accepted (2026-05-16, closes the MLP2-060 follow-up that MLP2-066 carries)

## Date

2026-05-16

## Context

`anvil-config` parses `.anvil.{yaml,yml}` and `anvil/policy.{yaml,yml}` via
`serde_yaml` (`serde_yaml 0.9.x`). The upstream crate was marked unmaintained
on `crates.io` in 2024; the workspace pin still resolves but no upstream
security fixes are expected. MLP2-060 added byte-level alias rejection, a
1 MiB size cap, and a `MAX_PARSED_DEPTH = 32` post-parse depth check to
contain the worst attack classes (billion laughs, deep nesting, oversized
payloads). The MLP2-060 closeout left "migrate to a maintained YAML parser"
as a separately tracked follow-up; MLP2-066 made that follow-up explicit and
required either a migration or a recorded deferral with an owner and a
review date.

`serde_yaml` is used by **15 workspace crates** today (the `anvil-config`
parse path plus 14 surfaces that hand-roll their own `serde_yaml::from_str`
for typed config). A whole-tree swap is a cross-cutting refactor with a
non-trivial test-fixture cost: every typed config reader needs to be retested
against the replacement parser's edge cases (anchor semantics, key ordering,
implicit type coercion).

## Decision

**Defer the parser migration.** Keep `serde_yaml` as the workspace's YAML
parser. Re-evaluate by **2026-08-15** (one quarter from this ADR) under
joint owner **kernel-maintainer** + **security-analyst**. Triggers that
move the re-evaluation earlier:

- A new published advisory against `serde_yaml` for any input shape that
  bypasses the alias / depth / size pre-pass.
- A maintained successor surfacing a serde-compatible drop-in (the current
  candidates — `serde_yml`, `serde-yaml-ng`, `saphyr` — each carry blockers:
  ecosystem fork-and-rebrand churn, missing forward-compat guarantees, no
  serde adapter respectively).

## Rationale

The MLP2-060 byte-level pre-pass already neutralises the published-CVE
attack surface against `serde_yaml`:

- 1 MiB file-size cap (`anvil_config::MAX_CONFIG_FILE_BYTES`) — refuses
  resource-exhaustion payloads before allocation. MLP2-063 hardened the
  cap to be TOCTOU-resistant (open-once + fstat + `Read::take`).
- Anchor / alias rejection at the lexer level — neutralises the
  billion-laughs / quadratic-blowup vector before `serde_yaml`
  materialises the alias graph.
- `MAX_PARSED_DEPTH = 32` — bounds nesting depth after parsing.

The marginal correctness gain from migrating today is small and the
churn cost is large: 14 typed-config readers means 14 places to retest,
plus regression risk against operator-facing YAML shapes. Defer keeps the
team focused on the daemon-working slate (`v0.7.0-beta`) without leaving
the hardening incomplete — the byte-level pre-pass is the actual defence,
and it lives outside `serde_yaml`.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Defer migration, harden pre-pass** *(chosen)* | Zero churn; byte-level defences are the real safety net; preserves daemon-slate focus | `serde_yaml` stays unmaintained; review-cycle work in 90 days |
| Migrate to `serde_yml` (community fork) | Maintained; mostly drop-in API | Forked-and-rebranded ecosystem volatility; no governance commitment |
| Migrate to `serde-yaml-ng` | Maintained; signals from contributor base | Smaller user base; forward-compat guarantees still informal |
| Migrate to `saphyr` (pure Rust, no serde) | First-party Rust YAML 1.2 lexer | No serde adapter — requires hand-written deserializers across 14 readers |

## Consequences

- **Positive:** No code or test churn on the MLP2 hardening landing
  window; the 1 MiB cap + alias reject + depth cap continue to do the
  load-bearing work. The deferral is bounded — owner and review date are
  recorded.
- **Negative:** `serde_yaml` stays in `Cargo.lock`. If a non-byte-level
  CVE lands against the crate before 2026-08-15, this ADR's review must
  fire early; `RustSec/advisory-db` watchers should flag it.
- **Operational:** Add the 2026-08-15 review date to the operator-owned
  cadence checklist (release lane). On review:
  1. Re-check `serde_yaml` upstream status + advisory feed.
  2. Re-rank the three candidate maintained parsers.
  3. Decide migrate / re-defer; if re-deferring, update this ADR with the
     new review date and a one-line rationale entry.

## Pin

This ADR pairs with **MLP2-066**. The MLP2-060 follow-up note said the
maintained-parser migration was tracked separately; MLP2-066 is where it
was made explicit, and this ADR is the recorded deferral. No source-code
change accompanies this ADR in PR2 of Group M.
