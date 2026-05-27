# Competitor Tier 2 Tracking

| Type  | Authority | Owner    | Status | Freshness                                        |
| ----- | --------- | -------- | ------ | ------------------------------------------------ |
| Guide | Advisory  | STRATEGY | Live   | Metadata backfilled 2026-05-24 during DOCGOV-009 |

| Upstream                 | Downstream                      |
| ------------------------ | ------------------------------- |
| External competitor scan | Borrow/adopt candidate tracking |

Purpose: keep a persistent Tier 2 watchlist in-repo and append new Tier 2
candidates as they appear in daily competitor scans.

## Workflow (for future updates)

- Add new items under the current date section (`## YYYY-MM-DD`).
- Keep each entry concise and structured.
- If an item is promoted to Top list, mark it as `promoted` and link to the Tier
  1/Top tracker (when created).

Entry format:

- **repo + link:** `owner/repo` — https://github.com/owner/repo
- **why it matters:** one-line signal worth borrowing
- **integration effort:** `S` | `M` | `L`
- **expected impact:** `Low` | `Med` | `High`
- **status:** `watch` | `promoted` | `dropped`

---

## 2026-03-08

- **repo + link:** `mcpjungle/MCPJungle` —
  https://github.com/mcpjungle/MCPJungle
  - **why it matters:** self-hosted MCP gateway with enterprise controls
    (auth/ACL/observability) relevant to control-plane positioning
  - **integration effort:** M
  - **expected impact:** Med
  - **status:** watch

- **repo + link:** `funstory-ai/aifw` — https://github.com/funstory-ai/aifw
  - **why it matters:** lightweight LLM firewall posture focused on deploy-fast
    PII/routing safeguards
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** watch

- **repo + link:** `confident-ai/deepteam` —
  https://github.com/confident-ai/deepteam
  - **why it matters:** accessible red-team workflow ergonomics suitable for
    non-security engineers
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** watch

- **repo + link:** `lasso-security/mcp-gateway` —
  https://github.com/lasso-security/mcp-gateway
  - **why it matters:** intermediary gateway architecture pattern for
    centralized policy-enabled routing
  - **integration effort:** M
  - **expected impact:** Med
  - **status:** watch

- **repo + link:** `ProjectRecon/awesome-ai-agents-security` —
  https://github.com/ProjectRecon/awesome-ai-agents-security
  - **why it matters:** control-taxonomy baseline that helps coverage mapping
    and gap analysis
  - **integration effort:** S
  - **expected impact:** Med
  - **status:** watch
