# MCP26-001 ratification and SDK readiness gate

| Field | Value |
| --- | --- |
| Type | APS gate evidence |
| Work item | MCP26-001 |
| Branch | `feat/mcp26-dual-era-support` |
| Date | 2026-07-27 |
| Status | In Progress — pre-ratification |
| ADR | [ADR-113](../decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md) (Proposed) |

## Purpose

Record the evidence collected for MCP26-001 before the final MCP
`2026-07-28` specification is published. This file is the living gate log:
nothing in MCP26-002+ may merge to `main` until the **Closeout checklist**
below is complete.

## 1. Upstream timeline

| Event | Date | Source |
| --- | --- | --- |
| RC locked | 2026-05-21 | [RC blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/) |
| Final specification scheduled | 2026-07-28 | Same |
| This gate audit started | 2026-07-27 | anvil MCP26-001 |

**Ratification state as of 2026-07-27:** final not yet published. Working
baseline is the locked RC / draft specification tree. Implementation wire
changes stay on the feature branch.

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
| Legacy | `2025-11-25`, `2025-06-18`, `2025-03-26`, `2024-11-05` | Initialise-era stdio | Spec intent; MCPX clients all stdio launch |

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

## 4. `rmcp` evaluation (2026-07-27)

| Fact | Value |
| --- | --- |
| crates.io stable max | `2.2.0` (2026-07-08) |
| crates.io newest | `3.0.0-beta.2` (2026-07-24) |
| Licence | Apache-2.0 |
| Repository | https://github.com/modelcontextprotocol/rust-sdk |
| ROADMAP target | v3.0.0 for 2026-07-28 features |
| Conformance (ROADMAP) | Server 39/40 on suite `0.2.0-alpha.9`; outstanding `json-schema-2020-12` |
| Legacy suite (ROADMAP) | Spec `2025-11-25` suite `0.1.16` — 30/30 server |
| Product pin decision | **Do not pin beta into product** until stable + §8.2 gate |
| Fallback | Temporary typed internal adapter (ADR-113) |

### §8.2 adoption gate status

| Criterion | Status |
| --- | --- |
| Stable crates.io release targets final `2026-07-28` | **Open** — no stable v3 yet |
| SDK wire matches ratified schema | **Open** — final not published |
| Conformance roadmap reviewed | **Done** — 39/40 server; JSON Schema outstanding |
| Four MiB frame ceiling | **Open** — requires spike on chosen pin |
| Stdout protocol purity / EOF / Windows | **Open** — requires spike |
| Sync domain handlers without blocking runtime | **Open** — requires spike |
| Startup / memory budgets | **Open** — requires spike |
| Licence / dependency review | **Partial** — Apache-2.0; full transitive review at pin time |

## 5. Branch and merge policy

- Branch: `feat/mcp26-dual-era-support` (Worktrunk worktree).
- Plan artefacts and gate evidence may land on this branch before ratification.
- **No dual-era wire implementation merges to `main`** until:
  1. Final MCP `2026-07-28` is published,
  2. Final-vs-RC (or final-vs-this-audit) diff is recorded in §7,
  3. ADR-113 is Accepted (or Amended with final pins),
  4. MCP26-001 status advances to Done / Merged with closeout evidence.

## 6. Closeout checklist (MCP26-001 Done)

- [ ] Final specification URL and schema artefact recorded
- [ ] Final vs locked-RC / draft diff completed; open questions in §2 resolved
- [ ] Legacy matrix sealed (confirm or amend provisional set)
- [ ] Decision path sealed: stable `rmcp` pin **or** temporary adapter
      authorised with removal condition
- [ ] Conformance suite versions pinned for modern and selected legacy
- [ ] ADR-113 Accepted (or Amended) and DECISION-LOG updated
- [ ] Module MCP26-001 Status closed with validation evidence
- [ ] Operator/release note: branch may open MCP26-002+ PRs

## 7. Final vs RC diff (fill after 2026-07-28)

| Area | RC/draft baseline | Final | Delta for anvil |
| --- | --- | --- | --- |
| Protocol version string | `2026-07-28` | _TBD_ | |
| `server/discover` schema | draft | _TBD_ | |
| `resultType` enum | draft (`complete` / input-required form) | _TBD_ | |
| Unsupported version error code | `-32022` (draft renumber) | _TBD_ | |
| Cache field requirements | draft `CacheableResult` | _TBD_ | |
| Other | | | |

## 8. References

- Spec: [`plans/specs/2026-07-27-mcp-2026-07-28-dual-era-support.md`](../specs/2026-07-27-mcp-2026-07-28-dual-era-support.md)
- Module: [`plans/modules/mcp-2026-07-28-dual-era-support.aps.md`](../modules/mcp-2026-07-28-dual-era-support.aps.md)
- ADR-113: [`plans/decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md`](../decisions/113-mcp-2026-07-28-dual-era-and-rmcp.md)
- [RC blog](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
- [Draft changelog](https://modelcontextprotocol.io/specification/draft/changelog)
- [rust-sdk ROADMAP](https://github.com/modelcontextprotocol/rust-sdk/blob/main/ROADMAP.md)
- [crates.io rmcp](https://crates.io/crates/rmcp)
