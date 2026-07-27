# Council Review — MCP 2026-07-28 dual-era (RC solution)

| Field | Value |
| --- | --- |
| Date | 2026-07-28 |
| Tier | **full** |
| Target | `feat/mcp26-dual-era-support` (`6087b391d`) vs `origin/main` |
| Roles | general, adversarial, operations, security, pragmatic |
| Status | **Converged** |
| Overall verdict | **ship-with-fixes** (branch OK; **do not merge to main** until MCP26-001 + closeout) |
| Merge readiness after final ratification | **needs-work** (short closeout lane, not a rewrite) |

## Scope reviewed

Dual-era MCP stdio host against the locked **2026-07-28 RC**: typed protocol
adapter (`mcp/protocol/*`), thin `commands/mcp.rs`, modern activation probe,
schema catalogue tests, process-local egress wording, W3C trace binding.
Branch held until final specification ratification (operator policy).

## Role verdicts

| Role | Verdict |
| --- | --- |
| general | ship-with-fixes |
| adversarial | ship-with-fixes |
| operations | ship-with-fixes |
| security | ship-with-fixes |
| pragmatic | ship-with-fixes; merge after ratification: **needs-work** |

## Findings (merged, by severity)

### Critical

None for *code security / auth*. Operations notes **main merge correctly blocked**
until MCP26-001 (process critical for production, not a code defect).

### Major (fix before treating dual-era wire as done / before main PR)

1. **Legacy `initialize` still echoes any `protocolVersion` string**  
   - Roles: general, adversarial, pragmatic, operations  
   - `crates/anvil-cli/src/mcp/protocol/domain.rs` (~87–100): `is_legacy_version`
     discarded; violates “never echo arbitrary version” / sealed matrix claim.  
   - **Fix:** accept only `LEGACY_PROTOCOL_VERSIONS` (or clamp to default); add
     fixtures for all four sealed versions.

2. **Activation discover probe accepts top-level `serverInfo` as modern**  
   - Roles: general, adversarial, security, operations (nit)  
   - `crates/anvil-cli/src/activation/mcp_client.rs` (~1157–1166): comment says
     reject legacy-shaped identity; code `or_else`s to `result.serverInfo`.  
   - **Exploit:** stub returns initialise-shaped body →
     `RestartHandshakeVerified` + `protocolEra=modern` + invented
     `protocolVersion=2026-07-28`.  
   - **Fix:** require `_meta["io.modelcontextprotocol/serverInfo"]` only;
     require `resultType` / `supportedVersions` when feasible; unit-test stub.

3. **Modern process lifetime vs bare `exit`**  
   - Role: adversarial  
   - Bare `{"method":"exit"}` without modern `_meta` still terminates the
     process after modern-only traffic.  
   - **Fix:** honour exit only after legacy initialise completed, or EOF-only
     process stop; golden tests.

4. **Test / platform gaps vs claimed matrix**  
   - Roles: general, operations  
   - Legacy goldens effectively pin `2024-11-05` only; no modern
     `resources/read` `ttlMs: 0` fixture; Windows timeout/exit probe tests are
     Unix-only.  
   - **Fix:** parametrise legacy versions; add resources/read cache test;
     Windows-safe probe fixture or explicit platform exception.

5. **Ship gates still open (expected)**  
   - Roles: operations, pragmatic  
   - MCP26-001 (final seal + ADR Accept), MCP26-010 (conformance/client matrix),
     MCP26-011 (docs) required before production claim / main merge.  
   - Architecture docs still describe initialise-era-only server.

### Minor

| # | Finding | Roles |
| --- | --- | --- |
| M1 | Dead `finish` pre-stamp path / unreachable legacy `server/discover` arm | general, adversarial, pragmatic |
| M2 | Dual-era probe worst-case ~2s/client under-documented (docs say 1s) | operations, adversarial |
| M3 | Meta depth walk no early abort; size = frame only | adversarial, security |
| M4 | Schema catalogue test-only (intentional); CI must always run unit tests | adversarial, pragmatic |
| M5 | Trace metrics thin (no era/fallback counters) | operations |
| M6 | Session-pinned wording remains in some comments | general |
| M7 | Weak modern short-circuit if future domain returns `resultType: complete` without stamp | adversarial |
| M8 | GCTX tool egress charged after execution (CPU residual, not leak) | security |
| M9 | External `$ref` policy only blocks http(s) | security |
| M10 | Spec §6.2 uninitialised legacy reject not enforced (open tools without init) | adversarial |

### Nits

Dead code comments, dual timeout test “~1s” wording, reader-thread join,
RC still unsealed (process), clientInfo dead_code (correct non-trust).

## Solid (all roles)

- Typed temporary adapter under `mcp/protocol/`; host thinned; four MiB frame kept  
- Modern routing, `-32022`, lifecycle gating, era-specific rendering  
- Discover does not wait on warm-up; process-local egress user copy  
- Activation: modern-first, reap, fresh legacy child; tier label unchanged  
- `clientInfo` / baggage not authority; redaction/auth paths preserved  
- Traceparent bind non-authoritative; no panic on garbage  
- Scope control: no HTTP/Apps/Tasks/tool renames; no `rmcp` beta pin  
- Branch-until-ratification discipline honest  

## Validation evidence (2026-07-28)

| Suite | Result |
| --- | --- |
| `cargo test -p eddacraft-anvil --bin anvil mcp::protocol::` | 16 passed |
| `cargo test -p eddacraft-anvil --test mcp_serve_stdio` | 42 passed |
| `cargo test -p eddacraft-anvil --test mcp_activation_probe` | 1 passed |
| `cargo test -p eddacraft-anvil --bin anvil mcp::tools::schema_catalogue::` | 4 passed |
| `cargo test -p eddacraft-anvil --bin anvil activation::mcp_client::tests::` | 33 passed |

Guidance (`scripts/agent/guidance.sh --branch`): risk `source`, review tier
`targeted` (escalated to **full** by operator request + protocol/activation
surface).

## Recommended fix order (on branch, before main)

1. Tighten **discover probe** identity (major #2)  
2. **Seal or reword** legacy version negotiation (major #1)  
3. **Exit lifecycle** policy for modern processes (major #3)  
4. Add **legacy version + modern resources/read** fixtures; probe latency docs  
5. After final publish: MCP26-001 closeout → 010 minimum evidence → 011 docs → PR  

## Publish note

Council findings are review evidence, not CI proof. Critical/major findings
must be fixed, deferred with rationale, or waived before a main PR.

**Status:** Converged  
**Tier:** full  
**Target:** `origin/main...feat/mcp26-dual-era-support` @ `6087b391d`
