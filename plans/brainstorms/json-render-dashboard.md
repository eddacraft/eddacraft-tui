# JSON-Render Dashboard Brainstorm

## Context

Anvil's web dashboard (`anvil-ui`) is planned for v1.1+ but not yet built. The core question: how do we let end users get the dashboards they actually want without building every permutation by hand?

**Key discovery**: [vercel-labs/json-render](https://github.com/vercel-labs/json-render) — an open-source library that bridges AI model output to React components through a constrained JSON intermediary.

---

## What json-render Does

```
Define guardrails → Users prompt → AI generates JSON → Render progressively
```

1. **Developer defines a component catalog** — the only components AI is allowed to use
2. **End user describes what they want** in natural language
3. **AI generates constrained JSON** matching the schema (not arbitrary code)
4. **React renders progressively** from the streamed JSON output

The critical property: AI output is **guardrailed by design**. It can only reference components, actions, and data bindings from the developer-defined vocabulary.

---

## Why This Fits Anvil

### Philosophical alignment

Anvil's entire thesis is guardrailed AI output. json-render applies the same principle to UI generation. The parallel is direct:

| Anvil Core | json-render |
|---|---|
| Anti-pattern detection constrains code | Component catalog constrains UI |
| Architecture boundaries limit scope | Schema validation limits output |
| Evidence trails audit changes | JSON intermediary is inspectable |
| Zod-first schema validation | Built-in validation (required, format) |
| Suppression requires justification | Actions require confirmation dialogs |

### Technical alignment

- **TypeScript-first** — matches our stack
- **React-based** — matches planned anvil-ui tech (React or Solid)
- **Zod-compatible** — their schemas and ours speak the same language
- **Monorepo-friendly** — `@json-render/core` + `@json-render/react` split mirrors our package architecture
- **Progressive rendering** — streams JSON as AI generates it, good UX for dashboard assembly

---

## Concrete Use Cases

### 1. Custom gate result views

User prompt: *"Show me all failed gates from the last week grouped by project, with expandable error details"*

AI generates JSON referencing our catalog: `DataTable`, `StatusBadge`, `ExpandableRow`, `DateFilter` — all pre-approved components bound to Anvil's gate result data.

### 2. Anti-pattern trend dashboards

User prompt: *"Chart showing ts-ignore usage across our monorepo over the last 3 months, broken down by package"*

AI generates: `LineChart` + `PackageSelector` + `DateRange` — catalog components with data bindings to our anti-pattern history.

### 3. Audit log exploration

User prompt: *"Table of recent suppression requests with who approved them and their justifications"*

AI generates: `AuditTable` + `UserAvatar` + `ApprovalStatus` + `TextPreview` — all pulling from the evidence trail API.

### 4. Team health overview

User prompt: *"Dashboard with architecture drift score per team, warning counts this sprint, and gate pass rate"*

AI generates: `MetricCard` x3 + `GridLayout` + `SparklineChart` — composing pre-built widgets into the requested layout.

### 5. Role-based conditional views

json-render supports conditional visibility based on auth status and data paths. This means:
- Admins see suppression approval queues
- Developers see their own warning history
- Team leads see aggregate metrics
- All from the same component catalog, visibility controlled declaratively

---

## Proposed Component Catalog (starter)

These would be the guardrailed building blocks users can compose:

### Layout

- `GridLayout` — responsive grid container
- `Section` — titled content section
- `TabGroup` — tabbed content areas
- `Sidebar` — collapsible sidebar panel

### Data display

- `DataTable` — sortable/filterable table with pagination
- `MetricCard` — single-value metric with trend indicator
- `StatusBadge` — colored status pill (pass/fail/warn/info)
- `Timeline` — chronological event display
- `CodeBlock` — syntax-highlighted code snippet

### Charts

- `LineChart` — time series data
- `BarChart` — categorical comparisons
- `SparklineChart` — inline mini chart for metric cards
- `HeatMap` — density visualization (e.g., warnings by file by day)

### Anvil-specific

- `GateResultCard` — gate pass/fail with details
- `WarningList` — anti-pattern warnings with severity
- `DriftIndicator` — architecture drift score display
- `SuppressionRequest` — suppression with approval status
- `PlanCard` — APS plan summary with progress
- `EvidenceEntry` — audit trail line item
- `FileViolationMap` — file tree with violation markers

### Interactive

- `DateRangeFilter` — date range selector
- `PackageSelector` — monorepo package picker
- `SearchInput` — text search with suggestions
- `RefreshButton` — manual data refresh

---

## Data Binding Strategy

json-render supports data path bindings. We would expose Anvil's data through a structured context:

```typescript
// Dashboard data context shape
interface AnvilDashboardContext {
  gates: {
    recent: GateResult[];
    byProject: Record<string, GateResult[]>;
    passRate: number;
  };
  warnings: {
    active: Warning[];
    history: Warning[];
    byPattern: Record<string, Warning[]>;
    trend: TimeSeriesPoint[];
  };
  drift: {
    current: DriftScore;
    history: DriftScore[];
    byBoundary: Record<string, DriftScore>;
  };
  suppressions: {
    pending: Suppression[];
    approved: Suppression[];
    denied: Suppression[];
  };
  audit: {
    entries: AuditEntry[];
    byUser: Record<string, AuditEntry[]>;
  };
}
```

AI-generated JSON references paths like `gates.recent`, `warnings.trend`, `drift.current.score` — all resolved at render time against live data.

---

## Architecture Sketch

```
┌─────────────────────────────────────────────────────┐
│  anvil-ui (React app)                               │
│                                                     │
│  ┌──────────────┐  ┌────────────────────────────┐   │
│  │ Prompt Input  │  │  @json-render/react         │   │
│  │ (user types   │──│  Renderer                   │   │
│  │  what they    │  │                              │   │
│  │  want)        │  │  ┌────────────────────────┐ │   │
│  └──────────────┘  │  │ Component Catalog       │ │   │
│         │          │  │ (GridLayout, DataTable,  │ │   │
│         ▼          │  │  MetricCard, GateResult, │ │   │
│  ┌──────────────┐  │  │  WarningList, etc.)      │ │   │
│  │ AI Backend    │  │  └────────────────────────┘ │   │
│  │ (constrained  │──│                              │   │
│  │  JSON output) │  │  ┌────────────────────────┐ │   │
│  └──────────────┘  │  │ Data Context Provider   │ │   │
│                    │  │ (gates, warnings, drift, │ │   │
│                    │  │  suppressions, audit)    │ │   │
│                    │  └────────────────────────┘ │   │
│                    └────────────────────────────────┘   │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │  anvil-api (data layer)                       │   │
│  │  Gate results, warnings, drift, audit trail   │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1 — Foundation

- Install `@json-render/core` and `@json-render/react`
- Define initial component catalog (layout + data display basics)
- Build data context provider wired to anvil-api
- Static dashboards using json-render (no AI yet, hand-written JSON)

### Phase 2 — AI generation

- Add prompt input with AI backend
- Constrain model output to component catalog schema
- Progressive rendering of streamed JSON
- Save/load dashboard configurations (JSON is the persistence format)

### Phase 3 — Personalisation

- Per-user saved dashboards
- Conditional visibility by role
- Dashboard sharing (share the JSON, not code)
- Template library of common dashboard patterns

### Phase 4 — Advanced

- Custom actions (e.g., approve suppression from dashboard)
- Real-time data updates (WebSocket/SSE push)
- Dashboard embedding in CLI reports (render JSON to terminal via Ink)
- Cross-team dashboard aggregation

---

## Open Questions

1. **Model choice** — Which AI model generates the JSON? Could use any model with function calling / structured output. Anthropic, OpenAI, or local models all work since the output is just JSON matching a schema.

2. **Offline mode** — Can we support dashboard building without AI? Yes — json-render JSON is human-writable too. We could offer a visual builder alongside the prompt interface.

3. **Performance** — How does progressive rendering perform with large datasets? Need to benchmark with realistic gate result volumes (thousands of entries).

4. **Ink crossover** — json-render is React-based. Our CLI TUI uses Ink (also React). Could we share component logic between terminal and web dashboards? The catalog definitions could be shared even if rendering differs.

5. **Auditability** — Dashboard JSON configs should themselves be tracked in evidence trails. Meta-guardrails: Anvil auditing its own dashboard configs.

---

## Verdict

This approach lets users describe what they want and get it — within safe boundaries we define. The JSON intermediary is inspectable, auditable, and deterministic. It matches Anvil's philosophy of constrained AI output with evidence trails.

The component catalog acts as an architecture boundary for the UI, the same way Anvil enforces architecture boundaries for code. We're applying our own principles to our own product.

**Recommendation**: Adopt json-render as the rendering foundation for anvil-ui dashboards in v1.1.
