# MCP26-001 ratification and SDK readiness gate

| Field | Value |
| --- | --- |
| Type | APS gate evidence |
| Work item | MCP26-001 |
| Branch | `feat/mcp26-dual-era-support` |
| Date | 2026-07-27; sealed 2026-07-29 |
| Status | Gate passed — final contract sealed; branch review and CI pending |
| ADR | [ADR-113](../decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md) (Accepted 2026-07-29) |

## Purpose

Record the prerelease baseline and the evidence that sealed the final MCP
`2026-07-28` contract. The ratification hold is lifted; normal review, CI,
merge, release, and APS lifecycle gates still apply.

## 1. Upstream timeline

| Event | Date | Source |
| --- | --- | --- |
| RC locked | 2026-05-21 | [RC blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/) |
| Final specification scheduled | 2026-07-28 | Same |
| This gate audit started | 2026-07-27 | anvil MCP26-001 |
| Final specification ratified | 2026-07-28 | [MCP `2026-07-28`](https://modelcontextprotocol.io/specification/2026-07-28) |
| Final seal completed | 2026-07-29 | anvil MCP26-001 |

**Ratification state as of 2026-07-29:** final is published. The branch may
proceed to PR after its implementation, evidence, documentation, and review
gates pass.

## 2. RC / draft contract inventory (locked baseline)

Drawn from the RC announcement and the draft changelog (against
`2025-11-25`). This is the pre-ratification contract MCP26 designs against;
MCP26-001 closeout must re-diff the final schema and note deltas.

### Breaking / major (relevant to anvil stdio server)

| # | Change | anvil impact |
| --- | --- | --- |
| 1 | Remove protocol-level sessions / `Mcp-Session-Id` | Rename process-local egress budget wording; no sticky session |
| 2 | Remove modern `initialize` / `initialized`; per-request `_meta` | Dual-era host; modern path must not require handshake |
| 3 | Require `server/discover` | MCP26-004; activation modern probe (MCP26-007) |
| 4 | `subscriptions/listen` replaces list/resource subscribe model | Out of scope — anvil does not advertise subscriptions |
| 5 | Remove modern `ping` (and logging roots notifications) | Gate lifecycle methods to legacy path only |
| 6 | Tasks → extension | Out of scope |
| 7 | MRTR / `InputRequiredResult` | Out of scope for anvil tools |
| 8 | Required `resultType` (`complete` / input-required form) | MCP26-005; **confirm exact string enum in final schema** |
| 9 | SSE resumability removal | N/A for stdio-only anvil |

### Minor / schema (relevant)

| Change | anvil impact |
| --- | --- |
| Trace context keys in `_meta` | MCP26-009 |
| Deterministic `tools/list` order | Already true via static registry |
| `ttlMs` / `cacheScope` on list/read | MCP26-005 |
| Resource not found → `-32602` | Already true for unknown URI |
| JSON Schema 2020-12 for tools | MCP26-008 |
| Error allocation: `UnsupportedProtocolVersion` → **`-32022`** | MCP26-003; confirm final |
| Deprecate Roots / Sampling / Logging | anvil does not advertise them |

### Open schema questions for final diff

1. Exact `resultType` string values (`complete` vs input-required spelling —
   draft changelog uses `"input_required"`; some prose uses camelCase).
2. Exact shape of `UnsupportedProtocolVersion` error data (`supported`,
   `requested`).
3. Whether `server/discover` requires cache fields with the same
   `CacheableResult` rules as list/read.
4. Any post-RC renumbers of error codes in the final schema package.

## 3. Provisional version matrix

| Era | Versions | Behaviour | Evidence |
| --- | --- | --- | --- |
| Modern | `2026-07-28` | Stateless dual-era modern path | Spec + draft |
| Legacy (default) | `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` | Initialise-era stdio | Operator default: keep all |
| Legacy (optional narrow) | `2025-11-25` only | Smaller surface if MCPX evidence allows | Operator open; decide at closeout |

### Current anvil pins (pre-MCP26)

| Surface | Value | Path |
| --- | --- | --- |
| Server default echo version | `2024-11-05` | `commands/mcp.rs` `DEFAULT_PROTOCOL_VERSION` |
| Activation probe | `2025-06-18` | `activation/mcp_client.rs` `PROBE_PROTOCOL_VERSION` |
| Integration fixtures | `2024-11-05` | `tests/mcp_serve_stdio.rs` |

### MCPX client compatibility note

MCPX first wave (Done): Claude Code, Cursor, Codex, OpenCode, Gemini CLI,
Antigravity, OpenClaw, VS Code, Copilot CLI, Grok, Warp, project-scoped Zed —
all launch `anvil mcp serve --stdio`. None require Streamable HTTP for the
supported install path. **Seal condition:** after ratification, confirm no
promoted client already requires modern-only discover without legacy fallback;
until then dual-era is mandatory.

## 4. `rmcp` evaluation (sealed 2026-07-29)

| Fact | Value |
| --- | --- |
| crates.io stable | `3.0.0` |
| Licence | Apache-2.0 |
| Repository | https://github.com/modelcontextprotocol/rust-sdk |
| Evaluated source | tag `rmcp-v3.0.0`, commit `4e361b715fc70b8a09f0a8aeaedc160712a3472d` |
| Modern conformance pin | `@modelcontextprotocol/conformance@0.2.0-alpha.10`, source `49103de6ed70804e940637bf3e9e29e4a3f54e64` |
| Legacy conformance pin | `@modelcontextprotocol/conformance@0.1.16` |
| Product decision | Keep the temporary typed adapter for this ship |
| Removal condition | MCP26-012 after a bounded `rmcp` transport passes §8.2 |

### §8.2 adoption gate status

| Criterion | Status |
| --- | --- |
| Stable crates.io release targets final `2026-07-28` | **Done** — `rmcp 3.0.0` |
| SDK wire matches ratified schema | **Done for schema target** |
| Conformance runner reviewed | **Done** — official server runner is HTTP-only; stdio applicability recorded under MCP26-010 |
| Four MiB frame ceiling | **Failed** — built-in async read transport uses unbounded `read_until` |
| Stdout protocol purity / EOF / Windows | **Open** — requires spike |
| Sync domain handlers without blocking runtime | **Open** — requires spike |
| Startup / memory budgets | **Open** — requires spike |
| Licence / dependency review | **Partial** — Apache-2.0; full transitive review when adopted |

Stable availability alone is insufficient. The frame-ceiling failure is a
release-blocking mismatch with anvil's transport contract, so ADR-113 accepts
the typed adapter and leaves SDK removal work in MCP26-012.

## 4b. Operator ratification (2026-07-27)

| Item | Decision |
| ---- | -------- |
| Direction A–F (dual-era, SDK-first, temporary typed adapter, no product beta pin, anvil domain ownership, branch until gate) | **Approved** |
| Modern `2026-07-28` | **Yes** |
| Legacy matrix | **Keep all** by default; open to latest-only (`2025-11-25`) if convinced by evidence |
| Unsupported-version error | **Hold** for final schema |
| Cache policy | **Approved** (`ttlMs`/`private` as in spec) |
| Non-goals | **Approved** |
| Egress/session wording, warm-up, activation probe | **OK** |
| Release window | **First anvil release after ratification, or next+1** |

### Legacy full-set vs latest-only (decision still open)

**Default:** keep the full provisional legacy set (matches current anvil
default `2024-11-05` fixtures and probe `2025-06-18`).

**Optional narrow (operator open):** support only `2025-11-25` as legacy if
MCP26-001/MCPX evidence shows promoted clients do not need older pins.
Trade-off: less golden-fixture surface and simpler negotiation vs risk that
older installed clients or probes still send `2024-11-05` / `2025-06-18`.
Recommend **keep full set** unless a client inventory pass after ratification
shows zero need for the older three.

## 4c. RC implementation authorisation (2026-07-27)

Operator authorised building dual-era support **against the locked RC /
draft contract** on `feat/mcp26-dual-era-support` without waiting for final
publication. Constraints:

- Wire implementation may land on this branch and be tested against RC shapes.
- **Merge to `main` still requires** MCP26-001 closeout (final schema diff,
  formal ADR-113 Accept, sealed pins).
- Temporary protocol path is the **typed internal dual-era adapter** under
  `crates/anvil-cli/src/mcp/protocol/` (not `rmcp` beta product pin).

## 5. Branch and merge policy

- Branch: `feat/mcp26-dual-era-support` (Worktrunk worktree).
- Plan artefacts and gate evidence may land on this branch before ratification.
- **No dual-era wire implementation merges to `main`** until:
  1. Final MCP `2026-07-28` is published,
  2. Final-vs-RC (or final-vs-this-audit) diff is recorded in §7,
  3. ADR-113 is Accepted (or Amended with final pins),
  4. This audit records the ratification seal as passed; APS lifecycle status
     may remain In Progress until review and merge evidence exists.

## 6. Closeout checklist (MCP26-001 ratification seal)

- [x] Final specification URL and schema artefact recorded
- [x] Final vs locked-RC / draft diff completed; open questions in §2 resolved
- [x] Legacy matrix sealed as the full four-version set
- [x] Decision path sealed: temporary adapter
      authorised with removal condition
- [x] Conformance suite versions pinned for modern and selected legacy
- [x] ADR-113 Accepted and DECISION-LOG updated
- [x] Module MCP26-001 records final seal evidence
- [x] Operator/release note: branch may open MCP26-002+ PRs
- [x] Release target named: first post-ratification anvil release, or that
      release + 1 (operator-approved window)
- [x] Legacy matrix sealed as **full set**

## 7. Final vs RC diff

| Area | RC/draft baseline | Final | Delta for anvil |
| --- | --- | --- | --- |
| Protocol version string | `2026-07-28` | `2026-07-28` | None |
| `clientInfo` | Required when modern metadata is present | Optional; when present, `name` and `version` are required strings | Parser and black-box fixtures tightened |
| `server/discover` | Cacheable result with identity in result `_meta` | Same | None |
| `resultType` | `complete`, `input_required`, or extension string | Same | None |
| Unsupported version error | `-32004` in literal RC artefact | `-32022` with `supported` and `requested` | Existing branch implementation matches final |
| Cache fields | `ttlMs` and `cacheScope` required on `CacheableResult` | Same | None |

Artefact SHA-256 pins:

- RC `schema.json`: `bd19a7a9e6ee3a7394aec816cd0660a0156864e828a348129baccdb83457d234`
- Final `schema.json`: `ef70b61f99b6d2e5e3b46863822eab08dff6a45bedc7a08914e0e5b133f40203`
- RC `schema.ts`: `0ab5accf367ddd1405ccb738c544e259c40dd2b03a7d8f921f289a43c1fbcb7a`
- Final `schema.ts`: `742750af0bb8c716e7030c4977c992b55d1adc4407e9e66997db5846baedc2cd`

## 8. References

- Spec: [`plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md)
- Module: [`plans/modules/mcp-dual-era-support.aps.md`](../modules/mcp-dual-era-support.aps.md)
- ADR-113: [`plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md`](../decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md)
- [RC blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
- [Final specification](https://modelcontextprotocol.io/specification/2026-07-28)
- [Final changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)
- [rust-sdk ROADMAP](https://github.com/modelcontextprotocol/rust-sdk/blob/main/ROADMAP.md)
- [crates.io rmcp](https://crates.io/crates/rmcp)
