# ADR-116: Kindling Product Profiles and Governance Record

## Status

Accepted 2026-08-03 (operator)

## Date

2026-08-03

## Context

Anvil needs durable local evidence for selected governance outcomes without
turning every save-time evaluation into retained noise. Kindling already
provides local capture, storage, query, and outage-spool mechanisms, while Anvil
currently has several partially overlapping observation paths and legacy NDJSON
sidecars. The product boundary, admission semantics, retention classes,
identities, migration posture, and package ownership must be fixed before the
embedded runtime is activated or those paths are consolidated.

The decision must preserve these existing contracts:

- [ADR-012](012-rust-cli-replacement.md) makes the Rust `anvil` binary the
  shipped product.
- [ADR-035](035-three-pipe-observability-rule.md) assigns durable governance
  facts to Kindling, current user-visible state to notifications, and diagnostic
  instrumentation to tracing.
- [ADR-064](064-intercept-graph-cache-crate-boundary.md) keeps the resident
  intercept daemon's dependency boundary narrow and parser-free.
- [ADR-088](088-dpo-observation-kind-taxonomy.md) distinguishes
  `gate_evaluated` from `constraint_applied` and owns the save-time/fence
  taxonomy.
- [ADR-089](089-fp-telemetry-destination.md) keeps
  `false_positive_reported` local and air-gap safe.

This ADR narrows how those decisions are realised through Kindling. It does not
move policy into Kindling or make tracing authoritative. It does amend the
"write-once" wording in ADR-035 and the "immutable" wording in ADR-089:
governance admission is append-only during normal operation, while the
authenticated, explicit prune operation defined here is the sole removal path.

## Decision

### 1. One product with two profiles

Kindling is one local product mechanism with two supported consumer profiles:

1. **Standalone memory** captures and retrieves useful working context across
   sessions.
2. **Embedded Anvil governance** retains selected protection and provenance
   evidence for Anvil.

The embedded profile is a deployment mode inside the single shipped `anvil`
binary. Users do not install or operate a second Kindling product to obtain the
Anvil profile.

The embedded profile makes no outbound network calls and opens no TCP listener,
including on loopback. It runs in-process or uses owner-restricted local IPC:
Unix sockets require matching peer credentials and Windows named pipes require
an owner-only ACL. A future export or other egress path requires a separate,
explicit-consent ADR and must pass the existing air-gap harness. Standalone
memory export, retention, and forget commands cannot address the embedded
governance profile.

Kindling owns reliable local lifecycle, capture, storage, retrieval, and health.
Anvil owns which events are governance evidence, their typed payload, policy
meaning, aggregation, and presentation.

### 2. Generic transport with an Anvil-owned schema

Kindling accepts a generic `Event` envelope rather than adding Anvil-specific
variants to its product taxonomy. Its payload carries a closed, versioned Anvil
envelope. The required upstream capability is named `generic-event@1`; the
planned compatibility floor is `kindling-runtime` 0.4.0. Versions through 0.3.x
are not compatible. KFIT-005 must link the publishing Kindling change and record
the actual release evidence before Anvil can activate the profile.

Rust is authoritative for that envelope and its closed `event_kind` set.
TypeScript declarations, JSON Schema, and parity fixtures are generated from the
Rust contract. Hand-maintained TypeScript and Rust registries are not permitted.

`anvil.governance.v1` has these required common fields: `schema`,
`event_kind`, `scope`, `run`, `event_id`, `timestamp`, and `payload`.
`scope` is normally the closed `attributed` variant containing
`repository_id` and `worktree_id`. Only `recording_gap` may use the `aggregate`
variant, which carries no invented identity and declares whether scope, cause,
or kinds are unknown. Only a `governance_pruned` receipt for an
`aggregate_gaps` prune may use the `profile_administrative` variant containing
the opaque owner-local embedded-profile instance ID; that scope cannot carry
ordinary evidence or authorise access. `run` is normally the closed `known`
variant containing `run_id`; only `recording_gap` may use
`multiple_or_unknown` when coalescing cannot truthfully name one run.
`traceparent` and `gate_eval_id` are optional only where the originating surface
cannot supply them. The closed governance inventory is:

| Canonical `event_kind` | Required payload and meaning |
| ---------------------- | ---------------------------- |
| `gate_evaluated` | `gate_id`, `evaluation_mode`, `outcome`, `enforcement`, evaluated-rule counts, and minimised input references. `outcome` is exactly `pass`, `fail`, `error`, or `skipped`; `enforcement` is exactly `blocking`, `warning`, or `informational` and describes the configured action on failure, not the actual outcome. |
| `constraint_applied` | `constraint_id`, `gate_id`, normalised reason, and cascade flag. It is never represented as a failed gate. |
| `action_executed` | Stable action kind and identifier, closed outcome, and redacted target/diff summary where present. |
| `false_positive_reported` | Registered check ID, salted path hash, line, anonymised principal, and an absent-by-default source snippet under ADR-089. |
| `recording_gap` | Stable `gap_incident_id`, attributed or aggregate scope, known or multiple/unknown run, cause class or `cause_unknown`, affected canonical kinds or `kinds_unknown`, first/last time, affected count, and known sequence bounds or `unknown_range`; it never reconstructs missing payloads or attribution. |
| `governance_pruned` | Non-prunable receipt containing a closed prune scope (`repository` with ID or `aggregate_gaps`), selector hash, removed count and identity range, time, authenticated local operator identity, normalised reason, and migration state; an aggregate-gap receipt uses the envelope's `profile_administrative` scope, and the receipt contains no removed payload. |

`command.invoked` remains the separate existing usage envelope and is not an
`anvil.governance.v1` event. The selected governance inventory above is closed;
the other historic TypeScript observation kinds are standalone-memory concepts
and are not silently imported as Anvil governance. The legacy Anvil spelling
`gate.evaluated` maps to canonical `gate_evaluated` during migration and the
one-release dual-read window only. New writers emit `gate_evaluated`; queries
accept the alias during that window and return the canonical spelling.

Every producer validates before admission, and every store, spool-replay, read,
and migration boundary revalidates before trusting the row. Validation rejects
unknown schema versions, event kinds, enum values, and extra fields; enforces
field, nesting, and payload-size limits; and applies minimisation and redaction
before the final validation pass. Rejected rows create a recording gap rather
than entering the store. CI regenerates Rust-derived TypeScript, JSON Schema,
and fixtures and fails on a diff; hostile fixtures cover oversized, deeply
nested, extra-field, unknown-version, invalid-identity, and pre-redaction input.

### 3. Canonical identity and local authorisation

Every attributed governance event has canonical identities with these
semantics. The only non-attributed variants are an aggregate `recording_gap`,
which declares its missing dimensions explicitly, and the narrowly constrained
`profile_administrative` scope on an aggregate-gap `governance_pruned` receipt:

| Identity | Contract |
| -------- | -------- |
| Repository | Required by attributed scope: ADR-036's authoritative tracked `anvil/project-id` `project_uuid`. It is shared by linked worktrees and inherited by clones/forks by default; ADR-036's explicit `--new-identity` flow is the only way to fork that governance domain. No second repository UUID is introduced. |
| Worktree | Required by attributed scope: a subordinate opaque UUID stored in owner-only per-worktree Git administrative metadata. It separates linked-worktree and local runtime scope without replacing the project UUID. Its binding includes an owner-local fingerprint of the Git administrative path and ADR-036 execution scope. |
| Profile administration | Used only by an aggregate-gap prune receipt: an opaque instance UUID created and protected in the owner-local embedded-profile state root. It identifies the administrative receipt domain, not a repository, worktree, or evidence producer, and grants no authority by itself. |
| Run | The `known` variant names one Anvil work session or explicit command run; related events reuse it across CLI, daemon, hook, audit, and replay paths. Only coalesced `recording_gap` evidence may declare `multiple_or_unknown`; it never uses the recovery run as a substitute. |
| Event | Assigned once and preserved through spool replay. Imported legacy rows use deterministic IDs so retries deduplicate. |
| Correlation | `traceparent` joins the three ADR-035 pipes; `gate_eval_id` joins facts belonging to one gate evaluation where applicable. |

Identity resolution reads ADR-036's tracked project UUID and Git administrative
paths obtained from Git, then opens owner-matched regular files without
following symlinks. Raw working-directory paths, remote URLs, and filesystem
canonicalisation are not identities. Nested repositories resolve independently;
linked worktrees and clones share the repository ID according to ADR-036 but
receive distinct local worktree IDs. A filesystem copy that also copies Git
administrative metadata cannot reuse the worktree ID when its owner-local path
or execution-scope binding changes. An owner-only registry in Anvil's platform
user-state directory, outside every copied Git tree, binds each active
repository/worktree pair to one Git-admin file identity, path fingerprint, and
ADR-036 execution scope. A second root or copied binding is rejected; the
dry-run-first `anvil kindling identity rebind --move|--copy --apply` operation
must either preserve the worktree ID after proving the former move source is
absent, or mint a new worktree ID for a copy. Forking the repository governance
domain still uses ADR-036's `--new-identity`. The daemon registers the exact
repository/worktree pair and OS owner.
Append and query calls must arrive over the owner-restricted local channel with
that registered pair; cross-root access is rejected. If identity is missing,
ambiguous, symlinked, owner-mismatched, or unregistered, Anvil must not fall back
to the process working directory. It reports an aggregate recording gap with
`scope_unknown` and does not require or invent canonical IDs. The integrity
ledger may hold an owner-local, salted Git-admin fingerprint for coalescing
health state, but never admits that fingerprint as repository or worktree
identity.

ADR-036's OS account remains the local operator trust boundary. Pair
registration and IPC credentials prevent cross-user access and accidental
cross-root attribution; they do not claim to sandbox a malicious process already
executing as the same UID or SID, which can act with the operator's filesystem
authority. Untrusted repository content is never itself an authority and is not
executed to register, query, or prune. Strong same-account process isolation
would require an OS sandbox or separate identity and a new ADR.

### 4. Selective retention, capacity, and prune

The embedded profile retains only facts that are useful as governance evidence:

| Event | Retention rule |
| ----- | -------------- |
| `gate_evaluated` | Retain every `fail` or `error`. Retain `pass` or `skipped` iff `evaluation_mode` is `audit` or `pre_push` and the user explicitly invoked that audit or pre-push gate; otherwise they are tracing-only. `enforcement` is retained as context but never independently selects a row: current contracts validly allow, for example, `pass + blocking` to mean a blocking policy would have applied on failure. |
| `constraint_applied` | Retain. This remains distinct from a failed gate under ADR-088. |
| `action_executed` | Retain. |
| `false_positive_reported` | Retain locally with ADR-089's redaction and air-gap posture. |
| `recording_gap` | Retain as integrity evidence; it is excluded from ordinary governance prune unless the whole repository scope is explicitly selected. |
| `governance_pruned` | Retain permanently in reserved integrity capacity; ordinary prune and `--all` never select it. |
| Routine successful high-frequency mid-edit and save-time checks | Tracing only; do not create a retained Kindling row. |
| `command.invoked` | Usage evidence, not governance evidence; retain on a rolling 30-day window. |

Governance evidence has no automatic expiry. Only a local operator can prune it
through an explicit operation; there is no remote or implicit retention
authority.

The embedded governance profile has a default 1 GiB hard store quota, a 256 MiB
hard per-repository budget, a 256 KiB validated-event limit, and an 80% warning
threshold. Sixteen MiB of the profile quota is reserved as separate 8 MiB
budgets for `recording_gap` and `governance_pruned` integrity rows, so receipts
cannot exhaust gap capacity. A local operator may lower or raise the store
budgets explicitly but cannot remove either integrity reserve; configuration
changes are reported by status. Admission refuses a projected write unless the
post-write free space remains above the greater of 128 MiB or 5% of filesystem
capacity. It never evicts retained governance automatically: quota or low-disk
rejection falls through to the bounded spool and then to recording-gap
accounting. Status reports total, per-repository, spool, and reserved-integrity
usage.

Status warns at 80% of either integrity reserve. A zero-result prune apply is
rejected and writes no receipt. Receipt-capacity exhaustion refuses prune
without deleting anything; its authenticated recovery path is an explicit local
receipt-reserve increase subject to the same projected low-disk check. The
operator may not shrink it below committed receipt bytes or borrow from the gap
reserve. Tests cover zero-result refusal, exhaustion, failed apply atomicity,
authorised capacity increase, and continuity of every prior receipt.

The store-quota path may use the spool only when the spool's own 64 MiB and
free-space budgets admit the projected write. A low-filesystem-space rejection
never writes to a spool on that same filesystem; it may use an explicitly
configured owner-only spool on a different filesystem with its own low-water
check, otherwise it updates only the already-preallocated integrity ledger and
reports a gap.

All embedded-profile state uses an owner-only platform state root: Unix
directories are mode 0700 and regular files, including SQLite/WAL/SHM, spool,
integrity ledger, receipt rows, identity registry, and migration state, are mode
0600; Windows grants only the current user SID and required system principals.
Creation is exclusive. Every open validates owner, mode/DACL, regular-file type,
link count where available, and confinement through a previously validated
directory handle; symlinks, hard links, permissive or wrong-owner paths, and
pre-created replacements are refused. Replacement is confined, atomic, and
synchronises the file and parent directory. Failure prevents activation and
surfaces degraded integrity health rather than silently creating a second path.

The only retained-evidence removal surface is `anvil kindling prune --profile
governance (--repository <id> | --aggregate-gaps) <selector> --reason <text>`.
The mutually exclusive `--aggregate-gaps` scope selects only unattributed
aggregate `recording_gap` rows; it cannot select attributed rows or receipts.
Its receipt uses the current owner-local `profile_administrative` envelope scope
and the `aggregate_gaps` payload prune scope; strict validation rejects that
envelope scope for every other event or receipt selector.
Prune is dry-run by default and requires `--apply`; selectors are explicit event
IDs, a closed time range, or `--all`, never an implicit age policy. It requires the same OS owner
and peer-credential or named-pipe authorisation as append/query, rejects
symlinks and cross-profile targets, and reports the exact count and identity
range before applying. Apply is transactional and atomically appends the
`governance_pruned` receipt before committing removal. The receipt is written
directly to the retained store, never the spool, from reserved integrity
capacity in the same store transaction as deletion. It is strictly validated,
queryable by KFIT-010, and never selected by ordinary prune; prune is refused if
the receipt cannot be durably committed. Dry-run reports and apply refuses
`blocked:legacy-copies`
whenever a declared legacy sidecar or migration backup exists, regardless of
migration state. The dual-read window must finish and explicit migration cleanup
must remove both retired source sidecars and their backups before prune can
apply, so no reader or pre-cutover rollback can resurrect removed evidence.
Standalone `forget` cannot invoke this path.
This is the sole exception to ADR-035's write-once and ADR-089's immutable
wording; outside an explicit prune transaction, governance rows remain
append-only.

During an outage, the bounded spool may retain pending events for at most seven
days and 64 MiB. Pending spool entries are not retained-store pruning targets:
if either bound prevents admission, or forces an accepted event out before
replay, the runtime records a gap rather than silently treating the event as
retained governance evidence.

### 5. Durable admission and crash-visible recording gaps

A selected event counts as recorded only after Kindling's retained store or the
bounded outage spool accepts it. Queueing into an unbounded or best-effort
in-memory channel is not admission.

Admission failure never changes Anvil's policy verdict or exit semantics. The
runtime maintains a preallocated 1 MiB integrity ledger outside the retained
store and spool quotas. Each of two checksummed ledger images reserves 64 intent
slots, a coalesced gap table, and a non-evictable overflow sentinel. Before
attempting admission it writes and synchronises an intent containing event ID,
canonical kind, run, sequence number, and either canonical repository/worktree
IDs or the unattributed salted Git-admin fingerprint used only by health state;
after store or spool acknowledgement it atomically clears that intent. If all intent
slots are busy, the runtime durably increments the overflow sentinel instead of
attempting admission. A monotonic generation leaves either the previous valid
image or the new image after a crash, never a silently torn success.

Store rejection converts the pending intent into a coalescible gap range. Spool
expiry or eviction uses a two-phase ledger intent: persist and synchronise
`eviction_pending` with event ID, sequence, scope, and run; delete and
synchronise the spool entry; then promote the intent to a gap. On restart, a valid matching row
still present in store or spool cancels the eviction intent, proven absence
promotes it, and unavailable or corrupt lookup state yields `unknown_range`.
Repeated failures first coalesce by repository, worktree, run, cause, and kind
while preserving count, first/last time, and known sequence bounds.
Each newly occupied gap-table bucket receives a stable, opaque
`gap_incident_id` derived with domain separation from a ledger-wide 128-bit
monotonic incident epoch and the owner-local profile instance ID; the underlying
profile ID is not emitted. The epoch is incremented and synchronised in the
dual-image ledger before the bucket can accept a loss. It remains stable across coalescing,
restart, and replay until the retained gap is acknowledged and the bucket is
cleared; a later episode for the same tuple receives a new epoch. Epoch
exhaustion is degraded integrity health and prevents activation rather than
wrapping or reusing an ID.
When distinct tuples fill the table, deterministic oldest-update-first
compaction folds them into the overflow sentinel, which stores a saturating
count, earliest/latest time, last healthy sequence watermark, and
`unknown_range`; one sentinel slot is always reserved. Its first transition
from empty to occupied allocates and synchronises its own `gap_incident_id`,
which is cleared only after the aggregate gap is durably acknowledged. After recovery, that
sentinel admits one valid aggregate `recording_gap` with `scope_unknown`,
`multiple_or_unknown` run, `cause_unknown`, and `kinds_unknown`, rather than
inventing or discarding attribution. They cannot consume retained-store or
spool capacity. Corrupt or
unavailable ledger state is reported as `unknown_range` from the last healthy
watermark; the sink cannot report healthy or recorded until repaired. The
verdict still proceeds.

On restart, each uncleared intent is looked up by `event_id` in both retained
store and spool before it becomes a gap. A matching, strictly valid row proves
the acknowledgement committed and clears the intent idempotently. Absence
converts it to a gap; unavailable or corrupt lookup state yields conservative
`unknown_range`, never a definite loss claim. This closes the crash window after
acknowledgement but before intent clear.

When identity resolution itself fails, the ledger uses an unattributed bucket
with an owner-local salted Git-admin fingerprint rather than inventing canonical
IDs. Status exposes that bucket immediately. A later retained aggregate
`recording_gap` declares `scope_unknown`; it never converts the fingerprint into
an identity or silently attributes the loss.

This creates visible `recording_gap` evidence:

- runtime health and `anvil` status expose the gap immediately; and
- once an admission path is available, a bounded gap fact records its scope,
  time range, and affected count without reconstructing or inventing the lost
  payload.

Gap facts use a deterministic key over `gap_incident_id`, the exact closed scope
and run variants, cause, kinds, and sequence range. Retries of one incident
deduplicate, while later same-shaped incidents and distinct known runs remain
separate. KFIT-007 must ship the ledger and a
minimum status surface behind the default-off gate; KFIT-009 requires them for
local cutover, and KFIT-010 adds the full query experience.
Kill-at-each-boundary, disk-full, quota, corrupt-slot,
intent-slot and tuple-table saturation, cross-run and cross-scope overflow
compaction, same-scope/same-sequence/different-run deduplication, expiry,
eviction, two separated aggregate-overflow episodes across restart,
crashes before and after every two-phase eviction transition, restart, replay,
and deduplication tests are activation gates.

The admission boundary is bounded local I/O. Activation requires a benchmark
that demonstrates this work remains inside Anvil's save-time latency budget;
the durable record cannot be implemented by blocking indefinitely.

### 6. Explicit, forward-only sidecar migration

Legacy sidecars move through `anvil kindling migrate`:

- the command is dry-run by default and writes only with `--apply`;
- supported rows receive deterministic event IDs and can be retried without
  duplicates;
- rows without unambiguous repository scope are reported and skipped rather
  than assigned from the current working directory;
- source files are backed up before an applying run mutates migration state;
- readers retain one release of dual-read compatibility; and
- writers cut over once, with no dual-write period.

An applying run uses the persisted state machine `discovered -> validated ->
backed_up -> importing -> verified -> fenced -> cutover`. State and backups live
in an owner-only migration directory. Inputs must be owner-matched regular files at
the declared sidecar paths, opened without following symlinks; hard-linked,
symlinked, cross-root, oversized, or changed-after-validation inputs are
rejected. The backup manifest records permissions, sizes, and checksums; files
and containing directories are synchronised before the state advances. Import
checkpoints are deterministic and resumable. Verification compares accepted,
skipped, duplicate, corrupt, and ambiguous counts plus a digest of canonical
rows. Each row digest covers schema, attributed or aggregate scope, identities,
kind, timestamp, and canonicalised post-redaction payload. An existing event ID
is a duplicate only when its row digest matches; a different digest is a
blocking collision. Migration metadata records which generation inserted each
row. The parity digest covers sorted `(event_id, row_digest, ownership)` tuples,
so skipped rows or incorrect transformations cannot disappear from the report.

Rollback is supported only before writer cutover: only rows marked as inserted
by that migration generation can be removed transactionally, while identical
pre-existing rows remain untouched, and the owner-only backup is restored. Cutover
requires verified parity, a healthy gap ledger, the compatible runtime, and an
explicit `--apply` confirmation. Every supported legacy writer participates in
one exclusive generation fence. Cutover acquires that fence, stops new legacy
writes, rechecks the still-open source handles and parity digest, aborts on any
divergence, then writes and synchronises the cutover marker before switching the
in-process route to canonical writes and releasing the fence. Restart treats the
marker as canonical, so no supported writer can enter the verify-to-disable
window. Concurrent or downgraded older binaries are unsupported; any later
sidecar creation/change remains a divergence health error, never silently read
as canonical. After that boundary the migration is
forward-only: downgrading to a sidecar-writing release is unsupported. Recovery
may replay surviving canonical rows or repair store structure, but the
payload-free manifest is verification evidence only and cannot reconstruct an
event. If canonical payload-bearing data is missing or corrupt after cleanup,
Anvil remains degraded, reports the affected manifest range as a
`recording_gap` (or `unknown_range` where the manifest cannot bound it), and
never claims restoration from checksums or reactivates legacy writers. Any
operator restore from separately managed system backup is outside this
migration contract and must pass normal strict read validation. During the
compatibility release, any sidecar change after the verified snapshot is
reported as divergence and requires an
explicit resume/import before cutover. Backups remain owner-only through the
dual-read release and are deleted only by dry-run-first `anvil kindling migrate
cleanup --apply`, using the same owner, confined-path, manifest, and exact-count
checks as migration. Cleanup reports every source, backup, and surviving copy;
apply removes both the retired source sidecars and their migration backups,
synchronises their parent directories, and refuses completion if any declared
copy survives. The payload-free checksum manifest remains as cutover evidence.

For each migrated profile, successful cutover retires production writers for
`usage.ndjson`, `audit-chain.ndjson`, and false-positive sidecars at the start of
its compatibility window. Migration does not make those files a second ongoing
source of truth.

### 7. Ownership and package cutover

Runtime lifecycle and Kindling transport stay in `anvil-cli`.
`anvil-intercept` exposes transport-free observation traits and does not acquire
a Kindling runtime dependency. This applies ADR-064's narrow resident-daemon
boundary to the governance path without changing ADR-064's graph-specific
decision.

KFIT owns the retained store, Anvil query foundation, migration, and operational
health. DPO retains governance-history consumer semantics and dashboard
component ownership. ADR-088 remains authoritative for save-time/fence
taxonomy; ADR-089 remains authoritative for false-positive privacy and egress.

`@eddacraft/anvil-kindling-integration` is removed in the cutover release as
soon as Rust-generated TypeScript declarations, JSON Schema, and fixtures
replace its useful contracts. There is no deprecation release for that package.

### 8. Dependency and release sequence

Delivery follows this order:

1. accept this product and governance contract;
2. implement `generic-event@1`, harden the embedded facade, and publish the
   compatible Kindling runtime (planned floor 0.4.0) with a linked KINTEG/CONV
   change and `aps lint plans` evidence;
3. keep the merged Anvil consumption seam default-off while canonical
   repository/worktree identity and local authorisation are implemented;
4. wire the typed sink, crash-durable gap ledger, and minimum status surface
   behind the default-off release gate;
5. implement the fenced migration, explicit local writer cutover, and
   one-release dual-read window;
6. expose governance queries and operational status; and
7. enable release-default embedded selection only for a newly initialised
   profile with no legacy state or a profile carrying KFIT-009's persisted
   cutover marker; keep any other existing profile on its legacy writer until
   explicit migration. Remove the TypeScript integration package in that same
   release, reconcile documentation, and record release evidence.

Anvil release activation and package cutover are both owned by KFIT-011 and
cannot precede the compatible upstream publication, canonical identity, strict
schema boundary, gap ledger, migration/cutover path, or query/status surface.
Activation is evaluated per profile: legacy sources or backups without a valid
cutover marker force the legacy writer and prohibit canonical writes; no
automatic migration or dual-write occurs. A merged default-off consumption or
typed-sink seam is not activation or release evidence.

## Rationale

One mechanism with two explicit profiles avoids making governance storage a
second product while allowing memory and governance to have different selection
and retention rules. A generic upstream event keeps Anvil policy out of
Kindling; a Rust-authoritative payload prevents the cross-language schema drift
that the retiring TypeScript package currently permits.

Selective retention preserves the evidentiary value required by ADR-035 without
making the local store unusable through routine success noise. Durable admission
and explicit gap evidence make the system honest during outages while
preserving ADR-002's verdict semantics. Explicit migration and a bounded
dual-read window avoid both silent data loss and an indefinite dual-store
architecture.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| **One Kindling product, two profiles; generic event with Anvil-owned schema** | One lifecycle and store; clear policy ownership; independent retention classes; one shipped Anvil binary | Requires generated contracts, profile-aware queries, and coordinated releases |
| Separate standalone and governance products | Independent branding and defaults | Duplicates lifecycle/storage concepts and exposes a second product to Anvil users |
| Add Anvil-specific kinds to Kindling | Strong upstream typing | Moves Anvil taxonomy and release cadence into a policy-neutral mechanism |
| Retain every save-time success | Complete raw history | High-frequency noise, storage growth, and conflict with ADR-035 |
| Best-effort drop-on-full admission | Lowest hot-path work | Claims evidence exists when it was silently lost |
| Automatic sidecar import and dual-write | Minimal operator action and easy rollback | Risks ambiguous scope, duplicates, and an indefinite second source of truth |
| Deprecate the TypeScript integration package for one release | More consumer notice | Preserves a package without a live runtime role and extends schema drift |

## Consequences

- **Positive:** Product, retention, identity, migration, and ownership contracts
  are explicit before runtime activation.
- **Positive:** Governance evidence remains local, selectively useful, and
  honest about admission gaps.
- **Positive:** One Rust schema authority replaces parallel hand-maintained
  TypeScript contracts.
- **Negative:** Cross-repository release ordering and a compatibility floor are
  mandatory.
- **Negative:** Explicit-prune-only governance retention can grow without an
  operator action; hard quotas can reject new evidence until the operator
  prunes or raises a budget, so status and query surfaces must make size and
  gaps visible.
- **Negative:** Immediate package removal is a breaking cutover for any
  unobserved external consumer.
- **Risk:** The bounded spool or durable acknowledgement may threaten save-time
  latency.
- **Mitigation:** Benchmark the real admission boundary, enforce the seven-day
  and 64 MiB bounds, and surface gaps without changing the policy verdict.
- **Risk:** A failed migration could lose or duplicate legacy evidence.
- **Mitigation:** Dry-run first, deterministic IDs, an owner-only atomic state
  machine and backups, ambiguous-scope skips, parity digests, interruption
  recovery, a pre-cutover rollback boundary, and a one-release read-only
  compatibility path.

## References

- Related ADRs: [ADR-012](012-rust-cli-replacement.md),
  [ADR-035](035-three-pipe-observability-rule.md),
  [ADR-064](064-intercept-graph-cache-crate-boundary.md),
  [ADR-088](088-dpo-observation-kind-taxonomy.md), and
  [ADR-089](089-fp-telemetry-destination.md). This ADR amends the
  write-once/immutable retention wording in ADR-035 and ADR-089. It preserves
  ADR-088's gate-versus-constraint taxonomy and ADR-089's privacy/egress
  decision while superseding ADR-088's dotted spelling and KDS/upstream-kind
  mechanism and ADR-089's legacy ordinal product-kind model with Anvil-owned
  `anvil.governance.v1` payload kinds over `generic-event@1`. ADR-064 remains
  unchanged.
- APS module: [KFIT](../modules/kindling-product-fit.aps.md), especially
  KFIT-001 and KFIT-005..011
- Coordinating modules:
  [DPO](../modules/daemon-protection-observability.aps.md),
  [KDS](../archive/modules/kindling-daemon-sink.aps.md),
  [USAGE](../archive/modules/usage-analytics.aps.md), and
  [MLP2](../modules/multilayer-protection-v2.aps.md)
- Design authority: operator-approved KFIT-001 design session, 2026-08-03
