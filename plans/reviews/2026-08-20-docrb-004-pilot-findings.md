# DOCRB-004 Component Pilot Findings — 2026-08-20

| Type   | Authority | Owner | Status |
| ------ | --------- | ----- | ------ |
| Review | Advisory  | DOCRB | Closed |

## Scope and source revision

This report records the single evidence set for the six DOCRB-002-selected
component roots:

- `crates/anvil-kernel`
- `crates/anvil-intercept`
- `apps/dashboard`
- `crates/anvil-dashboard-server`
- `apps/anvil-api`
- `apps/docs-shell`

The comparison base for this report is
`9a0c906b27ca3325cd9674d002b0f37c51ce6149`. The immutable reviewed-content
target is `210fd0bde3ea160f4352b68e379c54a8b924ca3d`. The immediately following
report-only commit changes no reviewed component claim; it only finalises this
provenance record. Detailed source review began at
`d6c8b565c375e9e75db44c5d20d2acb066e4471c`, which was `origin/main` when
DOCRB-004 started. A targeted diff from that review snapshot to the comparison
base found no changes in the cited product source roots or
`infra/src/vercel.ts`; re-review at `88bd41647` confirmed the source-backed
intercept and API claims before this repair. No product, configuration, central
as-built, governance, public-diagram, or sibling APS status change is part of
the pilot.

## Result

**Final repair applied; Council resolution pending.** Each pilot root has a
concise authoritative orientation README and a derived, source-linked local
architecture map. Each map contains one Mermaid diagram for its bounded
component concern. Retained central authorities remain linked and explicitly
unsuperseded. Council review found over-broad trust, health, redirect, audit,
and entrypoint claims; the implementation now records the narrower source
truth below. This report does not mark those Council findings resolved.

## Council repair and current limitations

- Intercept documentation separates the `scan_buffer` and `validate_paths`
  lanes. Only the former can receive a `CrossCheckContext`: Linux production
  validates an optional session claim and environment tag before scanning
  caller bytes, while macOS and Windows currently skip those checks. The
  transport trust floor remains same-user: Unix compares peer UID and Windows
  applies an owner-only pipe DACL plus an explicit peer-SID comparison. This
  platform gap means non-Linux mid-edit requests do not have Linux's additional
  lineage assurance.
- `validate_paths` performs Open or Allowlist workspace admission before
  guarded path reads and validation; it does not run the spoof cross-check.
  Only a present confinement configuration that fails load or trust checks
  selects the empty-allowlist fail-closed posture.
- Interrupt safety/delivery failures and unattributed or unregistered changes
  request fences independently of spoof detection; cascade follows the fifth
  fence event within 60 seconds. Spoof-fence persistence may fail while the
  request remains blocked, and degraded assurance alone is not a trigger.
- Dashboard-server loopback is machine-local but unauthenticated. Browser
  header checks mitigate CSRF, not local-client identity; proxying or
  port-forwarding is unsupported. Per-user isolation would require a capability
  token or owner-only IPC transport.
- Dashboard generated types constrain TypeScript usage at compile time, its
  root-relative base selects the current browser origin, and the server enforces
  its request policy.
- Docs-shell callback outcomes now cover validation, BAUTH success, pending,
  error, and denial without granting a session. Only absolute redirects from a
  known renderer origin are rewritten; relative locations pass through.
- API rejection remains mandatory while audit persistence is best-effort and
  logs failure. The Hono/Vercel entrypoint convention is separate from Pulumi's
  deployment-root and framework selection.
- API health reports every dependency state, but only database, signing or
  verifying key, GitHub CLI credential, and Resend `invalid`/`unconfigured`
  states gate the overall result. Resend or network `unverifiable` remains
  visible but non-gating and can coexist with overall `status: ok`.

## Navigation trace

The trace started at each component root, followed local orientation to
internals, opened every linked load-bearing source path, and then followed the
retained central authority.

| Root | Navigation trace | Result |
| ---- | ---------------- | ------ |
| `crates/anvil-kernel` | README → local architecture → watcher/parser/graph/protocol source → retained kernel as-built | Pass; initial baseline and incremental-finding distinction is visible |
| `crates/anvil-intercept` | README → local architecture → IPC/platform wiring/admission/spoof/interrupt/unregistered/fence source → retained intercept and driver maps | Pass after final repair; `scan_buffer`, `validate_paths`, platform assurance, and independent fence triggers are distinct |
| `apps/dashboard` | README → local architecture → router/query/API client/generated contract → local dashboard guide | Pass after repair; compile-time typing, origin selection, and server enforcement are distinct |
| `crates/anvil-dashboard-server` | README → local architecture → loopback guard/capability/workspace/OpenAPI source → local dashboard guide | Pass after repair; machine-local unauthenticated boundary and per-user-isolation limitation are explicit |
| `apps/anvil-api` | README → local architecture → entrypoint/health/deployment/global middleware/routes/database source and tests → retained API and BAUTH auth maps | Pass after final repair; dependency reporting and health gates, rejection, best-effort audit, APGOV ownership, and BAUTH authority remain separate |
| `apps/docs-shell` | README → local architecture → OAuth/session/callback/proxy source → deployment truth and governance | Pass after repair; no-session outcomes, absolute versus relative redirects, live renderers, rollback truth, and ownership gap are explicit |

Root `CONTEXT.md` discovers the new thin docs-shell `AGENTS.md` spoke. No
other pilot received a local agent file.

## Mermaid render and source trace

Six Mermaid blocks were initially extracted directly from the six local
`ARCHITECTURE.md` files and rendered with
`@mermaid-js/mermaid-cli 11.16.0`. Council repairs changed the intercept,
dashboard, dashboard-server, and docs-shell blocks; all four were re-rendered
with the same pinned CLI. The final intercept lane correction was extracted
again from its source file and re-rendered with that CLI. Chromium cannot start
its nested sandbox in this container, so the successful manual preview used a
temporary Puppeteer
configuration with `--no-sandbox`; inputs and non-empty SVG outputs were
written only under `/tmp` and are not repository artefacts.

| Diagram concern | Render | Source-edge trace |
| --------------- | ------ | ----------------- |
| Kernel source → parse → graph → finding | Pass, non-empty SVG | `watch.rs`, `parser/`, `anvil-graph-cache`, `protocol/` |
| Intercept `scan_buffer` cross-check/buffer scan and separate `validate_paths` admission/guarded validation plus explicit fence triggers/cascade | Pass after final repair, 54,866-byte SVG | `ipc.rs`, `lib.rs`, `workspace_admission.rs`, `workspace_anchor.rs`, `save_time.rs`, `validate_paths.rs`, `interrupt.rs`, `unregistered.rs`, `fence.rs` |
| Dashboard UI → compile-time generated types → root-relative client → server policy | Pass after repair, 19,679-byte SVG | router/modules/hooks, API client, generated OpenAPI types, generator |
| Dashboard server capability/access boundary | Pass after repair, 18,104-byte SVG | loopback and browser-request guards, read-only routes, capability loaders, `WorkspaceAnchor` |
| Hosted API request → middleware → route → persistence/trust | Pass, non-empty SVG | `index.ts`, middleware, representative route boundaries, database client/queries |
| Docs shell auth/callback outcomes → private/public renderer | Pass after repair, 33,208-byte SVG | login/callback validation, BAUTH outcomes, licence verification, proxy, Vercel deployment |

Adjacent prose provides the same meaning for readers who cannot consume the
diagram.

## Metadata, links, ownership, and duplication

Manual checks are required because current `pnpm docs:check` does not enforce
component-root README/architecture metadata, cited paths, or links.

- All 126 repository-local Markdown links across the twelve pilot docs and the
  thin docs-shell spoke resolve. This trace caught and repaired three incorrect
  draft targets before closeout: kernel parser/protocol module links and a
  nonexistent API adapter path.
- Metadata source paths and globs were traced against the
  `d6c8b565c` source-review snapshot. The targeted product-source diff through
  the exact `9a0c906b2` range base was empty, and the final intercept/API
  source re-review at `88bd41647` found no relevant source drift. Each README is
  `Authoritative` for component orientation; each architecture file is `Derived` from current
  source.
- KERN owns kernel, INTD owns interception, DASH owns both dashboard roots, and
  APGOV owns the hosted API component. BAUTH remains authoritative for
  authentication.
- Docs-shell metadata deliberately says `DOCRB/DSITE gap`: DOCRB owns this
  pilot evidence, while DSITE still owns recorded legacy-host work and has not
  adopted the live shell. This is not a joint runtime ownership assignment.
- Local documents link rather than copy central diagrams, public guidance, ADR
  rationale, or operator procedures. Temporary overlap is labelled as a
  DOCRB-004 pilot, and DOCRB-005 retains authority over migration or deliberate
  central retention.

## Validation evidence

The original replacement RED assertions proved the local documents, metadata,
component flows, docs-shell spoke, and report were absent. A second replacement
RED at `f904cd8f` proved all nine Council repair assertions absent: intercept
flow and admission, dashboard-server boundary, dashboard typing, docs-shell
redirect/auth outcomes, API audit and entrypoint, index next action, and exact
report base. A final replacement RED at `88bd41647` proved eight remaining
distinctions absent: separate intercept lanes, Linux-only cross-check wiring,
the non-Linux platform gap, Windows SID trust, no save-time spoof check,
non-gating Resend `unverifiable`, and immutable provenance. Corresponding
GREEN assertions cover those repairs as well as all seven DOCRB-004 acceptance
behaviours.

Focused component validation:

```text
cargo test -p eddacraft-anvil-kernel
cargo test -p eddacraft-anvil-intercept
cargo test -p eddacraft-anvil-dashboard-server
pnpm --filter @eddacraft/anvil-dashboard test
pnpm --filter @eddacraft/anvil-api test -- --run
pnpm --filter @eddacraft/docs-shell test
```

All commands exited zero. The concise JavaScript summaries were 16 files /
71 tests for dashboard, 46 files / 757 tests for the API, and 6 files / 51 tests
for docs-shell. All Rust target test binaries passed; the kernel's primary unit
suite reported 337 passing tests and interception's primary unit suite reported
1,106.

Repository validation at report time:

```text
pnpm format:check
pnpm docs:index:check
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
pnpm aps:drift --json
git diff --check
```

Formatting and index freshness passed. Documentation checks passed all 11
surfaces with only pre-existing baselined/advisory warnings. APS active lint and
stored counts passed; drift reported `findingCount: 0`; the diff whitespace
check passed.

## Recommendations

### DOCRB-005 migration

1. Compare each retained central as-built with its new local source-linked map
   before moving or deleting content. Preserve genuinely cross-system concerns
   centrally.
2. Treat kernel, intercept, and API central maps as explicit reconciliation
   candidates. Do not delete them merely because a pilot exists.
3. Keep the local dashboard operator guide and any genuine multi-surface TUI
   view central; migrate only component-internal UI/server material.
4. Keep production docs topology and the DOCRB/DSITE ownership gap in ADR-123
   and documentation governance. A component map must not become a second
   deployment authority.
5. Use the pilot's conditional-edge pattern for fallback and degraded paths;
   compact diagrams become inaccurate when they collapse validation, fencing,
   auth, or availability into one unconditional edge.

### DOCRB-009 enforcement

1. Extend documentation checks to component-root README and architecture
   metadata, paths, and Markdown links only after the canonical migration map is
   established.
2. Add Mermaid syntax/render checking through an authorised, pinned toolchain
   with a documented container launch configuration and useful file-level
   diagnostics.
3. Enforce the ADR-123 trigger and exemptions at the changed-component
   boundary, not as a repository-wide requirement for every code change.
4. Keep enforcement advisory until DOCRB-009 is explicitly activated; this
   pilot adds no automated Mermaid tooling or CI rule.
