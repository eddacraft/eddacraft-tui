# Rewritten Product Narrative (Top of README / Landing Page)

## Current Implicit Narrative (Problematic)

"Anvil is a deterministic governance layer for AI-assisted development using plans, gates, and policies."

This is true, but it's not why anyone installs it.

## Proposed Narrative

### Headline

Ship AI-generated code with confidence.

### Subheading

Anvil is a developer-first safety layer that helps you use AI at full speed without breaking your system's architecture or intent.

### Core Paragraph

AI tools generate code faster than humans can review it.
The problem isn’t syntax or tests it’s subtle architectural drift, bad patterns, and “technically valid but wrong” decisions that slip through unnoticed.

Anvil sits between AI and production.
It understands your system’s structure, watches for dangerous patterns, and warns you at the moment you’re about to make a mistake — before it becomes a rewrite, an incident, or a blame game.

### What Anvil Is

- A trust broker between AI and humans
- A real-time safety net for AI-generated code
- A way to move faster without losing architectural integrity

### What Anvil Is Not

- Not another linter
- Not a CI rules engine
- Not a documentation replacement
- Not a process framework you must adopt to get value

### Primary Promise

If AI writes something that looks right but is wrong for your system, Anvil will catch it... early, clearly, and with context.

This framing should sit above any mention of plans, policies, or gates.

## v1 Feature Cut (Ruthless and Focused)

### v1 Goal (Single Sentence)

Give individual developers confidence to merge AI-generated code without introducing architectural drift or toxic patterns.

### v1 MUST HAVE (Ship These)

#### Local, Save-Time Warnings

- File-save detection in IDE / CLI
- Near-real-time feedback
- Same warnings later mirrored in PRs if needed

#### Architecture Boundary Detection (Baseline)

- Infer structure from codebase
- Detect new cross-boundary calls
- Warn only on new violations
- Explicitly acknowledge existing drift

#### AI Anti-Pattern Detection

High-confidence warnings for:

- `eslint-disable` / broad suppressions
- `any` / unsafe type escapes
- Known AI "make it pass" tactics

Each warning includes:

- Explanation
- Concrete suggestion (from built-in library)

#### Named Pattern / Rule References

Warnings reference:

> "This system uses bounded contexts"

> "Payments must not depend on Identity"

Not raw policy names.

#### Explicit Suppressions with Provenance

- Inline annotation in code
- Required human note
- Captured in provenance metadata

## v1 NICE TO HAVE (But Not Blockers)

- Basic PR comments
- Simple CLI summary output
- Minimal drift snapshot (counts, not history)

## v1 NOT INCLUDED (Explicitly Defer)

- Plan-first workflows
- Heavy CI gating
- Full OPA authoring experience
- Advanced architecture modelling
- AI auto-fixing code

This keeps v1 sharp and lovable.

## Keep / Reshape / Defer Map (Existing Features)

### ✅ KEEP (As-Is or Lightly Refined)

- Deterministic execution core
- Dependency analysis foundations
- CLI execution pipeline
- Basic CI integration
- Core violation normalisation

These are solid foundations.

### 🔄 RESHAPE (Conceptually Right, But Mispositioned)
#### Plans / APS
- Reshape as optional accelerator
- Move out of the primary narrative
- Introduce later as “lock intent once you trust Anvil”

#### OPA / Policies
- Keep internally
- Expose via:
  - Named patterns
  - Rules tied to boundaries
- Hide policy mechanics by default

#### Gate Checks (Lint, Tests, Coverage)
- De-emphasise
- Treat as supporting signals
- Not a headline feature

### ⏸️ DEFER (Important, But Later)
- Full drift history & trends
- Advanced architecture modelling
- SARIF export
- Organisation-wide governance dashboards
- AI-collaborative refactoring loops

These are Phase 2 features.

## The Local Warning UX (The Heart of Anvil)

This is where Anvil either earns love or gets ignored.

### Scenario: AI Just Wrote Code, Developer Hits Save

#### What Appears (in IDE / CLI)

##### ⚠️ Architectural Boundary Warning

**Title**

Architectural boundary crossed

**Primary line (first thing they see)**

Payments is calling into Identity

**Context**

This system uses bounded contexts.
Payments must not depend on Identity.

**Impact (product-oriented)**

This change appears to affect the Checkout journey
(confidence: medium)

**State**

This boundary is already violated elsewhere.
This change introduces a new dependency.

**Suggestion**

Consider moving this logic into a shared service or exposing it via an API boundary.

**Actions**

- View boundary map
- Suppress with note

##### ⚠️ AI Anti-Pattern Warning

**Title**

AI anti-pattern detected

**Primary line**

Broad `eslint-disable` added

**Explanation**

AI tools often use this to bypass constraints instead of fixing the root cause.

**Suggestion**

Narrow the rule scope or refactor the offending code path.

**Actions**

- Apply suggestion
- Suppress with note

#### Suppression Flow (Intentional, Visible)

When suppressing:

- Developer must add a note
- Note appears inline in code
- Anvil records it as provenance

This is how trust is maintained.

## Where We Are Now

You now have:

- A clear product story
- A sharp v1 scope
- A rationalised feature set
- A designed core UX

Nothing here contradicts what you’ve already built — it focuses it.

## Next Possible Moves (Pick One)

1. Rewrite the actual README using this narrative
2. Turn the v1 cut into an executable roadmap
3. Design `anvil init` and architecture inference UX
4. Map this UX into a VS Code extension first
5. Pressure-test this against a real AI-generated PR
