# Anvil Pitch Deck

## Quick Start

```
# Run the full pipeline
Spawn pitch-orchestrator to execute the Anvil pitch deck pipeline.

# Run a single phase
Spawn pitch-orchestrator: execute Phase 1 only.

# Resume from checkpoint
Spawn pitch-orchestrator: skip Phase 1, execute Phase 2-4.
```

## Pipeline

```
Phase 1: RESEARCH          Phase 2: STRATEGY         Phase 3: CONTENT           Phase 4: SYNTHESIS
─────────────────          ─────────────────         ────────────────           ──────────────────
                                                     ┌─ pitch-writer ──┐
pitch-researcher ────────► pitch-strategist ────────►│                  ├─────► pitch-exec-summary
                                                     └─ pitch-designer ─┘
```

Phases are sequential with quality gates. Phase 3 runs writer and designer in parallel.

## Agents

| Agent | Role | Phase |
|-------|------|-------|
| **pitch-orchestrator** | Pipeline conductor, quality gates, status tracking | All |
| **pitch-researcher** | TAM/SAM/SOM, competitive landscape, trend analysis | 1: Research |
| **pitch-strategist** | Win themes, narrative arc, slide structure | 2: Strategy |
| **pitch-writer** | Slide copy, talking points, investor FAQ | 3: Content |
| **pitch-designer** | Visual specs, layouts, data viz, brand compliance | 3: Content |
| **pitch-exec-summary** | SCQA summaries, investor one-pager | 4: Synthesis |

## Workspace

```
plans/pitch-deck/
├── research/          # Phase 1 — market data, competitive analysis
├── strategy/          # Phase 2 — win themes, narrative arc, slide outline
├── content/           # Phase 3 — slide copy, visual specs, talking points
├── deliverables/      # Phase 4 — executive summary, one-pager, FAQ
└── status.md          # Pipeline state (maintained by orchestrator)
```

## Brand Truth

Two co-equal sources — web and product:

| Source | Location | Governs |
|--------|----------|---------|
| Website | `apps/website/AGENTS.md` + `apps/website/app/globals.css` | Web palette, typography, components |
| TUI Spec | `docs/specs/anvil_tui_context.md` (dev branch) | Rust palette, layout, product aesthetic |

Design system: **Nordic Brutalist / Industrial Terminal**
Core accent: `--anvil` / `EMBER` (#cc5500)

## Source Agents

Adapted from [agency-agents](https://github.com/msitarzewski/agency-agents):
- `sales/sales-proposal-strategist.md` → pitch-strategist
- `product/product-trend-researcher.md` → pitch-researcher
- `design/design-visual-storyteller.md` + `design/design-brand-guardian.md` → pitch-designer
- `marketing/marketing-content-creator.md` → pitch-writer
- `support/support-executive-summary-generator.md` → pitch-exec-summary
- `specialized/agents-orchestrator.md` → pitch-orchestrator
