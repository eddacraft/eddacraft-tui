# SYSTEM PROMPT: EDDACRAFT / ANVIL — UNIFIED UI CONTEXT

## 1. PURPOSE

This document is the single source of truth for all **customer-facing control
surfaces** across the EddaCraft product suite. It governs visual identity,
interaction patterns, and brand enforcement for every touchpoint an engineer
encounters — from the first `anvil init` in their terminal to the web dashboard
they open in their browser.

Use this document when building, demoing, or producing marketing material for
any Anvil surface.

---

## 2. THE SURFACES

EddaCraft ships six distinct control surfaces. Each must feel like the same
system wearing different form factors.

| # | Surface | Technology | Status | Primary Audience |
|---|---------|-----------|--------|-----------------|
| 1 | **CLI** | Rust / clap | In Development | Engineers (CI/CD, scripts, daily workflow) |
| 2 | **TUI** | Rust / Ratatui | Complete | Engineers (interactive terminal sessions) |
| 3 | **Web Dashboard** | Next.js 16 / Tailwind 4 / shadcn/ui | Planned | Tech Leads, Principals, CTOs |
| 4 | **Documentation Site** | Docusaurus / Vercel | Deployed | All users (onboarding, reference) |
| 5 | **VS Code Extension** | VS Code API | Deployed | Engineers (real-time diagnostics) |
| 6 | **MCP Server** | Node.js / MCP Protocol | Deployed | AI Agents (tool integration) |

Supporting assets (not interactive, but customer-visible):

| Asset | Format | Use |
|-------|--------|-----|
| **Investor Decks** | PPTX (generated) | Fundraising, partner pitches |
| **OG / Social Images** | PNG (generated) | Link previews, social posts |
| **Transactional Email** | React Email | Beta invites, OTP codes, waitlist confirmation |

---

## 3. BRAND & AESTHETIC LAWS

These laws are absolute. They apply to every surface, every asset, every pixel.

### 3.1 Core Posture

> We do not "market" software; we document it.

- **Aesthetic:** Nordic Brutalist / Industrial Terminal.
- **Tone:** Strict, authoritative, structural, and quiet.
- **Language:** UK English spelling throughout (`colour`, `initialise`,
  `licence`).

### 3.2 The Three Commandments

1. If it looks like a template, kill it.
2. If it hides complexity, reveal it.
3. If it shouts, silence it.

### 3.3 Anti-Patterns (Governance Checklist)

Before shipping any surface or asset, run this diff:

- [ ] Does it have a gradient? → **REJECT** (unless heatmap).
- [ ] Does it use a drop shadow for depth? → **REJECT** (use 1px `--structure`
  border).
- [ ] Is the copy "exciting"? (e.g., "Revolutionize your workflow!") →
  **REJECT** (rewrite to be declarative: "Validate your workflow.").
- [ ] Is the primary font monospace? → Headers: **PASS**. Body paragraphs:
  **REJECT** (use sans).
- [ ] Does it use rounded corners? → **REJECT** (sharp corners only).
- [ ] Does it use emojis or decorative icons? → **REJECT** (use bracket
  syntax).

---

## 4. DESIGN SYSTEM & COLOUR TOKENS

### 4.1 The Five-Token Palette

Our world is dark. Light is used only for signal.

| Token | Hex | RGB | Usage |
|-------|-----|-----|-------|
| `--void` | `#0D0D0F` | `(13, 13, 15)` | Global background. Never use pure `#000000`. |
| `--structure` | `#2A2A2E` | `(42, 42, 46)` | Borders, dividers, grid lines. No drop shadows. |
| `--text-primary` | `#EBEBEB` | `(235, 235, 235)` | Primary content text. Never pure `#FFFFFF`. |
| `--anvil-ember` | `#CC5500` | `(204, 85, 0)` | Anvil action: headers, active borders, primary buttons, deletion. |
| `--edda-growth` | `#2E8B57` | `(46, 139, 87)` | Edda memory: success, additions, valid states. |

### 4.2 Extended Tokens (Semantic States)

| Token | Hex | RGB | Usage |
|-------|-----|-----|-------|
| `--text-muted` | `#85858A` | `(133, 133, 138)` | Inactive text, comments, footers, timestamps. |
| `--error` | `#C94A4A` | `(201, 74, 74)` | Blocked actions, failures, gate rejections. |
| `--warning` | `#D08C38` | `(208, 140, 56)` | Warnings, partial compliance. |

### 4.3 Surface-Specific Token Mapping

#### Rust / Ratatui (TUI)

```rust
pub struct EddaTheme;
impl EddaTheme {
    pub const VOID: Color = Color::Rgb(13, 13, 15);
    pub const BORDER: Color = Color::Rgb(42, 42, 46);
    pub const FG: Color = Color::Rgb(235, 235, 235);
    pub const MUTED: Color = Color::Rgb(133, 133, 138);
    pub const EMBER: Color = Color::Rgb(204, 85, 0);
    pub const GROWTH: Color = Color::Rgb(46, 139, 87);
    pub const ERROR: Color = Color::Rgb(201, 74, 74);
    pub const WARNING: Color = Color::Rgb(208, 140, 56);
}
```

#### Tailwind CSS (Web Dashboard, Docs)

```js
// tailwind.config.js
colors: {
  void: '#0D0D0F',
  structure: '#2A2A2E',
  ember: '#CC5500',
  growth: '#2E8B57',
  muted: '#85858A',
  error: '#C94A4A',
  warning: '#D08C38',
}
```

#### CSS Custom Properties (Global)

```css
:root {
  --void: #0D0D0F;
  --structure: #2A2A2E;
  --text-primary: #EBEBEB;
  --text-muted: #85858A;
  --anvil-ember: #CC5500;
  --edda-growth: #2E8B57;
  --error: #C94A4A;
  --warning: #D08C38;
}
```

#### ANSI Terminal (CLI Plain Output)

```
Red    → --anvil-ember
Green  → --edda-growth
Grey   → --text-muted
White  → --text-primary (never bold white)
```

---

## 5. TYPOGRAPHY

### 5.1 The Hybrid Rule

- **System Voice (Mono):** JetBrains Mono / IBM Plex Mono.
  - Where: Headers, buttons, KPIs, code blocks, CLI output, TUI surfaces.
  - Why: Signals "This is machine-generated truth."
- **Narrative Voice (Sans):** Inter / Geist Sans.
  - Where: Long-form reading — documentation, blog posts, email body text.
  - Why: Monospace is exhausting in paragraphs.

### 5.2 Per-Surface Typography

| Surface | Headers | Body | Code |
|---------|---------|------|------|
| CLI (plain) | N/A (terminal font) | N/A | N/A |
| TUI | Terminal mono | Terminal mono | Terminal mono |
| Web Dashboard | JetBrains Mono | Inter | JetBrains Mono |
| Documentation | JetBrains Mono | Inter | JetBrains Mono |
| Decks / Slides | JetBrains Mono (UPPERCASE) | Inter | JetBrains Mono |
| OG Images | JetBrains Mono | — | JetBrains Mono |
| Email | IBM Plex Mono (headers) | Inter | IBM Plex Mono |

---

## 6. ICONOGRAPHY

### 6.1 The Bracket System

We do not use metaphors (clouds, gears, shields). We use syntax.

- Style: 2px stroke, sharp corners, no fill.
- Container: Every icon is framed by brackets `[ ]`.
- Semantics:
  - `[ ]` — Context (neutral, structural)
  - `[ = ]` — Action (Anvil governance)
  - `[ ≡ ]` — History (Edda memory)
  - `[ ■ ]` — Parent system (EddaCraft)
  - `[ > ]` — Signal (live data, interception)

### 6.2 Macro Logos

#### Anvil Header (TUI / Slides / Marketing)

Rendered in `--anvil-ember` with text in `--text-primary` / `--text-muted`:

```
████     ████
██         ██
██  █████  ██
██         ██   a n v i l
██  █████  ██
██         ██
████     ████
```

#### EddaCraft Watermark (Footer / Credits)

Rendered in `--text-muted` and `--structure`:

```
  [ ■ ] e d d a c r a f t
        v0.9.2-beta
```

---

## 7. SURFACE 1: CLI (Rust / clap)

### 7.1 Overview

The CLI is the primary entry point. It is the "flagship store" — it must feel
indistinguishable from the brand.

- **Binary:** `anvil`
- **Framework:** Rust + clap (derive macros)
- **Output modes:** TUI (default in TTY), Plain text (`--no-tui`), JSON
  (`--json`)

### 7.2 Command Surface Map

Every interactive command delegates to a Ratatui TUI surface when running in a
TTY. Non-TTY environments (CI, pipes) receive plain text or JSON.

| Command | TUI Surface | Description |
|---------|-------------|-------------|
| `anvil init` | Init | Project initialisation wizard |
| `anvil watch` | Watch | Live file-watch dashboard (kernel events) *(Planned)* |
| `anvil gate` | Gate | Check explorer with pass/fail detail *(Planned)* |
| `anvil status` | Status | Project health overview |
| `anvil doctor [--fix]` | Doctor | Environment diagnostics |
| `anvil audit` | Audit | Repository scan results |
| `anvil tutorial [--reset]` | Tutorial | Guided learning paths |
| `anvil start` | Welcome | First-run quick-start menu |
| `anvil new` | Browser | Template catalogue browser |
| `anvil wizard` | Wizard | APS onboarding wizard |

Non-interactive commands (no TUI surface):

| Command | Description | Status |
|---------|-------------|--------|
| `anvil auth` | Authentication management | *(Planned)* |
| `anvil admin` | Beta user approval | *(Planned)* |
| `anvil policy` | Policy operations | *(Planned)* |
| `anvil architecture` | Architecture enforcement | *(Planned)* |
| `anvil hooks` | Git hook management | *(Planned)* |
| `anvil export` | Constraint export | *(Planned)* |

> **Note:** The commands above are defined in clap but their handlers are
> stubbed (`bail!("not yet implemented")`). They are listed here for surface
> completeness — do not use them in demos or scripts until implemented.

> **Note:** The Rust CLI currently exposes these as flat subcommands (e.g.,
> `anvil auth`, `anvil policy`). Nested subcommands (`anvil auth login`,
> `anvil policy list`, etc.) are planned but not yet wired in clap.

### 7.3 Output Conventions

- **Silence over noise.** Do not print `✅ Success!` — print `STATUS: OK [200ms]`.
- **No emojis.** Ever.
- **Exit codes are contracts:**
  - `0` — Success
  - `1` — General error
- **Structured output:** `--json` is supported on commands that have data to
  serialise (e.g., `status`, `doctor`). Interactive-only commands (`tutorial`,
  `welcome`) fall back to plain text or TUI and do not honour `--json`.
  CI consumers should verify `--json` support per command.

### 7.4 Demo Scenarios (Marketing / Video)

1. **Cold start:** `anvil init` → wizard flow → `.anvilrc` generated.
2. **Live governance:** `anvil watch` → make a violating change → real-time
   block with policy citation. *(Planned — `watch` not yet implemented.)*
3. **Gate check:** `anvil gate plan.aps.md` → pass/fail breakdown with
   colour-coded results. *(Planned — `gate` not yet implemented in the Rust
   CLI; use the TypeScript CLI or `anvil status` for current demos.)*
4. **Auth flow:** `anvil auth` → device code displayed → browser activation
   → token stored. *(Planned — `auth` is not yet implemented in the Rust CLI.)*

---

## 8. SURFACE 2: TUI (Ratatui)

### 8.1 Overview

The TUI is the richest interactive experience. It runs inside the CLI when a
TTY is detected. All 10 surfaces share the same theme, keyboard handling, and
layout shell.

### 8.2 Architecture & Layout

The shared shell (`crates/anvil-tui/src/shell.rs`) uses a minimal 3-part
vertical layout:

```
┌──────────────────────────────────────────────────────┐
│  Anvil > SurfaceName                                 │  ← Header (1 line)
├──────────────────────────────────────────────────────│
│                                                      │
│  Surface content area                                │
│  (each surface renders its own internal panels)      │  ← Content (flexible)
│                                                      │
│                                                      │
├──────────────────────────────────────────────────────│
│  q: quit  ?: help                                    │  ← Footer (1 line)
└──────────────────────────────────────────────────────┘
```

- **Header:** Fixed `Constraint::Length(1)`. Shows `Anvil > SurfaceName` with
  the surface name in `--text-primary` and `>` separator in `--text-muted`.
- **Content:** `Constraint::Min(1)`. Each surface owns its internal layout
  (panels, splits, etc.).
- **Footer:** Fixed `Constraint::Length(1)`. Contextual help text in
  `--text-muted`.

### 8.3 The 10 Surfaces

| Surface | Purpose | Key Visual Elements |
|---------|---------|-------------------|
| **Welcome** | First-run menu | Action list, version info, quick-start options |
| **Tutorial** | Guided paths | Step indicators, progress tracking, instruction panes |
| **Doctor** | Diagnostics | Check list with pass/fail/warn badges, detail expand |
| **Status** | Project overview | Policy summary, check counts, file stats |
| **Gate** | Check explorer | Check tree with expandable detail, severity badges |
| **Watch** | Live dashboard | Real-time event stream from kernel, file change log |
| **Init** | Setup wizard | Multi-step form, config preview, file generation |
| **Wizard** | APS onboarding | Module selection, dependency graph, plan generation |
| **Audit** | Scan results | Finding list, severity breakdown, remediation hints |
| **Browser** | Template catalogue | Searchable list, template preview, scaffold action |

### 8.4 Rendering Rules

- **Borders:** `BorderType::Plain` only. No rounded corners. Ever.
- **Spacing:** Generous and deliberate. Left-align with hardcoded spaces, not
  terminal padding.
- **Colours:** Use `EddaTheme` constants exclusively. No ANSI or Tailwind
  terminal colours.
- **ASCII art:** Only the provided block logos. No arbitrary art.
- **Lists/logs:** Deep, deliberate indentation to maintain the Brutalist grid.

### 8.5 Demo Scenarios (Marketing / Video)

1. **Watch surface:** Split-screen showing policy on left, live events on right
   as files change — the "flight recorder" experience. *(Planned — not yet
   implemented.)*
2. **Gate surface:** Expanding a failed check to see the policy rule, the
   violation, and the file location.
3. **Doctor surface:** Running `anvil doctor --fix` — fixes are applied before
   the TUI launches, then the surface shows the updated check results.
4. **Tutorial surface:** Walking through a learning path with step-by-step
   guidance.

---

## 9. SURFACE 3: WEB DASHBOARD (Next.js)

### 9.1 Overview

The dashboard is the strategic layer — it surfaces the same data the CLI
produces, but for sustained observation. It runs as a local dev tool, reading
from each engineer's `.anvil/` workspace storage.

- **Stack:** Next.js 16, React, Tailwind CSS 4, shadcn/ui, Recharts, TanStack
  Query
- **Route:** `/dashboard` (route group `(dashboard)`)
- **Deployment:** Local dev server (reads from engineer's `.anvil/` workspace)

### 9.2 View Modules *(Planned)*

> **Note:** The dashboard route group is not yet implemented. All routes below
> are planned — see DASH-001 in `plans/modules/dashboard-foundation.aps.md`.

#### Core Views (DASHCORE) *(Planned)*

| Route | Purpose |
|-------|---------|
| `/dashboard` | Overview — KPIs, status summary, recent activity |
| `/dashboard/gates` | Gate results list with filtering |
| `/dashboard/gates/[id]` | Individual gate run detail |
| `/dashboard/warnings` | Warning aggregation |
| `/dashboard/warnings/breakdown` | Warning breakdown by category |
| `/dashboard/warnings/patterns` | Warning pattern analysis |

#### Architecture Views (DASHARCH) *(Planned)*

| Route | Purpose |
|-------|---------|
| `/dashboard/architecture` | Architecture overview |
| `/dashboard/architecture/violations` | Violation list |
| `/dashboard/architecture/graph` | Dependency graph visualisation |
| `/dashboard/drift` | Drift snapshot list |
| `/dashboard/drift/[name]` | Individual snapshot detail |
| `/dashboard/drift/compare` | Snapshot comparison |
| `/dashboard/suppressions` | Suppression management |
| `/dashboard/suppressions/trends` | Suppression trend analysis |

#### Ops Views (DASHOPS) *(Planned)*

| Route | Purpose |
|-------|---------|
| `/dashboard/audit` | Audit log |
| `/dashboard/audit/users` | Per-user audit trail |
| `/dashboard/audit/ai-tools` | AI tool usage tracking |
| `/dashboard/plans` | APS plan browser |
| `/dashboard/plans/[id]` | Plan detail |
| `/dashboard/config` | Configuration viewer |
| `/dashboard/diagnostics` | System diagnostics |

#### AI Builder Views (DASHAI) *(Planned)*

| Route | Purpose |
|-------|---------|
| `/dashboard/builder` | Custom dashboard builder |
| `/dashboard/builder/templates` | Dashboard templates |
| `/dashboard/dashboards` | Saved dashboards |
| `/dashboard/dashboards/[id]` | Individual dashboard |

### 9.3 Component Library

Shared dashboard components (all in `components/dashboard/`):

| Component | Purpose |
|-----------|---------|
| `sidebar.tsx` | Navigation sidebar |
| `top-bar.tsx` | Header with breadcrumbs |
| `dashboard-shell.tsx` | Layout wrapper |
| `metric-card.tsx` | KPI display card |
| `data-table.tsx` | Sortable, filterable table |
| `status-badge.tsx` | Pass/fail/warn status indicator |
| `severity-badge.tsx` | Severity level indicator |
| `code-block.tsx` | Code display with syntax highlighting |
| `empty-state.tsx` | Zero-data placeholder |
| `loading-skeleton.tsx` | Loading state placeholder |
| `command-palette.tsx` | `⌘K` quick navigation |
| `charts/` | Recharts-based visualisations |

### 9.4 Visual Rules

- **Background:** `--void` (`#0D0D0F`). Never white. Never light mode.
- **Cards:** 1px `--structure` border. No drop shadows. No rounded corners
  (override all shadcn defaults to `rounded-none`).
- **Tables:** `--structure` grid lines. Row hover in subtle `--structure`
  lighten.
- **Charts:** Use only the five core tokens. Desaturated palette for secondary
  series.
- **Typography:** JetBrains Mono for headers and KPIs. Inter for table content
  and descriptions.
- **Empty states:** Bracket-syntax icons. Declarative copy ("No gate results
  found." not "Oops! Nothing here yet 🎉").

### 9.5 Interactive Patterns

- **Command Palette:** `⌘K` / `Ctrl+K` opens quick navigation. Same feel as
  the CLI — type to filter, enter to navigate.
- **Deep Linking:** Every view supports URL-based state (filters, selections,
  expanded rows) for shareable links.
- **Data Hooks:** TanStack Query with `use-status`, `use-gates`, `use-warnings`,
  `use-drift`, etc. The dashboard runs as a local dev tool and reads from the
  engineer's `.anvil/` storage via localhost API routes (see
  `plans/modules/dashboard-foundation.aps.md`). It is not a hosted Vercel
  deployment for team-wide visibility — each engineer runs their own instance.

### 9.6 Demo Scenarios (Marketing / Video) *(Planned)*

> **Note:** All dashboard demo scenarios below require the `/dashboard` route
> group, which is not yet implemented (see DASH-001). Use staging data or
> mockups for pre-implementation marketing material.

1. **Dashboard overview:** Landing page with KPI cards showing gate pass rate,
   warning count, drift status.
2. **Gate drill-down:** Click a failed gate → see check-by-check breakdown →
   click a check → see the policy rule and code location.
3. **Architecture graph:** Interactive dependency visualisation with violation
   highlighting.
4. **Drift comparison:** Side-by-side snapshot diff showing what changed between
   two points in time.
5. **Command palette:** `⌘K` → type "gate" → instant navigation.

---

## 10. SURFACE 4: DOCUMENTATION SITE (Docusaurus)

### 10.1 Overview

- **Framework:** Docusaurus / Vercel
- **Content:** `docs/public/` (quickstarts, tutorials, concepts, guides)
- **Audience:** All users — from first-time onboarding to deep reference

### 10.2 Visual Rules

- **Dark theme preferred.** The default is dark (`--void` background), but the
  deployed Docusaurus config currently allows theme switching
  (`disableSwitch: false`, `respectPrefersColorScheme: true`). A future update
  should enforce dark-only to match the brand spec.
- **Code blocks:** JetBrains Mono. Syntax highlighting uses brand palette
  (`--anvil-ember` for keywords, `--edda-growth` for strings, `--text-muted`
  for comments).
- **Navigation:** Sidebar with `--structure` borders. Active item highlighted
  with `--anvil-ember`.
- **Headers:** JetBrains Mono. Body: Inter.
- **Diagrams:** White/ember lines on dark background. Mermaid diagrams preferred.

### 10.3 Content Tone

- **Declarative, not persuasive.** "Anvil validates your workflow." not "Anvil
  will supercharge your development process!"
- **Show the terminal.** Every concept page includes a CLI example with real
  output.
- **Code-first.** Lead with the code snippet, then explain. Engineers stop
  scrolling for code.

### 10.4 Demo Scenarios (Marketing / Video)

1. **Quickstart:** Walk through `docs/public/anvil/quickstart.md` (served at
   `/anvil/quickstart`) — from install to first gate check in under 5 minutes.
   *(Note: the quickstart currently documents the TypeScript CLI surface —
   commands like `anvil login`, `anvil check`, and `anvil watch` are not
   available in the Rust CLI. Update the quickstart before using it in Rust CLI
   demos.)*
2. **Concept deep-dive:** Show a concept page with embedded CLI output and
   architecture diagrams.

---

## 11. SURFACE 5: VS CODE EXTENSION

### 11.1 Overview

- **Package:** `packages/vscode-extension/`
- **Purpose:** Real-time diagnostics integration — surfaces Anvil findings
  directly in the editor.

### 11.2 Visual Rules

- **Diagnostic colours:** Map to VS Code severity levels but use brand tokens
  where VS Code allows customisation.
  - Error → `--error` (`#C94A4A`)
  - Warning → `--warning` (`#D08C38`)
  - Information → `--anvil-ember` (`#CC5500`)
  - Hint → `--text-muted` (`#85858A`)
- **Status bar:** Display Anvil status using VS Code codicon strings. The
  deployed extension (`packages/vscode-extension/src/services/statusBar.ts`)
  uses: `$(shield) Anvil` (idle), `$(loading~spin) Anvil: Running gates...`
  (active), `$(check) Anvil: Passed` (success), `$(error) Anvil: Failed`
  (error), `$(warning) Anvil: Warning` (warning).
- **Tree views:** Use `--structure` for borders. No icons beyond VS Code
  defaults.

### 11.3 Demo Scenarios (Marketing / Video)

1. **Inline diagnostics:** Show a policy violation appearing as a squiggly
   underline in the editor with hover detail.
2. **Status bar:** The codicon status updating as files are saved (e.g.,
   `$(loading~spin)` → `$(check) Anvil: Passed`).

---

## 12. SURFACE 6: MCP SERVER

### 12.1 Overview

- **Package:** `packages/mcp-server/`
- **Protocol:** MCP (Model Context Protocol)
- **Purpose:** Expose Anvil tools and resources to AI agents (Claude, etc.)

### 12.2 Exposure

The MCP server is not visually rendered — it is an API surface. However, it is
customer-facing because AI agents present its output to users.

- **Tool responses:** Return structured JSON. Note that MCP tool responses and
  CLI `--json` output use different schemas today — for example, `anvil_status`
  returns `{status, workspaceRoot, availableChecks, config, hasBaseline,
  version}` while `anvil status --json` serialises `{hooks, profile,
  recent_runs}`. Consumers should not assume the two surfaces share a single
  contract; schema unification is a future goal.
- **Resource descriptions:** Use declarative, technical language. No marketing
  copy.
- **Error messages:** Return `isError: true` with a JSON text body shaped as
  `{"error": "<message>"}`. This matches the envelope used by `registerStatusTool()`
  and `registerGateTool()` in the deployed server — do not use CLI-style
  `STATUS: FAIL [reason]` strings.

### 12.3 Demo Scenarios (Marketing / Video)

1. **Agent integration:** Show Claude using the `anvil_gate` tool to run gate
   checks against the workspace (via `workspaceRoot` and optional `targetFiles`),
   receiving structured results, and presenting them to the user.
2. **Status check:** Show an agent calling the `anvil_status` tool to retrieve
   project health, then summarising the results for the user.

---

## 13. SUPPORTING ASSETS

### 13.1 Investor Decks (PPTX)

- **Background:** Always `--void` (`#0D0D0F`). No "white mode" slides. Ever.
- **Titles:** JetBrains Mono, UPPERCASE.
- **Imagery:** High-contrast terminal screenshots, architecture diagrams (white
  lines on dark), typography-only slides (big numbers). Never stock photos.
- **Grid:** Visible. 5% opacity grid on slide master.
- **Tool:** Generated via Node.js script using EddaCraft brand templates.

### 13.2 OG / Social Images

- **Container:** Terminal window floating on `--void`.
- **Content:** Code snippet or error log, not blog title.
- **Hook:** Engineers stop scrolling for code. They scroll past marketing
  headlines.

### 13.3 Transactional Email

- **Templates:** React Email (`packages/transactional/`)
- **Types:** Beta invite, OTP code, waitlist confirmation
- **Visual rules:** `--void` background, JetBrains Mono headers, Inter body,
  `--anvil-ember` for CTA buttons.

---

## 14. CROSS-SURFACE CONSISTENCY RULES

### 14.1 Data Contract Parity

All surfaces consume the same underlying data structures:

```
anvil-kernel (Rust)
    ├── CLI plain output → serialises kernel data as text
    ├── CLI JSON output  → serialises kernel data as JSON
    ├── TUI surfaces     → renders kernel data via Ratatui
    ├── Dashboard API    → reads .anvil/ storage (kernel output)
    ├── MCP server       → exposes kernel data as tools/resources
    └── VS Code ext      → reads kernel diagnostics
```

When data appears on one surface, it must be representable on every other
surface. The underlying data should be semantically equivalent, though the
serialisation schema may differ between surfaces today (see §12.2 for the
current CLI vs MCP divergence). Schema unification is a future goal.

### 14.2 Navigation Parity *(Target State)*

The CLI command structure will map to dashboard routes once both surfaces are
implemented. Today, only `anvil status` and `anvil doctor` are functional in the
Rust CLI; the remaining CLI commands and all dashboard routes are planned.

| CLI Command | Dashboard Route | TUI Surface | Status |
|-------------|----------------|-------------|--------|
| `anvil status` | `/dashboard` | Status | CLI: Live, Dashboard: Planned |
| `anvil gate` | `/dashboard/gates` | Gate | Both Planned |
| `anvil audit` | `/dashboard/audit` | Audit | Both Planned |
| `anvil watch` | — (real-time only) | Watch | Both Planned |
| `anvil doctor` | `/dashboard/diagnostics` | Doctor | CLI: Live, Dashboard: Planned |
| `anvil policy` | `/dashboard/config` | — | Both Planned |
| `anvil architecture` | `/dashboard/architecture` | — | Both Planned |

### 14.3 Terminology

Use consistent terminology across all surfaces:

| Term | Meaning | Do Not Use |
|------|---------|-----------|
| Gate | A policy check pass/fail | Test, scan, check |
| Check | An individual rule within a gate | Rule, test, assertion |
| Watch | Real-time file monitoring | Monitor, listen, observe |
| Drift | Configuration change over time | Deviation, delta, diff |
| Surface | A TUI view | Screen, page, panel |
| Ember | A candidate pattern for promotion | Suggestion, recommendation |
| Edda | Long-term memory store | History, log, archive |
| Kindling | Ephemeral observations | Events, signals, logs |

### 14.4 Silence Protocol

All surfaces follow the same silence protocol:

- Do not congratulate the user. Do not use "Great job!" or "Well done!".
- Do not use exclamation marks in status messages.
- Do not print unnecessary whitespace or decorative separators.
- Do not use loading spinners with cute messages ("Hang tight!", "Almost
  there!").
- Status messages are factual: `GATE: 12/14 PASSED [340ms]`.

---

## 15. DEMO & MARKETING VIDEO PLAYBOOK

### 15.1 Recording Environment

- **Terminal:** Alacritty or Kitty with JetBrains Mono, `--void` background.
- **Browser:** Dark mode, no bookmarks bar, minimal chrome.
- **Resolution:** 1920×1080 or 3840×2160 (4K).
- **Font size:** 14px terminal, 16px browser.

### 15.2 Recommended Demo Flow

1. **The Hook (30s):** Show `anvil watch` running. Make a bad change. Watch it
   get blocked in real-time. No narration needed — the terminal speaks.
   *(Planned — `watch` is not yet implemented; use `anvil status` for demos
   until the watch surface ships.)*

2. **The Setup (60s):** `anvil init` → wizard completes → `.anvilrc`
   generated. Show the config file briefly.

3. **The Core Loop (90s):**
   - `anvil gate plan.aps.md` → TUI shows check results. *(Planned — `gate`
     is not yet implemented in the Rust CLI.)*
   - Open dashboard → same data, richer visualisation. *(Planned — the
     `/dashboard` route group is not yet implemented; see DASH-001 in
     `plans/modules/dashboard-foundation.aps.md`.)*
   - Show architecture graph with a violation highlighted.

4. **The Stack (60s):**
   - `anvil status` → shows Kindling → Ember → Edda memory layers.
   - Demonstrate a pattern being observed, proposed, and promoted.

5. **The Integration (30s):**
   - VS Code showing inline diagnostics.
   - Claude using MCP tools to query Anvil.

### 15.3 B-Roll Shots

- The TUI Watch surface with events streaming (30s loop). *(Planned.)*
- Dashboard overview with KPI cards (static or slow scroll).
- `anvil doctor` fixing issues (checks flipping green).
- Architecture graph rotating/zooming.
- Terminal showing `anvil auth` → device code flow. *(Planned — `auth` not
  yet implemented.)*

### 15.4 What NOT to Show

- Light-mode anything.
- Rounded corners or gradients anywhere.
- Stock photography or generic illustrations.
- Excited copy or marketing superlatives.
- Incomplete or buggy surfaces (use staging data if needed).

---

## 16. INSTRUCTIONS FOR CLAUDE

When working on any EddaCraft customer-facing surface:

- Adhere strictly to the colour token system. Never introduce arbitrary colours.
- Never use rounded corners in TUI or dashboard components.
- Never introduce ASCII art beyond the provided block logos.
- Match the silence protocol — no congratulatory or excited copy.
- Use UK English spelling in all code, comments, and user-facing text.
- Ensure data contract parity — if you add a field to one surface, consider
  whether it should appear on others.
- When creating demo content or screenshots, use the recording environment
  settings specified above.
- Prefer bracket-syntax iconography over metaphorical icons.
- All lists and log outputs must use deep, deliberate indentation.
- Assume all code is going into a professional, heavy-duty environment.
